From: GEOMETRY   To: FORGE (cc TRACKS, VIEWER, ARCH)   Needed by: slice 2 stage B (the carrier skin in the next package)   Status: OPEN

# Interface request: the tracked carrier's stations, belt and joint layout

Background: design note section 12 (`docs/lanes/geometry/design-note.md`). GEOMETRY builds the carrier (an M113 archetype, fictional, about 11 t) from the module and socket kernel; this file says what it reads from FORGE's rig and what it needs FORGE to settle. Every default below is `PROVISIONAL(status:geometry)`: GEOMETRY takes it and carries on.

## What I need
1. **The skin is built from the compiled rig.** `Skin::from_rig(&PhysRig, &VehicleDef)` reads `rig.stations` (kind, side, `rest_pos_m`, `wheel.radius_m`, `wheel.width_m`, `bump_dir`) and `rig.tracks` (loop order, `pitch_m`, `thickness_m`, `sprocket_teeth`, `belt_length_m`, `contact_length_m`) and draws each wheel exactly where the rig puts it. So **station order and names are yours**: the skin's joint coordinates follow `rig.stations` and `PhysRig::joint_names()`, whatever order you build. Default: yes.
2. **The conventions the contract already states, kept.** Sprocket, idler, road wheels and return rollers are all stations (rollers in the loop, red-team H-4). `wheel.radius_m` is the tip radius, except the sprocket's, which is its **pitch** radius `pitch_m / (2 sin(pi / teeth))`. The ground plane is `thickness_m` below the lowest road-wheel bottoms. The hull frame has its origin at the hull box centre (`PROVISIONAL(D3)`).
3. **Return rollers on the top run.** Put each roller's top on the straight line from the idler's top to the sprocket's top (belt thickness allowed for), so the belt touches it. GEOMETRY draws the belt as the taut band round the wheels and a test names any wheel that does not touch it (a roller too low would be drawn floating under the belt). The def fields are yours (`return_rollers`, `idler { diameter_m, from_front_m, tensioner_travel_m }`, `belt_thickness_m`, `sprocket_teeth`: your CCR T1); GEOMETRY needs nothing else added to `TrackedDef`. Shape-only numbers (guide horn, grouser, sponson depth, glacis angle) stay in GEOMETRY's template until `ShapeDef` (CCR section (a)) is accepted.
4. **Which box `HullDef` measures for a tracked vehicle.** Default: `width_m` is the overall width over the tracks (outer faces; the sponsons are no wider), `height_m` the hull box top above the belly (hatches, ring mount, antennas are fittings), `ground_clearance_m` belly to ground, `length_m` the hull body (the tracks may overhang it at the idler and sprocket). The A10 dossier comparison will say which definition matched.
5. **Vertical travel for road wheels and rollers, horizontal for the idler's tensioner** (`bump_dir`), zero travel for the sprocket and rollers, as in `box_tank()`; every station keeps its spin and travel coordinate, so every skin node finds its twin in `w5k_replay::skin::retarget`.
6. **A name for the def** (default `carrier_tracked`; the skin id and template follow it).

## What GEOMETRY will give back
`Skin::from_rig`, `Skin::parts(detail)` and `export::render_rig`, with the joint layout equal to `joint_names()` (a test); `w5k geometry carrier`; and the belt's numbers (length, links, effective pitch, straight ground run) so FORGE and TRACKS can check them against `belt_length_m` and `contact_length_m` (a GEOMETRY test fails if the drawn ground run differs from `contact_length_m` by more than 5%).

## Contract text for ARCH (a CCR for contract 0.3, once FORGE and VIEWER agree): animating the belt
Refines section (b) of `geometry-ccr-shapes-and-tracks.md`. The only fields the contract lacks today to draw a moving belt:
```rust
// RenderRig += #[serde(default)] pub track_runs: Vec<TrackRun>,
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackRun {
    pub node: usize,              // the Track node of this side; links are drawn in its frame
    pub wheels: Vec<TrackWheel>,  // in loop order (`TrackDef.stations`); the band round them is the belt path
    pub link_mesh: usize,         // index into `meshes`: ONE link at the origin, +X along travel, +Y out of the belt (instanced)
    pub links: u16,               // design count n; the viewer uses p' = L / n for the path length L it computes
    pub sprocket: usize,          // index into `wheels` of the driven wheel
    pub sprocket_joint: usize,    // joint index of its spin (rad, positive = rolling forward)
    pub direction: i8,            // +1 if positive spin advances the links along `wheels` order, -1 if against it (front sprocket)
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackWheel { pub node: usize, pub radius_m: f64 }  // node carrying the wheel's spin; path radius: tip + half belt thickness, the sprocket's pitch radius
```
Viewer's job, under 50 lines: each frame take every wheel centre from its node's current pose (road wheels move with the suspension), build the path (a tangent segment between each consecutive pair of circles, an arc on each; length L), place link k at arc length `(k p' + direction * R_sprocket * spin) mod L`, oriented by the path tangent. `RenderRig::validate()` adds: indices in range, `links >= 3`, `wheels.len() >= 3`. Triangle counts treat an instanced link mesh once (ARCH's tracked budget). Affects FORGE (fills it from `TrackDef`), VIEWER (draws it), GODOT later, VALIDATION (none).

## What I will do meanwhile
Build against `PhysRig` as the contract stands: the belt is one static mesh per side (the links at phase 0 merged into the `Track` node), which every viewer already draws; the wheels spin and move with their joints. When `track_runs` lands, the static mesh becomes the instanced link and nothing else changes. Stand-in dimensions for pictures and tests come from `box_tank()`'s layout scaled to the carrier, `PROVISIONAL(C-002)`.

--- ARCH answer (date):
