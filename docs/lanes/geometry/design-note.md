# GEOMETRY design note (settling round)

**Purpose.** One closed, tagged, measured geometry feeds the look, the mass, the armour and the collision. Vehicles are built from parametric parts (card C-004: never hand-modelled), so every generator takes numbers and returns a part. Evidence for the flag decisions: `spike-g.md`. CCRs: `docs/swarm/requests/geometry-ccr-shapes-and-tracks.md`.

## 1. Pipeline of one part (pure function of its parameters; f64 inside, f32 once at export; ordered iteration, no `HashMap`)
`template (RON, normalised) + Params (metres) -> sections -> loft / extrude / bevel -> weld -> checks (closed, outward, no sliver) -> Part`. Then, for a whole vehicle in its design pose: `edge` per part, `cavity` on the assembled set, `f32` buffers, `MeshPart`.

## 2. Flags (decisions; spike S-G)
- **`edge`**: convex dihedral only, ramp 20 to 70 degrees, concave 0, vertex = max of its edges. `PROVISIONAL(status:geometry)` until LOOK agrees; the three numbers are `Param`s with provenance ESTIMATE ("looks right on a chamfer", band 10 to 30 and 55 to 85 degrees).
- Generator rules that make it work: one inset loop per hard edge at the band width (default 5 cm, ESTIMATE); a vertex at least every 0.5 m on a hard edge so corner values do not run along it; hard-edge loops are budgeted separately (a CI check: loops under 25% of a vehicle's triangles; measured at build step 6).
- **`cavity`**: cosine-weighted Hammersley rays, cap 0.5 m, origin lifted 0.5 mm (ESTIMATE), baked on the assembled vehicle in its design pose, BVH, one thread. **Default 256 rays** (3.1 s for 30k triangles; 1024 is 11.6 s and breaks the 10 s kill line, so 1024 is for the oracle tests only).

## 3. Mass API for FORGE (`w5k_geo::mass`, build step 1)
```rust
pub struct MassProps { pub volume_m3: f64, pub centroid_m: Vec3, pub second_moment_m5: Mat3 /* about the centroid, density 1 */ }
impl Mesh { pub fn mass_props(&self) -> MassProps; pub fn shell_props(&self, thickness_m: f64) -> MassProps /* thin shell for armour */ }
impl MassProps { pub fn translate(&self, d: Vec3) -> MassProps; pub fn rotate(&self, r: &Mat3) -> MassProps; pub fn scale(&self, s: f64) -> MassProps /* V s^3, I s^5 */;
                 pub fn sum(parts: &[MassProps]) -> MassProps /* parallel-axis */; pub fn at_density(&self, kg_m3: f64) -> (f64 /*kg*/, Mat3 /*kg m^2*/) }
```
Unit-density means FORGE multiplies by a density (or scales to a stated mass: mass = density x volume, so a stated mass fixes the density). Exact for the mesh as meshed (divergence theorem, no sampling); tests A2 and A3. `shell_props` is the same sum over triangles with area instead of volume, for COMBAT's armour in M3; I will build it only when asked.

## 4. Part API
```rust
pub struct Part { pub name: String, pub role: NodeRole, pub station: Option<u8> /* axle or road wheel index */, pub side: Side /* Left, Right, Centre */,
                  pub slot: SlotKind, pub fitting: bool /* mirrors, handles: not in the hull box */, pub mesh: Mesh }
pub fn build_vehicle(shapes: &ShapeSet, detail: u8) -> Vec<Part>;     // ShapeSet from the defs (CCR a); a hull-frame, design-pose set
impl Part { pub fn to_mesh_part(&self, node: usize, slot: usize) -> MeshPart }   // the one f64 -> f32 conversion
```
FORGE builds the nodes (hull > travel > steer > wheel, one `Track` node per side) and maps `(role, station, side)` to a node index. Wheel parts are in the wheel node frame, hub at the origin, axle along local X; the hull parts are in the hull frame.

## 5. Where shape parameters live
Normalised shape templates (RON, `include_str!`) in `crates/w5k_geo/shapes/`: station sections as fractions of length, width, height, which is how a loft is authored (a ship's lines plan). Real dimensions come from the `VehicleDef` as `Param`s; a shape number the dossier lacks (glacis angle, bevel, sponson depth) is an ESTIMATE with a band and a reason, carried in `ShapeDef` (CCR a). Nothing physical is typed in Rust.

## 6. Datum, ride height, dimensions (to agree with FORGE; `PROVISIONAL(D3)`)
Hull frame origin = the centre of the **hull box** (FORGE's D3), +Y up, -Z forward. Hull box = the bounding box of the `Hull` parts with `fitting == false`, greenhouse included, mirrors, aerials and tow hooks excluded; `HullDef::length_m/width_m/height_m` mean that box and A10 prints which definition matched. Ride height = clearance + height / 2, from FORGE; wheel bottoms touch -ride_height within 1 mm (A8). VALIDATION decides what a dossier's "height" includes; GEOMETRY follows and says so in the test output.

## 7. `detail` and the budget
`detail: u8` = 0 coarse (24-gon wheels, no tread blocks, fewer loops), 1 default (48-gon), 2 reference (128-gon, tread, full loops). It scales circular segment counts and loop rings only; shapes and masses do not change except by the 1.0 mm sag limit at level 2. Level 2 is what A8 tests (sag R (1 - cos(pi/N)) = 0.4 m x 0.0003 = 0.1 mm at N = 128). It is a tessellation level for LOOK's LOD test, not a LOD system. Budget as in the brief: wheeled 40k triangles, tracked 80k with instanced links counted once, 30 draw calls; the real numbers come at build steps 6 and 7 and VIEWER's software-GL timing.

## 8. The hull is one shell, and everything else is anchored to it (owner feedback 2026-10-10)
The first truck was a heap of separate boxes: windows, mirrors and bumpers floated off the body and each mud guard was a slab lying on its wheel. The rebuilt hull (`truck.rs`, `shapes/utility_4x4.ron`) follows three rules.
- **One skin.** The shell is a single loft along the length. Each section is a T-shaped ring: a full-width lower body (bottom to belt line) with a narrower hood, cab or bed block on top. The **wheel arches are openings cut into the section**: at the wheel stations the lower outer corners of the ring are notched, and the notch height follows the wheel circle (`sqrt((R + gap)^2 - dz^2)`), sampled at 12 extra sections per arch. A lip, swept around the arch circle (`sweep_arc`), stands proud of the wall, so the mud guard wraps over and down both sides of the tyre and flows into the body.
- **Anchors, not coordinates.** A fitting is placed relative to a surface of the shell: `Upper` / `Lower` (metres from the walls), `Belt` / `Top` (from the belt and roof lines), `Front` / `Rear` (from the end faces), and it overlaps the surface by a couple of centimetres. Glass follows the roof line (`Pane`, `Windscreen`), panels follow the top line (`Overlay`). Changing the dimensions moves the surfaces and the fittings with them.
- **A test says so.** `every_fitting_is_embedded_in_the_hull_or_in_a_fitting_that_is` (volumes must overlap, chained through the mirror arm and handle), `plates_and_glass_hug_the_shell_within_3_cm` and `wheels_clear_the_shell_and_the_arches_open_over_every_tyre` fail the build when something floats or a wheel pokes through; the 200 random in-range sets still close every part.

Why not boolean operations? Cutting an arch out of a box needs a mesh CSG kernel (robust intersection, tolerance handling), which is a project of its own and a determinism risk. Cutting the arch into the *section* costs nothing: the loft already interpolates between sections, so the notch height is just one more number per station.

**Vertex rows for the flags** (spike S-G, applied along the length): `loft_beveled` gives every part a chamfered end, an inner ring `band` further in (so the end face has a zero-flag interior), repeats the ring `bevel` and `bevel + band` from each end, and adds a row `band` either side of every crease station. Without them the `edge` flag smeared across large faces as clouds. Costs: the truck is 34.1k triangles of the 40k budget (subdivision edge 0.4 m; 0.3 m gave 41.8k). `triangulate_ring` (ear clipping with collinear points put back by fanning) caps non-convex end rings, so the T-shaped nose face closes.

## 9. Risks
- Per-vertex flags cost triangles: measured on the hood only; whole-vehicle test at step 6, fallback in the spike note.
- Welding at 1e-5 m must not merge distinct features of small chamfers (15 mm is 1500x the tolerance: fine).
- The dossier definitions (A10) are VALIDATION's; until the M998 dossier lands the test prints PLACEHOLDER with wide bands, `PROVISIONAL(C-002)`.
- `track_runs` needs ARCH, VIEWER, GODOT and FORGE; until contract 0.1.2 lands the track is a simple belt mesh per side (the stand-in).
- Windows bake and golden bits are unchecked here (Linux only).
