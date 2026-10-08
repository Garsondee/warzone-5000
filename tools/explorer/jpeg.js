// node jpeg.js out.json name=path.png [name=path.png ...]: PNG files to JPEG data URLs (via the browser's canvas).
const { chromium } = require('playwright');
const fs = require('fs');
(async () => {
  const [out, ...pairs] = process.argv.slice(2);
  const browser = await chromium.launch({ executablePath: process.env.CHROME || undefined });
  const page = await browser.newPage();
  const result = {};
  for (const pair of pairs) {
    const [name, file] = pair.split('=');
    const b64 = fs.readFileSync(file).toString('base64');
    result[name] = await page.evaluate(async (b64) => {
      const img = new Image();
      img.src = 'data:image/png;base64,' + b64;
      await img.decode();
      const c = document.createElement('canvas');
      c.width = img.naturalWidth; c.height = img.naturalHeight;
      c.getContext('2d').drawImage(img, 0, 0);
      return c.toDataURL('image/jpeg', 0.86);
    }, b64);
    console.log(name, (result[name].length / 1024).toFixed(0) + ' KB');
  }
  fs.writeFileSync(out, JSON.stringify(result));
  await browser.close();
})();
