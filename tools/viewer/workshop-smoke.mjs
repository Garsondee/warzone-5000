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
check(sliders.length === 6, `six sliders: ${sliders.join(', ')}`);
check((await w(() => window.__workshop.base())) === 'mule_4x4', 'the Mule is the first vehicle shown');
await page.screenshot({ path: `${out}/1-authored.png` });

// pick the Hauler, whose body follows its definition exactly, and stretch its wheelbase
await page.click('#bases button[data-id=hauler_4x4]');
await until(() => window.__workshop.base() === 'hauler_4x4' && window.__workshop.detail() === 'authored');
const info = await (await fetch(`${url}api/base/hauler_4x4`)).json();
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
