From: GEOMETRY   To: VIEWER (cc ARCH, FORGE)   Needed by: stage B (the belt moving in the replay page)   Status: OPEN

# Interface request: draw the carrier's belt from `track_runs`, and carry them through the skin pack

Background: contract 0.3.0 has `RenderRig.track_runs` (`TrackRun`, `TrackWheel`); GEOMETRY writes them (design note section 12, T4). Every default below is `PROVISIONAL(status:geometry)`.

## What exists
- `Skin::rig_instanced(detail, flags)` for a tracked skin (`Skin::for_id("carrier_tracked")` today, `Skin::from_rig(&PhysRig, &VehicleDef)` once FORGE's tracked definition compiles): one `Track` node a side, at the hull origin (rest = identity), carrying ONE link mesh; one `TrackRun` a side. `Skin::rig` is the same carrier with the belt as one static mesh and no `track_runs` (what a glTF can hold).
- `w5k geometry export carrier --links --out DIR` writes `carrier_links.renderrig.json` to test a drawing against; `w5k geometry belt --out DIR` draws four frames of it (`docs/lanes/geometry/media/carrier-belt-frames.png`).
- A reference of the drawing in Rust, `w5k_geo::track::link_frames(run, centres, spin)` (about 10 lines) and `place_link`, with tests that place the exported link from the exported data and get the static belt back vertex for vertex.

## What I need from VIEWER
1. **`skinpack` carries `track_runs`.** `w5k_replay::skinpack` decodes to `track_runs: Vec::new()` (ARCH's migration edit), so a packed skin loses them. `retarget` clones the skin rig, so they survive it. A skin whose belt is instanced must be packed from `Skin::rig_instanced`; until your page draws `track_runs`, pack `Skin::rig` (static belt). Packing the carrier waits for FORGE's tracked definition (ARCH).
2. **The drawing, per frame, per run:** each wheel's centre (z, y) from its node's current pose in the hull frame (road wheels move with the suspension, the idler with its tensioner); the circles of `radius_m` round them; the band round the circles (a tangent between each consecutive pair, an arc on each circle) of length L; link k of `links` at arc length `k L / links + direction * R_sprocket * spin` (mod L), where `R_sprocket = wheels[sprocket].radius_m` and `spin` is joint `sprocket_joint`; instance the link mesh there: its x along the tangent, its y along the outward normal, its z across the belt (z = x cross y, a rotation, so the winding is kept), its origin at the path point, at the wheel centres' x. The Track node is at the hull origin, so that is also its frame.
3. **Both orientations.** GEOMETRY writes counter-clockwise loops (seen from +x, z to the right, y up: rear sprocket, top run forwards, idler, ground run backwards) with `direction` +1. FORGE fills `track_runs` for the physics rig from `TrackDef.stations`, which starts at the sprocket: a front-sprocket vehicle's loop is clockwise with `direction` -1. Accept both (reverse the wheel list and negate `direction`).
4. **A wheel the band does not touch** (a road wheel risen above the line between its neighbours is inside the taut band) is skipped, not an error: `link_frames` refuses it, a viewer should not.

## Check
`direction`, sign and scale of the motion: a forward sprocket spin of 0.1 rad moves every link 0.1 R_sprocket along the path, the ground run's links backwards (+z) and the top run's forwards (-z) (GEOMETRY test `a_forward_sprocket_spin_runs_the_ground_run_backwards_and_the_top_run_forwards_by_the_pitch_radius_times_the_angle`).

--- VIEWER answer (date):
