// Screenshot the page at desktop and phone width and report console errors (a smoke test for the viewer).
//
//   node shot.js page.html outdir [vehicle]
//
// Set THREE_JS to a local three.min.js (r128) when the CDN is blocked.
const { chromium } = require('playwright');
const path = require('path');

(async () => {
  const [file, out, vehicle] = process.argv.slice(2);
  const browser = await chromium.launch({ executablePath: process.env.CHROME || undefined });
  const errors = [];
  for (const [name, w, h] of [['desktop', 1440, 900], ['phone', 400, 900]]) {
    const page = await browser.newPage({ viewport: { width: w, height: h }, deviceScaleFactor: 1 });
    page.on('console', m => { if (m.type() === 'error') errors.push(`[${name}] ${m.text()}`); });
    page.on('pageerror', e => errors.push(`[${name}] pageerror: ${e.message}`));
    if (process.env.THREE_JS) await page.route('https://cdnjs.cloudflare.com/**', r => r.fulfill({ path: process.env.THREE_JS, contentType: 'application/javascript' }));
    await page.route('https://fonts.googleapis.com/**', r => r.abort());
    await page.route('https://fonts.gstatic.com/**', r => r.abort());
    await page.goto('file://' + path.resolve(file), { waitUntil: 'load' });
    await page.evaluate(() => window.__trial.ready);
    if (vehicle) await page.evaluate(id => window.__trial.select(id), vehicle);
    await page.evaluate(() => window.__trial.pause());
    const dur = await page.evaluate(() => window.__trial.duration());
    for (const [label, f] of [['start', 0.02], ['mid', 0.45], ['late', 0.75], ['end', 0.97]]) {
      await page.evaluate(t => window.__trial.renderAt(t, 0.5), dur * f);
      await page.waitForTimeout(80);
      await page.screenshot({ path: path.join(out, `${name}_${label}.png`), fullPage: name === 'phone' && label === 'mid' });
    }
    if (name === 'desktop') {
      for (const cam of ['side', 'orbit', 'overview']) {
        await page.evaluate(([c, t]) => { window.__trial.camera(c); window.__trial.renderAt(t, 0.5); }, [cam, dur * 0.4]);
        await page.waitForTimeout(80);
        await page.screenshot({ path: path.join(out, `desktop_cam_${cam}.png`) });
      }
    }
    const over = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    console.log(`horizontal overflow (${name}):`, over);
    await page.close();
  }
  await browser.close();
  console.log(errors.length ? errors.join('\n') : 'no console errors');
})();
