# CCR (text): shape parameters, track runs, flag validation (contract 0.1.2)

From lane GEOMETRY, settling round. ARCH owns the contract; this file is the request, not the edit. All additions are `#[serde(default)]` so existing RON and JSON still load.

## (a) Shape parameters: `ShapeDef` on the defs
```rust
/// Which GEOMETRY template builds the shape and its shape-only parameters (glacis angle, bevel, sponson depth): every value a `Param` with a band and provenance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShapeDef {
    pub template: String,                 // e.g. "utility_4x4", "tracked_apc"; unknown name = a check() error
    pub params: Vec<(String, Param)>,     // ordered list, not a map: iteration order is part of the determinism rule
}
// HullDef   += #[serde(default)] pub shape: Option<ShapeDef>,
// TurretDef += #[serde(default)] pub shape: Option<ShapeDef>,
// TrackedDef += #[serde(default)] pub shape: Option<ShapeDef>,   // road wheel, sprocket, idler, link style
// TyreSliders += #[serde(default)] pub tread: Option<TreadDef>,
pub struct TreadDef { pub pattern: String, pub depth_m: Param, pub blocks_around: u16 }
```
Dimensions the vehicle already has (`length_m`, `width_m`, `height_m`, `ground_clearance_m`, wheel diameters, track gauge) stay where they are and drive the template; `ShapeDef` carries only what the dossier lacks. Affects FORGE (passes the defs through to `w5k_geo`) and VALIDATION (their dossier definitions of each dimension decide which box `HullDef::length_m` measures; see the design note, section 6).

## (b) `RenderRig::track_runs`: animating the belt
A `Track` node has no joint and no belt data, so a viewer cannot move links. Proposal:
```rust
// RenderRig += #[serde(default)] pub track_runs: Vec<TrackRun>,
pub struct TrackRun {
    pub node: usize,                 // the Track node of this side (the links are drawn in its frame)
    pub wheels: Vec<TrackWheel>,     // in belt order, the belt running outside every circle, so the path is the taut wrap of the circles
    pub link_mesh: usize,            // index into `meshes`, one link at the origin, +X along travel, +Y out of the belt
    pub pitch_m: f64,                // design pitch
    pub sprocket: usize,             // index into `wheels` of the driven wheel
    pub sprocket_joint: usize,       // `VehicleFrame::joints` index of its spin (rad), positive rolling forward
}
pub struct TrackWheel { pub node: usize, pub radius_m: f64 }   // radius to the belt's inner surface plus half its thickness
```
**The viewer's job, under 50 lines:** (1) each frame take every wheel centre in the Track node's frame (its node pose with the joint applied: road wheels move with the suspension); (2) build the path: a tangent segment between each consecutive pair of circles and an arc around each circle; its length is L; (3) n = round(L / pitch), p' = L / n; (4) link k sits at arc length s = (k p' + phase) mod L with phase = sprocket pitch radius x sprocket angle; (5) put the link at the path point of s, with +X along the path tangent. GEOMETRY supplies the link mesh and the wheel order and radii; the test A11 pins the maths in `w5k_geo` so VIEWER and GODOT can copy a reference function (`w5k_geo::track::path_point(wheels, s)`) into their language. Affects FORGE (fills it), VIEWER and GODOT (draw it).

## (c) `RenderRig::validate()` and the flags
Today `validate()` checks that `edge` and `cavity` are empty or one value per vertex, nothing else. Add: every value finite and inside 0..=1; every position and normal finite; and document on `MeshPart` that **empty `edge` or `cavity` means "no flag", which a renderer treats as 0**, and that GEOMETRY bakes `cavity` in the vehicle's design pose (a part moved by a joint carries the occlusion of that pose). No type change, one doc comment and about ten lines of checks.
