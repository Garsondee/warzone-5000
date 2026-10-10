import { buildRig, applyVehicle, poseRig, sample, makeScene } from './viewer.js';
import { decodeReplay } from './replay.js';
import { makeDebug, updateHud } from './debug.js';
import { makeScope } from './scope.js';
import * as THREE from 'three';

const { rig, replay: b64 } = JSON.parse(document.getElementById('data').textContent);
const replay = decodeReplay(Uint8Array.from(atob(b64), (c) => c.charCodeAt(0)));
const canvas = document.getElementById('c');
const W = canvas.clientWidth || innerWidth, H = canvas.clientHeight || innerHeight;
const { renderer, scene, camera } = makeScene(canvas, W, H);
camera.setViewOffset(W, H, 0, 110, W, H); // lift the picture above the scope panel
const built = buildRig(rig);
scene.add(built.root);
const debug = makeDebug(scene, built, replay.header);
const scope = makeScope(document.getElementById('scope'), replay, (tt) => { t = tt; renderAt(t); });
for (const k of Object.keys(debug.flags)) document.getElementById('t_' + k).onchange = (e) => { debug.flags[k] = e.target.checked; renderAt(t); };
const duration = (replay.frames.length - 1) * replay.header.frame_dt_s;

// Camera state: orbit (drag to turn, wheel to zoom) around the vehicle, or chase (behind the hull, follows its heading).
let freeze = false;
const cam = { mode: 'orbit', yaw: 0.6, pitch: 0.35, dist: 14 };
const look = new THREE.Vector3(), eye = new THREE.Vector3(), back = new THREE.Vector3();
function placeCamera() {
  const p = built.root.position;
  if (cam.focus) built.nodes.find((n) => n.def.name === cam.focus).g.getWorldPosition(look);
  else look.copy(p).add({ x: 0, y: 1, z: 0 });
  let yaw = cam.yaw;
  if (cam.mode === 'chase') {
    back.set(0, 0, 1).applyQuaternion(built.root.quaternion); // the hull's +Z is behind it
    yaw = Math.atan2(back.x, back.z);
  }
  const c = Math.cos(cam.pitch);
  eye.set(look.x + cam.dist * c * Math.sin(yaw), look.y + cam.dist * Math.sin(cam.pitch), look.z + cam.dist * c * Math.cos(yaw));
  camera.position.copy(eye);
  camera.lookAt(look);
}

function renderAt(t) {
  const s = sample(replay, Math.min(Math.max(t, 0), duration));
  applyVehicle(built, s);
  if (freeze) { built.root.position.set(0, 0, 0); built.root.quaternion.identity(); }
  placeCamera();
  debug.update(s.a < 0.5 ? s.f0 : s.f1);
  updateHud(document.getElementById('hud'), document.getElementById('ledger'), s.a < 0.5 ? s.f0 : s.f1, t);
  scope.draw(t);
  renderer.render(scene, camera);
}

// Playback
const ui = (id) => document.getElementById(id);
let t = 0, playing = true, last = performance.now();
ui('play').onclick = () => { playing = !playing; ui('play').textContent = playing ? 'pause' : 'play'; };
ui('scrub').oninput = (e) => { t = e.target.value * duration; renderAt(t); };
ui('cam').onchange = (e) => { cam.mode = e.target.value; };
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
  setDebug: (on) => { debug.group.visible = on; }, only, jointPixels, worldShift, meshCount, jointAt, setFreeze: (f) => { freeze = f; },
  renderAt, duration, poseOf, setCamera: (m) => Object.assign(cam, m),
  nodeNames: built.nodes.map((n) => n.def.name),
  triangles: built.triangles, expectedTriangles: rig.meshes.reduce((s, m) => s + m.indices.length / 3, 0),
  renderer: gl.getParameter(gl.VERSION), frames: replay.frames.length,
  pause: () => { playing = false; },
};
renderAt(0);
window.__ready = true;
if (!location.search.includes("static")) requestAnimationFrame(tick);
