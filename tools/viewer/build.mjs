// Build one self-contained HTML page: three.js and the viewer modules inlined (as blob modules), the replay, the rig, and LOOK's
// shader chunk and baked schemes embedded. No network at run time.
//   node build.mjs --rig rig.json[,rig2.json,...] (one rig per vehicle, the last repeats) --replay replay.w5kr --out page.html   (the replay is embedded as base64 of the binary file)
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const here = path.dirname(fileURLToPath(import.meta.url)), repo = path.join(here, '../..');
const arg = (k) => process.argv[process.argv.indexOf('--' + k) + 1];
const read = (p) => fs.readFileSync(p, 'utf8');
const j = (s) => JSON.stringify(s).replace(/</g, '\\u003c');

// `--live` builds the test-drive page instead: no replay and no rig inside, it fetches those from `w5k drive` (same origin) at run time.
//   node build.mjs --live [--out dist/index.html] [--check]     (--check: fail if the committed file is not what this would write)
// `--workshop` builds the Workshop page (served by `w5k viewer workshop`, which sends the rigs and skins): `node build.mjs --workshop`.
const workshop = process.argv.includes('--workshop');
const live = process.argv.includes('--live') || workshop;
// Modules in dependency order; `from 'three'` and `from './x.js'` are rewritten to blob URLs at run time.
const order = workshop ? ['world', 'viewer', 'look', 'skin', 'workshop'] : live ? ['world', 'viewer', 'look', 'skin', 'live-audio', 'live-input', 'live'] : ['replay', 'scope', 'world', 'viewer', 'look', 'debug', 'main'];
const sources = { three: read(path.join(here, 'node_modules/three/build/three.module.min.js')) };
for (const m of order) sources[m] = read(path.join(here, `src/${m}.js`));

const camoDir = path.join(repo, 'assets/materials/camo');
const schemes = {};
for (const f of fs.readdirSync(path.join(camoDir, 'baked')).sort()) schemes[f.replace('.json', '')] = JSON.parse(read(path.join(camoDir, 'baked', f)));
const look = {
  glsl: read(path.join(repo, 'assets/shaders/glsl/w5k_camo.glsl')),
  schemes,
  weathering: JSON.parse(read(path.join(repo, 'assets/materials/baked/weathering.json'))),
  cdf: JSON.parse(read(path.join(camoDir, 'cdf.json')))['o3_l2_g0.5'],
};
const terrain = !live && process.argv.includes('--terrain') ? JSON.parse(read(arg('terrain'))) : null; // heightfield from `w5k viewer render --strip`
// the skins the page may ask for: whatever `dist/skins/*.skin` holds when it is built (so it never probes for a file that is not there)
const skins = live && !workshop && fs.existsSync(path.join(here, 'dist/skins')) ? fs.readdirSync(path.join(here, 'dist/skins')).filter((f) => f.endsWith('.skin')).map((f) => f.replace(/\.skin$/, '')).sort() : [];
const data = workshop ? { look } : live ? { look, skins } : { terrain, rigs: arg('rig').split(',').map((f) => JSON.parse(read(f))), replay: fs.readFileSync(arg('replay')).toString('base64'), look };

const boot = `
const blob = (s) => URL.createObjectURL(new Blob([s], { type: 'text/javascript' }));
const src = ${j(sources)};
const url = { three: blob(src.three) };
for (const m of ${j(order)}) {
  url[m] = blob(src[m].replace(/from '(three|\\.\\/([\\w-]+)\\.js)'/g, (_, a, b) => "from '" + url[b ?? a] + "'"));
}
await import(url.${workshop ? 'workshop' : live ? 'live' : 'main'});`;
const html = read(path.join(here, workshop ? 'src/workshop.html' : live ? 'src/live.html' : 'src/page.html')).replace('__DATA__', () => j(data)).replace('__BOOT__', () => boot);
const out = process.argv.includes('--out') ? arg('out') : path.join(here, workshop ? 'dist/workshop.html' : 'dist/index.html');
if (process.argv.includes('--check')) {
  if (!fs.existsSync(out) || read(out) !== html) { console.error(`${out} is not up to date: run node tools/viewer/build.mjs ${workshop ? '--workshop' : '--live'}`); process.exit(1); }
  console.log(`${out} is up to date`);
} else {
  fs.mkdirSync(path.dirname(out), { recursive: true });
  fs.writeFileSync(out, html);
  console.log(`wrote ${out} (${(html.length / 1e6).toFixed(2)} MB)`);
}
