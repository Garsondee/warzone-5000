// Build one self-contained HTML page: three.js inlined (as a blob module), the viewer source inlined, the replay and rig embedded.
//   node build.mjs --rig rig.json --replay replay.w5kr --out page.html   (the replay is embedded as base64 of the binary file)
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const here = path.dirname(fileURLToPath(import.meta.url));
const arg = (k) => process.argv[process.argv.indexOf('--' + k) + 1];
const three = fs.readFileSync(path.join(here, 'node_modules/three/build/three.module.min.js'), 'utf8');
const core = fs.readFileSync(path.join(here, 'src/viewer.js'), 'utf8').replace("from 'three'", "from '__three__'");
const main = fs.readFileSync(path.join(here, 'src/main.js'), 'utf8').replace("from './viewer.js'", "from '__viewer__'").replace("from './replay.js'", "from '__replay__'").replace("from './debug.js'", "from '__debug__'").replace("from './scope.js'", "from '__scope__'").replace("from 'three'", "from '__three__'");
const debugSrc = fs.readFileSync(path.join(here, 'src/debug.js'), 'utf8').replace("from 'three'", "from '__three__'");
const scopeSrc = fs.readFileSync(path.join(here, 'src/scope.js'), 'utf8');
const decoder = fs.readFileSync(path.join(here, 'src/replay.js'), 'utf8');
const rig = fs.readFileSync(arg('rig'), 'utf8'), replay = fs.readFileSync(arg('replay')).toString('base64');
const j = (s) => JSON.stringify(s).replace(/</g, '\\u003c');
const html = `<!doctype html><meta charset="utf-8"><title>W5K viewer</title>
<style>html,body{margin:0;background:#111;overflow:hidden}canvas{display:block;width:100vw;height:100vh}#ui{position:fixed;left:8px;right:8px;bottom:8px;display:flex;gap:8px;align-items:center;color:#eee;font:13px system-ui;background:#0008;padding:6px 8px;border-radius:6px}#scrub{flex:1}#hud{position:fixed;left:10px;top:8px;color:#fff;font:600 15px system-ui;text-shadow:0 1px 3px #000}#ledger{position:fixed;right:10px;top:8px;color:#fff;font:11px system-ui;text-shadow:0 1px 2px #000;width:230px}#ledger .row{display:flex;align-items:center;gap:6px;height:14px}#ledger span{width:90px;text-align:right}#ledger i{display:block;height:8px;background:#ffb347}#toggles{position:fixed;left:10px;top:32px;color:#eee;font:12px system-ui;text-shadow:0 1px 2px #000}#scope{position:fixed;left:8px;right:8px;bottom:50px;width:calc(100vw - 16px);height:150px;background:#0007;border-radius:6px}</style>
<canvas id="c"></canvas>
<div id="hud"></div><div id="ledger"></div>
<canvas id="scope" width="1200" height="180"></canvas>
<div id="toggles"><label><input type="checkbox" id="t_contacts" checked>contacts</label> <label><input type="checkbox" id="t_com" checked>datum</label> <label><input type="checkbox" id="t_velocity" checked>velocity</label></div>
<div id="ui"><button id="play">pause</button> <input id="scrub" type="range" min="0" max="1" step="0.0005" value="0"> <span id="clock"></span>
 <select id="speed"><option>0.25</option><option selected>1</option><option>2</option><option>4</option></select>x
 <select id="cam"><option value="orbit">orbit</option><option value="chase">chase</option></select></div>
<script id="data" type="application/json">${j({ rig: JSON.parse(rig), replay })}</script>
<script type="module">
const blob = (s) => URL.createObjectURL(new Blob([s], { type: 'text/javascript' }));
const three = blob(${j(three)});
const viewer = blob(${j(core)}.replace('__three__', three));
const replay = blob(${j(decoder)});
const debug = blob(${j(debugSrc)}.replace('__three__', three));
const scope = blob(${j(scopeSrc)});
const main = ${j(main)}.replace('__viewer__', viewer).replace('__replay__', replay).replace('__debug__', debug).replace('__scope__', scope).replace('__three__', three);
await import(blob(main));
</script>`;
fs.writeFileSync(arg('out'), html);
console.log(`wrote ${arg('out')} (${(html.length / 1e6).toFixed(2)} MB)`);
