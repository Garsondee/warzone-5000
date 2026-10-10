// Spike S-L harness: python3 -I ref.py data -> params.json; serve; headless Chromium (SwiftShader); write tile / hash / block outputs.
import http from 'http'; import fs from 'fs'; import path from 'path'; import { createRequire } from 'module';
const require = createRequire('/opt/node-tools/node_modules/'); const { chromium } = require('playwright');
const here = path.dirname(new URL(import.meta.url).pathname), three = process.env.THREE_DIR;
const srv = http.createServer((q, s) => { const u = q.url.split('?')[0]; const f = u.startsWith('/three/') ? path.join(three, u.slice(7)) : path.join(here, u === '/' ? 'harness.html' : u);
  fs.readFile(f, (e, b) => { if (e) { s.writeHead(404); s.end(); } else { s.writeHead(200, { 'content-type': f.endsWith('.js') ? 'text/javascript' : f.endsWith('.json') ? 'application/json' : 'text/html' }); s.end(b); } }); }).listen(0);
const port = srv.address().port; const out = process.argv[2];
const br = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const pg = await br.newPage(); pg.on('console', m => console.log('[page]', m.text())); pg.on('pageerror', e => console.log('[err]', e.message));
await pg.goto(`http://localhost:${port}/`); await pg.waitForFunction('window.ready', null, { timeout: 30000 });
const res = {};
res.hash = await pg.evaluate(() => window.run.hash());
res.tile = await pg.evaluate(() => window.run.tile(256, 0.0));
res.tileFar = await pg.evaluate(() => window.run.tile(256, 4.0 / 256));
for (const [k, camo] of [['plain', false], ['camo', true]]) { const r = await pg.evaluate(([c]) => window.run.block(640, 360, c, 20, 0.5), [camo]); res['ms_' + k] = r.ms; fs.writeFileSync(`${out}/block_${k}.png`, Buffer.from(r.png.split(',')[1], 'base64')); }
fs.writeFileSync(`${out}/gpu.json`, JSON.stringify(res)); console.log('ms plain', res.ms_plain.toFixed(1), 'camo', res.ms_camo.toFixed(1));
await br.close(); srv.close();
