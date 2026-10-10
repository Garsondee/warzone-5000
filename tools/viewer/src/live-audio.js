// Engine sound from the engine speed (WebAudio). Browsers only allow sound after a key press or a click, so `start()` is called from
// the first one. A four-cylinder engine fires twice per turn, so the fundamental is rpm / 30 Hz; a sawtooth carries the overtones that
// small speakers can reproduce, a low-pass filter opens with revs and throttle, and a little noise gives it grit.
export function makeEngineSound() {
  let ctx = null, n = null, muted = false;
  const start = () => {
    if (ctx) { if (ctx.state === 'suspended') ctx.resume(); return; }
    const AC = window.AudioContext || window.webkitAudioContext;
    if (!AC) return;
    ctx = new AC();
    const master = ctx.createGain(); master.gain.value = muted ? 0 : 1; master.connect(ctx.destination);
    const filter = ctx.createBiquadFilter(); filter.type = 'lowpass'; filter.frequency.value = 500; filter.Q.value = 1.4;
    const engine = ctx.createGain(); engine.gain.value = 0;
    filter.connect(engine); engine.connect(master);
    const osc = (type, gain) => { const o = ctx.createOscillator(); o.type = type; const g = ctx.createGain(); g.gain.value = gain; o.connect(g); g.connect(filter); o.start(); return o; };
    const saw = osc('sawtooth', 0.6), octave = osc('square', 0.18), sub = osc('triangle', 0.5);
    const noiseBuf = ctx.createBuffer(1, ctx.sampleRate, ctx.sampleRate), d = noiseBuf.getChannelData(0);
    for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1; // const-ok: unit-range noise
    const noise = ctx.createBufferSource(); noise.buffer = noiseBuf; noise.loop = true;
    const nf = ctx.createBiquadFilter(); nf.type = 'bandpass'; nf.frequency.value = 600; nf.Q.value = 0.7;
    const ng = ctx.createGain(); ng.gain.value = 0;
    noise.connect(nf); nf.connect(ng); ng.connect(master); noise.start();
    n = { master, filter, engine, saw, octave, sub, ng };
  };
  return {
    start,
    // rpm: engine speed; throttle 0..1: the pedal as pressed; speed in m/s
    update({ rpm, throttle, speed }) {
      if (!ctx || !n) return;
      const t = ctx.currentTime, f = Math.max(18, Math.min(220, rpm / 30)); // const-ok: Hz range of a small engine
      n.saw.frequency.setTargetAtTime(f, t, 0.05);
      n.octave.frequency.setTargetAtTime(f * 2, t, 0.05); // const-ok: octave
      n.sub.frequency.setTargetAtTime(f * 0.5, t, 0.05); // const-ok: octave
      n.filter.frequency.setTargetAtTime(260 + rpm * 0.35 + throttle * 700, t, 0.08);
      n.engine.gain.setTargetAtTime(0.07 + 0.09 * throttle + Math.min(rpm, 5000) / 5000 * 0.05, t, 0.08);
      n.ng.gain.setTargetAtTime(0.01 + 0.03 * throttle + Math.min(Math.abs(speed), 20) / 20 * 0.02, t, 0.1); // wind and tyre hiss
    },
    chime() { // two friendly notes for "back on the road"
      if (!ctx || muted) return;
      [660, 880].forEach((hz, i) => { // const-ok: musical notes (E5, A5)
        const o = ctx.createOscillator(), g = ctx.createGain(), t0 = ctx.currentTime + i * 0.16;
        o.type = 'sine'; o.frequency.value = hz;
        g.gain.setValueAtTime(0.0001, t0); g.gain.exponentialRampToValueAtTime(0.25, t0 + 0.02); g.gain.exponentialRampToValueAtTime(0.0001, t0 + 0.5);
        o.connect(g); g.connect(n ? n.master : ctx.destination); o.start(t0); o.stop(t0 + 0.55);
      });
    },
    setMuted(m) { muted = m; if (n) n.master.gain.setTargetAtTime(m ? 0 : 1, ctx.currentTime, 0.05); },
    get muted() { return muted; },
    get state() { return ctx ? ctx.state : 'none'; },
    suspend() { if (ctx && ctx.state === 'running') ctx.suspend(); },
    resume() { if (ctx && ctx.state === 'suspended') ctx.resume(); },
  };
}
