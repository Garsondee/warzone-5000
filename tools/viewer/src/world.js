// The ground: WORLD's `terrain.json` (format w5k-terrain-1, docs/swarm/requests/world-viewer-terrain.md): a heightfield with a material id
// per node, plus props. Display only: the physics surface is the bilinear patch, so never judge contact from this mesh.
import * as THREE from 'three';

const GROUND = { asphalt: 0x4a4a4c, dirt: 0x7d8a58, mud: 0x4b3a28, gravel: 0x8a8a84, sand: 0xc2b280 };
const ROAD_WIDTH_M = 6; // PROVISIONAL: the file has no road width yet (docs/swarm/requests/viewer-world-road-width.md); the slice course is two 6 m lanes
const PROP = { Tree: 0x5c4630, Barricade: 0xb9582c };

export function terrainMesh(t) {
  if (t.format !== 'w5k-terrain-1') throw new Error(`unknown terrain format ${t.format}`);
  const { nx, nz, cell_m: s } = t, ox = t.origin_m.x, oz = t.origin_m.z;
  const pos = new Float32Array(nx * nz * 3), col = new Float32Array(nx * nz * 3), c = new THREE.Color();
  const lo = Math.min(...t.heights_m), hi = Math.max(...t.heights_m);
  for (let j = 0; j < nz; j++) for (let i = 0; i < nx; i++) {
    const k = j * nx + i, h = t.heights_m[k], name = t.materials[t.material_ids[k]];
    pos.set([ox + i * s, h, oz + j * s], k * 3);
    c.setHex(GROUND[name === 'asphalt' ? 'dirt' : name] ?? 0x7d8a58); // asphalt is drawn as a ribbon over the graded ground (roadMesh): vertex colours alone give a sawtooth edge
    if (name === 'dirt') c.offsetHSL(0, 0, (((h - lo) / (hi - lo || 1)) - 0.5) * 0.12); // height tints the open ground a little
    col.set([c.r, c.g, c.b], k * 3);
  }
  const idx = [];
  // The diagonal runs from (i, j) to (i+1, j+1), as the file format says; winding faces up.
  for (let j = 0; j < nz - 1; j++) for (let i = 0; i < nx - 1; i++) {
    const a = j * nx + i, b = a + 1, d = a + nx, e = d + 1;
    idx.push(a, d, e, a, e, b);
  }
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.BufferAttribute(pos, 3));
  g.setAttribute('color', new THREE.BufferAttribute(col, 3));
  g.setIndex(idx);
  g.computeVertexNormals();
  return { mesh: new THREE.Mesh(g, new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 1 })), min: lo, centre: [ox + (nx * s) / 2, oz + (nz * s) / 2] };
}

// Props: cylinders (trunks) get a crown so a tree reads as a tree, boxes are drawn as they are. Instanced: a course has about a thousand.
export function propsGroup(t) {
  const group = new THREE.Group(), q = new THREE.Quaternion(), m = new THREE.Matrix4(), one = new THREE.Vector3(1, 1, 1);
  const byKind = {};
  for (const p of t.props ?? []) (byKind[p.kind + p.shape.type] ??= []).push(p);
  for (const list of Object.values(byKind)) {
    const { kind, shape } = list[0], colour = PROP[kind] ?? 0x888888;
    const make = (geo, mat, place) => {
      const im = new THREE.InstancedMesh(geo, mat, list.length);
      list.forEach((p, i) => { place(p); im.setMatrixAt(i, m); });
      group.add(im);
    };
    const mat = new THREE.MeshStandardMaterial({ color: colour, roughness: 0.9 });
    if (shape.type === 'cylinder') {
      make(new THREE.CylinderGeometry(1, 1, 1, 8), mat, (p) => m.compose(new THREE.Vector3(p.pos_m[0], p.pos_m[1] + p.shape.height_m / 2, p.pos_m[2]), q.identity(), new THREE.Vector3(p.shape.radius_m, p.shape.height_m, p.shape.radius_m)));
      if (kind === 'Tree') make(new THREE.IcosahedronGeometry(1, 1), new THREE.MeshStandardMaterial({ color: 0x3f5a30, roughness: 1 }), (p) => m.compose(new THREE.Vector3(p.pos_m[0], p.pos_m[1] + p.shape.height_m * 0.85, p.pos_m[2]), q.identity(), new THREE.Vector3(2.4, 3.2, 2.4)));
    } else if (shape.type === 'box') {
      make(new THREE.BoxGeometry(1, 1, 1), mat, (p) => m.compose(new THREE.Vector3(...p.pos_m), q.set(p.rot_wxyz[1], p.rot_wxyz[2], p.rot_wxyz[3], p.rot_wxyz[0]), new THREE.Vector3(...p.shape.half_m.map((h) => h * 2))));
    } else if (shape.type === 'sphere') {
      make(new THREE.SphereGeometry(1, 12, 8), mat, (p) => m.compose(new THREE.Vector3(...p.pos_m), q.identity(), one.clone().multiplyScalar(p.shape.radius_m)));
    }
  }
  return group;
}

// The road as a ribbon along the graded centreline, coloured by the surface under it (asphalt, or mud where the road crosses a patch).
export function roadMesh(t) {
  const r = t.road_m ?? [];
  if (r.length < 2) return null;
  const { nx, nz, cell_m: s } = t, near = (x, z) => t.materials[t.material_ids[Math.min(nz - 1, Math.max(0, Math.round((z - t.origin_m.z) / s))) * nx + Math.min(nx - 1, Math.max(0, Math.round((x - t.origin_m.x) / s)))]];
  const pos = [], col = [], idx = [], c = new THREE.Color();
  r.forEach((p, i) => {
    const a = r[Math.max(0, i - 1)], b = r[Math.min(r.length - 1, i + 1)];
    let dx = b[0] - a[0], dz = b[2] - a[2];
    const l = Math.hypot(dx, dz) || 1;
    dx /= l; dz /= l;
    const nxn = -dz, nzn = dx, h = ROAD_WIDTH_M / 2;
    pos.push(p[0] + nxn * h, p[1] + 0.05, p[2] + nzn * h, p[0] - nxn * h, p[1] + 0.05, p[2] - nzn * h);
    c.setHex(near(p[0], p[2]) === 'mud' ? GROUND.mud : GROUND.asphalt);
    col.push(c.r, c.g, c.b, c.r, c.g, c.b);
    if (i > 0) { const k = i * 2; idx.push(k - 2, k, k - 1, k - 1, k, k + 1); }
  });
  const g = new THREE.BufferGeometry();
  g.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
  g.setAttribute('color', new THREE.Float32BufferAttribute(col, 3));
  g.setIndex(idx);
  g.computeVertexNormals();
  return new THREE.Mesh(g, new THREE.MeshStandardMaterial({ vertexColors: true, roughness: 0.95, side: THREE.DoubleSide }));
}

// Ground height at (x, z): bilinear on the heightfield, the same patch the physics reads (the display mesh is two triangles per cell and differs
// from it by a few millimetres). Used by the live page to keep the camera above the ground.
export function heightSampler(t) {
  const { nx, nz, cell_m: s } = t, ox = t.origin_m.x, oz = t.origin_m.z, h = t.heights_m;
  return (x, z) => {
    const u = Math.min(nx - 1.001, Math.max(0, (x - ox) / s)), v = Math.min(nz - 1.001, Math.max(0, (z - oz) / s));
    const i = Math.floor(u), j = Math.floor(v), a = u - i, b = v - j, k = j * nx + i;
    return h[k] * (1 - a) * (1 - b) + h[k + 1] * a * (1 - b) + h[k + nx] * (1 - a) * b + h[k + nx + 1] * a * b;
  };
}
