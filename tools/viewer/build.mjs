// Build one self-contained HTML page: three.js and the viewer modules inlined (as blob modules), the replay, the rig, and LOOK's
// shader chunk and baked schemes embedded. No network at run time.
//   node build.mjs --rig rig.json --replay replay.w5kr --out page.html   (the replay is embedded as base64 of the binary file)
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const here = path.dirname(fileURLToPath(import.meta.url)), repo = path.join(here, '../..');
const arg = (k) => process.argv[process.argv.indexOf('--' + k) + 1];
const read = (p) => fs.readFileSync(p, 'utf8');
const j = (s) => JSON.stringify(s).replace(/</g, '\\u003c');

// Modules in dependency order; `from 'three'` and `from './x.js'` are rewritten to blob URLs at run time.
const order = ['replay', 'scope', 'viewer', 'look', 'debug', 'main'];
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
const terrain = process.argv.includes('--terrain') ? JSON.parse(read(arg('terrain'))) : null; // heightfield from `w5k viewer render --strip`
const data = { terrain, rig: JSON.parse(read(arg('rig'))), replay: fs.readFileSync(arg('replay')).toString('base64'), look };

const boot = `
const blob = (s) => URL.createObjectURL(new Blob([s], { type: 'text/javascript' }));
const src = ${j(sources)};
const url = { three: blob(src.three) };
for (const m of ${j(order)}) {
  url[m] = blob(src[m].replace(/from '(three|\\.\\/(\\w+)\\.js)'/g, (_, a, b) => "from '" + url[b ?? a] + "'"));
}
await import(url.main);`;
const html = read(path.join(here, 'src/page.html')).replace('__DATA__', () => j(data)).replace('__BOOT__', () => boot);
fs.writeFileSync(arg('out'), html);
console.log(`wrote ${arg('out')} (${(html.length / 1e6).toFixed(2)} MB)`);
