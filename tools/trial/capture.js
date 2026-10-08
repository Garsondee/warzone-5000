// Record a clip of one vehicle's run from the page, frame by frame (headless Chromium -> ffmpeg).
//
//   node capture.js page.html --vehicle ID --out clip.mp4 [--camera chase] [--fps 24] [--width 1280] [--height 720] [--rate 1]
//
// cdnjs is blocked in the cloud container, so point THREE_JS at a local copy of three.min.js r128 (npm pack three@0.128.0);
// the script serves it in place of the CDN URL. The page exposes window.__trial.renderAt(t) so every frame is rendered at an
// exact time, independent of how fast the browser runs.
const { chromium } = require('playwright');
const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');

const argv = process.argv.slice(2);
const page_file = argv[0];
const opt = (k, d) => { const i = argv.indexOf('--' + k); return i >= 0 ? argv[i + 1] : d; };
const vehicle = opt('vehicle'), out = opt('out', 'clip.mp4'), camera = opt('camera', 'chase');
const fps = +opt('fps', 24), W = +opt('width', 1280), H = +opt('height', 720), rate = +opt('rate', 1);
if (!page_file || !vehicle) { console.error('usage: node capture.js page.html --vehicle ID --out clip.mp4 [--camera chase|side|orbit|overview] [--fps 24] [--width 1280] [--height 720] [--rate 1]'); process.exit(2); }

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROME || undefined });
  const page = await browser.newPage({ viewport: { width: W, height: H }, deviceScaleFactor: 1 });
  const errors = [];
  page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
  page.on('pageerror', e => errors.push(String(e)));
  if (process.env.THREE_JS) await page.route('https://cdnjs.cloudflare.com/**', r => r.fulfill({ path: process.env.THREE_JS, contentType: 'application/javascript' }));
  await page.route('https://fonts.googleapis.com/**', r => r.abort());
  await page.route('https://fonts.gstatic.com/**', r => r.abort());
  await page.goto('file://' + path.resolve(page_file), { waitUntil: 'load' });
  await page.addStyleTag({ content: `.layout{display:block}.side,.bar,.top{display:none}.app{padding:0;max-width:none;gap:0}.stage{border:0;border-radius:0}.view{aspect-ratio:auto;width:${W}px;height:${H}px}` });
  await page.evaluate(() => window.__trial.ready);
  await page.evaluate(async ([id, cam]) => { await window.__trial.select(id); window.__trial.camera(cam); window.__trial.pause(); }, [vehicle, camera]);
  await page.waitForTimeout(300);
  const duration = await page.evaluate(() => window.__trial.duration());
  const dtSim = rate / fps, n = Math.ceil(duration / dtSim);
  console.log(`${vehicle}: ${duration.toFixed(1)} s of replay, ${n} frames at ${fps} fps (x${rate})`);
  const ff = spawn('ffmpeg', ['-y', '-loglevel', 'error', '-f', 'image2pipe', '-framerate', String(fps), '-c:v', 'mjpeg', '-i', '-',
    '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-crf', '20', '-preset', 'veryfast', '-movflags', '+faststart', out], { stdio: ['pipe', 'inherit', 'inherit'] });
  const done = new Promise(res => ff.on('close', res));
  const t0 = Date.now();
  for (let i = 0; i < n; i++) {
    const url = await page.evaluate(([t, dt]) => { window.__trial.renderAt(t, dt); return window.__trial.snapshot(0.9); }, [i * dtSim, 1 / fps]);
    if (!ff.stdin.write(Buffer.from(url.slice(url.indexOf(',') + 1), 'base64'))) await new Promise(r => ff.stdin.once('drain', r));
  }
  ff.stdin.end();
  await done;
  await browser.close();
  console.log(`wrote ${out} in ${((Date.now() - t0) / 1000).toFixed(0)} s` + (errors.length ? '\nconsole errors:\n' + errors.join('\n') : ''));
})();
