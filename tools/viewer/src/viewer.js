// Reference viewer core: a pure function of (rig, replay, t). No physics, no network.
import * as THREE from 'three';

const V = (o) => new THREE.Vector3(o.x, o.y, o.z);
const Q = (o) => new THREE.Quaternion(o.x, o.y, o.z, o.w);

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
  const meshes = [];
  // Meshes come as JSON (lists of [x, y, z]) or as flat typed arrays (a packed skin); both are accepted.
  const flat = (a) => (ArrayBuffer.isView(a) ? a : a.flat());
  for (const m of rig.meshes) {
    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.Float32BufferAttribute(flat(m.positions), 3));
    geo.setAttribute('normal', new THREE.Float32BufferAttribute(flat(m.normals), 3));
    const n = ArrayBuffer.isView(m.positions) ? m.positions.length / 3 : m.positions.length;
    // LOOK's per-vertex flags (contract: edge sharpness and cavity, 0..1); absent flags mean 0.
    geo.setAttribute('aEdge', new THREE.Float32BufferAttribute(m.edge.length ? m.edge : new Float32Array(n), 1));
    geo.setAttribute('aCavity', new THREE.Float32BufferAttribute(m.cavity.length ? m.cavity : new Float32Array(n), 1));
    geo.setIndex(ArrayBuffer.isView(m.indices) ? new THREE.BufferAttribute(m.indices, 1) : m.indices);
    const mesh = new THREE.Mesh(geo, mats[m.material_slot]);
    nodes[m.node].g.add(mesh);
    meshes.push(mesh);
    tris += m.indices.length / 3;
  }
  // Height above the ground in the design pose (every joint at zero): the splash term of the weathering reads it.
  nodes.forEach((n) => { n.g.position.copy(n.restPos); n.g.quaternion.copy(n.restQuat); });
  root.updateMatrixWorld(true);
  const v = new THREE.Vector3();
  const ys = meshes.map((mesh) => { const p = mesh.geometry.attributes.position, y = new Float32Array(p.count); for (let i = 0; i < p.count; i++) y[i] = v.fromBufferAttribute(p, i).applyMatrix4(mesh.matrixWorld).y; return y; });
  const ground = Math.min(...ys.map((y) => y.reduce((a, b) => Math.min(a, b), Infinity)));
  meshes.forEach((mesh, k) => mesh.geometry.setAttribute('aHeight', new THREE.Float32BufferAttribute(ys[k].map((y) => y - ground), 1)));
  const box = new THREE.Box3().setFromObject(root);
  return { root, nodes, triangles: tris, paint, radius: box.getBoundingSphere(new THREE.Sphere()).radius };
}

// Forward kinematics: rest pose, then the joint coordinate (rotation or translation along the axis in the parent frame).
export function poseRig(built, joints) {
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
}

// Sample a frame at time t: linear in position and joints, slerp in rotation (viewers run at any frame rate).
export function sample(replay, t) {
  const dt = replay.header.frame_dt_s, f = replay.frames;
  const u = Math.min(Math.max(t / dt, 0), f.length - 1);
  const i = Math.min(Math.floor(u), f.length - 2), a = u - i;
  return { a, f0: f[i], f1: f[i + 1] };
}

export function applyVehicle(built, s, vi = 0) {
  const v0 = s.f0.vehicles[vi], v1 = s.f1.vehicles[vi], a = s.a;
  built.root.position.copy(V(v0.pos_m)).lerp(V(v1.pos_m), a);
  built.root.quaternion.copy(Q(v0.rot)).slerp(Q(v1.rot), a);
  poseRig(built, v0.joints.map((x, k) => x + (v1.joints[k] - x) * a));
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
