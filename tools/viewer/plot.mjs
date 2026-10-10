// A small PNG chart tool for evidence (headless Chromium draws it): first CSV column is x, every other column a series.
//   node plot.mjs data.csv --out chart.png [--title "..."] [--xlabel t_s] [--ylabel "..."] [--width 900] [--height 480]
import { createRequire } from 'node:module';
import fs from 'node:fs';
const { chromium } = createRequire((process.env.PLAYWRIGHT_DIR ?? '/opt/node22/lib/node_modules') + '/')('playwright');
const argv = process.argv.slice(2);
const opt = (k, d) => (argv.includes('--' + k) ? argv[argv.indexOf('--' + k) + 1] : d);
const rows = fs.readFileSync(argv[0], 'utf8').trim().split(/\r?\n/).map((l) => l.split(','));
const head = rows[0], data = rows.slice(1).map((r) => r.map(Number));
if (head.length < 2 || data.some((r) => r.some((v) => !Number.isFinite(v)))) { console.error('plot: need a header row, an x column and at least one numeric series, no blanks or NaN'); process.exit(2); }
const W = +opt('width', 900), H = +opt('height', 480);
const browser = await chromium.launch({ executablePath: process.env.CHROME });
const page = await browser.newPage({ viewport: { width: W, height: H } });
await page.setContent(`<body style="margin:0"><canvas id=c width=${W} height=${H}></canvas>`);
await page.evaluate(([head, data, W, H, title, xl, yl]) => {
  const ctx = document.getElementById('c').getContext('2d');
  const colors = ['#1f77b4', '#d95f02', '#2ca02c', '#9467bd', '#c2185b', '#7f7f7f'];
  const L = 70, R = 20, T = 40, B = 55, w = W - L - R, h = H - T - B;
  ctx.fillStyle = '#fff'; ctx.fillRect(0, 0, W, H);
  const xs = data.map((r) => r[0]), x0 = Math.min(...xs), x1 = Math.max(...xs);
  let lo = Infinity, hi = -Infinity;
  data.forEach((r) => r.slice(1).forEach((v) => { lo = Math.min(lo, v); hi = Math.max(hi, v); }));
  if (hi === lo) { hi += 1; lo -= 1; }
  const nice = (a, b) => { const raw = (b - a) / 6, p = 10 ** Math.floor(Math.log10(raw)), f = raw / p, s = (f < 1.5 ? 1 : f < 3.5 ? 2 : f < 7.5 ? 5 : 10) * p; return [Math.floor(a / s) * s, Math.ceil(b / s) * s, s]; };
  const [ylo, yhi, ys] = nice(lo, hi), [xlo, xhi, xstep] = nice(x0, x1);
  const X = (v) => L + ((v - xlo) / (xhi - xlo)) * w, Y = (v) => T + h - ((v - ylo) / (yhi - ylo)) * h;
  ctx.font = '12px sans-serif'; ctx.fillStyle = '#333'; ctx.strokeStyle = '#ddd';
  for (let v = ylo; v <= yhi + 1e-9; v += ys) { ctx.beginPath(); ctx.moveTo(L, Y(v)); ctx.lineTo(L + w, Y(v)); ctx.stroke(); ctx.textAlign = 'right'; ctx.fillText(+v.toPrecision(4), L - 6, Y(v) + 4); }
  for (let v = xlo; v <= xhi + 1e-9; v += xstep) { ctx.beginPath(); ctx.moveTo(X(v), T); ctx.lineTo(X(v), T + h); ctx.stroke(); ctx.textAlign = 'center'; ctx.fillText(+v.toPrecision(4), X(v), T + h + 16); }
  ctx.strokeStyle = '#333'; ctx.strokeRect(L, T, w, h);
  head.slice(1).forEach((_, k) => { ctx.strokeStyle = colors[k % colors.length]; ctx.lineWidth = 2; ctx.beginPath(); data.forEach((r, i) => (i ? ctx.lineTo(X(r[0]), Y(r[k + 1])) : ctx.moveTo(X(r[0]), Y(r[k + 1])))); ctx.stroke(); });
  ctx.lineWidth = 1; ctx.textAlign = 'left'; ctx.font = 'bold 15px sans-serif'; ctx.fillStyle = '#111'; ctx.fillText(title, L, 24);
  ctx.font = '12px sans-serif';
  let lx = L + w; head.slice(1).reverse().forEach((n, i) => { const k = head.length - 2 - i; ctx.textAlign = 'right'; ctx.fillStyle = colors[k % colors.length]; ctx.fillText('■ ' + n, lx, 24); lx -= ctx.measureText('■ ' + n).width + 14; });
  ctx.fillStyle = '#333'; ctx.textAlign = 'center'; ctx.fillText(xl, L + w / 2, H - 12);
  ctx.save(); ctx.translate(16, T + h / 2); ctx.rotate(-Math.PI / 2); ctx.fillText(yl, 0, 0); ctx.restore();
}, [head, data, W, H, opt('title', argv[0]), opt('xlabel', head[0]), opt('ylabel', '')]);
await page.screenshot({ path: opt('out', 'chart.png') });
await browser.close();
console.log(`wrote ${opt('out', 'chart.png')} (${data.length} points, ${head.length - 1} series)`);
