// End-to-end test of the Workshop page against a running `w5k viewer workshop` (headless Chromium, software GL):
//   w5k viewer workshop --port 8791 &      (from the folder that holds content/)
//   node tools/viewer/workshop-smoke.mjs [http://127.0.0.1:8791/] [out-dir]
import { createRequire } from 'node:module';
import fs from 'node:fs';
const { chromium } = createRequire((process.env.PLAYWRIGHT_DIR ?? '/opt/node22/lib/node_modules') + '/')('playwright');
const url = process.argv[2] ?? 'http://127.0.0.1:8791/', out = process.argv[3] ?? 'out/workshop';
fs.mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ executablePath: process.env.CHROME, args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
const errors = [];
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
page.on('pageerror', (e) => errors.push(String(e)));
let failed = false;
const check = (ok, msg) => { console.log(`${ok ? 'PASS' : 'FAIL'}  ${msg}`); failed ||= !ok; };
const w = (fn, arg) => page.evaluate(fn, arg);
const until = (fn, arg, timeout = 60000) => page.waitForFunction(fn, arg, { timeout, polling: 100 });
const slide = (id, value) => page.$eval(`#s-${id}`, (el, v) => { el.value = v; el.dispatchEvent(new Event('input', { bubbles: true })); }, value);
const lengthOf = async () => (await w(() => window.__workshop.size()))[2];

await page.goto(url);
await until(() => window.__workshop && window.__workshop.detail() === 'authored');
const sliders = await page.$$eval('#levers input[type=range]', (els) => els.map((e) => e.id));
check(sliders.length === 6 && sliders.includes('s-tyre_pressure') && !sliders.includes('s-mass'), `six sliders, all choices (tyre pressure, no mass): ${sliders.join(', ')}`);
const rows0 = await w(() => window.__workshop.consequences());
check(['Total mass', 'Axle load', 'Power to weight', 'Ground pressure', 'Centre of mass', 'Ride frequency', 'Approach'].every((k) => rows0.some((r) => r.label.startsWith(k))) && rows0.every((r) => r.because.length > 10 && /\d/.test(r.value)),
  `the Consequences are there from the start, each with a value and a because: ${rows0.map((r) => `${r.label} ${r.value}`).join(' | ')}`);
check((await w(() => window.__workshop.flags())).length === 1 && /None yet/.test((await w(() => window.__workshop.flags()))[0]), 'the Flags panel says none yet until the audit exists');
check((await w(() => window.__workshop.base())) === 'mule_4x4', 'the Mule is the first vehicle shown');
await page.screenshot({ path: `${out}/1-authored.png` });

// pick the Hauler, whose body follows its definition exactly, and stretch its wheelbase
await page.click('#bases button[data-id=hauler_4x4]');
await until(() => window.__workshop.base() === 'hauler_4x4' && window.__workshop.detail() === 'authored');
const info = await (await fetch(new URL('api/base/hauler_4x4', url))).json();
const wheelbase = info.levers.find((l) => l.id === 'wheelbase').base;
const before = await lengthOf();
const t0 = Date.now();
await slide('wheelbase', 1.3);
await until(() => window.__workshop.detail() === 'preview');
const took = (Date.now() - t0) / 1000;
const grown = (await lengthOf()) - before;
check(Math.abs(grown - 0.3 * wheelbase) < 0.06, `the body grew by the wheelbase change: ${grown.toFixed(3)} m for ${(0.3 * wheelbase).toFixed(3)} m asked`);
check(took < 10, `the quick body arrived in ${took.toFixed(1)} s (budget 10 s)`);
await page.screenshot({ path: `${out}/2-wheelbase-130-quick.png` });

// a later change supersedes an earlier one: two quick moves, the page ends on the second
await slide('wheelbase', 1.1);
await slide('wheelbase', 1.2);
await until(() => window.__workshop.detail() === 'preview');
await page.waitForTimeout(400);
const second = (await lengthOf()) - before;
check(Math.abs(second - 0.2 * wheelbase) < 0.06, `after two quick moves the body is the second one: grew ${second.toFixed(3)} m for ${(0.2 * wheelbase).toFixed(3)} m`);

// the final detail follows when the sliders rest
await until(() => window.__workshop.detail() === 'final', null, 90000);
check(true, 'the final detail replaced the quick one');
await page.screenshot({ path: `${out}/3-wheelbase-120-final.png` });

// the scoreboard: measured against the base, and more power is a better launch and a better climb
await slide('wheelbase', 1);
const tBoard = Date.now();
const massBefore = parseFloat((await w(() => window.__workshop.consequences())).find((r) => r.label === 'Total mass').value);
await slide('engine_peak_power', 1.5);
check(await w(() => window.__workshop.consequencesStale()), 'the old consequences are greyed the moment a slider moves');
await until(([m]) => !window.__workshop.consequencesStale() && parseFloat(window.__workshop.consequences().find((r) => r.label === 'Total mass').value) !== m, [massBefore], 20000);
const conseqTook = (Date.now() - tBoard) / 1000, rows1 = await w(() => window.__workshop.consequences());
const massAfter = parseFloat(rows1.find((r) => r.label === 'Total mass').value);
check(massAfter > massBefore + 100 && conseqTook < 1.5, `engine power has a price: mass ${massBefore} to ${massAfter} kg, shown in ${conseqTook.toFixed(2)} s, without waiting for the body (${await w(() => window.__workshop.detail())})`);
check(parseFloat(rows1.find((r) => r.label === 'Power to weight').value) < 1.5 * parseFloat(rows0.find((r) => r.label === 'Power to weight').value) * 1.2, `power to weight grows by less than the power does: ${rows1.find((r) => r.label === 'Power to weight').value}`);
await until(() => window.__workshop.detail() === 'preview');
await until(() => !window.__workshop.stale() && window.__workshop.board().length === 6, null, 120000);
const boardTook = (Date.now() - tBoard) / 1000;
check(boardTook < 10, `the scoreboard arrived in ${boardTook.toFixed(1)} s (budget 10 s, body and numbers together)`);
const board = await w(() => window.__workshop.board());
const row = (name) => board.find((r) => r.name.startsWith(name));
check(/^\u2212.*better$/.test(row('0-48').change), `more power shortens the 0-48 km/h time: ${row('0-48').change}`);
// (the gradient row is not asserted: the model says more power does NOT climb steeper on the Hauler, which is VALIDATION's finding 2; the page shows it as it is)
check(board.every((r) => /^(no change|[+\u2212]\d+(\.\d+)?% (better|worse))$/.test(r.change)), `every row says how it changed in words: ${board.map((r) => r.change).join(' | ')}`);
check(board.every((r) => r.name && r.change !== undefined), `six rows, each with a name and a change: ${board.map((r) => r.name).join(' / ')}`);
await page.screenshot({ path: `${out}/4-scoreboard.png` });
await slide('tyre_pressure', 1.5);
check(await w(() => window.__workshop.stale()), 'the old numbers are greyed the moment a slider moves');
check(await page.$eval('#drive', (b) => b.disabled), 'DRIVE waits for the body that goes with the sliders');
await until(() => window.__workshop.detail() === 'preview');
await until(() => !window.__workshop.stale(), null, 120000);
const kpa0 = info.levers.find((l) => l.id === 'tyre_pressure').base, pressure = (await w(() => window.__workshop.consequences())).find((r) => r.label === 'Ground pressure').value;
check(Math.abs(parseFloat(pressure) - 1.5 * kpa0) < 1, `a harder tyre has a higher ground pressure: ${pressure} for ${kpa0} kPa x 1.5`);
await page.screenshot({ path: `${out}/5-consequences.png` });

// DRIVE: the real simulation starts on exactly this design, in another tab
const [game] = await Promise.all([page.waitForEvent('popup'), page.click('#drive')]);
const drove = Date.now();
await game.waitForFunction(() => window.__live && window.__live.streaming() && window.__live.frame(), null, { timeout: 120000, polling: 200 });
const vehicle = await game.evaluate(() => window.__live.frame().vehicle);
check(vehicle === 'hauler_4x4_design', `the game is driving the design: ${vehicle}`);
check(!(await game.evaluate(() => window.__live.picking())), 'the game skipped its start screen');
check((await game.evaluate(() => window.__live.frame().pos_m)).length === 3, `frames arrive from the simulation (${((Date.now() - drove) / 1000).toFixed(1)} s after DRIVE)`);
await game.close();

// tyres: wider tyres make the vehicle wider and nothing else crash; reset puts the sliders back
await slide('tyre_width', 1.4);
await until(() => window.__workshop.detail() === 'preview');
await page.click('#levers .lever:nth-child(2) button');
await until(() => window.__workshop.detail() === 'preview');
check((await page.$eval('#s-tyre_width', (e) => +e.value)) === 1, 'reset puts a slider back to the base');
check((await w(() => window.__workshop.error())) === null, 'no error is shown');
check(errors.length === 0, `no console errors${errors.length ? ': ' + errors.join(' | ') : ''}`);
await browser.close();
console.log(failed ? 'FAILED' : 'all passed');
process.exit(failed ? 1 : 0);
