// Render chosen moments of one vehicle's run to PNG files (to check an animation, or to illustrate a doc).
//
//   node frames.js page.html outdir --vehicle ID --times 2,2.1,2.2 [--camera side] [--width 960] [--height 540] [--clean]
//
// `--clean` hides the HUD strips and labels' backdrop is kept; the page's own overlay is drawn as in a clip. Set THREE_JS to a
// local three.min.js (r128) when the CDN is blocked (see capture.js).
const { chromium } = require('playwright');
const path = require('path');

const argv = process.argv.slice(2);
const [file, out] = argv;
const opt = (k, d) => { const i = argv.indexOf('--' + k); return i >= 0 ? argv[i + 1] : d; };
const vehicle = opt('vehicle'), camera = opt('camera', 'side'), W = +opt('width', 960), H = +opt('height', 540);
const times = opt('times', '1').split(',').map(Number);
if (!file || !out || !vehicle) { console.error('usage: node frames.js page.html outdir --vehicle ID --times t1,t2,... [--camera chase|side|orbit|overview] [--width 960] [--height 540]'); process.exit(2); }

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROME || undefined });
  const page = await browser.newPage({ viewport: { width: W, height: H }, deviceScaleFactor: 1 });
  const errors = [];
  page.on('console', m => { if (m.type() === 'error' && !/Failed to load resource/.test(m.text())) errors.push(m.text()); });
  page.on('pageerror', e => errors.push(String(e)));
  if (process.env.THREE_JS) await page.route('https://cdnjs.cloudflare.com/**', r => r.fulfill({ path: process.env.THREE_JS, contentType: 'application/javascript' }));
  await page.route('https://fonts.googleapis.com/**', r => r.abort());
  await page.route('https://fonts.gstatic.com/**', r => r.abort());
  await page.goto('file://' + path.resolve(file), { waitUntil: 'load' });
  await page.addStyleTag({ content: `.layout{display:block}.side,.bar,.top{display:none}.app{padding:0;max-width:none;gap:0}.stage{border:0;border-radius:0}.view{aspect-ratio:auto;width:${W}px;height:${H}px}` });
  await page.evaluate(() => window.__trial.ready);
  await page.evaluate(async ([id, cam]) => { await window.__trial.select(id); window.__trial.camera(cam); window.__trial.pause(); }, [vehicle, camera]);
  await page.waitForTimeout(300);
  for (const t of times) {
    // Render the moment twice: the first pass lets the chase camera settle where it belongs.
    await page.evaluate(t0 => { window.__trial.renderAt(t0, 0.5); window.__trial.renderAt(t0, 0.5); }, t);
    await page.screenshot({ path: path.join(out, `${vehicle}_${camera}_${String(t).replace('.', '_')}.png`) });
  }
  await browser.close();
  console.log(errors.length ? 'console errors:\n' + errors.join('\n') : `wrote ${times.length} frames`);
})();
