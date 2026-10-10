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
  const mats = rig.material_slots.map((s, i) => new THREE.MeshStandardMaterial({
    color: [0x6b7a4a, 0x888888, 0x222222, 0x88aacc, 0x7a6a4a, 0x333333, 0x99cc99][i % 7], roughness: 0.8, metalness: 0.1, flatShading: false,
  }));
  let tris = 0;
  for (const m of rig.meshes) {
    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.Float32BufferAttribute(m.positions.flat(), 3));
    geo.setAttribute('normal', new THREE.Float32BufferAttribute(m.normals.flat(), 3));
    geo.setIndex(m.indices);
    const mesh = new THREE.Mesh(geo, mats[m.material_slot]);
    nodes[m.node].g.add(mesh);
    tris += m.indices.length / 3;
  }
  return { root, nodes, triangles: tris };
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

export function makeScene(canvas, W, H) {
  const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, preserveDrawingBuffer: true });
  renderer.setSize(W, H, false);
  const scene = new THREE.Scene();
  scene.background = new THREE.Color(0xbfd4e6);
  scene.add(new THREE.HemisphereLight(0xffffff, 0x556644, 1.1));
  const sun = new THREE.DirectionalLight(0xffffff, 2.2);
  sun.position.set(30, 60, 20);
  scene.add(sun);
  const ground = new THREE.Mesh(new THREE.PlaneGeometry(400, 400), new THREE.MeshStandardMaterial({ color: 0x8a9a6a }));
  ground.rotation.x = -Math.PI / 2;
  scene.add(ground);
  scene.add(new THREE.GridHelper(400, 200, 0x445533, 0x667755));
  const camera = new THREE.PerspectiveCamera(45, W / H, 0.1, 1000);
  return { renderer, scene, camera };
}
