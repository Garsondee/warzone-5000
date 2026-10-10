// Build one self-contained HTML page: three.js inlined (as a blob module), the viewer source inlined, the replay and rig embedded.
//   node build.mjs --rig rig.json --replay replay.w5kr --out page.html   (the replay is embedded as base64 of the binary file)
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const here = path.dirname(fileURLToPath(import.meta.url));
const arg = (k) => process.argv[process.argv.indexOf('--' + k) + 1];
const three = fs.readFileSync(path.join(here, 'node_modules/three/build/three.module.min.js'), 'utf8');
const core = fs.readFileSync(path.join(here, 'src/viewer.js'), 'utf8').replace("from 'three'", "from '__three__'");
const main = fs.readFileSync(path.join(here, 'src/main.js'), 'utf8').replace("from './viewer.js'", "from '__viewer__'").replace("from './replay.js'", "from '__replay__'").replace("from 'three'", "from '__three__'");
const decoder = fs.readFileSync(path.join(here, 'src/replay.js'), 'utf8');
const rig = fs.readFileSync(arg('rig'), 'utf8'), replay = fs.readFileSync(arg('replay')).toString('base64');
const j = (s) => JSON.stringify(s).replace(/</g, '\\u003c');
const html = `<!doctype html><meta charset="utf-8"><title>W5K viewer</title>
<style>html,body{margin:0;background:#111;overflow:hidden}canvas{display:block;width:100vw;height:100vh}#ui{position:fixed;left:8px;right:8px;bottom:8px;display:flex;gap:8px;align-items:center;color:#eee;font:13px system-ui;background:#0008;padding:6px 8px;border-radius:6px}#scrub{flex:1}</style>
<canvas id="c"></canvas>
<div id="ui"><button id="play">pause</button> <input id="scrub" type="range" min="0" max="1" step="0.0005" value="0"> <span id="clock"></span>
 <select id="speed"><option>0.25</option><option selected>1</option><option>2</option><option>4</option></select>x
 <select id="cam"><option value="orbit">orbit</option><option value="chase">chase</option></select></div>
<script id="data" type="application/json">${j({ rig: JSON.parse(rig), replay })}</script>
<script type="module">
const blob = (s) => URL.createObjectURL(new Blob([s], { type: 'text/javascript' }));
const three = blob(${j(three)});
const viewer = blob(${j(core)}.replace('__three__', three));
const replay = blob(${j(decoder)});
const main = ${j(main)}.replace('__viewer__', viewer).replace('__replay__', replay).replace('__three__', three);
await import(blob(main));
</script>`;
fs.writeFileSync(arg('out'), html);
console.log(`wrote ${arg('out')} (${(html.length / 1e6).toFixed(2)} MB)`);
