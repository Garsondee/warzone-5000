// The live test-drive page: talks to `w5k drive` (docs/swarm/requests/arch-drive-protocol.md) over same-origin HTTP and a server-sent event
// stream, draws the vehicle on the course, and sends the pedals. It computes no physics: the server runs the real simulation.
// Built for a five-year-old driver: big buttons, no small text, no plots. Sound starts on the first key press or click.
import * as THREE from 'three';
import { buildRig, poseRig } from './viewer.js';
import { terrainMesh, propsGroup, roadMesh, heightSampler } from './world.js';
import { makeLook } from './look.js';
import { unpackSkin, fitSkin } from './skin.js';
import { makeEngineSound } from './live-audio.js';
import { makeInput } from './live-input.js';

const { look: lookData, skins: skinNames } = JSON.parse(document.getElementById('data').textContent);
const $ = (id) => document.getElementById(id);
const params = new URLSearchParams(location.search);
const SKY = 0xbfd4e6;
// Each vehicle gets its own paint scheme so the three look different in the picker and on the road.
const SCHEMES = { scout_4x4: 'desert_three', mule_4x4: 'woodland', hauler_4x4: 'nato_three_tone' };
const names = Object.keys(lookData.schemes);
const hash = (s) => [...s].reduce((a, c) => (a * 31 + c.charCodeAt(0)) >>> 0, 7);
const schemeFor = (id) => SCHEMES[id] ?? names[hash(id) % names.length];
const getJSON = async (p) => { const r = await fetch(p); if (!r.ok) throw new Error(`${p}: ${r.status}`); return r.json(); };
const postJSON = (p, body) => fetch(p, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
const wrap = (a) => a - Math.PI * 2 * Math.round(a / (Math.PI * 2));

// ---- scene -------------------------------------------------------------------------------------------------------------------
const canvas = $('c');
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
renderer.setPixelRatio(Math.min(devicePixelRatio || 1, 2));
renderer.shadowMap.enabled = params.get('shadows') !== '0';
renderer.shadowMap.type = THREE.PCFSoftShadowMap;
const scene = new THREE.Scene();
scene.background = new THREE.Color(SKY);
scene.fog = new THREE.Fog(SKY, 140, 700);
scene.add(new THREE.HemisphereLight(0xffffff, 0x556644, 1.0));
const sun = new THREE.DirectionalLight(0xfff4e0, 2.4);
const SUN_OFFSET = new THREE.Vector3(35, 60, 25);
sun.castShadow = renderer.shadowMap.enabled;
sun.shadow.mapSize.set(2048, 2048);
Object.assign(sun.shadow.camera, { left: -40, right: 40, top: 40, bottom: -40, near: 1, far: 220 });
sun.shadow.bias = -0.0004;
sun.shadow.normalBias = 0.06;
scene.add(sun, sun.target);
const camera = new THREE.PerspectiveCamera(58, 1, 0.3, 2500);
function resize() {
  renderer.setSize(innerWidth, innerHeight, false);
  camera.aspect = innerWidth / innerHeight;
  camera.updateProjectionMatrix();
}
addEventListener('resize', resize);
resize();

let ground = () => 0, intro = null;
function buildWorld(terrain) {
  const t = terrainMesh(terrain);
  t.mesh.receiveShadow = true;
  scene.add(t.mesh);
  const props = propsGroup(terrain);
  props.traverse((o) => { if (o.isMesh) { o.castShadow = true; o.receiveShadow = true; } });
  scene.add(props);
  const road = roadMesh(terrain);
  if (road) { road.receiveShadow = true; scene.add(road); }
  const floor = new THREE.Mesh(new THREE.PlaneGeometry(4000, 4000), new THREE.MeshStandardMaterial({ color: 0x8a9a6a, roughness: 1 }));
  floor.rotation.x = -Math.PI / 2;
  floor.position.set(t.centre[0], t.min - 0.05, t.centre[1]);
  scene.add(floor);
  ground = heightSampler(terrain);
  const roadPts = terrain.road_m || [];
  if (roadPts.length > 1) { // where the start screen looks: the start of the road, along it
    const a = roadPts[0], b = roadPts[Math.min(4, roadPts.length - 1)], d = new THREE.Vector3(b[0] - a[0], 0, b[2] - a[2]);
    intro = { pos: new THREE.Vector3(a[0], a[1], a[2]), dir: d.lengthSq() > 1e-6 ? d.normalize() : new THREE.Vector3(0, 0, -1) };
  }
}

// ---- vehicles ----------------------------------------------------------------------------------------------------------------
const rigPromises = new Map();
const rigFor = (id) => { if (!rigPromises.has(id)) rigPromises.set(id, getJSON(`/api/rig/${encodeURIComponent(id)}`)); return rigPromises.get(id); };
// What to draw for a vehicle: the detailed skin (the look id the server names, else the truck) fitted onto the skeleton the server sent;
// the server's own rig (plain boxes) if no skin file can be read.
const FALLBACK_SKIN = 'utility_4x4';
const skinPromises = new Map();
function skinFile(name) {
  if (!skinNames.includes(name)) return Promise.resolve(null);
  if (!skinPromises.has(name)) skinPromises.set(name, fetch(`/skins/${encodeURIComponent(name)}.skin`).then((r) => (r.ok ? r.arrayBuffer() : Promise.reject(new Error(String(r.status))))).then(unpackSkin).catch(() => null));
  return skinPromises.get(name);
}
const modelPromises = new Map();
function modelFor(id) {
  if (!modelPromises.has(id)) {
    modelPromises.set(id, (async () => {
      const phys = await rigFor(id), want = (vehicles.find((v) => v.id === id) || {}).skin || id;
      const skin = (await skinFile(want)) || (want !== FALLBACK_SKIN ? await skinFile(FALLBACK_SKIN) : null);
      if (!skin) return phys;
      try { return fitSkin(skin, phys); } catch (e) { console.warn('skin does not fit', id, e); return phys; }
    })());
  }
  return modelPromises.get(id);
}
let cur = null, loadToken = 0; // cur: { id, built, radius }

function disposeTree(root) {
  root.traverse((o) => { if (o.isMesh) { o.geometry.dispose(); [].concat(o.material).forEach((m) => m.dispose()); } });
}
async function ensureVehicle(id) {
  const token = ++loadToken; // the latest request wins, whatever order the rigs arrive in
  const rig = await modelFor(id);
  if (token !== loadToken) return;
  const built = buildRig(rig);
  built.root.traverse((o) => { if (o.isMesh) o.castShadow = true; });
  makeLook(lookData, built.paint).set({ scheme: schemeFor(id), seed: hash(id) });
  if (cur) { scene.remove(cur.built.root); disposeTree(cur.built.root); }
  scene.add(built.root);
  cur = { id, built, radius: built.radius, fit: rig.fit || null, joints: new Array(rig.joint_count).fill(0) };
  snap = true;
}

// A picture of a vehicle for the picker: its own small renderer, three-quarter view from the front, a soft shadow disc under it.
function thumbnail(rig, id) {
  const r = new THREE.WebGLRenderer({ antialias: true, preserveDrawingBuffer: true, alpha: true });
  r.setPixelRatio(1);
  r.setSize(480, 300);
  r.setClearColor(0x000000, 0);
  const s = new THREE.Scene();
  s.add(new THREE.HemisphereLight(0xffffff, 0x667755, 1.2));
  const d = new THREE.DirectionalLight(0xfff4e0, 2.4);
  d.position.set(30, 50, 20);
  s.add(d);
  const b = buildRig(rig);
  poseRig(b, new Array(rig.joint_count).fill(0));
  makeLook(lookData, b.paint).set({ scheme: schemeFor(id), seed: hash(id) });
  s.add(b.root);
  const box = new THREE.Box3().setFromObject(b.root), c = box.getCenter(new THREE.Vector3());
  const shadow = new THREE.Mesh(new THREE.CircleGeometry(1, 40), new THREE.MeshBasicMaterial({ color: 0x000000, transparent: true, opacity: 0.25 }));
  shadow.rotation.x = -Math.PI / 2;
  shadow.scale.set(box.getSize(new THREE.Vector3()).x * 0.75, box.getSize(new THREE.Vector3()).z * 0.62, 1);
  shadow.position.set(c.x, box.min.y + 0.02, c.z);
  s.add(shadow);
  const cam = new THREE.PerspectiveCamera(30, 480 / 300, 0.1, 400);
  const dist = (b.radius / Math.sin((30 * Math.PI) / 360)) * 0.95, yaw = 2.45, pitch = 0.2;
  cam.position.set(c.x + dist * Math.cos(pitch) * Math.sin(yaw), c.y + dist * Math.sin(pitch), c.z + dist * Math.cos(pitch) * Math.cos(yaw));
  cam.lookAt(c);
  r.render(s, cam);
  const url = r.domElement.toDataURL('image/png');
  disposeTree(b.root);
  r.dispose();
  r.forceContextLoss();
  return url;
}

// ---- picker, banner, HUD -----------------------------------------------------------------------------------------------------
let vehicles = [], thumbs = 0, wanted = null, wantedAt = 0, selected = null, streaming = false;
const picking = () => document.body.classList.contains('picking');
function select(id) { // the highlighted card: tapping a card only chooses it, DRIVE goes
  selected = id;
  document.querySelectorAll('.card').forEach((c) => c.classList.toggle('sel', c.dataset.id === id));
}
function openPicker() { if (cur) select(cur.id); document.body.classList.add('picking'); }
function closePicker() { document.body.classList.remove('picking'); }
// DRIVE: sound on (a button press is the browser's permission), the stream opens (the first time), the server puts the chosen vehicle on the road start.
async function startDrive(id = selected) {
  if (!id) return;
  sound.start();
  select(id);
  closePicker();
  wanted = id; wantedAt = performance.now();
  if (!streaming) { streaming = true; openStream(); }
  try { await postJSON('/api/select', { vehicle: id }); } catch { /* the stream shows what happened */ }
}
async function buildPicker() {
  const cards = $('cards');
  cards.textContent = '';
  vehicles.forEach((v) => {
    const b = document.createElement('button');
    b.className = 'card';
    b.dataset.id = v.id;
    b.innerHTML = '<img alt=""><span></span>';
    b.querySelector('span').textContent = v.name;
    b.onclick = () => { sound.start(); select(v.id); };
    cards.appendChild(b);
  });
  const middle = vehicles.find((v) => v.id === 'mule_4x4') || vehicles[Math.floor(vehicles.length / 2)];
  select(cur ? cur.id : middle.id);
  $('drive').onclick = () => startDrive();
  addEventListener('keydown', (e) => { // on the start screen: 1 2 3 or the left and right arrows choose, Enter or space drives
    if (!picking()) return;
    const at = vehicles.findIndex((v) => v.id === selected), n = Number(e.key);
    if (n >= 1 && n <= vehicles.length) select(vehicles[n - 1].id);
    else if (e.code === 'ArrowLeft' && at > 0) select(vehicles[at - 1].id);
    else if (e.code === 'ArrowRight' && at < vehicles.length - 1) select(vehicles[at + 1].id);
    else if (e.code === 'Enter' || e.code === 'Space') { e.preventDefault(); startDrive(); }
  });
  for (const v of vehicles) {
    try {
      const url = thumbnail(await modelFor(v.id), v.id);
      cards.querySelector(`[data-id="${CSS.escape(v.id)}"] img`).src = url;
      thumbs++;
    } catch (e) { console.warn('no picture for', v.id, e); }
  }
}

let bannerTimer = 0;
function showBanner(text) {
  const b = $('banner');
  b.textContent = text.charAt(0).toUpperCase() + text.slice(1) + '!';
  b.classList.remove('on');
  void b.offsetWidth; // restart the animation
  b.classList.add('on');
  clearTimeout(bannerTimer);
  bannerTimer = setTimeout(() => b.classList.remove('on'), 2700);
}
let shownSpeed = 0, uncapped = false;
function showSpeed(kmh, assistOn) {
  // The kid cap is 25 km/h and the bar ends at 30; a faster vehicle means the cap is off (`--no-speed-limit`, `--no-assist`), and then the
  // bar covers road speeds, so it is not stuck at full.
  if (kmh > 27) uncapped = true;
  const speedMax = uncapped || !assistOn ? 100 : 30;
  shownSpeed += (kmh - shownSpeed) * 0.35;
  const v = Math.max(0, shownSpeed);
  $('speedfill').style.clipPath = `inset(0 ${(100 - Math.min(100, (v / speedMax) * 100)).toFixed(1)}% 0 0 round 999px)`;
  $('speednum').textContent = String(Math.round(v));
}

// ---- stream ------------------------------------------------------------------------------------------------------------------
let prev = null, last = null, snap = true, lastFrameAt = 0, started = false, pendingId = null;
const P = (a) => new THREE.Vector3(a[0], a[1], a[2]);
function onFrame(f) {
  const now = performance.now();
  if (wanted && f.vehicle !== wanted && now - wantedAt < 2000) return; // the old vehicle's last frames after a pick
  if (f.vehicle === wanted) wanted = null;
  if ((!cur || cur.id !== f.vehicle) && pendingId !== f.vehicle) { pendingId = f.vehicle; ensureVehicle(f.vehicle).finally(() => { pendingId = null; }); }
  prev = last;
  last = { at: now, f, pos: P(f.pos_m), quat: new THREE.Quaternion(f.rot[1], f.rot[2], f.rot[3], f.rot[0]) };
  if (prev && prev.pos.distanceTo(last.pos) > 12) { prev = null; snap = true; } // a reset or a new vehicle: cut, do not glide across the course
  lastFrameAt = now;
  started = true;
  showSpeed(f.speed_m_s * 3.6, f.assist ? f.assist.on : true);
  $('limit').classList.toggle('on', !!(f.assist && f.assist.message === 'speed limit'));
  if (f.message_event) { showBanner(f.message_event); sound.chime(); }
}
function openStream() {
  const es = new EventSource('/api/stream');
  es.onmessage = (e) => onFrame(JSON.parse(e.data));
}

// ---- camera ------------------------------------------------------------------------------------------------------------------
const cam = { mode: 'chase', yaw: 0, look: new THREE.Vector3(), eye: new THREE.Vector3(), ready: false };
const fwd = new THREE.Vector3(), aim = new THREE.Vector3();
function updateCamera(dt, pos, quat, speed) {
  fwd.set(0, 0, -1).applyQuaternion(quat);
  fwd.y = 0;
  if (fwd.lengthSq() < 1e-4) fwd.set(0, 0, -1); else fwd.normalize();
  const behind = Math.atan2(-fwd.x, -fwd.z); // the eye sits on this side of the vehicle
  if (snap || !cam.ready) { cam.yaw = behind; cam.look.copy(pos); cam.ready = true; snap = false; }
  const k = (tau) => 1 - Math.exp(-dt / tau);
  const rts = cam.mode === 'rts';
  cam.yaw += wrap(behind - cam.yaw) * k(rts ? 1.3 : 0.45); // heading follows gently; a bend does not whip the picture round
  cam.look.x += (pos.x - cam.look.x) * k(0.1);
  cam.look.z += (pos.z - cam.look.z) * k(0.1);
  cam.look.y += (pos.y - cam.look.y) * k(0.35); // bumps do not shake the horizon
  const r = cur ? cur.radius : 3;
  const dist = rts ? Math.max(40, 11 * r) : Math.max(8, 3.4 * r) * (1 + Math.min(Math.abs(speed), 12) / 48);
  const pitch = rts ? 0.96 : 0.3;
  const c = Math.cos(pitch);
  aim.copy(cam.look).addScaledVector(fwd, rts ? 18 : 1.5);
  aim.y += rts ? 0 : 0.9;
  cam.eye.set(aim.x + dist * c * Math.sin(cam.yaw), aim.y + dist * Math.sin(pitch), aim.z + dist * c * Math.cos(cam.yaw));
  cam.eye.y = Math.max(cam.eye.y, ground(cam.eye.x, cam.eye.z) + 1.2); // never under the hill
  camera.position.copy(cam.eye);
  camera.lookAt(aim);
}
// In the RTS view the truck is small and trees hide it: a bright ring on the ground under it, drawn over everything, always shows where it is.
const marker = new THREE.Mesh(new THREE.RingGeometry(2.1, 2.8, 48), new THREE.MeshBasicMaterial({ color: 0xffd23f, transparent: true, opacity: 0.95, depthTest: false, side: THREE.DoubleSide }));
marker.rotation.x = -Math.PI / 2;
marker.renderOrder = 20;
marker.visible = false;
scene.add(marker);
const setCamera = (mode) => { cam.mode = mode; snap = true; document.body.dataset.camera = mode; };
const toggleCamera = () => setCamera(cam.mode === 'chase' ? 'rts' : 'chase');

// Before any vehicle is driving (the start screen) the camera floats slowly over the start of the road: nothing moves but the view.
function introCamera(now) {
  if (!intro) return;
  const t = now / 1000, side = new THREE.Vector3(-intro.dir.z, 0, intro.dir.x);
  camera.position.copy(intro.pos).addScaledVector(intro.dir, -20).addScaledVector(side, Math.sin(t * 0.13) * 9);
  camera.position.y = Math.max(intro.pos.y + 8, ground(camera.position.x, camera.position.z) + 3);
  camera.lookAt(aim.copy(intro.pos).addScaledVector(intro.dir, 14).add({ x: 0, y: 1.5, z: 0 }));
  sun.position.copy(intro.pos).add(SUN_OFFSET);
  sun.target.position.copy(intro.pos);
}

// ---- input -------------------------------------------------------------------------------------------------------------------
const sound = makeEngineSound();
let wantReset = false;
const input = makeInput({
  onReset: () => { wantReset = true; },
  onCamera: toggleCamera,
  onGarage: () => (picking() ? (cur ? closePicker() : null) : openPicker()), // the start screen only closes with DRIVE
  onMute: () => { const m = !sound.muted; sound.setMuted(m); $('sound').classList.toggle('off', m); },
  onFirstGesture: () => sound.start(),
});
for (const [id, name] of [['left', 'left'], ['right', 'right'], ['go', 'go'], ['stop', 'stop'], ['back', 'back']]) input.bindHold($(id), name);
input.bindTap($('reset'), () => { wantReset = true; });
input.bindTap($('camera'), toggleCamera);
input.bindTap($('garage'), openPicker);
input.bindTap($('sound'), () => { const m = !sound.muted; sound.setMuted(m); $('sound').classList.toggle('off', m); });
document.addEventListener('gesturestart', (e) => e.preventDefault()); // no pinch-zoom on iPads
document.addEventListener('visibilitychange', () => { if (document.hidden) { sound.suspend(); input.pads.go = input.pads.stop = input.pads.back = false; } else sound.resume(); });

// The server holds the last input and lets go of the pedals after 0.5 s of silence: send on every change, and every 0.2 s while a pedal is down.
let lastSent = null, lastSentAt = 0, inflight = false, lastCmd = { throttle: 0, brake: 0, steer: 0, reverse: false };
function send(now, cmd) {
  const changed = !lastSent || cmd.throttle !== lastSent.throttle || cmd.brake !== lastSent.brake || cmd.steer !== lastSent.steer || cmd.reverse !== lastSent.reverse;
  const active = cmd.throttle > 0 || cmd.brake > 0 || cmd.steer !== 0;
  if (!wantReset && !changed && !(active && now - lastSentAt > 200)) return;
  if (inflight && !wantReset) return;
  inflight = true;
  const body = { ...cmd, reset: wantReset };
  wantReset = false;
  lastSent = cmd; lastSentAt = now;
  postJSON('/api/input', body).catch(() => { lastSent = null; }).finally(() => { inflight = false; });
}

// ---- loop --------------------------------------------------------------------------------------------------------------------
const q = new THREE.Quaternion(), pos = new THREE.Vector3();
let lastDraw = performance.now();
// The pedals are read and sent on their own 30 Hz timer, not in the draw loop: a slow computer drawing 10 pictures a second must still
// keep the server's 0.5 s pedal timeout fed.
let lastT = performance.now();
function inputTick() {
  const now = performance.now(), dt = Math.min(0.1, (now - lastT) / 1000);
  lastT = now;
  lastCmd = picking() ? { throttle: 0, brake: 0, steer: 0, reverse: false } : input.read(dt, last ? last.f.speed_m_s : 0); // no pedals on the start screen
  send(now, lastCmd);
}
function loop(now) {
  requestAnimationFrame(loop);
  const dt = Math.min(0.1, (now - lastDraw) / 1000);
  lastDraw = now;
  const speed = last ? last.f.speed_m_s : 0;
  $('conn').style.display = started && now - lastFrameAt > 1500 ? 'block' : 'none';
  if (cur && last && last.f.vehicle === cur.id) {
    let a = 1;
    if (prev) { const span = last.at - prev.at; a = span > 1 ? Math.min(1, Math.max(0, (now - last.at) / span)) : 1; }
    const A = prev || last;
    pos.copy(A.pos).lerp(last.pos, a);
    q.copy(A.quat).slerp(last.quat, a);
    const jA = A.f.joints, jB = last.f.joints;
    for (let i = 0; i < cur.joints.length; i++) cur.joints[i] = jA[i] + (jB[i] - jA[i]) * a;
    cur.built.root.position.copy(pos);
    cur.built.root.quaternion.copy(q);
    poseRig(cur.built, cur.joints);
    updateCamera(dt, pos, q, speed);
    marker.visible = cam.mode === 'rts';
    marker.position.set(pos.x, ground(pos.x, pos.z) + 0.25, pos.z);
    sun.position.copy(pos).add(SUN_OFFSET);
    sun.target.position.copy(pos);
    sound.update({ rpm: last.f.engine_rpm, throttle: lastCmd.throttle, speed });
  }
  else introCamera(now);
  renderer.render(scene, camera);
}

// ---- start -------------------------------------------------------------------------------------------------------------------
function fatal(text) { const f = $('fatal'); f.textContent = text; f.style.display = 'block'; }
async function boot() {
  for (;;) {
    try {
      [vehicles, ] = await Promise.all([getJSON('/api/vehicles'), getJSON('/api/world').then(buildWorld)]);
      break;
    } catch (e) {
      fatal('Can’t find the game. Start it with “w5k drive”, then this page will wake up.');
      await new Promise((r) => setTimeout(r, 2000));
    }
  }
  $('fatal').style.display = 'none';
  vehicles.sort((a, b) => a.mass_kg - b.mass_kg); // small to big: scout, mule, hauler
  openPicker(); // the start screen first: no stream, no moving truck
  setInterval(inputTick, 33);
  requestAnimationFrame(loop);
  await buildPicker();
}
window.__live = { // for the browser test only
  frame: () => (last ? last.f : null), vehicle: () => (cur ? cur.id : null), camera: () => cam.mode, input: () => lastCmd,
  thumbs: () => thumbs, fit: () => (cur ? cur.fit : null), sound: () => sound.state, picking, pick: startDrive, selected: () => selected, streaming: () => streaming, setCamera,
  vehicles: () => vehicles.map((v) => v.id),
};
boot();
