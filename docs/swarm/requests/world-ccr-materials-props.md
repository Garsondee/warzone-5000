# CCR (text): `Material`, `PropShape`, `bounds` additions

From lane WORLD, settling round. Evidence: `docs/lanes/world/spike-w.md` (F7) and the design note section 9. ARCH owns `crates/w5k_contract/src/world.rs`; this file is the request, not the edit. **Nothing here blocks M1**: WORLD builds on `contract-v0.1` and adapts when a CCR lands. All additions are `#[serde(default)]` or new variants.

- **W-1 `Material` += `wetness: f64`** (0 dry to 1 saturated; scales soil strength in TRACKS and rolling resistance in CHASSIS; `MaterialDef` gets a `Param`). Mud-by-drainage needs a way to say "this grass is soggy".
- **W-2 `Material` += `vegetation_drag_n_per_m_s: f64`** (drag a vehicle pays pushing through scrub, per metre of travel per m/s; 0 for bare ground). `Param` in `MaterialDef`.
- **W-3 `PropShape` += `Capsule { radius_m, height_m }`** (a tree trunk with a rounded top; cylinders catch on their rims) **and `ConvexHull { points: Vec<Vec3> }`** (rocks, wrecks). `PropShape` is `Copy` today; a hull needs either an index into a shared hull table (`ConvexHull { hull: u32 }`, my preference, keeps `Copy`) or dropping `Copy`. ARCH to choose.
- **W-4 `WorldQuery::bounds`** stays `(min, max)`; document that the y range is the terrain minimum to the tallest prop top (the spike made a number up).
- **W-5 (optional, later) `WorldQuery::normal_smooth(x, z)`** returning the normal averaged over the surrounding corner normals, for wheel models that dislike the C0 kinks (spike F2).
