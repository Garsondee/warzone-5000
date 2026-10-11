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

const isArm = (n) => n.role === 'SuspensionArm' && n.joint != null;
const armsOf = (rig) => rig.nodes.map((n, i) => (isArm(n) ? i : -1)).filter((i) => i >= 0);
// Where a node sits in the hull frame at rest (the skeletons' rest rotations are all identity, so positions add up the parent chain).
const place = (rig, i) => { const at = [0, 0, 0]; for (let k = i; k != null; k = rig.nodes[k].parent) ['x', 'y', 'z'].forEach((a, c) => { at[c] += rig.nodes[k].rest.pos[a]; }); return at; };

// A wheeled skin and its rig have the same joint layout: arm for arm, matched by joint index and kind (names may differ).
function pairWheeled(skin, phys) {
  const mine = armsOf(phys);
  return armsOf(skin).map((s) => {
    const j = skin.nodes[s].joint, p = mine.find((q) => phys.nodes[q].joint.index === j.index && phys.nodes[q].joint.kind === j.kind);
    if (p === undefined) throw new Error(`skin node ${skin.nodes[s].name} (joint ${j.index}) has no counterpart in ${phys.id}`);
    return { s, p };
  });
}

// Two tracked skeletons need not share joint numbers or places (GEOMETRY's carrier drives from the rear and FORGE's from the front), but a belt is a loop of
// wheels in a fixed order of roles (a sprocket, return rollers, an idler, road wheels) and that they do share: the left belt of the skin is the left belt of the
// rig, wheel k of one is wheel k of the other, and so are their arms. Returns the arms to move, the wheels (skin node -> rig node and the ratio of the two
// belts' radii at that place in the loop) and the rig's belts re-addressed to the skin's nodes: the sim's wheels, radii, links and sprocket, drawn with the skin's link.
function pairTracked(skin, phys) {
  const mean = (rig, run) => run.wheels.reduce((a, w) => a + place(rig, w.node)[0], 0) / run.wheels.length;
  const leftToRight = (rig) => [...(rig.track_runs ?? [])].sort((a, b) => mean(rig, a) - mean(rig, b));
  const sRuns = leftToRight(skin), pRuns = leftToRight(phys);
  if (!sRuns.length || sRuns.length !== pRuns.length) throw new Error(`the skin has ${sRuns.length} belts and ${phys.id} has ${pRuns.length}`);
  const arms = [], wheels = new Map(), runs = [];
  sRuns.forEach((sr, side) => {
    const pr = pRuns[side];
    if (sr.wheels.length !== pr.wheels.length) throw new Error(`belt ${side} runs on ${sr.wheels.length} wheels in the skin and ${pr.wheels.length} in ${phys.id}`);
    sr.wheels.forEach((sw, k) => {
      const pw = pr.wheels[k], s = skin.nodes[sw.node], p = phys.nodes[pw.node];
      if (s.role !== p.role) throw new Error(`wheel ${k} of belt ${side} is a ${s.role} in the skin and a ${p.role} in ${phys.id}`);
      wheels.set(sw.node, { p: pw.node, scale: pw.radius_m / sw.radius_m });
      const sa = s.parent, pa = p.parent, sArm = sa != null && isArm(skin.nodes[sa]), pArm = pa != null && isArm(phys.nodes[pa]);
      if (sArm !== pArm) throw new Error(`wheel ${k} of belt ${side} hangs on a suspension arm in only one of the skin and ${phys.id}`);
      if (sArm) arms.push({ s: sa, p: pa });
    });
    runs.push({ ...pr, node: sr.node, link_mesh: sr.link_mesh, wheels: pr.wheels.map((w, k) => ({ node: sr.wheels[k].node, radius_m: w.radius_m })) });
  });
  const moved = new Set(arms.map(({ s }) => s));
  if (armsOf(skin).some((s) => !moved.has(s))) throw new Error(`a suspension arm of the skin carries no wheel of a belt, so ${phys.id} cannot place it`);
  return { arms, wheels, runs };
}

// Put `skin` on the skeleton of `phys`. A wheeled pair must have the same joint layout (pairWheeled), a tracked pair the same belts (pairTracked).
//  - the suspension arms take the physics rig's rest positions, so every hub is where the simulation put it;
//  - the body (the meshes on the root node) is scaled uniformly by the ratio of the wheelbases and moved so the scaled hubs meet the real ones,
//    which keeps the body's proportions and its relation to the wheels (a scout becomes a small truck, a hauler a big one);
//  - the wheels (every other mesh) are scaled by the ratio of tyre radii, so the tyre touches the ground where the simulation says it does; a
//    tracked wheel by the ratio of the two belts' radii at its place in the loop (a sprocket by the sprockets', an idler by the idlers'), and it
//    takes the rig's joint too, so the simulation's sprocket spin turns the skin's sprocket and its suspension travel moves the skin's road wheel;
//  - a tracked skin's belts are the rig's (their wheels, radii, links and drive) re-addressed to the skin's nodes and link mesh.
export function fitSkin(skin, phys) {
  const tracked = skin.track_runs.length > 0 || (phys.track_runs ?? []).length > 0;
  const { arms, wheels, runs } = tracked ? pairTracked(skin, phys) : { arms: pairWheeled(skin, phys), wheels: new Map(), runs: [] };
  if (!arms.length) throw new Error('the skin has no suspension arms');
  const s = arms.map((a) => skin.nodes[a.s].rest.pos), p = arms.map((a) => phys.nodes[a.p].rest.pos);
  const span = (v, a) => Math.max(...v.map((q) => q[a])) - Math.min(...v.map((q) => q[a]));
  const scale = span(p, 'z') / span(s, 'z') || 1;
  const mean = (v, a) => v.reduce((t, q) => t + q[a], 0) / v.length;
  const off = ['x', 'y', 'z'].map((a) => mean(p, a) - scale * mean(s, a));
  const kw = tracked ? mean([...wheels.values()], 'scale') : tyreRadius(phys) / tyreRadius(skin) || 1;
  const root = skin.nodes.findIndex((n) => n.parent == null);
  const follow = new Map([...arms.map((a) => [a.s, a.p]), ...[...wheels].map(([sn, w]) => [sn, w.p])]); // skin node -> the rig node it stands in for
  const nodes = skin.nodes.map((n, i) => {
    if (!follow.has(i)) return n;
    const q = phys.nodes[follow.get(i)];
    return { ...n, rest: { ...n.rest, pos: { ...q.rest.pos } }, ...(tracked ? { joint: q.joint } : {}) };
  });
  const meshes = skin.meshes.map((m) => {
    if (m.node === root) return transformed(m, (x, y, z) => [x * scale + off[0], y * scale + off[1], z * scale + off[2]]);
    const k = tracked ? wheels.get(m.node)?.scale ?? 1 : kw; // a tracked skin's link meshes (on the Track nodes) are not wheels: they keep their size
    return transformed(m, (x, y, z) => [x * k, y * k, z * k]);
  });
  return { ...skin, id: `${skin.id}@${phys.id}`, nodes, meshes, material_slots: skin.material_slots, joint_count: phys.joint_count, track_runs: runs, fit: { scale, wheel_scale: kw } };
}
