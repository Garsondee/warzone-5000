// Smoke test in headless Chromium: no console errors, drawn triangles equal the rig's, and each articulated part moves pixels.
//   node smoke.mjs page.html part1,part2,...      (parts are node names; each is shown alone, hull frozen, at the times its joint differs most)
import { createRequire } from 'node:module';
import path from 'node:path';
const { chromium } = createRequire((process.env.PLAYWRIGHT_DIR ?? '/opt/node22/lib/node_modules') + '/')('playwright');
const [pageFile, partList] = process.argv.slice(2);
const parts = partList.split(',');
const browser = await chromium.launch({ executablePath: process.env.CHROME, args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const page = await browser.newPage({ viewport: { width: 640, height: 360 } });
const errors = [];
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
page.on('pageerror', (e) => errors.push(String(e)));
await page.goto('file://' + path.resolve(pageFile) + '?static');
await page.waitForFunction('window.__ready === true', null, { timeout: 60000 });
let failed = false;
const check = (ok, msg) => { console.log(`${ok ? 'PASS' : 'FAIL'}  ${msg}`); failed ||= !ok; };
const tri = await page.evaluate(() => [window.__v.triangles, window.__v.expectedTriangles]);
check(tri[0] === tri[1] && tri[0] > 0, `triangle count ${tri[0]} equals RenderRig::triangle_count() ${tri[1]}`);
await page.evaluate(() => { window.__v.setDebug(false); window.__v.setFreeze(true); window.__v.pause(); window.__v.setCamera({ mode: 'orbit', yaw: 0.6, pitch: 0.3, dist: 8 }); });
for (const part of parts) {
  const r = await page.evaluate((name) => {
    // Show the part's subtree alone, every other joint at its t=0 value, and move this joint alone: by the largest excursion
    // the replay makes for travel, steer, aim and recoil, and by half a radian for a spinning wheel (spin never returns).
    const v = window.__v, meshes = v.meshCount(name), j0 = v.jointAt(name, 0);
    let value = j0 + 0.5;
    if (!/wheel/.test(name)) { let best = 0; for (let t = 0; t <= v.duration; t += 0.25) { const d = v.jointAt(name, t) - j0; if (Math.abs(d) > Math.abs(best)) best = d; } value = j0 + best; }
    v.only([name]);
    // Close-up on the part, seen from both sides (a lug is on one face only) and from near and far (a barrel is long).
    let px = 0;
    for (const dist of [1.6, 4, 8]) for (const yaw of [0.6, 0.6 + Math.PI]) { v.setCamera({ focus: name, yaw, dist }); px = Math.max(px, v.jointPixels(name, value)); }
    v.setCamera({ focus: null, yaw: 0.6, dist: 8 });
    v.only([]);
    return { meshes, delta: value - j0, px };
  }, part);
  if (r.meshes === 0) { console.log(`SKIP  ${part}: the rig has no mesh under this node (stand-in geometry), nothing to see`); continue; }
  // A plain tube sliding along its axis only changes its silhouette at the ends, so recoil gets a lower bar (finding for GEOMETRY: add a muzzle brake).
  check(r.px >= (/recoil/.test(part) ? 5 : 20), `${part}: moving this joint alone by ${r.delta.toFixed(3)} changes ${r.px} pixels`);
}
check(errors.length === 0, `no console errors${errors.length ? ': ' + errors.join(' | ') : ''}`);
await browser.close();
process.exit(failed ? 1 : 0);
