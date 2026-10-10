// Headless capture: node capture.mjs page.html --out clip.mp4 [--fps 30] [--seconds 10] [--start 0] [--width 960] [--height 540]
import { createRequire } from 'node:module';
const { chromium } = createRequire((process.env.PLAYWRIGHT_DIR ?? '/opt/node22/lib/node_modules') + '/')('playwright');
import { spawn } from 'node:child_process';
import path from 'node:path';
const argv = process.argv.slice(2);
const opt = (k, d) => (argv.includes('--' + k) ? argv[argv.indexOf('--' + k) + 1] : d);
const fps = +opt('fps', 30), secs = +opt('seconds', 10), t0 = +opt('start', 0), W = +opt('width', 960), H = +opt('height', 540);
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
const n = Math.round(secs * fps), t = Date.now();
for (let i = 0; i < n; i++) {
  const url = await page.evaluate((tt) => { window.__v.renderAt(tt); return document.getElementById('c').toDataURL('image/jpeg', 0.9); }, t0 + i / fps);
  if (!ff.stdin.write(Buffer.from(url.slice(url.indexOf(',') + 1), 'base64'))) await new Promise((r) => ff.stdin.once('drain', r));
}
ff.stdin.end(); await done; await browser.close();
console.log(`${n} frames in ${((Date.now() - t) / 1000).toFixed(1)} s = ${((Date.now() - t) / n).toFixed(0)} ms/frame; errors: ${errors.length ? errors.join(' | ') : 'none'}`);
