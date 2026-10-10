// Build one self-contained HTML page: three.js inlined (as a blob module), the viewer source inlined, the replay and rig embedded.
//   node build.mjs --rig rig.json --replay replay.json --out page.html
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const here = path.dirname(fileURLToPath(import.meta.url));
const arg = (k) => process.argv[process.argv.indexOf('--' + k) + 1];
const three = fs.readFileSync(path.join(here, 'node_modules/three/build/three.module.min.js'), 'utf8');
const core = fs.readFileSync(path.join(here, 'src/viewer.js'), 'utf8').replace("from 'three'", "from '__three__'");
const main = fs.readFileSync(path.join(here, 'src/main.js'), 'utf8').replace("from './viewer.js'", "from '__viewer__'").replace("from 'three'", "from '__three__'");
const rig = fs.readFileSync(arg('rig'), 'utf8'), replay = fs.readFileSync(arg('replay'), 'utf8');
const j = (s) => JSON.stringify(s).replace(/</g, '\\u003c');
const html = `<!doctype html><meta charset="utf-8"><title>W5K viewer</title>
<style>html,body{margin:0;background:#111;overflow:hidden}canvas{display:block;width:100vw;height:100vh}</style>
<canvas id="c"></canvas>
<script id="data" type="application/json">${j({ rig: JSON.parse(rig), replay: JSON.parse(replay) })}</script>
<script type="module">
const blob = (s) => URL.createObjectURL(new Blob([s], { type: 'text/javascript' }));
const three = blob(${j(three)});
const viewer = blob(${j(core)}.replace('__three__', three));
const main = ${j(main)}.replace('__viewer__', viewer).replace('__three__', three);
await import(blob(main));
</script>`;
fs.writeFileSync(arg('out'), html);
console.log(`wrote ${arg('out')} (${(html.length / 1e6).toFixed(2)} MB)`);
