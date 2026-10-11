// The replay page's track footprint overlay on a real recording: the carrier driving into the slice course's mud crossing at 48 km/h
// (tools/viewer/samples/carrier-mud.w5kr with its rig; FORGE's rig, TRACKS's gear, recorded by `w5k drive --record`). Headless Chromium, software GL.
//   node tools/viewer/footprint-smoke.mjs [out-dir]
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { decodeReplay } from './src/replay.js';
const { chromium } = createRequire((process.env.PLAYWRIGHT_DIR ?? '/opt/node22/lib/node_modules') + '/')('playwright');
const here = path.dirname(fileURLToPath(import.meta.url)), out = process.argv[2] ?? 'out/footprint';
fs.mkdirSync(out, { recursive: true });
const pageFile = path.resolve(out, 'page.html');
execFileSync('node', [path.join(here, 'build.mjs'), '--rig', path.join(here, 'samples/carrier-rig.json'), '--replay', path.join(here, 'samples/carrier-mud.w5kr'), '--out', pageFile], { stdio: 'inherit' });
const replay = decodeReplay(new Uint8Array(fs.readFileSync(path.join(here, 'samples/carrier-mud.w5kr'))));
const frameAt = (t) => replay.frames[Math.round(t / replay.header.frame_dt_s)].vehicles[0];

const browser = await chromium.launch({ executablePath: process.env.CHROME, args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const page = await browser.newPage({ viewport: { width: 1100, height: 560 } });
const errors = [];
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
page.on('pageerror', (e) => errors.push(String(e)));
let failed = false;
const check = (ok, msg) => { console.log(`${ok ? 'PASS' : 'FAIL'}  ${msg}`); failed ||= !ok; };
await page.goto('file://' + pageFile + '?static');
await page.waitForFunction('window.__ready === true', null, { timeout: 60000 });
await page.evaluate(() => { window.__v.pause(); window.__v.setFrameRate(30); window.__v.setLayout('off'); });
const luminance = (hex) => 0.2126 * ((hex >> 16) & 255) + 0.7152 * ((hex >> 8) & 255) + 0.0722 * (hex & 255);
const at = async (t) => { await page.evaluate((tt) => window.__v.renderAt(tt), t); return { tracks: await page.evaluate(() => window.__v.footprints()), hud: await page.evaluate(() => document.getElementById('hud').textContent) }; };

// on the road, before the mud: a ribbon for every sample of both tracks, all the colour of no sinkage
const road = await at(10);
check(road.tracks.length === 2 && road.tracks.every((t) => t.ribbons.length === 12 && t.ribbons.every((r) => r.visible)), 'two tracks of twelve ribbons, every sample in contact on the road');
check(road.tracks.every((t) => t.ribbons.every((r) => r.colour === road.tracks[0].ribbons[0].colour)) && /track_l: sinkage max 0 mm/.test(road.hud) && /track_r: sinkage max 0 mm/.test(road.hud), `on firm ground every ribbon is the pale colour and the HUD says sinkage 0 (${road.hud.match(/track_l:.*/)?.[0]})`);
// the ribbons lie along the belt, front last: cell 0 is behind cell 11, and the cells are evenly spaced (a cell is its share of the footprint's length; a tilted footprint is a little longer than its z span)
const z = road.tracks[0].ribbons.map((r) => r.pos[2]);
check(z.every((v, k) => k === 0 || v < z[k - 1]) && Math.abs(((z[0] - z[11]) / 11) / (road.tracks[0].ribbons[0].scale[1] / 0.94) - 1) < 0.01, `the cells are evenly spaced from the rear to the front: ${z[0].toFixed(2)} m to ${z[11].toFixed(2)} m`);

// in the mud: the deepest cell is the darkest ribbon, and the HUD numbers are the recording's
const t = 22.07, mud = await at(t), v = frameAt(t);
const right = v.contacts.slice(12, 24).map((c) => c.sinkage_m), deepest = right.indexOf(Math.max(...right)), shade = mud.tracks[1].ribbons.map((r) => luminance(r.colour));
check(shade.indexOf(Math.min(...shade)) === deepest && luminance(mud.tracks[1].ribbons[deepest].colour) < luminance(road.tracks[0].ribbons[0].colour) - 60, `the_deepest_cell_is_the_darkest_ribbon: cell ${deepest} sunk ${(right[deepest] * 1000).toFixed(0)} mm`);
const maxMm = Math.round(Math.max(...right) * 1000), hudMm = +mud.hud.match(/track_r: sinkage max (\d+) mm/)[1];
check(hudMm === maxMm, `the HUD's deepest sinkage is the recording's: ${hudMm} mm = ${maxMm} mm`);
const width = 0.38, length = (2 * 0.28 + 2.4) / 12, pMax = Math.max(...v.contacts.slice(12, 24).map((c) => c.normal_force_n)) / (width * length) / 1000, hudKpa = +mud.hud.match(/track_r: .*pressure max (\d+) kPa/)[1];
check(Math.abs(hudKpa - pMax) / pMax < 0.03, `ground pressure is force over the cell's footprint: the HUD says ${hudKpa} kPa for ${pMax.toFixed(0)} kPa from the recording (a ${(width * 1000).toFixed(0)} mm belt, ${(length * 1000).toFixed(0)} mm cells)`);
await page.evaluate(() => window.__v.setCamera({ mode: 'orbit', yaw: 1.45, pitch: 0.62, dist: 8 }));
await page.screenshot({ path: `${out}/footprint-mud.png` });

// the toggle hides the ribbons
await page.evaluate(() => document.getElementById('t_footprint').click()); // the panel is hidden in this layout, a click from the page still works
const off = await at(t);
check(off.tracks.every((tr) => tr.ribbons.every((r) => !r.visible)), 'the footprint toggle hides the ribbons');
check(errors.length === 0, `no console errors${errors.length ? ': ' + errors.join(' | ') : ''}`);
await browser.close();
console.log(failed ? 'FAILED' : 'all passed');
process.exit(failed ? 1 : 0);
