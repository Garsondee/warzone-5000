// Scope plots under the picture: time series of the whole replay with a cursor synced to playback; click or drag to seek.
const SERIES = [
  { name: 'speed km/h', color: '#5cc8ff', f: (v) => Math.hypot(v.lin_vel_m_s.x, v.lin_vel_m_s.y, v.lin_vel_m_s.z) * 3.6 },
  { name: 'rpm', color: '#ffb347', f: (v) => v.engine_rpm },
  { name: 'gear', color: '#c4a3ff', f: (v) => v.gear, step: true },
  { name: 'normal force kN', color: '#2ee6a8', f: (v) => v.contacts.reduce((s, c) => s + c.normal_force_n, 0) / 1000 },
];

export function makeScope(canvas, replay, onSeek, colors = []) {
  const ctx = canvas.getContext('2d'), n = replay.frames.length, dt = replay.header.frame_dt_s;
  const data = SERIES.map((s) => replay.frames.map((f) => s.f(f.vehicles[0])));
  const range = data.map((d) => [Math.min(...d), Math.max(...d)]);
  // With several vehicles the first row draws every vehicle's speed in its accent colour (the other rows follow vehicle 0).
  const speeds = replay.header.vehicles.length > 1 ? replay.header.vehicles.map((_, vi) => replay.frames.map((f) => SERIES[0].f(f.vehicles[vi]))) : null;
  if (speeds) range[0] = [Math.min(...speeds.map((d) => Math.min(...d))), Math.max(...speeds.map((d) => Math.max(...d)))];
  let cursor = 0;
  const seek = (e) => { const r = canvas.getBoundingClientRect(); onSeek(Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)) * (n - 1) * dt); };
  canvas.onpointerdown = (e) => { canvas.setPointerCapture(e.pointerId); seek(e); canvas.onpointermove = seek; };
  canvas.onpointerup = () => { canvas.onpointermove = null; };
  function draw(t) {
    cursor = t;
    const W = canvas.width, H = canvas.height, rowH = H / SERIES.length;
    ctx.clearRect(0, 0, W, H);
    SERIES.forEach((s, k) => {
      const [lo, hi] = range[k], y0 = k * rowH, sy = (v) => y0 + rowH - 4 - ((v - lo) / (hi - lo || 1)) * (rowH - 10);
      const lines = k === 0 && speeds ? speeds : [data[k]];
      lines.forEach((line, li) => {
        ctx.strokeStyle = k === 0 && speeds ? colors[li] ?? s.color : s.color; ctx.lineWidth = 1.5; ctx.beginPath();
        for (let i = 0; i < n; i++) { const x = (i / (n - 1)) * W, y = sy(line[i]); if (i === 0) ctx.moveTo(x, y); else { if (s.step) ctx.lineTo(x, sy(line[i - 1])); ctx.lineTo(x, y); } }
        ctx.stroke();
      });
      ctx.fillStyle = s.color; ctx.font = '11px system-ui';
      ctx.fillText(`${s.name}  ${hi.toFixed(hi < 10 ? 1 : 0)}`, 4, y0 + 11);
      ctx.strokeStyle = '#ffffff22'; ctx.beginPath(); ctx.moveTo(0, y0 + rowH); ctx.lineTo(W, y0 + rowH); ctx.stroke();
    });
    const x = (cursor / ((n - 1) * dt)) * W;
    ctx.strokeStyle = '#fff'; ctx.lineWidth = 1; ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, H); ctx.stroke();
  }
  return { draw };
}
