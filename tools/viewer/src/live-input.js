// Pedals and steering from the keyboard (arrow keys, WASD, space), the on-screen buttons (touch or mouse) and a gamepad, merged into the
// single input the server takes: { throttle, brake, steer, reverse }. The server replaces the whole input with each message, so this
// returns the complete picture every time.
const KEYS = {
  go: ['ArrowUp', 'KeyW'], down: ['ArrowDown', 'KeyS'], left: ['ArrowLeft', 'KeyA'], right: ['ArrowRight', 'KeyD'], stop: ['Space'],
};
const STEER_RATE = 3.0, STEER_RETURN = 4.5; // per second, towards the target / back to centre: a gentle wheel, not a switch
const STOPPED_MPS = 1.0, REVERSE_DWELL_S = 0.35; // holding "down" brakes, and once stopped for a moment, backs up

export function makeInput({ onReset, onCamera, onGarage, onMute, onFirstGesture }) {
  const keys = new Set(), pads = { go: false, stop: false, back: false, left: false, right: false };
  let steer = 0, stoppedFor = 0, gpReset = false, gpCamera = false;
  const down = (name) => KEYS[name].some((k) => keys.has(k));
  const gesture = () => onFirstGesture && onFirstGesture();
  addEventListener('keydown', (e) => {
    if (e.repeat) { if (Object.values(KEYS).flat().includes(e.code)) e.preventDefault(); return; }
    gesture();
    if (Object.values(KEYS).flat().includes(e.code)) { keys.add(e.code); e.preventDefault(); return; }
    if (e.code === 'KeyR') onReset();
    else if (e.code === 'KeyC') onCamera();
    else if (e.code === 'KeyG' || e.code === 'Escape') onGarage();
    else if (e.code === 'KeyM') onMute();
  });
  addEventListener('keyup', (e) => keys.delete(e.code));
  addEventListener('blur', () => { keys.clear(); for (const k of Object.keys(pads)) pads[k] = false; });
  addEventListener('pointerdown', gesture, { capture: true });

  // On-screen button: held while the pointer is down on it (touch, mouse or pen), released on up, cancel or leaving.
  function bindHold(el, name) {
    const set = (v) => { pads[name] = v; el.classList.toggle('held', v); };
    el.addEventListener('pointerdown', (e) => { e.preventDefault(); el.setPointerCapture(e.pointerId); set(true); });
    for (const t of ['pointerup', 'pointercancel', 'lostpointercapture']) el.addEventListener(t, () => set(false));
    el.addEventListener('contextmenu', (e) => e.preventDefault());
  }
  const bindTap = (el, fn) => { el.addEventListener('pointerdown', (e) => { e.preventDefault(); fn(); }); el.addEventListener('contextmenu', (e) => e.preventDefault()); };

  function gamepad() {
    const gp = (navigator.getGamepads ? [...navigator.getGamepads()] : []).find((g) => g && g.connected);
    if (!gp) return { throttle: 0, brake: 0, steer: 0, back: false };
    const b = (i) => (gp.buttons[i] ? gp.buttons[i].value : 0), ax = gp.axes[0] ?? 0;
    const reset = b(3) > 0.5, cam = b(2) > 0.5; // Y resets, X flips the camera
    if (reset && !gpReset) onReset();
    if (cam && !gpCamera) onCamera();
    gpReset = reset; gpCamera = cam;
    if (b(7) + b(6) + b(0) + Math.abs(ax) > 0.1) gesture(); // const-ok: any pad activity counts as the first gesture
    return { throttle: b(7), brake: b(6), steer: Math.abs(ax) < 0.12 ? 0 : ax, back: b(0) > 0.5 }; // const-ok: stick dead zone
  }

  // dt in seconds; speed in m/s along the nose (negative when backing up). Returns the full input.
  function read(dt, speed) {
    const pad = gamepad();
    const downHeld = down('down');
    stoppedFor = downHeld && Math.abs(speed) < STOPPED_MPS ? stoppedFor + dt : 0;
    const smartBrake = downHeld && !(stoppedFor > REVERSE_DWELL_S || speed < -STOPPED_MPS);
    const smartBack = downHeld && !smartBrake;
    const back = pads.back || pad.back || smartBack;
    let throttle = Math.max(down('go') || pads.go ? 1 : 0, pad.throttle);
    const brake = Math.max(down('stop') || pads.stop ? 1 : 0, smartBrake ? 1 : 0, pad.brake);
    if (back) throttle = Math.max(throttle, 1);
    const target = Math.max(-1, Math.min(1, (down('right') || pads.right ? 1 : 0) - (down('left') || pads.left ? 1 : 0) + pad.steer));
    const rate = (target === 0 || Math.sign(target) !== Math.sign(steer) ? STEER_RETURN : STEER_RATE) * dt;
    steer += Math.max(-rate, Math.min(rate, target - steer));
    return { throttle, brake: back ? 0 : brake, steer: Math.round(steer * 100) / 100, reverse: back };
  }
  return { read, bindHold, bindTap, pads };
}
