// Skins: a detailed body (GEOMETRY's truck) drawn on another vehicle's skeleton (what the simulation ran). `unpackSkin` reads the compact
// file `w5k viewer pack-skin` writes (crates/w5k_replay/src/skinpack.rs: u16 positions in the mesh's box, i8 normals, u8 flags), and
// `fitSkin` is the JS twin of `w5k_replay::skin::retarget` plus a fit to the vehicle's size.

export function unpackSkin(buffer) {
  const dv = new DataView(buffer), u8 = new Uint8Array(buffer);
  if (String.fromCharCode(...u8.subarray(0, 4)) !== 'W5KS') throw new Error('not a W5KS skin');
  if (dv.getUint32(4, true) !== 1) throw new Error('unsupported skin version');
  const hlen = dv.getUint32(8, true);
  const h = JSON.parse(new TextDecoder().decode(u8.subarray(12, 12 + hlen)));
  const base = Math.ceil((12 + hlen) / 4) * 4; // every section starts on a 4-byte boundary
  const meshes = h.meshes.map((e) => {
    const q = new Uint16Array(buffer, base + e.pos, e.vertices * 3), positions = new Float32Array(e.vertices * 3);
    for (let i = 0; i < positions.length; i++) positions[i] = e.min[i % 3] + q[i] * e.scale[i % 3];
    const n = new Int8Array(buffer, base + e.nor, e.vertices * 3), normals = new Float32Array(e.vertices * 3);
    for (let i = 0; i < normals.length; i++) normals[i] = n[i] / 127;
    const unit = (at) => { if (at == null) return new Float32Array(0); const b = new Uint8Array(buffer, base + at, e.vertices), f = new Float32Array(e.vertices); for (let i = 0; i < f.length; i++) f[i] = b[i] / 255; return f; };
    const indices = e.index_bytes === 2 ? new Uint16Array(buffer, base + e.idx, e.indices) : new Uint32Array(buffer, base + e.idx, e.indices);
    return { name: e.name, node: e.node, material_slot: e.material_slot, positions, normals, edge: unit(e.edge), cavity: unit(e.cav), indices };
  });
  return { id: h.id, nodes: h.nodes, material_slots: h.material_slots, joint_count: h.joint_count, meshes, track_runs: h.track_runs ?? [] };
}

// Visit every vertex of a mesh whatever its form: a flat typed array, or the JSON form (a list of [x, y, z]).
function eachVertex(m, fn) {
  const p = m.positions;
  if (ArrayBuffer.isView(p)) for (let i = 0; i < p.length; i += 3) fn(p[i], p[i + 1], p[i + 2]);
  else for (const v of p) fn(v[0], v[1], v[2]);
}
const transformed = (m, f) => {
  const out = ArrayBuffer.isView(m.positions) ? Float32Array.from(m.positions) : Float32Array.from(m.positions.flat());
  for (let i = 0; i < out.length; i += 3) { const [x, y, z] = f(out[i], out[i + 1], out[i + 2]); out[i] = x; out[i + 1] = y; out[i + 2] = z; }
  return { ...m, positions: out, normals: ArrayBuffer.isView(m.normals) ? m.normals : Float32Array.from(m.normals.flat()), indices: ArrayBuffer.isView(m.indices) ? m.indices : Uint32Array.from(m.indices) };
};
// Tyre radius of a rig: the largest distance from the wheel axis (the node's x axis) of any vertex of any wheel mesh.
function tyreRadius(rig) {
  let r = 0;
  rig.meshes.forEach((m) => { if (rig.nodes[m.node].role === 'Wheel') eachVertex(m, (x, y, z) => { r = Math.max(r, Math.hypot(y, z)); }); });
  return r;
}

// Put `skin` on the skeleton of `phys`. Both must have the same joint layout (matched by joint index and kind, names may differ).
//  - the suspension arms take the physics rig's rest positions, so every hub is where the simulation put it;
//  - the body (the meshes on the root node) is scaled uniformly by the ratio of the wheelbases and moved so the scaled hubs meet the real ones,
//    which keeps the body's proportions and its relation to the wheels (a scout becomes a small truck, a hauler a big one);
//  - the wheels (every other mesh) are scaled by the ratio of tyre radii, so the tyre touches the ground where the simulation says it does.
export function fitSkin(skin, phys) {
  const arms = (rig) => rig.nodes.filter((n) => n.role === 'SuspensionArm' && n.joint);
  const pairs = arms(skin).map((s) => {
    const p = arms(phys).find((q) => q.joint.index === s.joint.index && q.joint.kind === s.joint.kind);
    if (!p) throw new Error(`skin node ${s.name} (joint ${s.joint.index}) has no counterpart in ${phys.id}`);
    return { s, p };
  });
  if (!pairs.length) throw new Error('the skin has no suspension arms');
  const span = (f) => Math.max(...pairs.map(f)) - Math.min(...pairs.map(f));
  const scale = span(({ p }) => p.rest.pos.z) / span(({ s }) => s.rest.pos.z) || 1;
  const mean = (f) => pairs.reduce((a, x) => a + f(x), 0) / pairs.length;
  const off = ['x', 'y', 'z'].map((a) => mean(({ p }) => p.rest.pos[a]) - scale * mean(({ s }) => s.rest.pos[a]));
  const kw = tyreRadius(phys) / tyreRadius(skin) || 1;
  const root = skin.nodes.findIndex((n) => n.parent == null);
  const nodes = skin.nodes.map((n) => { const t = pairs.find(({ s }) => s === n); return t ? { ...n, rest: { ...n.rest, pos: { ...t.p.rest.pos } } } : n; });
  const meshes = skin.meshes.map((m) => (m.node === root
    ? transformed(m, (x, y, z) => [x * scale + off[0], y * scale + off[1], z * scale + off[2]])
    : transformed(m, (x, y, z) => [x * kw, y * kw, z * kw])));
  return { ...skin, id: `${skin.id}@${phys.id}`, nodes, meshes, material_slots: skin.material_slots, joint_count: phys.joint_count, fit: { scale, wheel_scale: kw } };
}
