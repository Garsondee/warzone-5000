// node shot.js page.html outdir
const { chromium } = require('playwright');
const path = require('path');
(async () => {
  const [file, out] = process.argv.slice(2);
  const browser = await chromium.launch({ executablePath: process.env.CHROME || undefined });
  const errors = [];
  for (const [name, w, h] of [['desktop', 1440, 900], ['phone', 400, 900]]) {
    const page = await browser.newPage({ viewport: { width: w, height: h }, deviceScaleFactor: 1 });
    page.on('console', m => { if (m.type() === 'error' || m.type() === 'warning') errors.push(`[${name}] ${m.type()}: ${m.text()}`); });
    page.on('pageerror', e => errors.push(`[${name}] pageerror: ${e.message}`));
    await page.goto('file://' + path.resolve(file), { waitUntil: 'load' });
    await page.waitForTimeout(600);
    await page.screenshot({ path: path.join(out, `${name}.png`), fullPage: true });
    if (name === 'desktop') {
      // exercise: preset, hover a point, pin it, toggle frontier
      await page.click('button.preset:nth-child(2)');
      await page.waitForTimeout(200);
      await page.click('.xrow:nth-of-type(3)');
      await page.waitForTimeout(250);
      await page.screenshot({ path: path.join(out, 'desktop_fire.png'), fullPage: true });
      await page.click('button.preset:nth-child(3)');
      await page.waitForTimeout(200);
      await page.screenshot({ path: path.join(out, 'desktop_sight.png'), fullPage: false });
      const horiz = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
      console.log('horizontal overflow (desktop):', horiz);
    } else {
      await page.click('.xrow:nth-of-type(2)');
      await page.waitForTimeout(250);
      await page.screenshot({ path: path.join(out, 'phone_pinned.png'), fullPage: true });
      const horiz = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
      console.log('horizontal overflow (phone):', horiz);
    }
    await page.close();
  }
  await browser.close();
  console.log(errors.length ? errors.join('\n') : 'no console errors');
})();
