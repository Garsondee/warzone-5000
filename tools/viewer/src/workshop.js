// The Workshop page: sliders for a design's levers; each change asks `w5k viewer workshop` (same origin) to rebuild the vehicle, and the body
// on screen is the skin the server generated for exactly that design. It computes no physics and no geometry. Two answers per change, the
// quick one (a 16-ray bake, about a second) and, once the sliders rest, the final one.
import * as THREE from 'three';
import { buildRig, poseRig } from './viewer.js';
import { makeLook } from './look.js';
import { unpackSkin, fitSkin } from './skin.js';

const { look: lookData } = JSON.parse(document.getElementById('data').textContent);
const $ = (id) => document.getElementById(id);
const getJSON = async (p) => { const r = await fetch(p); if (!r.ok) throw new Error(`${p}: ${r.status}`); return r.json(); };
const SCHEMES = { scout_4x4: 'desert_three', mule_4x4: 'woodland', hauler_4x4: 'nato_three_tone' };
const hash = (s) => [...s].reduce((a, c) => (a * 31 + c.charCodeAt(0)) >>> 0, 7);
const QUICK_DEBOUNCE_MS = 150; // a slider that is still moving is not asked about
const FINAL_AFTER_MS = 1200; // the final bake starts when the sliders have rested this long
const STILL = new URLSearchParams(location.search).has('still'); // ?still keeps the turntable still, for pictures that must share one angle

// ---- scene: the vehicle on a shadow disc, orbited by dragging, turning slowly when left alone ------------------------------------
const canvas = $('c');
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
renderer.setPixelRatio(Math.min(devicePixelRatio || 1, 2));
const scene = new THREE.Scene();
scene.add(new THREE.HemisphereLight(0xffffff, 0x667755, 1.2));
const sun = new THREE.DirectionalLight(0xfff4e0, 2.4);
sun.position.set(30, 50, 20);
scene.add(sun);
const shadow = new THREE.Mesh(new THREE.CircleGeometry(1, 48), new THREE.MeshBasicMaterial({ color: 0x000000, transparent: true, opacity: 0.22 }));
shadow.rotation.x = -Math.PI / 2;
scene.add(shadow);
const camera = new THREE.PerspectiveCamera(36, 1, 0.1, 400);
const orbit = { yaw: 0.9, pitch: 0.28, dist: 12, target: new THREE.Vector3(), idle: 0 };
function resize() {
  const r = canvas.getBoundingClientRect();
  renderer.setSize(r.width, r.height, false);
  camera.aspect = r.width / Math.max(r.height, 1);
  camera.updateProjectionMatrix();
}
addEventListener('resize', resize);
let drag = null;
canvas.addEventListener('pointerdown', (e) => { drag = { x: e.clientX, y: e.clientY }; canvas.setPointerCapture(e.pointerId); });
canvas.addEventListener('pointerup', () => { drag = null; });
canvas.addEventListener('pointermove', (e) => {
  if (!drag) return;
  orbit.yaw -= (e.clientX - drag.x) * 0.008;
  orbit.pitch = Math.max(0.02, Math.min(1.3, orbit.pitch + (e.clientY - drag.y) * 0.006));
  Object.assign(drag, { x: e.clientX, y: e.clientY });
  orbit.idle = 0;
});
canvas.addEventListener('wheel', (e) => { e.preventDefault(); orbit.dist = Math.max(3, Math.min(60, orbit.dist * Math.exp(e.deltaY * 0.001))); }, { passive: false });
let last = performance.now();
renderer.setAnimationLoop((now) => {
  const dt = Math.min((now - last) / 1000, 0.1);
  last = now;
  orbit.idle += dt;
  if (!drag && !STILL && orbit.idle > 3) orbit.yaw += dt * 0.25;
  const { yaw, pitch, dist, target } = orbit;
  camera.position.set(target.x + dist * Math.sin(yaw) * Math.cos(pitch), target.y + dist * Math.sin(pitch), target.z + dist * Math.cos(yaw) * Math.cos(pitch));
  camera.lookAt(target);
  renderer.render(scene, camera);
});

// ---- the vehicle on show -----------------------------------------------------------------------------------------------------
let shown = null, base = null, factors = {}, seq = 0, quick = 0, final = 0, detail = 'none';
function setStatus(kind, text) { const s = $('status'); s.className = kind; s.textContent = text; }

async function show(d, reframe) {
  let model = d.rig;
  if (d.skin) {
    try { model = fitSkin(unpackSkin(await (await fetch(d.skin.url)).arrayBuffer()), d.rig); } catch (e) { console.warn('the skin does not fit; drawing the rig', e); }
  }
  const built = buildRig(model);
  poseRig(built, new Array(model.joint_count).fill(0));
  makeLook(lookData, built.paint).set({ scheme: SCHEMES[base.id] ?? 'woodland', seed: hash(base.id) });
  built.root.traverse((o) => { if (o.isMesh) o.castShadow = true; });
  built.root.updateMatrixWorld(true);
  const box = new THREE.Box3().setFromObject(built.root);
  built.root.position.y -= box.min.y; // wheels on the ground
  box.translate(new THREE.Vector3(0, -box.min.y, 0));
  if (shown) { scene.remove(shown.root); shown.root.traverse((o) => { if (o.isMesh) { o.geometry.dispose(); [].concat(o.material).forEach((m) => m.dispose()); } }); }
  scene.add(built.root);
  shown = built;
  const size = box.getSize(new THREE.Vector3()), centre = box.getCenter(new THREE.Vector3());
  shadow.scale.set(size.x * 0.75, size.z * 0.62, 1);
  shadow.position.set(centre.x, 0.02, centre.z);
  if (reframe) { orbit.target.copy(centre); orbit.dist = Math.max(size.x, size.y, size.z) * 2.6; }
  $('report').textContent = (d.report || []).join('\n');
}

// ---- sliders -------------------------------------------------------------------------------------------------------------------
const pct = (f) => `${f >= 1 ? '+' : '−'}${Math.abs((f - 1) * 100).toFixed(0)}%`;
function readout(l) {
  const f = factors[l.id] ?? 1;
  if (l.base == null) return `×${f.toFixed(2)}`;
  const v = l.base * f, digits = v >= 100 ? 0 : v >= 10 ? 1 : 2;
  return `${v.toFixed(digits)} ${l.unit}  ${Math.abs(f - 1) < 0.005 ? '' : pct(f)}`;
}
function buildSliders(levers) {
  $('levers').replaceChildren(...levers.map((l) => {
    const el = document.createElement('div');
    el.className = 'lever';
    el.innerHTML = `<div class="row"><label for="s-${l.id}">${l.label}</label><span class="val" id="v-${l.id}"></span></div>
      <input type="range" id="s-${l.id}" min="${l.min}" max="${l.max}" step="0.01" value="1"><button type="button" aria-label="Reset ${l.label}">reset</button>`;
    const input = el.querySelector('input'), out = el.querySelector('.val');
    const set = (f) => { if (Math.abs(f - 1) < 0.005) { delete factors[l.id]; input.value = 1; } else factors[l.id] = f; out.textContent = readout(l); changed(); };
    input.addEventListener('input', () => set(+input.value));
    el.querySelector('button').addEventListener('click', () => set(1));
    out.textContent = readout(l);
    return el;
  }));
}

function changed() {
  clearTimeout(quick); clearTimeout(final);
  seq++; // whatever is on its way is now out of date
  detail = 'pending';
  quick = setTimeout(() => ask('preview'), QUICK_DEBOUNCE_MS);
}
async function ask(quality) {
  const mine = ++seq, t0 = performance.now();
  setStatus('', quality === 'final' ? 'Baking the final detail…' : 'Building…');
  try {
    const r = await fetch('/api/design', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ base: base.id, levers: factors, quality }) });
    const d = await r.json();
    if (mine !== seq) return; // a newer change has been made since: drop this answer
    if (!r.ok) { setStatus('error', d.error); return; }
    await show(d, false);
    if (mine !== seq) return;
    detail = quality;
    setStatus('', `${quality === 'final' ? 'Final detail' : 'Quick detail'}, ${((performance.now() - t0) / 1000).toFixed(1)} s`);
    if (quality === 'preview') final = setTimeout(() => ask('final'), FINAL_AFTER_MS);
  } catch (e) { if (mine === seq) setStatus('error', String(e.message || e)); }
}

async function pick(id) {
  const info = await getJSON(`/api/base/${encodeURIComponent(id)}`);
  base = info; factors = {}; seq++;
  clearTimeout(quick); clearTimeout(final);
  [...$('bases').children].forEach((b) => b.setAttribute('aria-pressed', String(b.dataset.id === id)));
  buildSliders(info.levers);
  await show(info.design, true);
  detail = 'authored';
  setStatus('', 'As authored');
}

resize();
const bases = await getJSON('/api/bases');
$('bases').replaceChildren(...bases.map((b) => {
  const el = document.createElement('button');
  el.type = 'button'; el.dataset.id = b.id; el.textContent = b.name;
  el.addEventListener('click', () => pick(b.id));
  return el;
}));
await pick(bases.find((b) => b.id === 'mule_4x4')?.id ?? bases[0].id);
// what the end-to-end test (workshop-smoke.mjs) looks at
window.__workshop = {
  base: () => base && base.id,
  levers: () => (base ? base.levers.map((l) => l.id) : []),
  detail: () => detail,
  error: () => ($('status').className === 'error' ? $('status').textContent : null),
  size: () => { const b = new THREE.Box3().setFromObject(shown.root).getSize(new THREE.Vector3()); return [b.x, b.y, b.z]; },
};
