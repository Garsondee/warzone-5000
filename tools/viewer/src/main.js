import { buildRig, applyVehicle, poseRig, sample, makeScene } from './viewer.js';
import { decodeReplay } from './replay.js';
import { makeDebug, updateHud } from './debug.js';
import { makeScope } from './scope.js';
import { makeLook } from './look.js';
import * as THREE from 'three';

const { rig, replay: b64, look: lookData, terrain } = JSON.parse(document.getElementById('data').textContent);
const replay = decodeReplay(Uint8Array.from(atob(b64), (c) => c.charCodeAt(0)));
const canvas = document.getElementById('c');
const W = canvas.clientWidth || innerWidth, H = canvas.clientHeight || innerHeight;
const { renderer, scene, camera } = makeScene(canvas, W, H, terrain);
camera.setViewOffset(W, H, 0, 110, W, H); // lift the picture above the scope panel
const built = buildRig(rig);
scene.add(built.root);
// Camo: the livery in the replay header (scheme id, seed) if there is one, else the first scheme.
const livery = replay.header.vehicles[0].livery;
const camo = makeLook(lookData, built.paint);
camo.set({ scheme: livery && camo.schemes.includes(livery.camo) ? livery.camo : camo.schemes.includes('woodland') ? 'woodland' : camo.schemes[0], seed: livery ? Number(BigInt.asUintN(32, BigInt(livery.seed))) : 1 });
const schemeEl = document.getElementById('scheme'), seedEl = document.getElementById('seed'), camoEl = document.getElementById('t_camo');
camo.schemes.forEach((s) => schemeEl.add(new Option(s, s)));
schemeEl.value = camo.state.scheme; seedEl.value = camo.state.seed;
const relook = () => { camo.set({ scheme: schemeEl.value, seed: Number(seedEl.value) || 0, on: camoEl.checked }); renderAt(t); };
schemeEl.onchange = seedEl.onchange = camoEl.onchange = relook;
// Steered wheels: joints named "<station>.steer"; the station name ends in l or r for the side.
const steerJoints = replay.header.vehicles[0].joint_names.map((n, i) => ({ n, i })).filter((j) => j.n.endsWith('.steer')).map((j) => ({ i: j.i, side: /l\.steer$/.test(j.n) ? 'L' : /r\.steer$/.test(j.n) ? 'R' : '?' }));
const debug = makeDebug(scene, built, replay.header);
const scope = makeScope(document.getElementById('scope'), replay, (tt) => { t = tt; renderAt(t); });
for (const k of Object.keys(debug.flags)) document.getElementById('t_' + k).onchange = (e) => { debug.flags[k] = e.target.checked; renderAt(t); };
const duration = (replay.frames.length - 1) * replay.header.frame_dt_s;

// Camera state: orbit (drag to turn, wheel to zoom) around the vehicle, or chase (behind the hull, follows its heading).
let freeze = false;
const cam = { mode: 'orbit', yaw: 0.6, pitch: 0.35, quarter: 0.75, front: 2.35, /* front-quarter: the steered wheels are the near ones */ /* rear-quarter: 43 degrees round from straight behind */ dist: Math.max(6, 3.2 * built.radius) }; // frame the whole vehicle
const look = new THREE.Vector3(), eye = new THREE.Vector3(), back = new THREE.Vector3();
// Followed heading: the direction of travel over the last 0.8 s (a deterministic function of time, so scrubbing and recording agree),
// not the hull's own yaw, which pitches and wobbles over bumps. Look-at height is averaged over +-0.5 s for the same reason.
const FOLLOW_S = 0.8, SMOOTH_S = 0.5;
const posAt = (tt) => { const s = sample(replay, Math.min(Math.max(tt, 0), duration)); return new THREE.Vector3(s.f0.vehicles[0].pos_m.x, s.f0.vehicles[0].pos_m.y, s.f0.vehicles[0].pos_m.z).lerp(new THREE.Vector3(s.f1.vehicles[0].pos_m.x, s.f1.vehicles[0].pos_m.y, s.f1.vehicles[0].pos_m.z), s.a); };
let clockT = 0;

// RTS camera: high-angle strategy view, heading-up. It follows the centroid of every vehicle in the replay (positions averaged
// over +-0.5 s) and turns with the direction the centroid travels over the last 2.5 s (a long window so the picture does not swing in
// bends); it aims 25 m ahead of the centroid and backs off until the vehicles and about 60 m of road ahead fit. All of it is a
// function of the replay time only, so scrubbing and recording agree.
const RTS_PITCH = 0.96, RTS_HEADING_S = 2.5, RTS_AHEAD_M = 60;
const centroidAt = (tt) => {
  const s = sample(replay, Math.min(Math.max(tt, 0), duration)), c = new THREE.Vector3();
  for (const v of s.f0.vehicles) c.add(new THREE.Vector3(v.pos_m.x, v.pos_m.y, v.pos_m.z));
  return c.multiplyScalar(1 / s.f0.vehicles.length);
};
const spreadAt = (tt, c) => {
  const s = sample(replay, Math.min(Math.max(tt, 0), duration));
  return Math.max(0, ...s.f0.vehicles.map((v) => Math.hypot(v.pos_m.x - c.x, v.pos_m.z - c.z)));
};
function placeRts() {
  const c = centroidAt(clockT - 0.5).add(centroidAt(clockT)).add(centroidAt(clockT + 0.5)).multiplyScalar(1 / 3);
  let d = centroidAt(clockT).sub(centroidAt(clockT - RTS_HEADING_S));
  if (d.x * d.x + d.z * d.z < 0.04) d = centroidAt(clockT + RTS_HEADING_S).sub(centroidAt(clockT)); // standing still: look where it goes next
  const h = new THREE.Vector3(d.x, 0, d.z);
  if (h.lengthSq() < 0.04) h.set(0, 0, -1); else h.normalize();
  const dist = Math.min(220, Math.max(45, 0.9 * (RTS_AHEAD_M + 10 + 2 * spreadAt(clockT, c))));
  look.copy(c).addScaledVector(h, 25);
  eye.copy(look).addScaledVector(h, -dist * Math.cos(RTS_PITCH)).add({ x: 0, y: dist * Math.sin(RTS_PITCH), z: 0 });
  camera.position.copy(eye);
  camera.lookAt(look);
}
function placeCamera() {
  const p = built.root.position;
  if (cam.focus) built.nodes.find((n) => n.def.name === cam.focus).g.getWorldPosition(look);
  else look.copy(p).add({ x: 0, y: 1, z: 0 });
  let yaw = cam.yaw;
  if (cam.mode === 'rts') return placeRts();
  if (cam.mode === 'chase' || cam.mode === 'quarter' || cam.mode === 'front') {
    const d = posAt(clockT).sub(posAt(clockT - FOLLOW_S));
    if (d.x * d.x + d.z * d.z > 0.01) back.set(-d.x, 0, -d.z).normalize(); // behind the direction of travel
    else back.set(0, 0, 1).applyQuaternion(built.root.quaternion);
    yaw = Math.atan2(back.x, back.z) + (cam.mode === 'quarter' ? cam.quarter : cam.mode === 'front' ? cam.front : 0);
    const mean = (posAt(clockT - SMOOTH_S).y + posAt(clockT).y + posAt(clockT + SMOOTH_S).y) / 3;
    look.set(p.x, mean + 1, p.z);
  }
  const c = Math.cos(cam.pitch);
  eye.set(look.x + cam.dist * c * Math.sin(yaw), look.y + cam.dist * Math.sin(cam.pitch), look.z + cam.dist * c * Math.cos(yaw));
  camera.position.copy(eye);
  camera.lookAt(look);
}

function renderAt(t) {
  clockT = t;
  const s = sample(replay, Math.min(Math.max(t, 0), duration));
  applyVehicle(built, s);
  if (freeze) { built.root.position.set(0, 0, 0); built.root.quaternion.identity(); }
  placeCamera();
  debug.update(s.a < 0.5 ? s.f0 : s.f1);
  updateHud(document.getElementById('hud'), document.getElementById('ledger'), s.a < 0.5 ? s.f0 : s.f1, t, steerJoints);
  scope.draw(t);
  renderer.render(scene, camera);
}

// Playback
const ui = (id) => document.getElementById(id);
let t = 0, playing = true, last = performance.now();
ui('play').onclick = () => { playing = !playing; ui('play').textContent = playing ? 'pause' : 'play'; };
ui('scrub').oninput = (e) => { t = e.target.value * duration; renderAt(t); };
ui('cam').onchange = (e) => { cam.mode = e.target.value; };
const setLayout = (mode) => {
  document.body.className = mode === 'full' ? '' : mode;
  ui('layout').value = mode;
  if (mode !== 'full') { debug.flags.com = false; ui('t_com').checked = false; } // recordings: no datum marker over the paint
};
ui('layout').onchange = (e) => setLayout(e.target.value);
let drag = null;
canvas.onpointerdown = (e) => { drag = e; canvas.setPointerCapture(e.pointerId); };
canvas.onpointerup = () => { drag = null; };
canvas.onpointermove = (e) => { if (drag) { cam.yaw -= (e.clientX - drag.clientX) * 0.01; cam.pitch = Math.min(1.4, Math.max(0.02, cam.pitch + (e.clientY - drag.clientY) * 0.01)); drag = e; } };
canvas.onwheel = (e) => { cam.dist = Math.min(80, Math.max(3, cam.dist * (e.deltaY > 0 ? 1.1 : 0.9))); e.preventDefault(); };
addEventListener('keydown', (e) => { if (e.key === ' ') ui('play').click(); });
function tick(now) {
  if (playing) t = (t + ((now - last) / 1000) * Number(ui('speed').value)) % duration;
  last = now;
  ui('scrub').value = t / duration;
  ui('clock').textContent = `${t.toFixed(1)} / ${duration.toFixed(1)} s`;
  renderAt(t);
  requestAnimationFrame(tick);
}

// Scripted entry points: headless capture and the smoke test render exact times and never depend on the browser's frame rate.
const gl = renderer.getContext();
const poseOf = (name) => { const n = built.nodes.find((x) => x.def.name === name); return n && { p: n.g.position.toArray(), q: n.g.quaternion.toArray() }; };
// Smoke-test hooks. `freeze` pins the hull at the origin so only articulation can change pixels; `only` shows one subtree.
const pixels = () => { const px = new Uint8Array(W * H * 4); gl.readPixels(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight, gl.RGBA, gl.UNSIGNED_BYTE, px); return px; };
function only(names) {
  const under = (i) => { for (let k = i; k != null; k = built.nodes[k].def.parent) if (names.includes(built.nodes[k].def.name)) return true; return false; };
  built.nodes.forEach((n, i) => n.g.children.forEach((c) => { if (c.isMesh) c.visible = !names.length || under(i); }));
}
// Pose the rig with every joint at its t=0 value except `name`'s, which takes `value`, and count pixels that differ from the
// all-t=0 pose: this proves that joint (and nothing else) moves the picture.
function jointPixels(name, value) {
  const node = built.nodes.find((x) => x.def.name === name);
  const base = sample(replay, 0).f0.vehicles[0].joints.slice();
  const shot = (j) => { applyVehicle(built, sample(replay, 0)); built.root.position.set(0, 0, 0); built.root.quaternion.identity(); poseRig(built, j); placeCamera(); renderer.render(scene, camera); return pixels(); };
  const a = shot(base);
  const moved = base.slice();
  moved[node.def.joint.index] = value;
  const b = shot(moved);
  let n = 0;
  for (let i = 0; i < a.length; i += 4) if (Math.abs(a[i] - b[i]) + Math.abs(a[i + 1] - b[i + 1]) + Math.abs(a[i + 2] - b[i + 2]) > 24) n++;
  return n;
}
const meshCount = (name) => { const i = built.nodes.findIndex((x) => x.def.name === name); const kids = (k) => [k, ...built.nodes.flatMap((n, j) => (n.def.parent === k ? kids(j) : []))]; return kids(i).reduce((s, k) => s + built.nodes[k].g.children.filter((c) => c.isMesh).length, 0); };
// How far the node's world position moves when only this joint changes by `delta`: the exact oracle for sliding joints.
function worldShift(name, delta) {
  const node = built.nodes.find((x) => x.def.name === name), base = sample(replay, 0).f0.vehicles[0].joints.slice();
  const at = (j) => { applyVehicle(built, sample(replay, 0)); poseRig(built, j); built.root.updateMatrixWorld(true); return node.g.getWorldPosition(new THREE.Vector3()); };
  const a = at(base), moved = base.slice();
  moved[node.def.joint.index] += delta;
  return at(moved).distanceTo(a);
}
const jointAt = (name, t) => { const j = built.nodes.find((n) => n.def.name === name).def.joint; const s = sample(replay, t); return s.f0.vehicles[0].joints[j.index]; };
window.__v = {
  setLayout,
  setLook: (o) => { camo.set(o); renderAt(t); }, schemes: camo.schemes,
  setDebug: (on) => { debug.group.visible = on; }, only, jointPixels, worldShift, meshCount, jointAt, setFreeze: (f) => { freeze = f; },
  renderAt, duration, poseOf, setCamera: (m) => { if (m.mode === 'front' && m.dist === undefined) Object.assign(cam, { dist: 7, pitch: 0.22 }); Object.assign(cam, m); if (m.mode) ui('cam').value = m.mode; },
  nodeNames: built.nodes.map((n) => n.def.name),
  triangles: built.triangles, expectedTriangles: rig.meshes.reduce((s, m) => s + m.indices.length / 3, 0),
  renderer: gl.getParameter(gl.VERSION), frames: replay.frames.length,
  pause: () => { playing = false; },
};
renderAt(0);
window.__ready = true;
if (!location.search.includes("static")) requestAnimationFrame(tick);
