// The skin reader and the fit, without a browser: node test-skin.mjs   (reads dist/skins/utility_4x4.skin and carrier_tracked.skin, samples/carrier-rig.json)
import fs from 'node:fs';
import { unpackSkin, fitSkin } from './src/skin.js';
import { linkFrames } from './src/tracks.js';
const read = (path) => { const bytes = fs.readFileSync(new URL(path, import.meta.url)); return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength); };
const skin = unpackSkin(read('./dist/skins/utility_4x4.skin'));
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

// Tracked: GEOMETRY's carrier skin (rear sprocket, its own joint numbers) on FORGE's carrier rig (front sprocket), the pair the live page meets.
const carrier = unpackSkin(read('./dist/skins/carrier_tracked.skin'));
const rig = JSON.parse(fs.readFileSync(new URL('./samples/carrier-rig.json', import.meta.url), 'utf8'));
const fitted = fitSkin(carrier, rig);
const world = (r, i) => { const at = [0, 0, 0]; for (let k = i; k != null; k = r.nodes[k].parent) ['x', 'y', 'z'].forEach((a, c) => { at[c] += r.nodes[k].rest.pos[a]; }); return at; };
const rigRun = (run) => rig.track_runs.find((q) => q.sprocket_joint === run.sprocket_joint); // the rig's belt that a fitted one is
const skinRun = (run) => carrier.track_runs.find((q) => q.node === run.node); // the skin's own belt on the same Track node
const centres = (r, run) => run.wheels.map((w) => { const q = world(r, w.node); return { z: q[2], y: q[1] }; });
const radial = (r, node) => Math.max(...r.meshes.filter((m) => m.node === node).map((m) => { let q = 0; for (let i = 0; i < m.positions.length; i += 3) q = Math.max(q, Math.hypot(m.positions[i + 1], m.positions[i + 2])); return q; }));
check(world(carrier, carrier.track_runs[0].wheels[0].node)[2] > 0 && world(rig, rig.track_runs[0].wheels[0].node)[2] < 0, 'the two carriers really differ: GEOMETRY drives from the rear (its sprocket at +z), FORGE from the front (-z)');
check(fitted.track_runs.length === 2 && fitted.track_runs.every((r) => rigRun(r) && skinRun(r)), 'the fitted skin has the simulation\'s two belts, one on each of the skin\'s Track nodes');
const away = Math.max(...fitted.track_runs.flatMap((run) => run.wheels.map((w, k) => Math.hypot(...world(fitted, w.node).map((v, c) => v - world(rig, rigRun(run).wheels[k].node)[c])))));
check(away < 1e-9, `every_tracked_wheel_sits_where_the_simulation_put_the_wheel_at_its_place_in_the_loop (worst ${away.toExponential(1)} m; fitting by joint index put wheels up to 3.18 m out)`);
check(fitted.track_runs.every((run) => run.wheels.every((w, k) => fitted.nodes[w.node].role === rig.nodes[rigRun(run).wheels[k].node].role)), 'a sprocket stays a sprocket, a return roller a return roller, an idler an idler');
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
check(fitted.track_runs.every((run) => run.wheels.every((w, k) => { const p = rig.nodes[rigRun(run).wheels[k].node]; return same(fitted.nodes[w.node].joint, p.joint) && same(fitted.nodes[fitted.nodes[w.node].parent].joint, rig.nodes[p.parent].joint); })), 'the simulation\'s joints drive the skin: each wheel spins and each arm travels on the rig\'s own joint number');
check(fitted.track_runs.every((run) => fitted.nodes[run.wheels[run.sprocket].node].role === 'Sprocket' && fitted.nodes[run.wheels[run.sprocket].node].joint.index === run.sprocket_joint), 'the belt is driven by the sprocket joint of the sprocket the skin draws');
const lengthOf = (r, run) => linkFrames(run, centres(r, run), 0).length;
check(fitted.track_runs.every((run) => { const o = rigRun(run); return Math.abs(lengthOf(fitted, run) - lengthOf(rig, o)) < 1e-9 && run.links === o.links && run.direction === o.direction && run.wheels.every((w, k) => w.radius_m === o.wheels[k].radius_m); }), `the_fitted_belt_is_as_long_as_the_simulations_belt (${lengthOf(fitted, fitted.track_runs[0]).toFixed(3)} m, ${fitted.track_runs[0].links} links, the rig's radii and direction)`);
check(fitted.track_runs.every((run) => run.link_mesh === skinRun(run).link_mesh && fitted.meshes[run.link_mesh].node === run.node && fitted.nodes[run.node].role === 'Track'), 'the links drawn are the skin\'s own link meshes on its Track nodes');
check(fitted.track_runs.every((run) => { const a = fitted.meshes[run.link_mesh].positions, b = carrier.meshes[run.link_mesh].positions; return a.length === b.length && a.every((v, i) => v === b[i]); }), 'a link keeps the size GEOMETRY modelled it at: only wheels are scaled');
const run0 = fitted.track_runs[0], was = skinRun(run0), ratio = (k) => radial(fitted, run0.wheels[k].node) / radial(carrier, was.wheels[k].node), want = (k) => run0.wheels[k].radius_m / was.wheels[k].radius_m;
check(run0.wheels.every((_, k) => Math.abs(ratio(k) - want(k)) < 1e-6) && want(0) < 1 && want(1) > 1, `a_wheel_is_scaled_by_the_ratio_of_the_belts_radii_at_its_place (sprocket x${want(0).toFixed(3)}, return roller x${want(1).toFixed(3)}, road wheel x${want(5).toFixed(3)}: some shrink, some grow)`);
const zs = (r) => r.nodes.filter((n) => n.role === 'SuspensionArm').map((n) => n.rest.pos.z), span = (r) => Math.max(...zs(r)) - Math.min(...zs(r));
check(Math.abs(fitted.fit.scale - span(rig) / span(carrier)) < 1e-9, `the hull is scaled by the ratio of the wheelbases, whichever end the sprocket is at (${fitted.fit.scale.toFixed(4)})`);
const refuses = (s, r) => { try { fitSkin(s, r); return false; } catch { return true; } };
check(refuses(carrier, { ...rig, track_runs: rig.track_runs.map((r, i) => (i === 0 ? { ...r, wheels: r.wheels.filter((_, k) => k !== 2) } : r)) }), 'a belt that runs on a different number of wheels is refused');
check(refuses(carrier, { ...rig, nodes: rig.nodes.map((n) => (n.role === 'Idler' ? { ...n, role: 'RoadWheel' } : n)) }), 'a belt whose loop of roles differs (a road wheel where the skin has an idler) is refused');
check(refuses(carrier, { ...rig, track_runs: rig.track_runs.slice(0, 1) }) && refuses(carrier, { ...rig, track_runs: undefined }) && refuses(skin, rig), 'a skin and a rig that disagree about having belts are refused, either way round');
process.exit(failed ? 1 : 0);
