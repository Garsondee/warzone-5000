// Reference viewer core: a pure function of (rig, replay, t). No physics, no network.
import * as THREE from 'three';
import { linkFrames, linkMatrix, contrast, groundRun } from './tracks.js';

const V = (o) => new THREE.Vector3(o.x, o.y, o.z);
const Q = (o) => new THREE.Quaternion(o.x, o.y, o.z, o.w);

const LINK_MEAN = 1.0, LINK_SWING = 0.4, _shade = new THREE.Color(); // light and dark pads are the mean shade +- the swing, multipliers of the track material's colour
// Alternate the links light and dark by `c` (1: the full swing, 0: all the mean shade): the pattern is what shows the belt turning, and it is faded
// where it would show false motion (tracks.js `contrast`). Link k keeps its shade as it travels, so the pattern rides the belt.
function shadeLinks(b, c) {
  for (let k = 0; k < b.run.links; k++) b.mesh.setColorAt(k, _shade.setScalar(LINK_MEAN + (k % 2 ? -1 : 1) * LINK_SWING * c));
  b.mesh.instanceColor.needsUpdate = true;
  b.contrast = c;
}
const DEFAULT_FRAME_S = 1 / 60; // replay time one displayed frame covers when the host has not said

export function buildRig(rig) {
  const nodes = rig.nodes.map((n) => {
    const g = new THREE.Group();
    g.name = n.name;
    g.position.copy(V(n.rest.pos));
    g.quaternion.copy(Q(n.rest.rot));
    return { g, def: n, restPos: g.position.clone(), restQuat: g.quaternion.clone() };
  });
  const root = new THREE.Group();
  nodes.forEach((n, i) => (n.def.parent == null ? root : nodes[n.def.parent].g).add(n.g));
  // Plain colours per slot kind; LOOK's hook (look.js) replaces the albedo of Paint slots with camo and weathering.
  const KIND_COLOUR = { Paint: 0x6b7a4a, Metal: 0x888888, Rubber: 0x222222, Glass: 0x88aacc, Canvas: 0x7a6a4a, Track: 0x333333, Optics: 0x99cc99 };
  const mats = rig.material_slots.map((s) => new THREE.MeshStandardMaterial({ color: KIND_COLOUR[s.kind] ?? 0x808080, roughness: 0.8, metalness: 0.1 }));
  const paint = rig.material_slots.map((s, i) => (s.kind === 'Paint' ? mats[i] : null)).filter(Boolean);
  let tris = 0;
  const meshes = [], belts = [];
  const runs = rig.track_runs ?? []; // a track belt's link mesh is ONE link at the origin: drawn as instances along the belt's path (poseBelts)
  // Meshes come as JSON (lists of [x, y, z]) or as flat typed arrays (a packed skin); both are accepted.
  const flat = (a) => (ArrayBuffer.isView(a) ? a : a.flat());
  rig.meshes.forEach((m, mi) => {
    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.Float32BufferAttribute(flat(m.positions), 3));
    geo.setAttribute('normal', new THREE.Float32BufferAttribute(flat(m.normals), 3));
    const n = ArrayBuffer.isView(m.positions) ? m.positions.length / 3 : m.positions.length;
    // LOOK's per-vertex flags (contract: edge sharpness and cavity, 0..1); absent flags mean 0.
    geo.setAttribute('aEdge', new THREE.Float32BufferAttribute(m.edge.length ? m.edge : new Float32Array(n), 1));
    geo.setAttribute('aCavity', new THREE.Float32BufferAttribute(m.cavity.length ? m.cavity : new Float32Array(n), 1));
    geo.setIndex(ArrayBuffer.isView(m.indices) ? new THREE.BufferAttribute(m.indices, 1) : m.indices);
    const run = runs.find((r) => r.link_mesh === mi);
    const mesh = run ? new THREE.InstancedMesh(geo, mats[m.material_slot], run.links) : new THREE.Mesh(geo, mats[m.material_slot]);
    if (run) { // the instances move, the geometry's own bounds do not
      mesh.frustumCulled = false;
      geo.computeBoundingBox();
      const belt = { run, mesh, contrast: 1, width_m: geo.boundingBox.max.z - geo.boundingBox.min.z, half_thickness_m: geo.boundingBox.max.y };
      shadeLinks(belt, 1);
      belts.push(belt);
    }
    nodes[m.node].g.add(mesh);
    meshes.push(mesh);
    tris += m.indices.length / 3; // as authored: a belt's link counts once, however many instances are drawn
  });
  // Height above the ground in the design pose (every joint at zero): the splash term of the weathering reads it.
  nodes.forEach((n) => { n.g.position.copy(n.restPos); n.g.quaternion.copy(n.restQuat); });
  root.updateMatrixWorld(true);
  const v = new THREE.Vector3();
  const ys = meshes.map((mesh) => { const p = mesh.geometry.attributes.position, y = new Float32Array(p.count); for (let i = 0; i < p.count; i++) y[i] = v.fromBufferAttribute(p, i).applyMatrix4(mesh.matrixWorld).y; return y; });
  const ground = Math.min(...ys.map((y) => y.reduce((a, b) => Math.min(a, b), Infinity)));
  meshes.forEach((mesh, k) => mesh.geometry.setAttribute('aHeight', new THREE.Float32BufferAttribute(ys[k].map((y) => y - ground), 1)));
  const box = new THREE.Box3().setFromObject(root);
  const built = { root, nodes, triangles: tris, paint, belts, radius: box.getBoundingSphere(new THREE.Sphere()).radius };
  if (belts.length) poseBelts(built, []); // the links at rest
  return built;
}

// Forward kinematics: rest pose, then the joint coordinate (rotation or translation along the axis in the parent frame).
// `perFrame` (optional, one entry per joint) is how far each joint moves between this displayed frame and the next: the host's estimate (rest poses: none).
export function poseRig(built, joints, perFrame) {
  const q = new THREE.Quaternion();
  for (const n of built.nodes) {
    const j = n.def.joint;
    n.g.position.copy(n.restPos);
    n.g.quaternion.copy(n.restQuat);
    if (!j) continue;
    const x = joints[j.index];
    const axis = V(j.axis);
    if (j.kind === 'Revolute') n.g.quaternion.premultiply(q.setFromAxisAngle(axis, x));
    else n.g.position.addScaledVector(axis, x);
  }
  if (built.belts.length) poseBelts(built, joints, perFrame);
}

// Each belt's links ride the taut band round its wheels as they stand now (the suspension moves them), advanced by the sprocket's spin.
const _inv = new THREE.Matrix4(), _v = new THREE.Vector3();
function poseBelts(built, joints, perFrame) {
  built.root.updateMatrixWorld(true);
  for (const b of built.belts) {
    const { run, mesh } = b;
    _inv.copy(built.nodes[run.node].g.matrixWorld).invert();
    const centres = run.wheels.map((w) => _v.setFromMatrixPosition(built.nodes[w.node].g.matrixWorld).applyMatrix4(_inv).toArray()); // [x, y, z] in the Track node's frame
    const x = centres.reduce((a, c) => a + c[0], 0) / centres.length; // the belt's centre line
    const out = linkFrames(run, centres.map(([, y, z]) => ({ z, y })), joints[run.sprocket_joint] ?? 0);
    if (out.error) continue; // a belt that cannot be built keeps its last shape
    for (let k = 0; k < run.links; k++) linkMatrix(mesh.instanceMatrix.array, 16 * k, out.frames, k, x, out.mirrored);
    mesh.instanceMatrix.needsUpdate = true;
    const step = Math.abs(perFrame?.[run.sprocket_joint] ?? 0) * run.wheels[run.sprocket].radius_m; // metres the belt moves between displayed frames
    const c = contrast(step, out.length / run.links);
    if (Math.abs(c - b.contrast) > 0.005) shadeLinks(b, c);
    const road = run.wheels.map((w, i) => ({ z: centres[i][2], y: centres[i][1], r: w.radius_m })).filter((_, i) => built.nodes[run.wheels[i].node].def.role === 'RoadWheel');
    Object.assign(b, { length_m: out.length, skipped: out.skipped, step_m: step, ground: groundRun(road), x }); // the footprint the contact overlay draws on
  }
}

// Sample a frame at time t: linear in position and joints, slerp in rotation (viewers run at any frame rate).
export function sample(replay, t) {
  const dt = replay.header.frame_dt_s, f = replay.frames;
  const u = Math.min(Math.max(t / dt, 0), f.length - 1);
  const i = Math.min(Math.floor(u), f.length - 2), a = u - i;
  return { a, f0: f[i], f1: f[i + 1], dt };
}

export function applyVehicle(built, s, vi = 0) {
  const v0 = s.f0.vehicles[vi], v1 = s.f1.vehicles[vi], a = s.a;
  built.root.position.copy(V(v0.pos_m)).lerp(V(v1.pos_m), a);
  built.root.quaternion.copy(Q(v0.rot)).slerp(Q(v1.rot), a);
  const per = (built.frame_s ?? DEFAULT_FRAME_S) / s.dt; // displayed frames are `frame_s` of replay time apart: the host sets it (recorder: 1 / fps)
  poseRig(built, v0.joints.map((x, k) => x + (v1.joints[k] - x) * a), v0.joints.map((x, k) => (v1.joints[k] - x) * per));
}

import { terrainMesh, propsGroup, roadMesh } from './world.js';

export function makeScene(canvas, W, H, terrain = null) {
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, preserveDrawingBuffer: true });
  renderer.setSize(W, H, false);
  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0xbfd4e6);
  scene.add(new THREE.HemisphereLight(0xffffff, 0x556644, 1.1));
  const sun = new THREE.DirectionalLight(0xffffff, 2.2);
  sun.position.set(30, 60, 20);
  scene.add(sun);
  if (terrain) {
    const t = terrainMesh(terrain);
    scene.add(t.mesh, propsGroup(terrain));
    const road = roadMesh(terrain);
    if (road) scene.add(road);
    // A broad plane just under the lowest point, so the edge of the sampled strip does not open onto the sky.
    const floor = new THREE.Mesh(new THREE.PlaneGeometry(2000, 2000), new THREE.MeshStandardMaterial({ color: 0x8a9a6a, roughness: 1 }));
    floor.rotation.x = -Math.PI / 2;
    floor.position.set(t.centre[0], t.min - 0.05, t.centre[1]);
    scene.add(floor);
  }
  else {
    const ground = new THREE.Mesh(new THREE.PlaneGeometry(400, 400), new THREE.MeshStandardMaterial({ color: 0x8a9a6a }));
    ground.rotation.x = -Math.PI / 2;
    scene.add(ground);
    scene.add(new THREE.GridHelper(400, 200, 0x445533, 0x667755));
  }
  const camera = new THREE.PerspectiveCamera(45, W / H, 0.3, 3000);
  return { renderer, scene, camera };
}
