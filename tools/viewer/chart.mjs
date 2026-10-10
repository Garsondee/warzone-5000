// Evidence charts as PNG (headless Chromium draws them, like plot.mjs). The shapes of the JSON are in docs/lanes/viewer/charts.md.
// A picture over 300 KB (the media lint allows 400 KB) is re-saved with a 64-colour palette by ffmpeg, which the flat colours of a chart survive; --shrink off keeps full colour.
//   node chart.mjs tornado impact.json --out tornado.png [--theme light|dark] [--width 1500] [--cols N] [--rows 8] [--scale 2]
//   node chart.mjs ladder  ladder.json --out ladder.png  [--theme light|dark] [--width 1000] [--height 560] [--scale 2]
import { createRequire } from 'node:module';
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { tornadoModel, ladderModel, tornadoSize, ladderSize } from './src/charts.js';

const { chromium } = createRequire((process.env.PLAYWRIGHT_DIR ?? '/opt/node22/lib/node_modules') + '/')('playwright');
const argv = process.argv.slice(2);
const opt = (k, d) => (argv.includes('--' + k) ? argv[argv.indexOf('--' + k) + 1] : d);
const num = (k) => (opt(k) === undefined ? undefined : +opt(k));
const [kind, file] = argv;
if (!['tornado', 'ladder'].includes(kind) || !file) { console.error('usage: chart.mjs tornado|ladder <spec.json> --out chart.png [--theme light|dark] [--width W] [--scale 2]'); process.exit(2); }

let spec;
try { spec = JSON.parse(fs.readFileSync(file, 'utf8')); } catch (e) { console.error(`chart: cannot read ${file}: ${e.message}`); process.exit(2); }
const o = { theme: opt('theme', 'light'), width: num('width'), height: num('height'), cols: num('cols') };
if (!['light', 'dark'].includes(o.theme)) { console.error(`chart: --theme is light or dark, not ${o.theme}`); process.exit(2); }
let model, size;
try {
  model = kind === 'tornado' ? tornadoModel(spec, { maxRows: num('rows') ?? 8 }) : ladderModel(spec);
  size = kind === 'tornado' ? tornadoSize(model, o) : ladderSize(model, o);
} catch (e) { console.error(`chart: ${e.message}`); process.exit(2); }

const scale = num('scale') ?? 2; // the canvas is drawn at this many pixels per CSS pixel so the text stays crisp
const lib = fs.readFileSync(fileURLToPath(new URL('./src/charts.js', import.meta.url)), 'utf8').replace(/^export /gm, '');
const browser = await chromium.launch({ executablePath: process.env.CHROME });
const page = await browser.newPage({ viewport: { width: 800, height: 600 } });
await page.setContent('<body style="margin:0"><canvas id=c></canvas>');
await page.addScriptTag({ content: `${lib}\nwindow.W5K = { drawTornado, drawLadder };` });
const png = await page.evaluate(([kind, model, o, size, scale]) => {
  const c = document.getElementById('c'); c.width = size.w * scale; c.height = size.h * scale;
  const ctx = c.getContext('2d'); ctx.scale(scale, scale);
  (kind === 'tornado' ? window.W5K.drawTornado : window.W5K.drawLadder)(ctx, model, o);
  return c.toDataURL('image/png');
}, [kind, model, o, size, scale]);
await browser.close();
const out = opt('out', kind + '.png');
fs.writeFileSync(out, Buffer.from(png.split(',')[1], 'base64'));
const LIMIT_KB = 300; // the media lint allows 400 KB per picture
if (fs.statSync(out).size > LIMIT_KB * 1024 && opt('shrink', 'auto') !== 'off') {
  const tmp = out + '.pal.png';
  const r = spawnSync('ffmpeg', ['-y', '-loglevel', 'error', '-i', out, '-vf', 'split[a][b];[a]palettegen=max_colors=64[p];[b][p]paletteuse=dither=none', tmp]);
  if (r.status === 0) fs.renameSync(tmp, out); else console.error('chart: the picture is over the 400 KB media limit and ffmpeg could not shrink it; try --scale 1.5');
}
console.log(`wrote ${out} (${size.w * scale}x${size.h * scale}, ${(fs.statSync(out).size / 1024).toFixed(0)} KB)`);
