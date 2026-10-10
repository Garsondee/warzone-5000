// The skin reader and the fit, without a browser: node test-skin.mjs   (reads dist/skins/utility_4x4.skin)
import fs from 'node:fs';
import { unpackSkin, fitSkin } from './src/skin.js';
const bytes = fs.readFileSync(new URL('./dist/skins/utility_4x4.skin', import.meta.url));
const skin = unpackSkin(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength));
let failed = false;
const check = (ok, msg) => { console.log(`${ok ? 'PASS' : 'FAIL'}  ${msg}`); failed ||= !ok; };

check(skin.meshes.length === 67 && skin.nodes.length === 11, `the packed skin has its ${skin.meshes.length} meshes and ${skin.nodes.length} nodes`);
const tris = skin.meshes.reduce((s, m) => s + m.indices.length / 3, 0);
check(tris === 35938, `the triangle count survives packing (${tris})`);
check(skin.meshes.every((m) => m.indices.every ? true : true) && skin.meshes.every((m) => Math.max(...m.indices) < m.positions.length / 3), 'every index points at a vertex');
check(skin.meshes.every((m) => m.normals.length === m.positions.length && m.edge.length === m.positions.length / 3 && m.cavity.length === m.positions.length / 3), 'normals and flags have one entry per vertex');

// A physics rig made from the skin itself: scaled 1.15, moved, wheels 1.3 times bigger. Fitting the skin on it must undo exactly that.
const S = 1.15, KW = 1.3, OFF = { x: 0, y: -0.2, z: 0.3 };
const phys = {
  id: 'phys', joint_count: skin.joint_count,
  nodes: skin.nodes.map((n) => (n.role === 'SuspensionArm' ? { ...n, name: 'a.' + n.name, rest: { ...n.rest, pos: { x: n.rest.pos.x * S + OFF.x, y: n.rest.pos.y * S + OFF.y, z: n.rest.pos.z * S + OFF.z } } } : n)),
  meshes: skin.meshes.map((m) => (skin.nodes[m.node].role === 'Wheel' ? { ...m, positions: m.positions.map((v) => v * KW) } : m)),
};
const fit = fitSkin(skin, phys);
check(Math.abs(fit.fit.scale - S) < 1e-6, `the body scale is the ratio of the wheelbases (${fit.fit.scale.toFixed(4)} for ${S})`);
check(Math.abs(fit.fit.wheel_scale - KW) < 1e-4, `the wheel scale is the ratio of the tyre radii (${fit.fit.wheel_scale.toFixed(4)} for ${KW})`);
const arms = fit.nodes.filter((n) => n.role === 'SuspensionArm'), parms = phys.nodes.filter((n) => n.role === 'SuspensionArm');
check(arms.every((a, i) => ['x', 'y', 'z'].every((k) => Math.abs(a.rest.pos[k] - parms[i].rest.pos[k]) < 1e-9)), 'every suspension arm sits exactly where the physics rig puts it');
const root = skin.nodes.findIndex((n) => n.parent == null);
const hm = fit.meshes.find((m) => m.node === root), sm = skin.meshes.find((m) => m.node === root);
const err = Math.max(...[0, 1, 2].map((a) => Math.max(...Array.from({ length: 50 }, (_, i) => Math.abs(hm.positions[i * 3 + a] - (sm.positions[i * 3 + a] * S + [OFF.x, OFF.y, OFF.z][a]))))));
check(err < 1e-5, `the body is the skin scaled and moved onto the hubs (error ${err.toExponential(1)} m)`);
// Belts: a tracked skin's `track_runs` (what a page needs to move the links) come out of the reader exactly as they went in; a wheeled skin has none.
const runs = [{ node: 1, wheels: [{ node: 2, radius_m: 0.3125 }, { node: 3, radius_m: 0.27 }], link_mesh: 0, links: 87, sprocket: 1, sprocket_joint: 3, direction: -1 }];
const json = new TextEncoder().encode(JSON.stringify({ id: 'belt', nodes: [], material_slots: [], joint_count: 0, meshes: [], track_runs: runs }));
const file = new ArrayBuffer(Math.ceil((12 + json.length) / 4) * 4), dv = new DataView(file);
new Uint8Array(file).set([0x57, 0x35, 0x4b, 0x53]); dv.setUint32(4, 1, true); dv.setUint32(8, json.length, true); new Uint8Array(file).set(json, 12);
check(JSON.stringify(unpackSkin(file).track_runs) === JSON.stringify(runs), 'a skin with belts keeps its track_runs through the reader');
check(skin.track_runs.length === 0, 'a wheeled skin has no track_runs');
let refused = false;
try { fitSkin(skin, { ...phys, nodes: phys.nodes.map((n) => (n.joint && n.joint.index === 6 ? { ...n, joint: { ...n.joint, index: 99 } } : n)) }); } catch { refused = true; }
check(refused, 'a skeleton with a different joint layout is refused');
process.exit(failed ? 1 : 0);
