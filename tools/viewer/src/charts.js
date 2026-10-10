// Evidence charts: the tornado (one lever at a time, one small panel per benchmark) and the ladder (sinkage up a ladder of mass or
// track width, beside what the soil theory predicts). The `*Model` functions turn a spec (shapes in docs/lanes/viewer/charts.md) into
// plain data and are tested under Node; the `draw*` functions paint a model on a 2D canvas context. chart.mjs runs them in headless
// Chromium to write a PNG; any page can load this file too. Colours are the dataviz reference palette, in its fixed order and checked with
// its validate_palette.js (the first three all-pairs, all eight on adjacent pairs, which is what lines need, in both modes); status
// colours are fixed and always come with an icon and words.

export const FONT = 'system-ui, -apple-system, "Segoe UI", sans-serif';
export const THEMES = {
  light: { surface: '#fcfcfb', ink: '#0b0b0b', ink2: '#52514e', muted: '#898781', grid: '#e1e0d9', axis: '#c3c2b7', series: ['#2a78d6', '#eb6834', '#1baf7a', '#eda100', '#e87ba4', '#008300', '#6250d6', '#e34948'] },
  dark: { surface: '#1a1a19', ink: '#f0efec', ink2: '#c3c2b7', muted: '#898781', grid: '#2c2c2a', axis: '#383835', series: ['#3987e5', '#d95926', '#199e70', '#c98500', '#d55181', '#008300', '#9085e9', '#e66767'] },
};
export const STATUS = { good: '#0ca30c', warning: '#fab219', serious: '#ec835a', critical: '#d03b3b' };

const MINUS = '−';
const ABS_EPS = 1e-12; // a baseline this small has no percentage; the panel shows the change in the benchmark's own unit
const DEFAULT_DEADBAND_PCT = 0.5; // a change smaller than this counts as "does not move"
const LEVER_COLOURS = [0, 1]; // theme.series slots for "lever -10%" and "lever +10%"

const sign = (v, dead) => (Math.abs(v) < dead ? '0' : v > 0 ? '+' : '-');
const signed = (v, d) => (v > 0 ? '+' : v < 0 ? MINUS : '') + Math.abs(v).toFixed(d);
const fmtPct = (v) => signed(v, Math.abs(v) >= 10 ? 0 : 1) + '%';
const fmtNum = (v) => (Math.abs(v) >= 100 ? v.toFixed(0) : Math.abs(v) >= 10 ? v.toFixed(1) : v.toPrecision(3));
const STEPS = [1, 1.2, 1.5, 2, 2.5, 3, 4, 5, 6, 8, 10];
/** Smallest "round" number (1, 1.2, 1.5, 2, 2.5, 3, 4, 5, 6, 8 times a power of ten) that is at least v. */
export function niceCeil(v) {
  if (!(v > 0)) return 1;
  const p = 10 ** Math.floor(Math.log10(v));
  return STEPS.find((s) => s * p >= v * (1 - 1e-9)) * p;
}
/** Axis ticks from lo to hi at a round step, about n of them. */
export function ticks(lo, hi, n) {
  const raw = (hi - lo) / n, p = 10 ** Math.floor(Math.log10(raw)), f = raw / p;
  const step = (f < 1.5 ? 1 : f < 3.5 ? 2 : f < 7.5 ? 5 : 10) * p, out = [];
  for (let v = Math.ceil(lo / step - 1e-9) * step; v <= hi + step * 1e-9; v += step) out.push(+v.toPrecision(10));
  return out;
}

// ------------------------------------------------------------------------------------------------------------------ tornado model

/** Did the +10% lever change move the benchmark the way the table expected? ok | wrong_sign | missing | spurious | unchecked. */
export function verdictOf(expected, observed) {
  if (!expected) return 'unchecked';
  if (expected.includes(observed)) return 'ok';
  if (observed === '0') return 'missing';
  return expected.every((e) => e === '0') ? 'spurious' : 'wrong_sign';
}

export function tornadoModel(spec, { maxRows = 8 } = {}) {
  if (spec.schema !== 'w5k-impact-1') throw new Error(`tornado: expected schema w5k-impact-1, got ${spec.schema}`);
  const dead = spec.deadband_pct ?? DEFAULT_DEADBAND_PCT;
  const levers = new Map(spec.levers.map((l, order) => [l.id, { ...l, order }]));
  const benches = new Map(spec.benchmarks.map((b) => [b.id, b]));
  for (const c of spec.cells) {
    if (!levers.has(c.lever) || !benches.has(c.benchmark)) throw new Error(`tornado: cell ${c.lever}/${c.benchmark} names an unknown lever or benchmark`);
  }
  const flagged = (r) => r.verdict !== 'ok' && r.verdict !== 'unchecked';
  const every = [];
  const panels = spec.benchmarks.map((bench) => {
    // a zero baseline has no percentage: that panel is in the benchmark's own unit and its dead band (`deadband`) is in that unit too
    const abs = Math.abs(bench.baseline) < ABS_EPS, band = abs ? bench.deadband ?? ABS_EPS : dead;
    const change = (v) => (abs ? v - bench.baseline : (100 * (v - bench.baseline)) / Math.abs(bench.baseline));
    const all = spec.cells.filter((c) => c.benchmark === bench.id).map((c) => {
      const l = levers.get(c.lever), dm = change(c.minus), dp = change(c.plus), swing = Math.max(Math.abs(dm), Math.abs(dp)), observed = sign(dp, band);
      return { lever: c.lever, name: l.name, order: l.order, dm, dp, swing, moves: swing >= band, observed, expected: c.expected ?? null, verdict: c.verdict ?? verdictOf(c.expected, observed) };
    });
    all.sort((a, b) => b.swing - a.swing || a.order - b.order);
    every.push(...all);
    const rows = all.filter((r, i) => (i < maxRows && r.moves) || flagged(r)), small = all.filter((r) => !r.moves && !flagged(r)).length;
    return { bench, abs, band, rows, hiddenSmall: small, hiddenBig: all.length - rows.length - small, moved: all.some((r) => r.moves), measured: all.length > 0 };
  });
  const movers = new Set(every.filter((r) => r.moves).map((r) => r.lever)), tried = new Set(every.map((r) => r.lever)), checked = every.filter((r) => r.verdict !== 'unchecked');
  return {
    spec, deadband: dead, panels,
    deadLevers: [...levers.keys()].filter((id) => tried.has(id) && !movers.has(id)).map((id) => levers.get(id).name),
    orphans: panels.filter((p) => p.measured && !p.moved).map((p) => p.bench),
    checks: { total: checked.length, right: checked.filter((r) => r.verdict === 'ok').length },
  };
}

// -------------------------------------------------------------------------------------------------------------------- ladder model

export function ladderModel(spec) {
  if (spec.schema !== 'w5k-ladder-1') throw new Error(`ladder: expected schema w5k-ladder-1, got ${spec.schema}`);
  const limit = spec.threshold?.value ?? null, tol = spec.tolerance_pct ?? null;
  if (!spec.vehicles?.length || spec.vehicles.length > THEMES.light.series.length) throw new Error(`ladder: needs 1 to ${THEMES.light.series.length} vehicles (the palette has that many validated colours); split a bigger field into two charts`);
  for (const v of spec.vehicles) if (!v.rungs?.length) throw new Error(`ladder: vehicle ${v.id} has no rungs`);
  const vehicles = spec.vehicles.map((v) => {
    const rungs = [...v.rungs].sort((a, b) => a.x - b.x);
    const bogIndex = rungs.findIndex((r) => r.bogged === true || (r.bogged === undefined && limit !== null && r.sim >= limit));
    const upTo = bogIndex < 0 ? rungs : rungs.slice(0, bogIndex + 1);
    const devs = upTo.filter((r) => r.theory > 0 && r.sim != null).map((r) => (100 * Math.abs(r.sim - r.theory)) / r.theory);
    return { id: v.id, name: v.name, rungs, bogIndex, maxDevPct: devs.length ? Math.max(...devs) : null };
  });
  const xs = vehicles.flatMap((v) => v.rungs.map((r) => r.x));
  const ys = vehicles.flatMap((v) => v.rungs.flatMap((r) => [r.sim, r.theory, tol ? r.theory * (1 + tol / 100) : 0]).filter((y) => y != null));
  const x0 = Math.min(...xs), x1 = Math.max(...xs), yTop = Math.max(...ys, limit ?? 0) * 1.08;
  return { spec, vehicles, tol, limit, x0, x1, ticksY: ticks(0, yTop, 5), yTop: Math.max(...ticks(0, yTop, 5), yTop) };
}

// ---------------------------------------------------------------------------------------------------------------------- 2D helpers

function text(ctx, s, x, y, { size = 12, weight = 400, color, align = 'left', base = 'alphabetic', halo = null } = {}) {
  ctx.font = `${weight} ${size}px ${FONT}`; ctx.textAlign = align; ctx.textBaseline = base;
  if (halo) { ctx.lineWidth = 4; ctx.lineJoin = 'round'; ctx.strokeStyle = halo; ctx.strokeText(s, x, y); }
  ctx.fillStyle = color; ctx.fillText(s, x, y);
  return ctx.measureText(s).width;
}
function fit(ctx, s, maxW, size) { // shorten with an ellipsis until it fits
  ctx.font = `400 ${size}px ${FONT}`;
  if (ctx.measureText(s).width <= maxW) return s;
  let t = s; while (t.length > 1 && ctx.measureText(t + '…').width > maxW) t = t.slice(0, -1);
  return t + '…';
}
/** A bar anchored (square) at x0 and rounded at its data end x1. A bar shorter than 2 px is drawn 2 px long so a measured zero still shows. */
function bar(ctx, x0, x1, y, h, r, fill) {
  const dir = x1 >= x0 ? 1 : -1, len = Math.max(Math.abs(x1 - x0), 2), rr = Math.min(r, h / 2, len), xe = x0 + dir * len;
  ctx.beginPath(); ctx.moveTo(x0, y); ctx.lineTo(xe - dir * rr, y); ctx.arcTo(xe, y, xe, y + rr, rr);
  ctx.lineTo(xe, y + h - rr); ctx.arcTo(xe, y + h, xe - dir * rr, y + h, rr); ctx.lineTo(x0, y + h); ctx.closePath();
  ctx.fillStyle = fill; ctx.fill();
}
function cross(ctx, x, y, r, color, width = 3) { // the "x" icon of a failed check (always paired with words)
  ctx.strokeStyle = color; ctx.lineWidth = width; ctx.lineCap = 'round';
  ctx.beginPath(); ctx.moveTo(x - r, y - r); ctx.lineTo(x + r, y + r); ctx.moveTo(x + r, y - r); ctx.lineTo(x - r, y + r); ctx.stroke();
}
function triangle(ctx, x, y, r, color) { // the "!" icon of a warning (always paired with words)
  ctx.fillStyle = color; ctx.beginPath(); ctx.moveTo(x, y - r); ctx.lineTo(x + r, y + r * 0.8); ctx.lineTo(x - r, y + r * 0.8); ctx.closePath(); ctx.fill();
}
function stubBanner(ctx, th, x, y) {
  triangle(ctx, x + 6, y, 6, STATUS.warning);
  return 18 + text(ctx, 'STUB DATA: invented numbers that follow the first-order table, to show the layout. Not measurements.', x + 18, y + 4, { size: 12, weight: 600, color: th.ink });
}

// -------------------------------------------------------------------------------------------------------------------- tornado draw

const T = { pad: 28, gapX: 30, gapY: 24, head: 124, labelW: 150, rowH: 24, barH: 9, pairGap: 2, panelHead: 46, axisH: 28, more: 16, foot: 96, labelRoom: 40, radius: 4 };
const bandText = (p) => (p.abs ? (p.band > ABS_EPS ? `${+p.band.toPrecision(2)} ${p.bench.unit}` : '0 (any change)') : `${p.band}%`);
const hasNote = (p) => p.hiddenSmall > 0 || p.hiddenBig > 0;
export function tornadoLayout(m, o = {}) {
  const n = m.panels.length, cols = o.cols ?? (n >= 10 ? 4 : n >= 5 ? 3 : Math.max(n, 1)), W = o.width ?? 1500;
  const pw = (W - 2 * T.pad - (cols - 1) * T.gapX) / cols, rowsCount = Math.ceil(n / cols), heights = [];
  for (let r = 0; r < rowsCount; r++) {
    const ps = m.panels.slice(r * cols, r * cols + cols);
    heights.push(T.panelHead + Math.max(1, ...ps.map((p) => p.rows.length)) * T.rowH + (ps.some(hasNote) ? T.more : 0) + T.axisH);
  }
  return { cols, W, pw, heights, H: T.head + heights.reduce((a, b) => a + b + T.gapY, 0) + T.foot };
}
export const tornadoSize = (m, o) => { const l = tornadoLayout(m, o); return { w: l.W, h: l.H }; };

export function drawTornado(ctx, m, o = {}) {
  const th = THEMES[o.theme ?? 'light'], lay = tornadoLayout(m, o), s = m.spec, pct = s.perturb_pct ?? 10, colours = LEVER_COLOURS.map((i) => th.series[i]);
  ctx.fillStyle = th.surface; ctx.fillRect(0, 0, lay.W, lay.H);
  text(ctx, `Design impact: how far each benchmark moves when one lever changes by ${MINUS}${pct}% and +${pct}%`, T.pad, 36, { size: 20, weight: 700, color: th.ink });
  text(ctx, `Baseline vehicle ${s.vehicle ?? ''}. One lever at a time, everything else held. Bars show the change in the benchmark as a percentage of its baseline; levers are sorted by the size of their effect.`, T.pad, 58, { size: 12, color: th.ink2 });
  let y = 82;
  if (s.stub) { stubBanner(ctx, th, T.pad, y); y += 24; }
  let lx = T.pad; // legend: two series, so a legend is always present; the bars are directly labelled as well
  [`lever ${MINUS}${pct}%`, `lever +${pct}%`].forEach((name, i) => { ctx.fillStyle = colours[i]; ctx.fillRect(lx, y - 8, 14, 9); lx += 20 + text(ctx, name, lx + 20, y, { size: 12, color: th.ink }) + 8; });
  cross(ctx, lx + 6, y - 4, 4, STATUS.critical, 2.4); lx += 20 + text(ctx, 'differs from the expected table', lx + 20, y, { size: 12, color: th.ink }) + 8;
  triangle(ctx, lx + 6, y - 4, 6, STATUS.serious); text(ctx, 'dead lever / orphan effect', lx + 20, y, { size: 12, color: th.ink });

  m.panels.forEach((p, i) => {
    const r = Math.floor(i / lay.cols), c = i % lay.cols, px = T.pad + c * (lay.pw + T.gapX);
    const py = T.head + lay.heights.slice(0, r).reduce((a, b) => a + b + T.gapY, 0), top = py + T.panelHead;
    const b = p.bench, unit = p.abs ? b.unit : '%', x0 = px + T.labelW, x1 = px + lay.pw - 8, zero = (x0 + x1) / 2, half = (x1 - x0) / 2;
    const biggest = Math.max(0, ...p.rows.map((q) => q.swing)), M = niceCeil(Math.max(biggest * half / (half - T.labelRoom), p.band * 2)), X = (v) => zero + (v / M) * half;
    text(ctx, fit(ctx, b.name, lay.pw, 14), px, py + 16, { size: 14, weight: 700, color: th.ink });
    text(ctx, `${b.id} · baseline ${fmtNum(b.baseline)} ${b.unit}`, px, py + 34, { size: 11, color: th.ink2 });
    if (p.measured && !p.moved) { const w = text(ctx, 'orphan effect: no lever moves this', px + lay.pw, py + 34, { size: 11, weight: 600, color: th.ink, align: 'right' }); triangle(ctx, px + lay.pw - w - 10, py + 30, 5, STATUS.serious); }
    if (!p.measured) text(ctx, 'not measured yet', px + lay.pw, py + 34, { size: 11, color: th.ink2, align: 'right' });
    const bottom = top + Math.max(1, p.rows.length) * T.rowH;
    ctx.lineWidth = 1; ctx.strokeStyle = th.grid;
    [-M, -M / 2, M / 2, M].forEach((v) => { ctx.beginPath(); ctx.moveTo(Math.round(X(v)) + 0.5, top - 4); ctx.lineTo(Math.round(X(v)) + 0.5, bottom + 2); ctx.stroke(); });
    ctx.strokeStyle = th.axis; ctx.lineWidth = 1.5; ctx.beginPath(); ctx.moveTo(X(0), top - 4); ctx.lineTo(X(0), bottom + 2); ctx.stroke();
    p.rows.forEach((q, k) => {
      const ry = top + k * T.rowH + (T.rowH - 2 * T.barH - T.pairGap) / 2, flagged = q.verdict !== 'ok' && q.verdict !== 'unchecked';
      bar(ctx, X(0), X(q.dm), ry, T.barH, T.radius, colours[0]);
      bar(ctx, X(0), X(q.dp), ry + T.barH + T.pairGap, T.barH, T.radius, colours[1]);
      const nameX = px + T.labelW - 10;
      text(ctx, fit(ctx, q.name, T.labelW - 26, 11), nameX, top + k * T.rowH + T.rowH / 2, { size: 11, color: th.ink, align: 'right', base: 'middle', weight: flagged ? 700 : 400 });
      if (flagged) cross(ctx, px + 5, top + k * T.rowH + T.rowH / 2, 4, STATUS.critical, 2.4);
      if (k < 3 || flagged) { // selective direct labels: the three biggest levers and any failed check
        [[q.dm, ry], [q.dp, ry + T.barH + T.pairGap]].forEach(([v, by]) => {
          if (Math.abs(v) < p.band) return;
          text(ctx, p.abs ? signed(v, 2) : fmtPct(v), v < 0 ? X(v) - 5 : X(v) + 5, by + T.barH / 2, { size: 10, color: th.ink2, align: v < 0 ? 'right' : 'left', base: 'middle' });
        });
      }
    });
    if (!p.rows.length) text(ctx, p.measured ? `no lever moves this by more than ${bandText(p)}` : 'no cells yet', zero, top + T.rowH / 2, { size: 11, color: th.ink2, align: 'center', base: 'middle' });
    const note = [p.hiddenBig ? `${p.hiddenBig} smaller effects` : '', p.hiddenSmall ? `${p.hiddenSmall} levers under ${bandText(p)}` : ''].filter(Boolean).join(' and ');
    if (note) text(ctx, 'not drawn: ' + note, px + T.labelW, bottom + 14, { size: 10, color: th.muted });
    const ay = bottom + (hasNote(p) ? T.more : 0) + 16;
    [-M, 0, M].forEach((v) => text(ctx, v === 0 ? '0' : (v < 0 ? MINUS : '+') + +Math.abs(v).toPrecision(3) + (p.abs ? '' : '%'), X(v), ay, { size: 10, color: th.muted, align: 'center' }));
    if (p.abs) text(ctx, `change in ${unit}`, x1, ay + 12, { size: 10, color: th.muted, align: 'right' });
  });

  const fy = lay.H - T.foot + 28, dead = m.deadLevers.length ? m.deadLevers.join(', ') : 'none', orph = m.orphans.length ? m.orphans.map((b) => `${b.id} ${b.name}`).join(', ') : 'none';
  const line = (k, label, body, icon) => { if (icon) triangle(ctx, T.pad + 6, fy + k * 20 - 4, 5, STATUS.serious); const w = text(ctx, label, T.pad + 18, fy + k * 20, { size: 12, weight: 700, color: th.ink }); text(ctx, body, T.pad + 18 + w + 6, fy + k * 20, { size: 12, color: th.ink }); };
  line(0, 'Dead levers (move no benchmark by more than ' + m.deadband + '%):', dead, m.deadLevers.length > 0);
  line(1, 'Orphan effects (no lever moves them by more than ' + m.deadband + '%):', orph, m.orphans.length > 0);
  if (m.checks.total) text(ctx, `Expected signs right: ${m.checks.right} of ${m.checks.total} (${Math.round((100 * m.checks.right) / m.checks.total)}%). The acceptance bar is 80%.`, T.pad + 18, fy + 40, { size: 12, color: th.ink });
}

// ---------------------------------------------------------------------------------------------------------------------- ladder draw

const LAD = { l: 76, r: 190, top: 152, bottom: 70, noteH: 20, radius: 5 };
export const ladderSize = (m, o = {}) => ({ w: o.width ?? 1000, h: (o.height ?? 560) + ladderNotes(m).length * LAD.noteH });
function ladderNotes(m) {
  return m.vehicles.map((v) => {
    const where = v.bogIndex >= 0 ? `bogs at ${v.rungs[v.bogIndex].x} ${m.spec.x.unit}` : 'never bogs on this ladder';
    if (v.maxDevPct === null) return { ok: null, text: `${v.name}: ${where}.` };
    const within = m.tol === null || v.maxDevPct <= m.tol;
    return { ok: within, text: `${v.name}: simulation within ${v.maxDevPct.toFixed(0)}% of the prediction up to ${v.bogIndex >= 0 ? 'its bog point' : 'the last rung'}${m.tol === null ? '' : within ? ` (acceptance ±${m.tol}%)` : `, outside the acceptance band of ±${m.tol}%`}; ${where}.` };
  });
}

export function drawLadder(ctx, m, o = {}) {
  const th = THEMES[o.theme ?? 'light'], s = m.spec, { w, h } = ladderSize(m, o), notes = ladderNotes(m);
  const pl = LAD.l, pr = w - LAD.r, pt = LAD.top, pb = h - LAD.bottom - notes.length * LAD.noteH;
  const pad = (m.x1 - m.x0) * 0.04 || 1, X = (v) => pl + ((v - (m.x0 - pad)) / (m.x1 - m.x0 + 2 * pad)) * (pr - pl), Y = (v) => pb - (v / m.yTop) * (pb - pt);
  ctx.fillStyle = th.surface; ctx.fillRect(0, 0, w, h);
  text(ctx, s.title ?? 'Sinkage up the ladder', LAD.l, 36, { size: 20, weight: 700, color: th.ink });
  text(ctx, s.subtitle ?? `Sinkage in the mud as the ${s.x.name} climbs, beside the soil theory (Bekker) for the same ground and footprint.`, LAD.l, 58, { size: 12, color: th.ink2 });
  let y = 82;
  if (s.stub) { stubBanner(ctx, th, LAD.l, y); y += 24; }
  let lx = LAD.l; // legend: vehicle colours first (identity), then the line styles (measured or theory)
  m.vehicles.forEach((v, i) => { ctx.fillStyle = th.series[i]; ctx.fillRect(lx, y - 8, 14, 9); lx += 20 + text(ctx, v.name, lx + 20, y, { size: 12, color: th.ink }) + 10; });
  lx += 14; ctx.strokeStyle = th.ink2; ctx.lineWidth = 2; ctx.beginPath(); ctx.moveTo(lx, y - 4); ctx.lineTo(lx + 24, y - 4); ctx.stroke();
  ctx.fillStyle = th.ink2; ctx.beginPath(); ctx.arc(lx + 12, y - 4, 4, 0, 7); ctx.fill(); lx += 30 + text(ctx, 'simulated', lx + 30, y, { size: 12, color: th.ink }) + 12;
  ctx.setLineDash([5, 4]); ctx.beginPath(); ctx.moveTo(lx, y - 4); ctx.lineTo(lx + 24, y - 4); ctx.stroke(); ctx.setLineDash([]);
  lx += 30 + text(ctx, 'soil theory (Bekker)', lx + 30, y, { size: 12, color: th.ink }) + 12;
  if (m.tol !== null) { ctx.globalAlpha = 0.25; ctx.fillStyle = th.ink2; ctx.fillRect(lx, y - 9, 24, 11); ctx.globalAlpha = 1; text(ctx, `acceptance band ±${m.tol}%`, lx + 30, y, { size: 12, color: th.ink }); }

  ctx.lineWidth = 1; ctx.strokeStyle = th.grid; m.ticksY.forEach((v) => { ctx.beginPath(); ctx.moveTo(pl, Math.round(Y(v)) + 0.5); ctx.lineTo(pr, Math.round(Y(v)) + 0.5); ctx.stroke(); text(ctx, String(v), pl - 8, Y(v), { size: 11, color: th.muted, align: 'right', base: 'middle' }); });
  ctx.strokeStyle = th.axis; ctx.lineWidth = 1.5; ctx.beginPath(); ctx.moveTo(pl, pb); ctx.lineTo(pr, pb); ctx.stroke();
  const xt = ticks(m.x0, m.x1, 8); xt.forEach((v) => text(ctx, String(v), X(v), pb + 18, { size: 11, color: th.muted, align: 'center' }));
  text(ctx, `${s.x.name} (${s.x.unit})`, (pl + pr) / 2, pb + 40, { size: 12, color: th.ink2, align: 'center' });
  text(ctx, `${s.y.name} (${s.y.unit})`, pl, pt - 14, { size: 12, color: th.ink2 });

  if (m.tol !== null) { // the acceptance band around each theory curve, under every line
    m.vehicles.forEach((v, i) => {
      ctx.globalAlpha = 0.12; ctx.fillStyle = th.series[i]; ctx.beginPath();
      v.rungs.forEach((r, k) => (k ? ctx.lineTo(X(r.x), Y(r.theory * (1 + m.tol / 100))) : ctx.moveTo(X(r.x), Y(r.theory * (1 + m.tol / 100)))));
      [...v.rungs].reverse().forEach((r) => ctx.lineTo(X(r.x), Y(r.theory * (1 - m.tol / 100)))); ctx.closePath(); ctx.fill(); ctx.globalAlpha = 1;
    });
  }
  if (m.limit !== null) { // the bog line, and the ground above it where the vehicle no longer moves
    ctx.globalAlpha = 0.05; ctx.fillStyle = STATUS.critical; ctx.fillRect(pl, pt, pr - pl, Y(m.limit) - pt); ctx.globalAlpha = 1;
    ctx.setLineDash([6, 4]); ctx.strokeStyle = th.ink2; ctx.lineWidth = 1.25; ctx.beginPath(); ctx.moveTo(pl, Y(m.limit)); ctx.lineTo(pr, Y(m.limit)); ctx.stroke(); ctx.setLineDash([]);
    const label = `${s.threshold.name ?? 'bogged'}: sinkage above ${m.limit} ${s.y.unit}`;
    const lw = text(ctx, label, pr - 8, Y(m.limit) - 9, { size: 12, color: th.ink, align: 'right', halo: th.surface });
    cross(ctx, pr - 8 - lw - 12, Y(m.limit) - 13, 4, STATUS.critical, 2.4);
  }
  const labels = [], bogs = [];
  m.vehicles.forEach((v, i) => {
    const col = th.series[i], sims = v.rungs.filter((r) => r.sim != null), last = v.rungs[v.rungs.length - 1];
    ctx.strokeStyle = col; ctx.lineWidth = 2; ctx.setLineDash([6, 4]); ctx.beginPath(); v.rungs.forEach((r, k) => (k ? ctx.lineTo(X(r.x), Y(r.theory)) : ctx.moveTo(X(r.x), Y(r.theory)))); ctx.stroke(); ctx.setLineDash([]);
    ctx.beginPath(); sims.forEach((r, k) => (k ? ctx.lineTo(X(r.x), Y(r.sim)) : ctx.moveTo(X(r.x), Y(r.sim)))); ctx.stroke();
    sims.forEach((r) => { ctx.fillStyle = col; ctx.strokeStyle = th.surface; ctx.lineWidth = 2; ctx.beginPath(); ctx.arc(X(r.x), Y(r.sim), LAD.radius, 0, 7); ctx.fill(); ctx.stroke(); });
    labels.push({ x: X(last.x) + 22, y: Y(last.theory), text: `${v.name} theory`, col }, { x: X(last.x) + 22, y: Y(sims[sims.length - 1].sim), text: `${v.name} simulated`, col });
    if (v.bogIndex >= 0) { const r = v.rungs[v.bogIndex]; bogs.push({ x: X(r.x), y: Y(r.sim ?? m.limit), text: `${v.name} bogs at ${r.x} ${s.x.unit}` }); }
  });
  bogs.forEach((b) => { cross(ctx, b.x, b.y, 7, th.surface, 7); cross(ctx, b.x, b.y, 7, STATUS.critical, 3.5); });
  const nudge = (list, gap, near) => list.sort((a, b) => a.y - b.y).forEach((e, k) => { for (let j = 0; j < k; j++) if (Math.abs(list[j].x - e.x) < near && e.y < list[j].y + gap) e.y = list[j].y + gap; });
  nudge(labels, 15, 140);
  labels.forEach((e) => { ctx.fillStyle = e.col; ctx.fillRect(e.x - 12, e.y - 4, 8, 8); text(ctx, e.text, e.x, e.y, { size: 12, color: th.ink, base: 'middle', halo: th.surface }); });
  const stack = bogs.map((b) => ({ x: b.x, y: b.y - 6, text: b.text })); nudge(stack, 17, 400);
  stack.forEach((b) => text(ctx, b.text, b.x - 14, b.y, { size: 12, weight: 700, color: th.ink, align: 'right', base: 'middle', halo: th.surface }));
  notes.forEach((n, k) => {
    const ny = pb + 66 + k * LAD.noteH;
    if (n.ok === true) { ctx.strokeStyle = STATUS.good; ctx.lineWidth = 2.4; ctx.beginPath(); ctx.moveTo(LAD.l, ny - 4); ctx.lineTo(LAD.l + 4, ny); ctx.lineTo(LAD.l + 11, ny - 9); ctx.stroke(); }
    if (n.ok === false) cross(ctx, LAD.l + 5, ny - 4, 4, STATUS.critical, 2.4);
    text(ctx, n.text, LAD.l + 20, ny, { size: 12, color: th.ink });
  });
}
