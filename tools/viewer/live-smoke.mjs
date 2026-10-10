// End-to-end test of the test-drive page against a running `w5k drive` (headless Chromium, software GL):
//   w5k drive --vehicle content/vehicles/game/mule_4x4.ron --course content/world/courses/slice.ron --web tools/viewer/dist &
//   node tools/viewer/live-smoke.mjs [http://127.0.0.1:8787/] [out-dir]
import { createRequire } from 'node:module';
import fs from 'node:fs';
const { chromium } = createRequire((process.env.PLAYWRIGHT_DIR ?? '/opt/node22/lib/node_modules') + '/')('playwright');
const url = process.argv[2] ?? 'http://127.0.0.1:8787/', out = process.argv[3] ?? 'out/live';
fs.mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ executablePath: process.env.CHROME, args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist', '--autoplay-policy=no-user-gesture-required'] });
const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
const errors = [];
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
page.on('pageerror', (e) => errors.push(String(e)));
let failed = false;
const check = (ok, msg) => { console.log(`${ok ? 'PASS' : 'FAIL'}  ${msg}`); failed ||= !ok; };
const live = (fn, arg) => page.evaluate(fn, arg);
const until = (fn, timeout = 90000) => page.waitForFunction(fn, null, { timeout, polling: 200 });

await page.goto(url);
await until(() => window.__live && window.__live.vehicles().length === 3 && window.__live.thumbs() === 3);
check(true, `the page found the game: vehicles ${await live(() => window.__live.vehicles().join(', '))}`);
const cards = await page.$$eval('.card', (cs) => cs.map((c) => ({ name: c.textContent.trim(), w: c.querySelector('img').naturalWidth, box: c.getBoundingClientRect().toJSON() })));
check(cards.length === 3 && cards.every((c) => c.w > 0), `three picture cards with pictures: ${cards.map((c) => c.name).join(' / ')}`);
check(cards.every((c) => c.box.width >= 150 && c.box.height >= 150), `cards are big (smallest ${Math.round(Math.min(...cards.map((c) => c.box.width)))} x ${Math.round(Math.min(...cards.map((c) => c.box.height)))} px)`);
check(await live(() => window.__live.picking()), 'the start screen is open at the start');
await page.waitForTimeout(1500);
check((await live(() => window.__live.frame())) === null && (await live(() => !window.__live.streaming())), 'nothing is driving before DRIVE: no stream, no moving truck');
check((await live(() => window.__live.selected())) === 'mule_4x4', 'the Mule is chosen to begin with');
const db = await page.$eval('#drive', (b) => b.getBoundingClientRect().toJSON());
check(db.width >= 200 && db.height >= 90, `the DRIVE button is huge (${Math.round(db.width)} x ${Math.round(db.height)} px)`);
await page.screenshot({ path: `${out}/picker.png` });
await page.click('.card[data-id="scout_4x4"]');
check((await live(() => window.__live.selected())) === 'scout_4x4' && (await live(() => window.__live.picking())) && (await live(() => !window.__live.streaming())), 'tapping a picture only chooses it (the Scout is highlighted, nothing starts)');
await page.click('.card[data-id="mule_4x4"]');

await page.click('#drive');
await until(() => !window.__live.picking() && window.__live.frame() && window.__live.vehicle() === 'mule_4x4');
check(true, 'DRIVE: the start screen closed and frames arrive for the Mule');
const p0 = await live(() => window.__live.frame().pos_m);

// Keyboard: up arrow = go.
await page.keyboard.down('ArrowUp');
await until(() => window.__live.frame().speed_m_s > 2.5, 60000);
const sp = await live(() => window.__live.frame().speed_m_s);
check(sp > 2.5, `the up arrow drives the truck: ${(sp * 3.6).toFixed(1)} km/h`);
check((await live(() => window.__live.input().throttle)) === 1, 'the page is sending full throttle');
check((await live(() => window.__live.sound())) === 'running', 'engine sound started on the first key press');
await page.keyboard.down('ArrowRight');
await page.waitForTimeout(1500);
const steer = await live(() => window.__live.input().steer);
check(steer > 0.5, `the right arrow steers right (steer ${steer})`);
await page.screenshot({ path: `${out}/chase.png` });
await page.keyboard.up('ArrowRight');
await page.keyboard.press('c');
check((await live(() => window.__live.camera())) === 'rts', 'the C key switches to the RTS camera');
await page.waitForTimeout(1200);
await page.screenshot({ path: `${out}/rts.png` });
await page.keyboard.press('c');
const limit = await live(() => window.__live.frame().speed_m_s * 3.6);
check(limit < 27, `the kid speed cap holds (${limit.toFixed(1)} km/h)`);
await page.keyboard.up('ArrowUp');
await page.keyboard.down('Space');
await until(() => window.__live.frame().speed_m_s < 1.0, 60000);
check(true, 'space brakes to a stop');
await page.keyboard.up('Space');

// Reset: the big button, then the banner.
const farBefore = await live(() => window.__live.frame().pos_m);
const rb = await page.$eval('#reset', (b) => b.getBoundingClientRect().toJSON());
check(rb.width >= 80 && rb.height >= 80, `the reset button is big (${Math.round(rb.width)} px)`);
await page.click('#reset');
await until(() => document.getElementById('banner').classList.contains('on'), 30000);
check((await page.textContent('#banner')).toLowerCase().includes('back on the road'), 'the banner says "Back on the road!"');
await page.waitForTimeout(700);
await page.screenshot({ path: `${out}/banner.png` });

// Down arrow: brakes, and held on after the stop, backs up.
await page.keyboard.down('ArrowUp');
await until(() => window.__live.frame().speed_m_s > 1.5, 60000);
await page.keyboard.up('ArrowUp');
await page.keyboard.down('ArrowDown');
await until(() => window.__live.frame().speed_m_s < -0.4, 60000);
check((await live(() => window.__live.input().reverse)) === true, `holding the down arrow after the stop backs up (${(await live(() => window.__live.frame().speed_m_s)).toFixed(1)} m/s)`);
await page.keyboard.up('ArrowDown');
await until(() => window.__live.input().throttle === 0 && window.__live.input().reverse === false, 10000);

// A gamepad: right trigger = throttle, left stick = steer (a fake pad the page polls).
await page.evaluate(() => { window.__pad = { connected: true, buttons: Array.from({ length: 17 }, () => ({ value: 0, pressed: false })), axes: [0, 0, 0, 0] }; navigator.getGamepads = () => [window.__pad]; });
await page.evaluate(() => { window.__pad.buttons[7] = { value: 0.8, pressed: true }; window.__pad.axes[0] = -0.7; });
await until(() => window.__live.input().throttle > 0.7 && window.__live.input().steer < -0.4, 10000);
check(true, 'a gamepad works: the right trigger is throttle and the left stick steers');
await page.evaluate(() => { window.__pad.buttons[7] = { value: 0, pressed: false }; window.__pad.axes[0] = 0; });
await until(() => window.__live.input().throttle === 0 && window.__live.input().steer === 0, 10000);

// On-screen buttons by pointer: hold GO.
const go = await page.$eval('#go', (b) => b.getBoundingClientRect().toJSON());
await page.mouse.move(go.x + go.width / 2, go.y + go.height / 2);
await page.mouse.down();
await until(() => window.__live.input().throttle === 1, 10000);
check(true, 'holding the on-screen GO button sends full throttle');
await until(() => window.__live.frame().speed_m_s > 1.5, 60000);
await page.mouse.up();
await until(() => window.__live.input().throttle === 0, 10000);
check(true, 'letting go of GO releases the throttle');
const sizes = await page.$$eval('.btn', (bs) => bs.map((b) => Math.min(b.getBoundingClientRect().width, b.getBoundingClientRect().height)));
check(Math.min(...sizes) >= 70, `every button is at least 70 px (smallest ${Math.round(Math.min(...sizes))})`);

// The detailed skins are fitted: smaller for the scout, bigger for the hauler (and no 404s probing for skins that are not there).
const fits = {};
for (const id of ['scout_4x4', 'mule_4x4', 'hauler_4x4']) {
  await live((id) => window.__live.pick(id), id);
  await page.waitForFunction((id) => window.__live.vehicle() === id && window.__live.fit(), id, { timeout: 60000 });
  fits[id] = await live(() => window.__live.fit());
}
// every vehicle with a skin of its own (scout, hauler) or the utility truck (mule) is fitted at about its own size
check(Object.values(fits).every((f) => f && f.scale > 0.8 && f.scale < 1.25), `the skin fits each vehicle: scale scout ${fits.scout_4x4.scale.toFixed(2)}, mule ${fits.mule_4x4.scale.toFixed(2)}, hauler ${fits.hauler_4x4.scale.toFixed(2)}`);
await page.screenshot({ path: `${out}/hauler.png` });

// The start screen by keyboard: 1 chooses the Scout, Enter drives.
await page.click('#garage');
await page.keyboard.press('1');
check((await live(() => window.__live.selected())) === 'scout_4x4', 'the 1 key chooses the first vehicle on the start screen');
await page.keyboard.press('Enter');
await page.waitForFunction(() => !window.__live.picking() && window.__live.vehicle() === 'scout_4x4', null, { timeout: 60000 });
check(true, 'Enter drives the chosen vehicle');

// The garage button opens the picker again.
await page.click('#garage');
check(await live(() => window.__live.picking()), 'the garage button reopens the picker');
check(errors.length === 0, `no console errors${errors.length ? ': ' + errors.join(' | ') : ''}`);
await browser.close();
process.exit(failed ? 1 : 0);
