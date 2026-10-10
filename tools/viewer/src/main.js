import { buildRig, applyVehicle, sample, makeScene } from './viewer.js';
import * as THREE from 'three';
const { rig, replay } = JSON.parse(document.getElementById('data').textContent);
const canvas = document.getElementById('c');
const W = canvas.clientWidth || innerWidth, H = canvas.clientHeight || innerHeight;
const { renderer, scene, camera } = makeScene(canvas, W, H);
const built = buildRig(rig);
scene.add(built.root);
const duration = (replay.frames.length - 1) * replay.header.frame_dt_s;
function renderAt(t) {
  applyVehicle(built, sample(replay, t));
  const p = built.root.position;
  camera.position.set(p.x + 9, p.y + 5, p.z + 12);
  camera.lookAt(p);
  renderer.render(scene, camera);
}
window.__v = { renderAt, duration, triangles: built.triangles, expectedTriangles: rig.meshes.reduce((s, m) => s + m.indices.length / 3, 0), renderer: renderer.getContext().getParameter(renderer.getContext().VERSION) };
renderAt(0);
window.__ready = true;
