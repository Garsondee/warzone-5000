# Interface request: modular vehicles (hull, running gear, weapon mount, weapon) between GEOMETRY and FORGE

From lane GEOMETRY. Owner goal (2026-10-10): a vehicle is a **separate hull, separate propulsion, separate weapon mount and separate weapons**. GEOMETRY has built the geometry side (design note section 9, `w5k_geo::module`, `truck`, `gear`, `mount`, `weapon`, `export`); this file says what FORGE can rely on and what GEOMETRY needs from FORGE. Nothing here edits the contract: the contract text is at the end, for ARCH, as a CCR to be written once FORGE agrees.

## What exists (`w5k_geo`, all pure functions of their parameters, f64, ordered, deterministic)
- **Module and socket kernel** (`module.rs`). A `Module` is parts in its own frame, the `Socket`s it offers and one `mount` socket. A `Socket` is a frame (outward normal = local +Y, reference direction = local -Z), a `SocketKind` (`Station` wheel position, `Ring` turret or mount ring, `Trunnion` weapon pivot; extended as families need: keel, belly, mast, engine bay, track run), a side, a size, an optional axle index, the role of the node that carries it and named numeric hints. `Assembly::attach(socket, module, spin, label)` puts the child's mount on the socket (normals opposed, references together), refuses with a reason on kind mismatch or `child.size > socket.size`, mirrors a non-symmetric module for a left socket, and names every part `<name>.<label>`.
- **Families.** `truck::utility_hull(dims, axles_z, detail)` (hull; publishes `station.<axle>.<r|l>` at each hub, size = wheel radius cut for, hints `well_x_m` and `max_width_m`, and the roof `Ring` socket from its RON), `gear::wheel_module(wheel, segments, knuckle)`, `mount::ring_mount(dims, detail)` (publishes `trunnion`, hint `cradle_w_m`), `weapon::gun_module(dims, cradle_w_m, detail)` with presets `machine_gun_12_7` and `autocannon_25`. The recipe `truck::utility_assembly(dims, axles_z, steered, detail)` is hull plus one wheel module per station; a caller then attaches a mount to `roof` and a gun to `trunnion.<mount label>`.
- **Rig export** (`export::render_rig(id, parts, flags)`): any number of axles; nodes per station `travel > steer (steered axles) > wheel`, per mount `turret_yaw > gun_pitch > gun_recoil` (axes +Y, +X, +Z; a second mount gets `.1`), each gun hung from the node that carries the socket it sits on; joint coordinates in the contract's layout (`PhysRig::articulation_offset`: spin per station, steer per steered station, travel per station, then the articulation chains in attach order). Test: forward kinematics of the exported nodes at rest reproduces the assembled geometry, and positive yaw turns the barrel left, positive pitch raises it, positive recoil slides it back.

## What FORGE can do with it
1. **Joint anchors for the physics rig, from the same source as the picture.** `RenderNode::rest` of a chain node is its joint's anchor in its parent's frame, so `JointDef::anchor_m` for `turret_yaw`, `gun_pitch` and `gun_recoil` can be read from the exported rig (or from `Assembly`) and the render and physics cannot disagree.
2. **Mass properties per articulated body.** `w5k_geo::mass` gives exact volume, centroid and inertia of every closed part; FORGE multiplies by its material densities. The parts of one node (all `Turret` parts, all `GunPitch` parts, all `Recoil` parts of one placement) are one `JointDef::body`.
3. **Fit checks that are physics, not rules.** `attach` gates only on kind and size; whether a legal combination is any good (mass on the roof, recoil impulse against the hull, balance) is FORGE's and COMBAT's to judge from the assembled numbers.

## What GEOMETRY needs from FORGE (proposals, `PROVISIONAL(status:geometry)`)
- **A place in `VehicleDef` for the modules.** Today `VehicleDef` carries hull dimensions and `RunningGearDef`, which GEOMETRY maps to `UtilityDims` plus `axles_z` (from `AxleDef::from_front_m`) and `steered`. For mounts and weapons: an ordered list of attachments, each naming the socket, the module family and its slider values, e.g. `mounts: Vec<MountDef { socket: String, family: String, params: Vec<(String, Param)>, weapon: Option<WeaponRef> }>`, with `weapon` a catalogue id (`WeaponDef::catalogue_id`) that GEOMETRY maps to a gun preset (`machine_gun_12_7`, `autocannon_25`) and its dimensions. Every number a `Param` with band and provenance, as everywhere.
- **Which hull sockets a hull template offers** is data in the template (the hull RON's `sockets`); FORGE only needs to know their names to validate a `VehicleDef` (a `MountDef` whose socket does not exist, or whose ring is wider than the socket, is a `check()` error with GEOMETRY's refusal text).
- **Ride height and datum** stay as agreed in design note section 6 (`PROVISIONAL(D3)`): the hull frame has its origin at the hull box centre; modules are in the hull frame.

## Contract text for ARCH (a CCR, once FORGE agrees)
```rust
// VehicleDef += #[serde(default)] pub mounts: Vec<MountDef>,
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MountDef {
    pub socket: String,                      // a socket the hull template offers, e.g. "roof"
    pub family: String,                      // e.g. "ring_mount"; unknown name = a check() error
    pub params: Vec<(String, Param)>,        // ordered list: iteration order is part of the determinism rule
    pub weapon: Option<WeaponRef>,
}
pub struct WeaponRef { pub catalogue_id: String, pub params: Vec<(String, Param)> }
```
`RenderRig` needs no change: the chains already fit its nodes, joints and `joint_count`.

## Not claimed
Tracked running gear (road wheels, sprocket, idler, track run) is not yet a module; it goes on the same sockets (a `Station` per road wheel plus a track-run socket) once the M113 is built. A turret (as opposed to a ring mount) is a bigger `Ring` module with a heavier `Trunnion` cradle and needs a hull with a ring that wide.
