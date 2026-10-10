// Headless capture: node capture.mjs page.html --out clip.mp4 [--fps 30] [--seconds 10] [--start 0] [--width 960] [--height 540] [--camera rts|orbit|chase|quarter|front]
import { createRequire } from 'node:module';
const { chromium } = createRequire((process.env.PLAYWRIGHT_DIR ?? '/opt/node22/lib/node_modules') + '/')('playwright');
import { spawn } from 'node:child_process';
import path from 'node:path';
const argv = process.argv.slice(2);
const opt = (k, d) => (argv.includes('--' + k) ? argv[argv.indexOf('--' + k) + 1] : d);
const fps = +opt('fps', 30), secs = +opt('seconds', 1e9), t0 = +opt('start', 0), W = +opt('width', 960), H = +opt('height', 540);
const browser = await chromium.launch({ executablePath: process.env.CHROME, args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const page = await browser.newPage({ viewport: { width: W, height: H } });
const errors = [];
page.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
page.on('pageerror', (e) => errors.push(String(e)));
await page.goto('file://' + path.resolve(argv[0]) + '?static');
await page.waitForFunction('window.__ready === true', null, { timeout: 60000 });
console.log('GL:', await page.evaluate(() => window.__v.renderer), 'triangles', await page.evaluate(() => `${window.__v.triangles}/${window.__v.expectedTriangles}`));
const ff = spawn('ffmpeg', ['-y', '-loglevel', 'error', '-f', 'image2pipe', '-framerate', String(fps), '-c:v', 'mjpeg', '-i', '-', '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-crf', '22', '-movflags', '+faststart', opt('out', 'clip.mp4')], { stdio: ['pipe', 'inherit', 'inherit'] });
const done = new Promise((r) => ff.on('close', r));
await page.evaluate(([m, l]) => { window.__v.pause(); window.__v.setCamera(m === 'rts' ? { mode: m } : { mode: m, dist: m === 'orbit' ? 12 : m === 'front' ? 7 : 9, pitch: m === 'orbit' ? 0.28 : m === 'front' ? 0.22 : 0.3 }); window.__v.setLayout(l); }, [opt('camera', 'rts'), opt('plots', 'inset')]);
const n = Math.round(Math.min(secs, (await page.evaluate(() => window.__v.duration)) - t0) * fps), t = Date.now();
for (let i = 0; i < n; i++) {
  // The orbit camera circles slowly (about 17 degrees a second); chase follows the hull.
  if (opt('camera', 'rts') === 'orbit') await page.evaluate((y) => window.__v.setCamera({ yaw: y }), 0.6 + (i / fps) * 0.3);
  await page.evaluate((tt) => window.__v.renderAt(tt), t0 + i / fps);
  // A page screenshot (not the bare canvas) so the HUD, ledger panel and scope plots are in the clip.
  const shot = await page.screenshot({ type: 'jpeg', quality: 88 });
  if (!ff.stdin.write(shot)) await new Promise((r) => ff.stdin.once('drain', r));
}
ff.stdin.end(); await done; await browser.close();
console.log(`${n} frames in ${((Date.now() - t) / 1000).toFixed(1)} s = ${((Date.now() - t) / n).toFixed(0)} ms/frame; errors: ${errors.length ? errors.join(' | ') : 'none'}`);
