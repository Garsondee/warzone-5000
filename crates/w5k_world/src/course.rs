//! The course generator: a `CourseDef` (RON, every number a `Param`) and a seed give a `GridWorld`, the same way every time.
//!
//! Stages (docs/lanes/world/design-note.md section 6): hills (domain-warped fBm plus an optional named hill, then a maximum-grade
//! clamp), then the road (slope-cost A* between waypoints, a graded profile, stamped into terrain and splat map). Every stage is a pure
//! function of the definition; the only containers are `Vec`s and a heap with a total order, so iteration order cannot leak in.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use serde::{Deserialize, Serialize};
use w5k_contract::param::Param;
use w5k_contract::world::{MaterialId, PropId, PropKind, PropRef, PropShape, WorldQuery};
use w5k_math::{scalar, Pcg32, Transform, Vec3};

use crate::bridge::{place, rails, BridgeDef, BridgeKind, PlacedBridge};
use crate::cliff::{switchback_path, CliffDef, SwitchbackDef};
use crate::corrugation::{Corrugation, Ripple};
use crate::detail::{gd_from_rms, unit_layer};
use crate::features::{
    barricade_blocks, cell_slope, dilate, mud_mask, poisson_disc, poisson_disc_in, BarricadeDef, MudDef, RockFieldDef,
    SoftPatchDef, TreesDef,
};
use crate::grid::{warped_fbm, GridWorld, CELL_M};
use crate::river::{carve, RiverDef};
use crate::strip::standard_material_table;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HillFeature {
    pub x_m: f64,
    pub z_m: f64,
    pub radius_m: Param,
    pub height_m: Param,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HillsDef {
    pub amplitude_m: Param,
    pub wavelength_m: Param,
    pub warp_m: Param,
    pub octaves: u32,
    /// Steepest terrain slope, rise over run: a promise, enforced by `limit_grades`.
    pub max_grade: Param,
    #[serde(default)]
    pub feature: Option<HillFeature>,
    /// ISO 8608 level `G(n0)` (m^3, at n0 = 0.1 cycles/m) of the ground's small-scale detail, which follows `n^-2` over 2.5 to 20 m
    /// (`detail.rs`). `None` leaves the hills alone (their own spectrum falls as `n^-3`).
    #[serde(default)]
    pub detail_gd_n0_m3: Option<Param>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoadDef {
    /// Plan positions `(x, z)`, first = start, last = finish.
    pub waypoints: Vec<(f64, f64)>,
    pub width_m: Param,
    pub shoulder_m: Param,
    /// Steepest road slope along the centreline, rise over run.
    pub max_grade: Param,
    /// Name of a material in `content/world/materials.ron`.
    pub surface: String,
    /// ISO 8608 level `G(n0)` (m^3) of the road's surface detail: the road's *condition* (class B good, C average, D poor). When absent
    /// the level follows from the surface material's `roughness_rms_m`. Only used when the course has ground detail (`hills.detail_gd_n0_m3`).
    #[serde(default)]
    pub roughness_gd_n0_m3: Option<Param>,
    /// Stretches where the road itself is mud (a ford, a washed-out section the vehicle must cross).
    #[serde(default)]
    pub mud_crossings: Vec<MudCrossing>,
    /// Zig-zag climbs up cliffs, spliced into the waypoint list.
    #[serde(default)]
    pub switchbacks: Vec<SwitchbackDef>,
    /// Bridges over rivers, spliced into the waypoint list.
    #[serde(default)]
    pub bridges: Vec<BridgeDef>,
    /// Corrugation (wavelength 0.2 to 2 m, a few cm deep).
    #[serde(default)]
    pub washboards: Vec<RoughSectionDef>,
    /// Whoops (wavelength 4 m or more, tenths of a metre deep).
    #[serde(default)]
    pub whoops: Vec<RoughSectionDef>,
}

/// A rough stretch of the road: washboard (ripples shorter than 2 m, finer than the grid, done as a query-time formula) or whoops
/// (long swells baked into the heightfield). `amplitude_m` is peak to peak.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoughSectionDef {
    /// Centre of the section along the road, 0 = start, 1 = finish.
    pub at_fraction: Param,
    pub length_m: Param,
    pub wavelength_m: Param,
    pub amplitude_m: Param,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MudCrossing {
    /// Position along the road, 0 = start, 1 = finish.
    pub at_fraction: Param,
    pub length_m: Param,
    /// A pit: the road dips this deep (m) over the crossing, with eased walls, so the mud lies in a hollow a vehicle must drive down
    /// into and climb out of. Absent: a flat stretch of mud.
    #[serde(default)]
    pub pit_depth_m: Option<Param>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourseDef {
    pub name: String,
    pub seed: u64,
    /// Grid nodes per side (1 m apart), e.g. 401 for a 400 m square.
    pub size_cells: usize,
    pub hills: HillsDef,
    pub road: RoadDef,
    /// More roads (tracks, detours); each is laid like the main one with its own width, grade, surface and features.
    #[serde(default)]
    pub extra_roads: Vec<RoadDef>,
    #[serde(default)]
    pub mud: Option<MudDef>,
    #[serde(default)]
    pub trees: Option<TreesDef>,
    #[serde(default)]
    pub barricade: Option<BarricadeDef>,
    #[serde(default)]
    pub rock_fields: Vec<RockFieldDef>,
    #[serde(default)]
    pub soft_patches: Vec<SoftPatchDef>,
    #[serde(default)]
    pub cliffs: Vec<CliffDef>,
    #[serde(default)]
    pub rivers: Vec<RiverDef>,
}

impl CourseDef {
    pub fn from_ron(text: &str) -> Result<CourseDef, String> {
        ron::from_str(text).map_err(|e| format!("course: {e}"))
    }
}

/// A generated course: the world plus the road centreline `(x, z, y)` at grid nodes, start to finish.
pub struct Course {
    pub world: GridWorld,
    pub road: Vec<(f64, f64, f64)>,
    /// The centrelines of `extra_roads`, in order.
    pub extra_roads: Vec<Vec<(f64, f64, f64)>>,
    /// Every bridge laid, on any road.
    pub bridges: Vec<PlacedBridge>,
    pub ground: MaterialId,
    pub road_material: MaterialId,
    /// The steepest grade (rise over run) each node is allowed: the hills limit, or a cliff's own near a cliff.
    pub grade_limit: Vec<f64>,
}

/// Salt separating the tree-stand noise from the hills noise (same seed, different field).
/// Noise values at which a stand fades from empty to full density.
const STAND_NOISE_EDGES: (f64, f64) = (-0.15, 0.35); // const-ok: shape of the clumping
const STANDS_SALT: u64 = 0x7EEE; // const-ok: noise stream label

/// A rock's centre sits this fraction of its radius above the ground, so it is partly buried and presents a rounded face.
pub const BURIAL: f64 = 0.4; // const-ok: how deep rocks sit in the ground

/// Headroom on a cliff zone's axis limit over the face grade plus the hills' own slope.
const CLIFF_LIMIT_SLACK: f64 = 1.1; // const-ok: margin so the clamp never shaves the face it is told to allow

/// Run-in and run-out of a switchback beyond the foot and the lip of the face, m.
const SWITCHBACK_APPROACH_M: f64 = 8.0; // const-ok: where the zig-zag meets the ordinary road

/// Shortest whoop wavelength the 1 m grid can hold (four cells), m.
const MIN_WHOOP_WAVELENGTH_M: f64 = 4.0; // const-ok: four cells per wave is the least a bilinear grid draws as a wave
/// Washboard wavelengths handled as a formula, m.
const MIN_WASHBOARD_WAVELENGTH_M: f64 = 0.2; // const-ok: below this a wheel does not follow the ripples
const MAX_WASHBOARD_WAVELENGTH_M: f64 = 2.0; // const-ok: the grid's Nyquist limit
/// Width over which rough sections fade out beyond the road edge, m.
const ROUGH_EDGE_M: f64 = 1.5; // const-ok: shoulder of a rough section
/// Shortest fade at the ends of a rough section, m.
const ROUGH_MIN_FADE_M: f64 = 2.0; // const-ok: ends of a rough section

/// Cells this close beyond a deck's edge are exempt from the grade limits (the drop into the water), m.
const DECK_LIMIT_MARGIN_M: f64 = 2.0; // const-ok: width of the drop beside a deck

/// Steepest slope of `smoothstep` relative to its mean: 3/2 (so a drop `D` eased over `L` has a slope of at most `1.5 D / L`).
const SMOOTHSTEP_PEAK: f64 = 1.5; // const-ok: d/dx of 3x^2 - 2x^3 at x = 1/2

/// Longest wall of a mud pit, m.
const PIT_WALL_M: f64 = 6.0; // const-ok: how gently the pit's walls are eased

/// The profile is graded to this fraction of the stated road grade; sampled bilinearly the road can add a sliver of slope.
const PROFILE_MARGIN: f64 = 0.9; // const-ok: safety factor on the stated grade

/// Raster sweeps of `v[p] = op(v[p], v[q] + sign * L)` over the 4-neighbour grid with a limit per cell (an edge uses the larger of its two
/// cells' limits), repeated until nothing changes: the distance transform that turns a field into the tightest Lipschitz field above
/// (`lower`, sign +) or below (sign -) it. With one limit everywhere a single forward and backward pass is exact; with varying limits the
/// repeat makes it so.
fn sweep(v: &mut [f64], n: usize, lim: &[f64], lower: bool) {
    let step = |a: f64, b: f64, l: f64| if lower { a.min(b + l) } else { a.max(b - l) };
    for _ in 0..SWEEP_REPEATS {
        let before = v.to_vec();
        for j in 0..n {
            for i in 0..n {
                let c = j * n + i;
                if i > 0 {
                    v[c] = step(v[c], v[c - 1], lim[c].max(lim[c - 1]));
                }
                if j > 0 {
                    v[c] = step(v[c], v[c - n], lim[c].max(lim[c - n]));
                }
            }
        }
        for j in (0..n).rev() {
            for i in (0..n).rev() {
                let c = j * n + i;
                if i + 1 < n {
                    v[c] = step(v[c], v[c + 1], lim[c].max(lim[c + 1]));
                }
                if j + 1 < n {
                    v[c] = step(v[c], v[c + n], lim[c].max(lim[c + n]));
                }
            }
        }
        if before == v {
            return;
        }
    }
}

/// Cap on repeated sweeps; one limit everywhere converges in the first, varying limits in a few.
const SWEEP_REPEATS: usize = 12; // const-ok: iteration cap

/// Make the field Lipschitz along the grid axes (no two axis neighbours differ by more than the larger of their cells' `axis_limit_m`),
/// in O(n^2) per sweep: peaks are shaved by a min-envelope and pits filled by a max-envelope. `anchors` are never moved: every free
/// cell is first clamped into the cone each anchor allows, which makes the two envelopes leave the anchors alone (provided the anchors
/// are mutually consistent).
pub fn limit_grades(h: &mut [f64], n: usize, axis_limit_m: &[f64], anchors: &[bool]) {
    let mut lo = vec![f64::NEG_INFINITY; n * n];
    let mut hi = vec![f64::INFINITY; n * n];
    for c in 0..n * n {
        if anchors[c] {
            lo[c] = h[c];
            hi[c] = h[c];
        }
    }
    sweep(&mut lo, n, axis_limit_m, false);
    sweep(&mut hi, n, axis_limit_m, true);
    for c in 0..n * n {
        // Inconsistent anchors (a road bend tighter than the cross-slope allows) are split down the middle rather than panicking.
        h[c] = if lo[c] > hi[c] { 0.5 * (lo[c] + hi[c]) } else { h[c].clamp(lo[c], hi[c]) };
    }
    sweep(h, n, axis_limit_m, true);
    sweep(h, n, axis_limit_m, false);
}

/// Headroom on a river bank's axis limit over its stated grade.
const RIVER_LIMIT_SLACK: f64 = 1.1; // const-ok: margin so the clamp never shaves the bank it is told to allow

/// A node counts as water only if its ground is at least this far below the surface, m (stops a hairline of one-node puddles).
const WATER_EPS_M: f64 = 0.01; // const-ok: keeps numerical dust from drawing water

/// Extra cost of a road step through water, m of road: high enough that A* goes round whenever it can.
const WET_STEP_COST_M: f64 = 200.0; // const-ok: a road never wades if there is any way round

/// Slope-cost A* over the 8-neighbour grid between two nodes. The cost of a step is `length * (1 + (slope / g)^2)`, so flat routes win
/// and the road looks for the saddle, but nothing is forbidden: the profile stage grades whatever path comes out.
fn astar(
    h: &[f64],
    n: usize,
    from: (usize, usize),
    to: (usize, usize),
    g: f64,
    wet: &[bool],
) -> Option<Vec<(usize, usize)>> {
    let scale = 1000.0; // const-ok: fixed-point scale so the heap key is an integer (a total order)
    let idx = |i: usize, j: usize| j * n + i;
    let mut best = vec![u64::MAX; n * n];
    let mut prev = vec![u32::MAX; n * n];
    let mut heap = BinaryHeap::new();
    let heur = |i: usize, j: usize| -> u64 {
        let (dx, dz) = (i as f64 - to.0 as f64, j as f64 - to.1 as f64);
        (scalar::hypot(dx, dz) * CELL_M * scale) as u64
    };
    best[idx(from.0, from.1)] = 0;
    heap.push(Reverse((heur(from.0, from.1), 0u64, idx(from.0, from.1) as u32)));
    while let Some(Reverse((_, cost, at))) = heap.pop() {
        let (ai, aj) = (at as usize % n, at as usize / n);
        if (ai, aj) == to {
            let mut path = vec![(ai, aj)];
            let mut cur = at;
            while prev[cur as usize] != u32::MAX {
                cur = prev[cur as usize];
                path.push((cur as usize % n, cur as usize / n));
            }
            path.reverse();
            return Some(path);
        }
        if cost > best[at as usize] {
            continue;
        }
        for dj in -1i64..=1 {
            for di in -1i64..=1 {
                if di == 0 && dj == 0 {
                    continue;
                }
                let (ni, nj) = (ai as i64 + di, aj as i64 + dj);
                if ni < 0 || nj < 0 || ni >= n as i64 || nj >= n as i64 {
                    continue;
                }
                let (ni, nj) = (ni as usize, nj as usize);
                let len = scalar::hypot(di as f64, dj as f64) * CELL_M;
                let slope = (h[idx(ni, nj)] - h[at as usize]).abs() / len;
                let wet_cost = if wet[idx(ni, nj)] { WET_STEP_COST_M } else { 0.0 };
                let step = ((len * (1.0 + (slope / g) * (slope / g)) + wet_cost) * scale) as u64;
                let c = cost + step;
                if c < best[idx(ni, nj)] {
                    best[idx(ni, nj)] = c;
                    prev[idx(ni, nj)] = at;
                    heap.push(Reverse((c + heur(ni, nj), c, idx(ni, nj) as u32)));
                }
            }
        }
    }
    None
}

/// Steepest plan gradient any bilinear cell of a node field can reach, per unit amplitude: the largest edge differences of each cell
/// combined (the gradient of a bilinear patch is largest at a corner, where it is made of two of those edge differences).
fn steepest_gradient(v: &[f64], n: usize) -> f64 {
    let mut best = 0.0f64;
    for j in 0..n - 1 {
        for i in 0..n - 1 {
            let (v00, v10, v01, v11) = (v[j * n + i], v[j * n + i + 1], v[(j + 1) * n + i], v[(j + 1) * n + i + 1]);
            let gx = (v10 - v00).abs().max((v11 - v01).abs()) / CELL_M;
            let gz = (v01 - v00).abs().max((v11 - v10).abs()) / CELL_M;
            best = best.max(scalar::hypot(gx, gz));
        }
    }
    best
}

/// The grade limiter alone, without the smoothing `grade_profile` does first.
fn grade_profile_keep(y: &mut [f64], ds: &[f64], g: f64) {
    for _ in 0..PROFILE_PASSES {
        let mut changed = false;
        for k in 1..y.len() {
            let lim = g * ds[k - 1];
            let c = y[k].clamp(y[k - 1] - lim, y[k - 1] + lim);
            changed |= (c - y[k]).abs() > PROFILE_TOL_M;
            y[k] = c;
        }
        for k in (0..y.len() - 1).rev() {
            let lim = g * ds[k];
            let c = y[k].clamp(y[k + 1] - lim, y[k + 1] + lim);
            changed |= (c - y[k]).abs() > PROFILE_TOL_M;
            y[k] = c;
        }
        if !changed {
            break;
        }
    }
}

/// Iteration cap and tolerance of the grade limiter.
const PROFILE_PASSES: usize = 200; // const-ok: iteration cap; it converges in a handful of rounds
const PROFILE_TOL_M: f64 = 1e-12; // const-ok: convergence tolerance, m

/// Moving average of a profile over `half` samples each side (shrinking symmetrically at the ends so they stay put).
fn smooth_profile(y: &mut [f64], half: usize) {
    let src = y.to_vec();
    for k in 0..y.len() {
        let w = half.min(k).min(y.len() - 1 - k);
        y[k] = src[k - w..=k + w].iter().sum::<f64>() / (2 * w + 1) as f64;
    }
}

/// Half width of the rounding of the graded profile, in path samples (about a metre each).
const PROFILE_ROUNDING_SAMPLES: usize = 16; // const-ok: engineered roads are smooth over about 30 m

/// Smooth a profile and then limit its slope to `g` (alternating forward and backward projection until stable).
fn grade_profile(y: &mut [f64], ds: &[f64], g: f64) {
    // `ds[k]` is the distance between sample k and k + 1.
    let window = 7usize; // const-ok: moving-average half width in samples, a smoothing choice
    let src = y.to_vec();
    for k in 0..y.len() {
        let (lo, hi) = (k.saturating_sub(window), (k + window).min(y.len() - 1));
        y[k] = src[lo..=hi].iter().sum::<f64>() / (hi - lo + 1) as f64;
    }
    for _ in 0..200 {
        // const-ok: iteration cap; the projection converges in a handful of rounds
        let mut changed = false;
        for k in 1..y.len() {
            let lim = g * ds[k - 1];
            let c = y[k].clamp(y[k - 1] - lim, y[k - 1] + lim);
            changed |= (c - y[k]).abs() > 1e-12; // const-ok: convergence tolerance, m
            y[k] = c;
        }
        for k in (0..y.len() - 1).rev() {
            let lim = g * ds[k];
            let c = y[k].clamp(y[k + 1] - lim, y[k + 1] + lim);
            changed |= (c - y[k]).abs() > 1e-12; // const-ok: convergence tolerance, m
            y[k] = c;
        }
        if !changed {
            break;
        }
    }
}

/// A road planned on the carved ground: its smoothed centreline, graded profile and, for every node within reach, the distance to it,
/// the arc position along it and the height it asks for.
struct Laid {
    pts: Vec<(f64, f64)>,
    y: Vec<f64>,
    total_arc: f64,
    w2: f64,
    weight: Vec<f64>,
    target: Vec<f64>,
    best_d: Vec<f64>,
    arc: Vec<f64>,
    /// Per point of `pts`: the bridge it belongs to, if any.
    bridge: Vec<Option<usize>>,
    bridges: Vec<PlacedBridge>,
}

#[allow(clippy::too_many_arguments)]
fn plan_road(
    def: &CourseDef,
    rd: &RoadDef,
    ri: usize,
    h: &[f64],
    n: usize,
    river_wet: &[bool],
    river_surface: &[f64],
) -> Result<Laid, String> {
    let half = (n - 1) as f64 * 0.5 * CELL_M;
    let snap = |p: (f64, f64)| -> Result<(usize, usize), String> {
        let (i, j) = (((p.0 + half) / CELL_M).round(), ((p.1 + half) / CELL_M).round());
        if i < 0.0 || j < 0.0 || i >= n as f64 || j >= n as f64 {
            return Err(format!("waypoint ({}, {}) is outside the course", p.0, p.1));
        }
        Ok((i as usize, j as usize))
    };
    // The road as a list of grid-unit points and a flag per point: `exact` points (a zig-zag climb) are laid as authored; the rest come
    // from A* between waypoints and are smoothed afterwards.
    let to_grid = |p: (f64, f64)| ((p.0 + half) / CELL_M, (p.1 + half) / CELL_M);
    let mut bridges: Vec<PlacedBridge> = Vec::new();
    let mut path: Vec<((f64, f64), bool, Option<usize>)> = Vec::new();
    let leg_to = |path: &mut Vec<((f64, f64), bool, Option<usize>)>, target: (f64, f64)| -> Result<(), String> {
        let b = snap(target)?;
        match path.last() {
            None => path.push(((b.0 as f64, b.1 as f64), false, None)),
            Some(&(last, _, _)) => {
                let a = (last.0.round() as usize, last.1.round() as usize);
                let leg = astar(h, n, a, b, rd.max_grade.v, river_wet).ok_or("no road path between waypoints")?;
                path.extend(leg[1..].iter().map(|&(i, j)| ((i as f64, j as f64), false, None)));
            }
        }
        Ok(())
    };
    for (k, &wp) in rd.waypoints.iter().enumerate() {
        leg_to(&mut path, wp)?;
        for sb in rd.switchbacks.iter().filter(|sb| sb.after_waypoint == k) {
            sb.span_m.check(&format!("{}.road.switchbacks.span_m", def.name))?;
            sb.hairpin_radius_m.check(&format!("{}.road.switchbacks.hairpin_radius_m", def.name))?;
            let cl = def
                .cliffs
                .get(sb.cliff)
                .ok_or_else(|| format!("switchback refers to cliff {} which does not exist", sb.cliff))?;
            let dense = switchback_path(cl, sb, rd.width_m.v, rd.shoulder_m.v, rd.max_grade.v, SWITCHBACK_APPROACH_M)?;
            leg_to(&mut path, dense[0])?;
            path.extend(dense.iter().skip(1).map(|&p| (to_grid(p), true, None)));
        }
        for bd in rd.bridges.iter().filter(|b| b.after_waypoint == k) {
            bd.check(&format!("{}.roads[{ri}].bridges", def.name))?;
            let river = def
                .rivers
                .get(bd.river)
                .ok_or_else(|| format!("bridge refers to river {} which does not exist", bd.river))?;
            let from = path.last().map_or(wp, |p| (p.0 .0 * CELL_M - half, p.0 .1 * CELL_M - half));
            let seed = def.seed.wrapping_add(bd.river as u64);
            // The deck is level with the floodplain: the water level at the crossing plus the river's freeboard.
            let line = river.centreline(seed);
            let total = line[line.len() - 1].2;
            let mid = line.iter().find(|p| p.2 >= bd.at_fraction.v * total).unwrap_or(&line[line.len() / 2]);
            let (mi, mj) = (((mid.0 + half) / CELL_M).round() as usize, ((mid.1 + half) / CELL_M).round() as usize);
            let surface = river_surface[mj.min(n - 1) * n + mi.min(n - 1)];
            if surface.is_nan() {
                return Err(format!(
                    "{}: the bridge at {} of river {} is outside the river's valley",
                    def.name, bd.at_fraction.v, bd.river
                ));
            }
            let pb = place(bd, river, seed, from, surface);
            leg_to(&mut path, pb.a)?;
            let bi = bridges.len();
            let len = scalar::hypot(pb.b.0 - pb.a.0, pb.b.1 - pb.a.1);
            let steps = (len / CELL_M).ceil() as usize;
            for i in 1..=steps {
                let f = i as f64 / steps as f64;
                let p = (pb.a.0 + (pb.b.0 - pb.a.0) * f, pb.a.1 + (pb.b.1 - pb.a.1) * f);
                path.push((to_grid(p), true, Some(bi)));
            }
            bridges.push(pb);
        }
    }
    let nodes: Vec<(f64, f64)> = path.iter().map(|p| p.0).collect();
    let bridge: Vec<Option<usize>> = path.iter().map(|p| p.2).collect();
    // The A* staircase has tight kinks that a real road would not: average the path over a window (symmetric, so the ends stay put),
    // but never across an exact point: averaging a tight authored arc would shrink its radius.
    let smooth = 16usize; // const-ok: moving-average half width in samples, a smoothing choice (radius of curvature of about 10 m)
    let mut near_exact = vec![usize::MAX; nodes.len()];
    for k in 0..nodes.len() {
        near_exact[k] = if path[k].1 {
            0
        } else if k > 0 {
            near_exact[k - 1].saturating_add(1)
        } else {
            usize::MAX
        };
    }
    for k in (0..nodes.len()).rev() {
        if k + 1 < nodes.len() {
            near_exact[k] = near_exact[k].min(near_exact[k + 1].saturating_add(1));
        }
    }
    let pts: Vec<(f64, f64)> = (0..nodes.len())
        .map(|k| {
            let w = smooth.min(k).min(nodes.len() - 1 - k).min(near_exact[k].saturating_sub(1));
            let (mut sx, mut sz) = (0.0, 0.0);
            for &(i, j) in &nodes[k - w..=k + w] {
                sx += i;
                sz += j;
            }
            let m = (2 * w + 1) as f64;
            (sx / m, sz / m)
        })
        .collect();
    let mut y: Vec<f64> = nodes.iter().map(|&(i, j)| h[j.round() as usize * n + i.round() as usize]).collect();
    for (k, b) in bridge.iter().enumerate() {
        if let Some(bi) = b {
            y[k] = bridges[*bi].deck_y_m; // over the water the profile follows the deck, not the channel bed
        }
    }
    let ds: Vec<f64> = pts.windows(2).map(|w| scalar::hypot(w[1].0 - w[0].0, w[1].1 - w[0].1) * CELL_M).collect();
    // Grade a little under the stated limit: the stamped cells are sampled bilinearly, which can add a sliver of slope.
    grade_profile(&mut y, &ds, rd.max_grade.v * PROFILE_MARGIN);
    // The rate limiter leaves corners where the slope jumps, and a corner is an `n^-4` feature in the road's spectrum (ISO 8608 roads fall
    // as `n^-2`). A moving average rounds them.
    smooth_profile(&mut y, PROFILE_ROUNDING_SAMPLES);
    // Where path samples are not evenly spaced (an A* leg meeting an authored one) an average can exceed the grade a little: clamp again.
    grade_profile_keep(&mut y, &ds, rd.max_grade.v * PROFILE_MARGIN);

    let (w2, shoulder) = (rd.width_m.v * 0.5, rd.shoulder_m.v);
    let reach = w2 + shoulder;
    // The distance and arc fields are kept a little beyond the shoulder (weight 0 there): a washboard's phase must be defined on a ring of
    // nodes around it, or interpolating the phase against an undefined neighbour makes spikes at the section's edge.
    let search = reach.max(w2 + ROUGH_EDGE_M + 2.0 * CELL_M);
    let mut weight = vec![0.0f64; n * n];
    let mut target = vec![0.0f64; n * n];
    let mut best_d = vec![f64::INFINITY; n * n];
    let mut arc = vec![0.0f64; n * n];
    let r_cells = (search / CELL_M).ceil() as i64;
    // Distance to the centreline polyline, with the profile height interpolated at the projection (so neighbouring cells get
    // nearly equal targets even at a bend); the nearest segment wins and ties keep the earlier one.
    for k in 0..nodes.len() - 1 {
        if bridge[k].is_some() && bridge[k] == bridge[k + 1] {
            continue; // the deck is laid after the grade clamp, not stamped into the ground
        }
        let seg_start: f64 = ds[..k].iter().sum(); // road arc length at the start of segment k, m
        let ((ax, az), (bx, bz)) = (pts[k], pts[k + 1]);
        let len2 = (bx - ax) * (bx - ax) + (bz - az) * (bz - az);
        for j in (az.min(bz).floor() as i64 - r_cells).max(0)..=(az.max(bz).ceil() as i64 + r_cells).min(n as i64 - 1) {
            for i in
                (ax.min(bx).floor() as i64 - r_cells).max(0)..=(ax.max(bx).ceil() as i64 + r_cells).min(n as i64 - 1)
            {
                let t = (((i as f64 - ax) * (bx - ax) + (j as f64 - az) * (bz - az)) / len2).clamp(0.0, 1.0);
                let d = scalar::hypot(i as f64 - (ax + t * (bx - ax)), j as f64 - (az + t * (bz - az))) * CELL_M;
                let c = j as usize * n + i as usize;
                if d < search && d < best_d[c] {
                    best_d[c] = d;
                    arc[c] = seg_start + t * ds[k];
                    target[c] = y[k] + t * (y[k + 1] - y[k]);
                    weight[c] = 1.0 - scalar::smoothstep(w2, reach, d).min(1.0);
                    if d >= reach {
                        weight[c] = 0.0;
                    }
                }
            }
        }
    }
    let total_arc: f64 = ds.iter().sum();
    Ok(Laid { pts, y, total_arc, w2, weight, target, best_d, arc, bridge, bridges })
}

pub fn generate(def: &CourseDef) -> Result<Course, String> {
    let n = def.size_cells;
    if n < 16 {
        // const-ok: smallest sensible course
        return Err("size_cells must be at least 16".into());
    }
    let hd = &def.hills;
    for (l, p) in [
        ("hills.amplitude_m", &hd.amplitude_m),
        ("hills.wavelength_m", &hd.wavelength_m),
        ("hills.warp_m", &hd.warp_m),
        ("hills.max_grade", &hd.max_grade),
    ] {
        p.check(&format!("{}.{l}", def.name))?;
    }
    for (k, rd) in std::iter::once(&def.road).chain(&def.extra_roads).enumerate() {
        if let Some(g) = &rd.roughness_gd_n0_m3 {
            g.check(&format!("{}.roads[{k}].roughness_gd_n0_m3", def.name))?;
        }
        for (l, p) in [("width_m", &rd.width_m), ("shoulder_m", &rd.shoulder_m), ("max_grade", &rd.max_grade)] {
            p.check(&format!("{}.roads[{k}].{l}", def.name))?;
        }
        if rd.waypoints.len() < 2 {
            return Err(format!("{}.roads[{k}] needs at least two waypoints", def.name));
        }
    }
    let materials = standard_material_table()?;
    let ground = materials.id_of("dirt").ok_or("material table has no `dirt`")?;
    let road_material =
        materials.id_of(&def.road.surface).ok_or_else(|| format!("unknown road surface `{}`", def.road.surface))?;
    let half = (n - 1) as f64 * 0.5 * CELL_M;
    let xz = |i: usize, j: usize| (i as f64 * CELL_M - half, j as f64 * CELL_M - half);

    // 1. Hills.
    let mut h = vec![0.0f64; n * n];
    for j in 0..n {
        for i in 0..n {
            let (x, z) = xz(i, j);
            let mut v = hd.amplitude_m.v * warped_fbm(def.seed, x, z, hd.wavelength_m.v, hd.warp_m.v, hd.octaves);
            if let Some(f) = &hd.feature {
                let d = scalar::hypot(x - f.x_m, z - f.z_m);
                v += f.height_m.v * scalar::smoothstep(0.0, 1.0, 1.0 - d / f.radius_m.v);
            }
            h[j * n + i] = v;
        }
    }
    // Small-scale detail with a stated spectrum (see `detail.rs`): built here, added after the roads are planned and stamped, so a road
    // follows the relief and not the ground's bumps (it carries its own, from its material).
    let detail: Option<(Vec<f64>, f64)> = match &hd.detail_gd_n0_m3 {
        Some(g) => {
            g.check(&format!("{}.hills.detail_gd_n0_m3", def.name))?;
            let layer = unit_layer(def.seed, n);
            let steepest = steepest_gradient(&layer, n);
            Some((layer, steepest))
        }
        None => None,
    };
    // Axis limit g / sqrt(2): the bilinear gradient magnitude is at most sqrt(2) x the largest axis difference.
    let axis_terrain = hd.max_grade.v * CELL_M * std::f64::consts::FRAC_1_SQRT_2;
    limit_grades(&mut h, n, &vec![axis_terrain; n * n], &vec![false; n * n]);

    // Cliffs: a separate layer on top of the clamped hills, with its own, larger grade limit in and around its face.
    let reach = def.road.width_m.v * 0.5 + def.road.shoulder_m.v;
    let mut axis_limit = vec![axis_terrain; n * n];
    let mut grade_limit = vec![hd.max_grade.v; n * n];
    for (k, cl) in def.cliffs.iter().enumerate() {
        cl.check(&format!("{}.cliffs[{k}]", def.name))?;
        // The face's axis differences can reach its full grade when it faces along an axis: allow that, plus a little.
        let axis_zone = (cl.steepest_grade() + hd.max_grade.v) * CELL_M * CLIFF_LIMIT_SLACK;
        for c in 0..n * n {
            let (x, z) = xz(c % n, c / n);
            h[c] += cl.height_at(x, z);
            if cl.in_zone(x, z, reach) {
                axis_limit[c] = axis_limit[c].max(axis_zone);
                grade_limit[c] = grade_limit[c].max(axis_zone * std::f64::consts::SQRT_2 / CELL_M);
            }
        }
    }

    // River: valley, channel and banks, cut into the clamped hills before the road is laid (the road goes round the water).
    let mut river_surface = vec![f64::NAN; n * n];
    for (k, r) in def.rivers.iter().enumerate() {
        r.check(&format!("{}.rivers[{k}]", def.name))?;
        let carved = carve(r, def.seed.wrapping_add(k as u64), &mut h, n);
        let axis_bank = r.bank_grade.v * CELL_M * RIVER_LIMIT_SLACK;
        for c in 0..n * n {
            if carved.bank_zone[c] {
                axis_limit[c] = axis_limit[c].max(axis_bank);
                grade_limit[c] = grade_limit[c].max(axis_bank * std::f64::consts::SQRT_2 / CELL_M);
            }
            if !carved.surface[c].is_nan() {
                river_surface[c] = carved.surface[c];
            }
        }
    }
    let river_wet: Vec<bool> = (0..n * n).map(|c| !river_surface[c].is_nan() && h[c] < river_surface[c]).collect();

    // 2. Roads: each is planned on the carved ground (path, profile, distance and arc fields), then stamped in turn.
    let mut road_defs: Vec<&RoadDef> = vec![&def.road];
    road_defs.extend(def.extra_roads.iter());
    let mut laid: Vec<Laid> = Vec::new();
    for (ri, rd) in road_defs.iter().enumerate() {
        laid.push(plan_road(def, rd, ri, &h, n, &river_wet, &river_surface)?);
    }
    let mud_id = materials.id_of("mud").ok_or("material table has no `mud`")?;
    let mut splat = vec![ground.0 as u8; n * n];
    let mut frozen = vec![false; n * n];
    // The ISO level of the detail each road cell carries (from its road's stated condition, or its material).
    let mut cell_gd = vec![0.0f64; n * n];
    for (rd, lr) in road_defs.iter().zip(&laid) {
        let surface = materials.id_of(&rd.surface).ok_or_else(|| format!("unknown road surface `{}`", rd.surface))?;
        let mut crossings = Vec::new();
        for (k, m) in rd.mud_crossings.iter().enumerate() {
            m.at_fraction.check(&format!("{}.road.mud_crossings[{k}].at_fraction", def.name))?;
            m.length_m.check(&format!("{}.road.mud_crossings[{k}].length_m", def.name))?;
            crossings.push((
                m.at_fraction.v * lr.total_arc - m.length_m.v * 0.5,
                m.at_fraction.v * lr.total_arc + m.length_m.v * 0.5,
            ));
        }
        for c in 0..n * n {
            if lr.best_d[c] <= lr.w2 {
                let wet = crossings.iter().any(|&(a, b)| lr.arc[c] >= a && lr.arc[c] <= b);
                splat[c] = if wet { mud_id.0 } else { surface.0 } as u8;
                frozen[c] = true;
                cell_gd[c] = match (&rd.roughness_gd_n0_m3, wet) {
                    (Some(g), false) => g.v,
                    _ => gd_from_rms(materials.get(MaterialId(u16::from(splat[c]))).roughness_rms_m),
                };
            }
            if lr.weight[c] > 0.0 {
                h[c] += (lr.target[c] - h[c]) * lr.weight[c];
            }
        }
    }
    // The centreline cells are exactly the profile (they have weight 1); keep the shoulders inside the terrain grade limit.
    limit_grades(&mut h, n, &axis_limit, &frozen);
    // Detail: the ground carries the course's stated level, a road its own material's (an ISO class from `roughness_rms_m`). Added after
    // the clamp, so each cell's grade limit grows by the layer's steepest slope at its amplitude.
    if let Some((layer, steepest)) = &detail {
        let ground_amp = scalar::sqrt(hd.detail_gd_n0_m3.as_ref().map_or(0.0, |g| g.v));
        for c in 0..n * n {
            let amp = if frozen[c] { scalar::sqrt(cell_gd[c]) } else { ground_amp };
            h[c] += amp * layer[c];
            grade_limit[c] += amp * steepest;
        }
    }
    // Water: wherever the final ground is below the river's surface. The bed is its own material.
    let wet_final: Vec<bool> =
        (0..n * n).map(|c| !river_surface[c].is_nan() && h[c] < river_surface[c] - WATER_EPS_M).collect();
    let riverbed = materials.id_of("riverbed").ok_or("material table has no `riverbed`")?;
    for c in 0..n * n {
        if wet_final[c] && !frozen[c] {
            splat[c] = riverbed.0 as u8;
        }
    }
    let water: Option<Vec<f32>> = def
        .rivers
        .first()
        .map(|_| (0..n * n).map(|c| if wet_final[c] { river_surface[c] as f32 } else { f32::NAN }).collect());
    // Bridge decks: laid after the clamp (it would build ramps into the channel) and after the water (the bed under a deck stays a bed).
    let mut deck_props: Vec<PropRef> = Vec::new();
    for (rd, lr) in road_defs.iter().zip(&laid) {
        let road_surface =
            materials.id_of(&rd.surface).ok_or_else(|| format!("unknown road surface `{}`", rd.surface))?;
        let planks = materials.id_of("planks").ok_or("material table has no `planks`")?;
        let mut deck_d = vec![f64::INFINITY; n * n];
        for k in 0..lr.pts.len() - 1 {
            let Some(bi) = lr.bridge[k].filter(|&b| lr.bridge[k + 1] == Some(b)) else { continue };
            let pb = &lr.bridges[bi];
            let (dw, reach) = (pb.width_m * 0.5, pb.width_m * 0.5 + DECK_LIMIT_MARGIN_M);
            let r_cells = (reach / CELL_M).ceil() as i64;
            let ((ax, az), (bx, bz)) = (lr.pts[k], lr.pts[k + 1]);
            let len2 = (bx - ax) * (bx - ax) + (bz - az) * (bz - az);
            for j in
                (az.min(bz).floor() as i64 - r_cells).max(0)..=(az.max(bz).ceil() as i64 + r_cells).min(n as i64 - 1)
            {
                for i in (ax.min(bx).floor() as i64 - r_cells).max(0)
                    ..=(ax.max(bx).ceil() as i64 + r_cells).min(n as i64 - 1)
                {
                    let t = (((i as f64 - ax) * (bx - ax) + (j as f64 - az) * (bz - az)) / len2).clamp(0.0, 1.0);
                    let d = scalar::hypot(i as f64 - (ax + t * (bx - ax)), j as f64 - (az + t * (bz - az))) * CELL_M;
                    let c = j as usize * n + i as usize;
                    if d <= reach {
                        grade_limit[c] = f64::INFINITY; // the drop beside a deck is a cliff by design
                    }
                    if d <= dw && d < deck_d[c] {
                        deck_d[c] = d;
                        h[c] = lr.y[k] + t * (lr.y[k + 1] - lr.y[k]);
                        splat[c] = if pb.kind == BridgeKind::Wooden { planks.0 } else { road_surface.0 } as u8;
                    }
                }
            }
        }
        for pb in &lr.bridges {
            deck_props.extend(rails(0, pb));
        }
    }
    // Rough sections. Whoops are long enough for the grid and are added to the heights (after the clamp: they are roughness on top of
    // the graded road, not part of its stated grade, so the cells' limits grow by their slope). Washboard is finer than the grid and
    // becomes a phase and weight layer evaluated per query (see `corrugation.rs`).
    let mut corr: Option<Corrugation> = None;
    for (rd, lr) in road_defs.iter().zip(&laid) {
        let (w2, total_arc, best_d, arc) = (lr.w2, lr.total_arc, &lr.best_d, &lr.arc);
        let lateral = |d: f64| 1.0 - scalar::smoothstep(w2, w2 + ROUGH_EDGE_M, d);
        for (kind, list) in [("whoops", &rd.whoops), ("washboards", &rd.washboards)] {
            for (k, r) in list.iter().enumerate() {
                let tag = format!("{}.road.{kind}[{k}]", def.name);
                for (l, p) in [
                    ("at_fraction", &r.at_fraction),
                    ("length_m", &r.length_m),
                    ("wavelength_m", &r.wavelength_m),
                    ("amplitude_m", &r.amplitude_m),
                ] {
                    p.check(&format!("{tag}.{l}"))?;
                }
                let lam = r.wavelength_m.v;
                if kind == "whoops" && lam < MIN_WHOOP_WAVELENGTH_M {
                    return Err(format!("{tag}: wavelength {lam} m is below {MIN_WHOOP_WAVELENGTH_M} m, too short for the 1 m grid: use a washboard"));
                }
                if kind == "washboards" && !(MIN_WASHBOARD_WAVELENGTH_M..=MAX_WASHBOARD_WAVELENGTH_M).contains(&lam) {
                    return Err(format!("{tag}: wavelength {lam} m is outside {MIN_WASHBOARD_WAVELENGTH_M} to {MAX_WASHBOARD_WAVELENGTH_M} m: use whoops for longer swells"));
                }
                let (a, b) = (
                    r.at_fraction.v * total_arc - r.length_m.v * 0.5,
                    r.at_fraction.v * total_arc + r.length_m.v * 0.5,
                );
                let fade = lam.max(ROUGH_MIN_FADE_M); // ripples fade in and out over a wavelength
                if kind == "washboards" && corr.is_none() {
                    corr = Some(Corrugation::new(n, Vec::new()));
                }
                let region = corr.as_ref().map_or(0, |c| c.ripples.len()) as u8;
                if kind == "washboards" {
                    if let Some(c) = corr.as_mut() {
                        c.ripples.push(Ripple { wavelength_m: lam, amplitude_m: r.amplitude_m.v });
                    }
                }
                for c in 0..n * n {
                    if best_d[c] >= w2 + ROUGH_EDGE_M + 2.0 * CELL_M || arc[c] < a || arc[c] > b {
                        continue;
                    }
                    let wgt = lateral(best_d[c])
                        * scalar::smoothstep(a, a + fade, arc[c])
                        * (1.0 - scalar::smoothstep(b - fade, b, arc[c]));
                    if wgt <= 0.0 && kind == "whoops" {
                        continue;
                    }
                    if kind == "whoops" {
                        h[c] += wgt * r.amplitude_m.v * 0.5 * (1.0 - scalar::cos(scalar::TAU * (arc[c] - a) / lam));
                        grade_limit[c] += r.amplitude_m.v * scalar::PI / lam;
                    } else if let Some(cc) = corr.as_mut() {
                        cc.set(c, region, arc[c] - a, wgt);
                        // The phase is interpolated between nodes whose road distances may differ by up to a diagonal step.
                        grade_limit[c] += r.amplitude_m.v * scalar::PI / lam * std::f64::consts::SQRT_2;
                    }
                }
            }
        }
    }
    // Mud pits: the road dips over a stated crossing. Added after the clamp, like whoops; the walls are eased over a third of the
    // crossing at most PIT_WALL_M, so their slope is bounded and counted in the cells' grade limits.
    for (rd, lr) in road_defs.iter().zip(&laid) {
        for (k, m) in rd.mud_crossings.iter().enumerate() {
            let Some(depth) = &rd.mud_crossings[k].pit_depth_m else { continue };
            depth.check(&format!("{}.road.mud_crossings[{k}].pit_depth_m", def.name))?;
            let (a, b) = (
                m.at_fraction.v * lr.total_arc - m.length_m.v * 0.5,
                m.at_fraction.v * lr.total_arc + m.length_m.v * 0.5,
            );
            let wall = (m.length_m.v / 3.0).min(PIT_WALL_M);
            for c in 0..n * n {
                if lr.best_d[c] >= lr.w2 + ROUGH_EDGE_M || lr.arc[c] < a || lr.arc[c] > b {
                    continue;
                }
                let lateral = 1.0 - scalar::smoothstep(lr.w2, lr.w2 + ROUGH_EDGE_M, lr.best_d[c]);
                let along =
                    scalar::smoothstep(a, a + wall, lr.arc[c]) * (1.0 - scalar::smoothstep(b - wall, b, lr.arc[c]));
                h[c] -= depth.v * along * lateral;
                grade_limit[c] += SMOOTHSTEP_PEAK * depth.v * (1.0 / wall + 1.0 / ROUGH_EDGE_M);
            }
        }
    }
    let centrelines: Vec<Vec<(f64, f64, f64)>> = laid
        .iter()
        .map(|lr| lr.pts.iter().zip(&lr.y).map(|(&(i, j), &yy)| (i * CELL_M - half, j * CELL_M - half, yy)).collect())
        .collect();
    let road = centrelines[0].clone();
    let w2 = laid[0].w2;

    // Round to the stored precision first: every later rule (drainage, slope) must see exactly what a query sees.
    for v in h.iter_mut() {
        *v = f64::from(*v as f32);
    }
    if let Some(m) = &def.mud {
        m.min_drainage_cells.check(&format!("{}.mud.min_drainage_cells", def.name))?;
        m.max_slope.check(&format!("{}.mud.max_slope", def.name))?;
        m.spread_m.check(&format!("{}.mud.spread_m", def.name))?;
        for (c, wet) in mud_mask(&h, n, m).into_iter().enumerate() {
            if wet && !frozen[c] {
                splat[c] = mud_id.0 as u8;
            }
        }
    }
    // Chutes: the notch straight up a cliff is laid with its own surface (loose gravel makes it a traction test).
    for cl in &def.cliffs {
        let Some(ch) = &cl.chute else { continue };
        let surf = materials.id_of(&ch.surface).ok_or_else(|| format!("unknown chute surface `{}`", ch.surface))?;
        for (c, cell) in splat.iter_mut().enumerate() {
            let (x, z) = xz(c % n, c / n);
            if !frozen[c] && cl.in_chute(x, z) {
                *cell = surf.0 as u8;
            }
        }
    }
    // Soft patches: sand or deep mud laid over plain ground (never over road, water or mud already there).
    for (k, sp) in def.soft_patches.iter().enumerate() {
        sp.radius_m.check(&format!("{}.soft_patches[{k}].radius_m", def.name))?;
        let mat = materials.id_of(&sp.surface).ok_or_else(|| format!("unknown soft patch surface `{}`", sp.surface))?;
        if materials.get(mat).soil.is_none() {
            return Err(format!(
                "{}.soft_patches[{k}]: `{}` has no soil parameters, so it is not soft ground",
                def.name, sp.surface
            ));
        }
        for (c, cell) in splat.iter_mut().enumerate() {
            let (x, z) = xz(c % n, c / n);
            if *cell == ground.0 as u8 && scalar::hypot(x - sp.x_m, z - sp.z_m) <= sp.radius_m.v {
                *cell = mat.0 as u8;
            }
        }
    }
    // Rock fields: the ground under a field becomes gravel (never over road or mud).
    for (k, r) in def.rock_fields.iter().enumerate() {
        for (l, p) in [
            ("radius_m", &r.radius_m),
            ("min_spacing_m", &r.min_spacing_m),
            ("density", &r.density),
            ("rock_radius_min_m", &r.rock_radius_min_m),
            ("rock_radius_max_m", &r.rock_radius_max_m),
            ("road_clearance_m", &r.road_clearance_m),
        ] {
            p.check(&format!("{}.rock_fields[{k}].{l}", def.name))?;
        }
        if r.rock_radius_min_m.v > r.rock_radius_max_m.v {
            return Err(format!("{}.rock_fields[{k}]: rock_radius_min_m exceeds rock_radius_max_m", def.name));
        }
        let gravel =
            materials.id_of(&r.surface).ok_or_else(|| format!("unknown rock field surface `{}`", r.surface))?;
        for (c, cell) in splat.iter_mut().enumerate() {
            let (x, z) = xz(c % n, c / n);
            if *cell == ground.0 as u8 && scalar::hypot(x - r.x_m, z - r.z_m) <= r.radius_m.v {
                *cell = gravel.0 as u8;
            }
        }
    }
    let heights: Vec<f32> = h.iter().map(|&v| v as f32).collect();
    let mut world = GridWorld::from_arrays(n, heights, splat.clone(), materials);
    if let Some(c) = corr {
        world.set_corrugation(c);
    }
    if let Some(w) = water {
        world.set_water(w);
    }

    // 3. Props.
    let mut props: Vec<PropRef> = Vec::new();
    if let Some(t) = &def.trees {
        for (l, p) in [
            ("min_spacing_m", &t.min_spacing_m),
            ("density", &t.density),
            ("max_slope", &t.max_slope),
            ("road_clearance_m", &t.road_clearance_m),
            ("stand_wavelength_m", &t.stand_wavelength_m),
            ("trunk_radius_m", &t.trunk_radius_m),
            ("height_m", &t.height_m),
            ("break_impulse_ns", &t.break_impulse_ns),
        ] {
            p.check(&format!("{}.trees.{l}", def.name))?;
        }
        let near_road = dilate(&frozen, n, t.road_clearance_m.v);
        let accept = |x: f64, z: f64| -> f64 {
            let (i, j) = (((x + half) / CELL_M).round() as usize, ((z + half) / CELL_M).round() as usize);
            let c = j.min(n - 1) * n + i.min(n - 1);
            if splat[c] != ground.0 as u8 || near_road[c] || cell_slope(&h, n, c % n, c / n) > t.max_slope.v {
                0.0
            } else {
                // Stands, not an even scatter: thin the density by a slow noise field so trees clump and leave clearings.
                t.density.v
                    * scalar::smoothstep(
                        STAND_NOISE_EDGES.0,
                        STAND_NOISE_EDGES.1,
                        warped_fbm(def.seed ^ STANDS_SALT, x, z, t.stand_wavelength_m.v, 0.0, 2),
                    )
            }
        };
        let mut rng = Pcg32::derive(def.seed, &[3]); // stage 3: trees
        for (k, (x, z)) in poisson_disc(&mut rng, half - 1.0, t.min_spacing_m.v, &accept).into_iter().enumerate() {
            props.push(PropRef {
                id: PropId(k as u32),
                kind: PropKind::Tree,
                shape: PropShape::Cylinder { radius_m: t.trunk_radius_m.v, height_m: t.height_m.v },
                transform: Transform::from_pos(Vec3::new(x, world.height_m(x, z), z)),
                break_impulse_ns: t.break_impulse_ns.v,
            });
        }
    }
    for (k, r) in def.rock_fields.iter().enumerate() {
        let gravel =
            world.materials().id_of(&r.surface).ok_or_else(|| format!("unknown rock field surface `{}`", r.surface))?.0
                as u8;
        let near_road = dilate(&frozen, n, r.road_clearance_m.v);
        let accept = |x: f64, z: f64| -> f64 {
            let (i, j) = (((x + half) / CELL_M).round() as usize, ((z + half) / CELL_M).round() as usize);
            let c = j.min(n - 1) * n + i.min(n - 1);
            if splat[c] == gravel && !near_road[c] && scalar::hypot(x - r.x_m, z - r.z_m) <= r.radius_m.v {
                r.density.v
            } else {
                0.0
            }
        };
        let mut rng = Pcg32::derive(def.seed, &[4, k as u64]); // stage 4: rock field k
        let pts = poisson_disc_in(&mut rng, (r.x_m, r.z_m), r.radius_m.v, r.min_spacing_m.v, &accept);
        for (x, z) in pts {
            // Many small and few large: u^2 skews the size toward the minimum.
            let u = rng.next_f64();
            let rad = r.rock_radius_min_m.v + (r.rock_radius_max_m.v - r.rock_radius_min_m.v) * u * u;
            props.push(PropRef {
                id: PropId(props.len() as u32),
                kind: PropKind::Rock,
                shape: PropShape::Sphere { radius_m: rad },
                transform: Transform::from_pos(Vec3::new(x, world.height_m(x, z) + BURIAL * rad, z)),
                break_impulse_ns: f64::INFINITY,
            });
        }
    }
    for mut p in deck_props {
        p.id = PropId(props.len() as u32);
        props.push(p);
    }
    if let Some(b) = &def.barricade {
        for (l, p) in [
            ("at_fraction", &b.at_fraction),
            ("gap_m", &b.gap_m),
            ("block_height_m", &b.block_height_m),
            ("block_depth_m", &b.block_depth_m),
        ] {
            p.check(&format!("{}.barricade.{l}", def.name))?;
        }
        let last = road.len() - 1;
        let k = ((b.at_fraction.v * last as f64).round() as usize).clamp(1, last - 1);
        let (p0, p1) = (road[k - 1], road[k + 1]);
        let len = scalar::hypot(p1.0 - p0.0, p1.1 - p0.1);
        let ids = props.len() as u32;
        props.extend(barricade_blocks(ids, road[k], ((p1.0 - p0.0) / len, (p1.1 - p0.1) / len), w2, b, &|x, z| {
            world.height_m(x, z)
        }));
    }
    world.set_props(props);
    Ok(Course {
        world,
        road,
        extra_roads: centrelines[1..].to_vec(),
        bridges: laid.iter().flat_map(|l| l.bridges.clone()).collect(),
        ground,
        road_material,
        grade_limit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::world::WorldQuery;
    use w5k_math::{Pcg32, StateHasher};

    fn def() -> CourseDef {
        CourseDef::from_ron(include_str!("../../../content/world/courses/slice.ron")).expect("slice.ron parses")
    }

    const GOLDEN_CROSSING_HASH: u64 = 0x8cd3_c254_aaeb_8122;
    const GOLDEN_RIVER_HASH: u64 = 0x1b2f_cee4_6477_75c7;
    const GOLDEN_RIDGE_HASH: u64 = 0x1b71_3bff_9dbe_e466;
    const GOLDEN_SLICE_HASH: u64 = 0xd3b3_7972_eafb_b094;

    fn hash(c: &Course) -> u64 {
        let mut s = StateHasher::new();
        let w = &c.world;
        for j in 0..w.n() {
            for i in 0..w.n() {
                s.write_u32((w.height_at_node(i, j) as f32).to_bits());
                s.write_u8(w.splat_at_node(i, j));
            }
        }
        s.write_u64(w.corrugation_hash());
        for p in w.props() {
            s.write_u32(p.id.0);
            s.write_vec3(p.transform.pos);
        }
        s.finish()
    }

    #[test]
    fn course_ron_round_trips_and_every_param_checks() {
        let d = def();
        let back: CourseDef = ron::from_str(&ron::to_string(&d).expect("serialise")).expect("re-parse");
        assert_eq!(d, back);
        assert!(generate(&d).is_ok());
        let mut bad = d;
        bad.road.width_m = Param::spec(6.0, "");
        assert!(generate(&bad).err().expect("refused").contains("roads[0].width_m"));
    }

    #[test]
    fn course_generator_is_deterministic_for_a_seed() {
        let d = def();
        let (a, b) = (generate(&d).expect("a"), generate(&d).expect("b"));
        assert_eq!(hash(&a), hash(&b));
        // Bit-identical on every platform: this constant is checked on Linux and Windows CI.
        assert_eq!(hash(&a), GOLDEN_SLICE_HASH, "slice hash was {:#018x}", hash(&a));
        let mut other = d;
        other.seed += 1;
        assert_ne!(hash(&a), hash(&generate(&other).expect("c")));
    }

    #[test]
    fn terrain_never_exceeds_its_own_stated_maximum_grade() {
        let d = def();
        let c = generate(&d).expect("course");
        let n = c.world.n();
        let mut rng = Pcg32::new(5, 5);
        for _ in 0..4000 {
            let (x, z) = (rng.range_f64(-199.0, 199.0), rng.range_f64(-199.0, 199.0));
            let nrm = c.world.normal(x, z);
            let grade = scalar::hypot(nrm.x, nrm.z) / nrm.y;
            let (i, j) = (((x + 200.0).floor() as usize).min(n - 2), ((z + 200.0).floor() as usize).min(n - 2));
            let lim = [(i, j), (i + 1, j), (i, j + 1), (i + 1, j + 1)]
                .iter()
                .map(|&(a, b)| c.grade_limit[b * n + a])
                .fold(0.0, f64::max);
            assert!(
                grade <= lim * 1.02,
                "grade {grade} over its limit {lim} at ({x}, {z}) material {} road dist {}",
                c.world.material_at(x, z).name,
                c.road.iter().map(|r| scalar::hypot(r.0 - x, r.1 - z)).fold(f64::INFINITY, f64::min)
            );
        }
    }

    #[test]
    fn roads_never_exceed_the_stated_maximum_grade() {
        let d = def();
        let c = generate(&d).expect("course");
        assert!(c.road.len() > 100);
        for w in c.road.windows(2) {
            let run = scalar::hypot(w[1].0 - w[0].0, w[1].1 - w[0].1);
            let grade = (w[1].2 - w[0].2).abs() / run;
            assert!(grade <= d.road.max_grade.v + 1e-4, "road grade {grade} at ({}, {})", w[0].0, w[0].1);
        }
    }

    #[test]
    fn road_cells_are_road_material_and_start_and_finish_are_where_the_waypoints_say() {
        let d = def();
        let c = smooth_slice(); // the profile is the graded road; rough sections ride on top of it
        for &(x, z, y) in &c.road {
            let m = c.world.material_id_at(x, z);
            assert!(
                m == c.road_material || c.world.material_at(x, z).soil.is_some(),
                "centreline is road or a stated mud crossing"
            );
            // A road carries the roughness of its own material on top of the graded profile (a mud crossing is rougher than asphalt).
            let tol = 0.05 + 4.0 * c.world.material_at(x, z).roughness_rms_m;
            assert!(
                (c.world.height_m(x, z) - y).abs() < tol,
                "centreline sits on the graded profile ({} vs {y} at ({x}, {z}))",
                c.world.height_m(x, z)
            );
            // bilinear sampling between stamped nodes
        }
        let (first, last) = (c.road[0], c.road[c.road.len() - 1]);
        assert!(scalar::hypot(first.0 - d.road.waypoints[0].0, first.1 - d.road.waypoints[0].1) < 1.0);
        let wl = d.road.waypoints[d.road.waypoints.len() - 1];
        assert!(scalar::hypot(last.0 - wl.0, last.1 - wl.1) < 1.0);
    }

    fn nodes(c: &Course) -> Vec<f64> {
        let n = c.world.n();
        (0..n * n).map(|k| c.world.height_at_node(k % n, k / n)).collect()
    }

    #[test]
    fn mud_appears_only_where_the_drainage_rule_puts_it() {
        use crate::features::{drainage_rule_mask, mud_mask};
        let d = def();
        let c = generate(&d).expect("course");
        let (m, n) = (d.mud.as_ref().expect("mud"), c.world.n());
        let h = nodes(&c);
        let (rule, spread) = (drainage_rule_mask(&h, n, m), mud_mask(&h, n, m));
        let mud = c.world.materials().id_of("mud").expect("mud").0 as u8;
        let mut count = 0;
        for k in 0..n * n {
            let is_mud = c.world.splat_at_node(k % n, k / n) == mud;
            count += usize::from(is_mud);
            let (x, z) = c.world.node_xz(k % n, k / n);
            let on_road = c.road.iter().any(|r| scalar::hypot(r.0 - x, r.1 - z) <= d.road.width_m.v * 0.5 + 1.0);
            if is_mud && !on_road {
                assert!(spread[k], "mud outside the rule's reach at node {k}");
            }
            if rule[k] && c.world.splat_at_node(k % n, k / n) != c.road_material.0 as u8 {
                assert!(is_mud, "rule cell without mud at node {k}");
            }
        }
        assert!(count > 500, "the slice should have mud ({count} cells)");
    }

    #[test]
    fn no_tree_overlaps_a_road_or_a_building() {
        let d = def();
        let c = generate(&d).expect("course");
        let clearance = d.trees.as_ref().expect("trees").road_clearance_m.v;
        let w2 = d.road.width_m.v * 0.5;
        let trees: Vec<_> = c.world.props().iter().filter(|p| p.kind == PropKind::Tree).collect();
        assert!(trees.len() > 300);
        for t in &trees {
            let (x, z) = (t.transform.pos.x, t.transform.pos.z);
            assert_ne!(c.world.material_id_at(x, z), c.road_material);
            let near = c.road.iter().map(|r| scalar::hypot(r.0 - x, r.1 - z)).fold(f64::INFINITY, f64::min);
            assert!(near >= w2 + clearance - 2.0, "tree {near} m from the road centre");
            for o in c
                .world
                .props()
                .iter()
                .filter(|o| matches!(o.kind, PropKind::Barricade | PropKind::Building | PropKind::Wall))
            {
                let (lo, hi) = (o.transform.pos - Vec3::new(8.0, 0.0, 8.0), o.transform.pos + Vec3::new(8.0, 0.0, 8.0));
                assert!(x < lo.x || x > hi.x || z < lo.z || z > hi.z, "tree beside a barricade block");
            }
        }
    }

    #[test]
    fn trees_keep_their_minimum_spacing() {
        let d = def();
        let c = generate(&d).expect("course");
        let r = d.trees.as_ref().expect("trees").min_spacing_m.v;
        let t: Vec<_> = c.world.props().iter().filter(|p| p.kind == PropKind::Tree).map(|p| p.transform.pos).collect();
        for (a, pa) in t.iter().enumerate() {
            for pb in &t[a + 1..] {
                assert!(scalar::hypot(pa.x - pb.x, pa.z - pb.z) >= r - 1e-9);
            }
        }
    }

    #[test]
    fn every_prop_lies_inside_the_bounds() {
        let c = generate(&def()).expect("course");
        let (lo, hi) = c.world.bounds();
        for p in c.world.props() {
            let q = p.transform.pos;
            assert!(
                q.x >= lo.x && q.x <= hi.x && q.z >= lo.z && q.z <= hi.z && q.y >= lo.y && q.y <= hi.y,
                "prop {:?} at {q:?}",
                p.id
            );
        }
    }

    #[test]
    fn barricade_stands_across_the_road_and_leaves_the_stated_gap() {
        let d = def();
        let c = generate(&d).expect("course");
        let b = d.barricade.as_ref().expect("barricade");
        let blocks: Vec<_> = c.world.props().iter().filter(|p| p.kind == PropKind::Barricade).collect();
        assert_eq!(blocks.len(), 2);
        // Stand on the road centreline 15 m before the barricade and look along the road: the middle is open, the sides are not.
        let k = ((b.at_fraction.v * (c.road.len() - 1) as f64).round() as usize).clamp(1, c.road.len() - 2);
        let (p, q) = (c.road[k - 1], c.road[k + 1]);
        let len = scalar::hypot(q.0 - p.0, q.1 - p.1);
        let t = Vec3::new((q.0 - p.0) / len, 0.0, (q.1 - p.1) / len);
        let side = Vec3::new(-t.z, 0.0, t.x);
        let at = |off: f64| Vec3::new(c.road[k].0, c.road[k].2 + 0.5, c.road[k].1) - t * 8.0 + side * off;
        let hit = |off: f64| c.world.raycast(at(off), t, 16.0).and_then(|h| h.prop).is_some();
        assert!(!hit(0.0), "the gap must be open");
        assert!(hit(b.gap_m.v * 0.5 + 0.4) && hit(-(b.gap_m.v * 0.5 + 0.4)), "both blocks must stand in the way");
        assert!(!hit(b.gap_m.v * 0.5 - 0.3), "the gap is as wide as stated");
    }

    #[test]
    fn road_is_mud_exactly_where_the_crossing_says() {
        let d = def();
        let c = generate(&d).expect("course");
        let m = &d.road.mud_crossings[0];
        let mut arc = vec![0.0];
        for w in c.road.windows(2) {
            arc.push(arc[arc.len() - 1] + scalar::hypot(w[1].0 - w[0].0, w[1].1 - w[0].1));
        }
        let total = arc[arc.len() - 1];
        let (a, b) = (m.at_fraction.v * total - m.length_m.v * 0.5, m.at_fraction.v * total + m.length_m.v * 0.5);
        let (mut inside, mut outside) = (0, 0);
        for (p, s) in c.road.iter().zip(&arc) {
            let soft = c.world.material_at(p.0, p.1).soil.is_some();
            if *s > a + 1.5 && *s < b - 1.5 {
                assert!(soft, "road at arc {s} should be mud");
                inside += 1;
            } else if *s < a - 1.5 || *s > b + 1.5 {
                assert!(!soft, "road at arc {s} should be firm");
                outside += 1;
            }
        }
        assert!(inside > 15 && outside > 300);
    }

    fn rocks(c: &Course) -> Vec<&PropRef> {
        c.world.props().iter().filter(|p| p.kind == PropKind::Rock).collect()
    }

    #[test]
    fn rocks_stay_inside_their_field_and_keep_off_the_road() {
        let d = def();
        let c = generate(&d).expect("course");
        let f = &d.rock_fields[0];
        let rocks = rocks(&c);
        assert!(rocks.len() > 100, "the field should be full of stones ({})", rocks.len());
        for r in &rocks {
            let (x, z) = (r.transform.pos.x, r.transform.pos.z);
            assert!(scalar::hypot(x - f.x_m, z - f.z_m) <= f.radius_m.v + 1e-9);
            let near = c.road.iter().map(|p| scalar::hypot(p.0 - x, p.1 - z)).fold(f64::INFINITY, f64::min);
            assert!(near >= d.road.width_m.v * 0.5 + f.road_clearance_m.v - 2.0, "rock {near} m from the road centre");
            assert_eq!(c.world.material_at(x, z).name, "gravel");
        }
    }

    #[test]
    fn rocks_keep_the_stated_minimum_spacing_and_leave_a_gap_to_drive_through() {
        let d = def();
        let c = generate(&d).expect("course");
        let f = &d.rock_fields[0];
        let rocks = rocks(&c);
        for (a, pa) in rocks.iter().enumerate() {
            for pb in &rocks[a + 1..] {
                let (p, q) = (pa.transform.pos, pb.transform.pos);
                assert!(scalar::hypot(p.x - q.x, p.z - q.z) >= f.min_spacing_m.v - 1e-9);
            }
        }
        // The widest rock allowed leaves this much clear between two of its kind: the field is hard, not a wall.
        assert!(
            f.min_spacing_m.v - 2.0 * f.rock_radius_max_m.v > 0.5,
            "a lane at least 0.5 m wide must exist between the largest rocks"
        );
    }

    #[test]
    fn rocks_sit_partly_buried_in_the_ground() {
        let c = generate(&def()).expect("course");
        for r in rocks(&c) {
            let PropShape::Sphere { radius_m } = r.shape else { panic!("rocks are spheres") };
            let ground = c.world.height_m(r.transform.pos.x, r.transform.pos.z);
            let above = r.transform.pos.y - ground;
            assert!((above - BURIAL * radius_m).abs() < 1e-9, "centre {above} m above the ground");
            assert!(above < radius_m && above > 0.0, "a rock must stand out of the ground without floating");
        }
    }

    #[test]
    fn rock_sizes_skew_to_small_stones() {
        let d = def();
        let c = generate(&d).expect("course");
        let f = &d.rock_fields[0];
        let radii: Vec<f64> = rocks(&c)
            .iter()
            .map(|r| match r.shape {
                PropShape::Sphere { radius_m } => radius_m,
                _ => 0.0,
            })
            .collect();
        let mid = 0.5 * (f.rock_radius_min_m.v + f.rock_radius_max_m.v);
        let small = radii.iter().filter(|&&r| r < mid).count();
        assert!(
            small * 5 > radii.len() * 3,
            "more than 60% should be below the mid radius ({small} of {})",
            radii.len()
        );
        assert!(radii.iter().all(|&r| r >= f.rock_radius_min_m.v - 1e-9 && r <= f.rock_radius_max_m.v + 1e-9));
    }

    fn ridge() -> CourseDef {
        CourseDef::from_ron(include_str!("../../../content/world/courses/ridge.ron")).expect("ridge.ron parses")
    }

    /// Smallest radius of curvature of the road, from circles through points `step_m` of arc apart, m.
    fn min_turn_radius_m(road: &[(f64, f64, f64)], step_m: f64) -> f64 {
        let mut pts = vec![road[0]];
        for &p in road {
            let q = pts[pts.len() - 1];
            if scalar::hypot(p.0 - q.0, p.1 - q.1) >= step_m {
                pts.push(p);
            }
        }
        let mut best = f64::INFINITY;
        for w in pts.windows(3) {
            let (a, b, c) = ((w[0].0, w[0].1), (w[1].0, w[1].1), (w[2].0, w[2].1));
            let area2 = ((b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)).abs();
            if area2 > 1e-9 {
                let (la, lb, lc) = (
                    scalar::hypot(b.0 - a.0, b.1 - a.1),
                    scalar::hypot(c.0 - b.0, c.1 - b.1),
                    scalar::hypot(c.0 - a.0, c.1 - a.1),
                );
                best = best.min(la * lb * lc / (2.0 * area2));
            }
        }
        best
    }

    #[test]
    fn switchback_road_climbs_the_whole_cliff_within_the_road_grade() {
        let d = ridge();
        let c = generate(&d).expect("ridge");
        let (first, last) = (c.road[0], c.road[c.road.len() - 1]);
        let h = d.cliffs[0].height_m.v;
        assert!(first.2 < 3.0 && last.2 > h - 3.0, "the road starts at {} m and ends at {} m", first.2, last.2);
        for w in c.road.windows(2) {
            let run = scalar::hypot(w[1].0 - w[0].0, w[1].1 - w[0].1);
            assert!(
                (w[1].2 - w[0].2).abs() / run <= d.road.max_grade.v + 1e-4,
                "road grade over the limit at ({}, {})",
                w[0].0,
                w[0].1
            );
        }
        // The road really zig-zags: it is several times longer than the straight line between its ends.
        let len: f64 = c.road.windows(2).map(|w| scalar::hypot(w[1].0 - w[0].0, w[1].1 - w[0].1)).sum();
        let direct = scalar::hypot(last.0 - first.0, last.1 - first.1);
        assert!(len > 1.5 * direct, "road {len} m vs straight {direct} m");
    }

    #[test]
    fn switchback_hairpins_are_wider_than_a_truck_can_turn_in() {
        let c = generate(&ridge()).expect("ridge");
        let r = min_turn_radius_m(&c.road, 6.0);
        assert!(r >= 5.0, "tightest bend on the road has radius {r} m");
    }

    #[test]
    fn cliff_face_is_as_steep_as_stated_and_the_hills_stay_inside_their_limit() {
        let d = ridge();
        let c = generate(&d).expect("ridge");
        let cl = &d.cliffs[0];
        let mut steepest = 0.0f64;
        for k in 0..4000 {
            let (s, t) = (cl.depth_m() * (k % 61) as f64 / 60.0, -110.0 + 140.0 * (k / 61) as f64 / 65.0);
            let (x, z) = cl.to_xz(s, t);
            let nrm = c.world.normal(x, z);
            if !c.road.iter().any(|r| scalar::hypot(r.0 - x, r.1 - z) < 14.0) {
                steepest = steepest.max(scalar::hypot(nrm.x, nrm.z) / nrm.y);
            }
        }
        let (g, base) = (cl.face_grade.v, d.hills.max_grade.v);
        assert!(steepest >= 0.9 * g && steepest <= g + base + 0.05, "steepest face grade {steepest}, stated {g}");
    }

    #[test]
    fn chute_is_as_steep_as_stated_and_laid_in_its_surface() {
        let d = ridge();
        let c = generate(&d).expect("ridge");
        let (cl, ch) = (&d.cliffs[0], d.cliffs[0].chute.as_ref().expect("chute"));
        let mut steepest = 0.0f64;
        for k in 0..=40 {
            let s = cl.max_depth_m() * k as f64 / 40.0;
            let (x, z) = cl.to_xz(s, ch.t_m);
            let nrm = c.world.normal(x, z);
            steepest = steepest.max(scalar::hypot(nrm.x, nrm.z) / nrm.y);
            if s > 1.0 && s < cl.max_depth_m() - 1.0 {
                assert_eq!(c.world.material_at(x, z).name, ch.surface, "the chute is laid in {}", ch.surface);
            }
        }
        assert!(
            steepest >= 0.9 * ch.grade.v && steepest <= ch.grade.v + d.hills.max_grade.v + 0.05,
            "chute grade {steepest}"
        );
        assert!(steepest < d.cliffs[0].face_grade.v, "the chute must be gentler than the wall");
    }

    #[test]
    fn terrain_on_the_ridge_never_exceeds_its_own_stated_maximum_grade() {
        let d = ridge();
        let c = generate(&d).expect("ridge");
        let n = c.world.n();
        let mut rng = Pcg32::new(8, 8);
        for _ in 0..6000 {
            let (x, z) = (rng.range_f64(-199.0, 199.0), rng.range_f64(-199.0, 199.0));
            let nrm = c.world.normal(x, z);
            let grade = scalar::hypot(nrm.x, nrm.z) / nrm.y;
            let (i, j) = (((x + 200.0).floor() as usize).min(n - 2), ((z + 200.0).floor() as usize).min(n - 2));
            let lim = [(i, j), (i + 1, j), (i, j + 1), (i + 1, j + 1)]
                .iter()
                .map(|&(a, b)| c.grade_limit[b * n + a])
                .fold(0.0, f64::max);
            assert!(grade <= lim * 1.02, "grade {grade} over its limit {lim} at ({x}, {z})");
        }
    }

    #[test]
    fn a_switchback_that_cannot_fit_or_cannot_climb_is_refused_with_the_numbers() {
        let mut d = ridge();
        d.road.switchbacks[0].span_m = Param::spec(60.0, "test: short legs need many lanes");
        let e = generate(&d).err().expect("refused");
        assert!(e.contains("between lanes"), "{e}");
        d.road.switchbacks[0].span_m = Param::spec(20.0, "test: too short to climb at all");
        let e = generate(&d).err().expect("refused");
        assert!(e.contains("cannot climb"), "{e}");
    }

    #[test]
    fn ridge_generator_is_deterministic_for_a_seed() {
        let d = ridge();
        let (a, b) = (generate(&d).expect("a"), generate(&d).expect("b"));
        assert_eq!(hash(&a), hash(&b));
        assert_eq!(hash(&a), GOLDEN_RIDGE_HASH, "ridge hash was {:#018x}", hash(&a));
    }

    /// The slice course without its rough sections: the same ground, no ripples.
    fn smooth_slice() -> Course {
        let mut d = def();
        d.road.whoops.clear();
        d.road.washboards.clear();
        d.road.mud_crossings.iter_mut().for_each(|m| m.pit_depth_m = None);
        generate(&d).expect("smooth slice")
    }

    /// Centreline sample of the slice road at road distance `s_m`, with the unit direction of travel.
    fn road_at(c: &Course, s_m: f64) -> ((f64, f64), (f64, f64)) {
        let mut acc = 0.0;
        for w in c.road.windows(2) {
            let len = scalar::hypot(w[1].0 - w[0].0, w[1].1 - w[0].1);
            if acc + len >= s_m {
                let f = (s_m - acc) / len;
                return (
                    (w[0].0 + (w[1].0 - w[0].0) * f, w[0].1 + (w[1].1 - w[0].1) * f),
                    ((w[1].0 - w[0].0) / len, (w[1].1 - w[0].1) / len),
                );
            }
            acc += len;
        }
        panic!("beyond the end of the road");
    }

    fn road_length(c: &Course) -> f64 {
        c.road.windows(2).map(|w| scalar::hypot(w[1].0 - w[0].0, w[1].1 - w[0].1)).sum()
    }

    #[test]
    fn whoops_have_the_stated_wavelength_and_peak_to_peak_amplitude() {
        let d = def();
        let (with, without) = (generate(&d).expect("slice"), smooth_slice());
        let r = &d.road.whoops[0];
        let start = r.at_fraction.v * road_length(&with) - r.length_m.v * 0.5;
        let (lam, amp) = (r.wavelength_m.v, r.amplitude_m.v);
        let diff = |s: f64| {
            let (p, _) = road_at(&with, s);
            with.world.height_m(p.0, p.1) - without.world.height_m(p.0, p.1)
        };
        // Mid-section (full weight): crest half a wavelength after a trough, spaced one wavelength apart.
        let s0 = start + 3.0 * lam;
        for k in 0..3 {
            let t = s0 + k as f64 * lam;
            assert!(diff(t).abs() < 0.15 * amp, "trough at {t}: {}", diff(t));
            assert!(
                (diff(t + lam * 0.5) - amp).abs() < 0.15 * amp,
                "crest at {}: {}",
                t + lam * 0.5,
                diff(t + lam * 0.5)
            );
        }
    }

    #[test]
    fn washboard_ripples_have_the_stated_wavelength_and_amplitude() {
        let d = def();
        let c = generate(&d).expect("slice");
        let r = &d.road.washboards[0];
        let mid = r.at_fraction.v * road_length(&c);
        let ((x, z), dir) = road_at(&c, mid);
        let lam = r.wavelength_m.v;
        let steps = 4000;
        let hs: Vec<f64> = (0..steps)
            .map(|k| {
                let s = k as f64 * lam * 5.0 / steps as f64;
                c.world.height_m(x + dir.0 * s, z + dir.1 * s)
            })
            .collect();
        // Remove the slow trend of the road under the ripples with a one-wavelength running mean.
        let win = (steps as f64 / 5.0) as usize;
        let ripple: Vec<f64> =
            (win..steps - win).map(|k| hs[k] - hs[k - win / 2..k + win / 2].iter().sum::<f64>() / win as f64).collect();
        let (lo, hi) = ripple.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), &v| (l.min(v), h.max(v)));
        assert!(
            (hi - lo - r.amplitude_m.v).abs() < 0.3 * r.amplitude_m.v,
            "peak to peak {} vs stated {}",
            hi - lo,
            r.amplitude_m.v
        );
        let crossings = ripple.windows(2).filter(|w| w[0] < 0.0 && w[1] >= 0.0).count() as f64;
        let span = (ripple.len() as f64) * lam * 5.0 / steps as f64;
        assert!((span / crossings - lam).abs() < 0.15 * lam, "wavelength {} vs stated {lam}", span / crossings);
    }

    #[test]
    fn washboard_normal_is_the_gradient_of_its_height() {
        let d = def();
        let c = generate(&d).expect("slice");
        let r = &d.road.washboards[0];
        let eps = 1e-5;
        for k in 0..200 {
            let ((x, z), dir) = road_at(&c, r.at_fraction.v * road_length(&c) + (k as f64 - 100.0) * 0.137);
            let (x, z) = (x + 0.3 * dir.1, z - 0.3 * dir.0 + 1e-3 * k as f64);
            let gx = (c.world.height_m(x + eps, z) - c.world.height_m(x - eps, z)) / (2.0 * eps);
            let gz = (c.world.height_m(x, z + eps) - c.world.height_m(x, z - eps)) / (2.0 * eps);
            let n = Vec3::new(-gx, 1.0, -gz).normalized_or_zero();
            assert!(
                (c.world.normal(x, z) - n).length() < 1e-4,
                "normal disagrees with the height gradient at ({x}, {z})"
            );
        }
    }

    #[test]
    fn rough_sections_leave_ground_away_from_the_road_untouched() {
        let d = def();
        let (with, without) = (generate(&d).expect("slice"), smooth_slice());
        let mut rng = Pcg32::new(21, 21);
        let mut checked = 0;
        for _ in 0..4000 {
            let (x, z) = (rng.range_f64(-190.0, 190.0), rng.range_f64(-190.0, 190.0));
            let near = with.road.iter().map(|p| scalar::hypot(p.0 - x, p.1 - z)).fold(f64::INFINITY, f64::min);
            if near > d.road.width_m.v * 0.5 + 3.0 {
                assert!(
                    (with.world.height_m(x, z) - without.world.height_m(x, z)).abs() < 1e-9,
                    "ground changed at ({x}, {z})"
                );
                checked += 1;
            }
        }
        assert!(checked > 3000);
    }

    #[test]
    fn a_whoop_too_short_for_the_grid_or_a_washboard_too_long_is_refused() {
        let mut d = def();
        d.road.whoops[0].wavelength_m = Param::spec(2.0, "test");
        assert!(generate(&d).err().expect("refused").contains("use a washboard"));
        let mut d = def();
        d.road.washboards[0].wavelength_m = Param::spec(5.0, "test");
        assert!(generate(&d).err().expect("refused").contains("use whoops"));
    }

    fn river_course() -> CourseDef {
        CourseDef::from_ron(include_str!("../../../content/world/courses/river.ron")).expect("river.ron parses")
    }

    /// A point on the river centreline at arc distance `arc_m`, its unit tangent and the unit normal (plan).
    fn river_frame(d: &CourseDef, arc_m: f64) -> ((f64, f64), (f64, f64), (f64, f64)) {
        let line = d.rivers[0].centreline(d.seed);
        let k = line.iter().position(|p| p.2 >= arc_m).expect("arc on the river").clamp(1, line.len() - 2);
        let (a, b) = (line[k - 1], line[k + 1]);
        let len = scalar::hypot(b.0 - a.0, b.1 - a.1);
        let t = ((b.0 - a.0) / len, (b.1 - a.1) / len);
        ((line[k].0, line[k].1), t, (-t.1, t.0))
    }

    fn river_total(d: &CourseDef) -> f64 {
        d.rivers[0].centreline(d.seed).last().expect("line").2
    }

    #[test]
    fn river_surface_is_flat_across_the_channel_and_falls_downstream() {
        let d = river_course();
        let c = generate(&d).expect("river");
        let slope = d.rivers[0].surface_slope.v;
        let level = |arc: f64, off: f64| {
            let (p, _, nrm) = river_frame(&d, arc);
            c.world.water_surface_m(p.0 + nrm.0 * off, p.1 + nrm.1 * off).expect("water at this offset")
        };
        for arc in [90.0, 150.0, 330.0] {
            for off in [-4.0, -2.0, 2.0, 4.0] {
                assert!(
                    (level(arc, off) - level(arc, 0.0)).abs() < 0.02,
                    "surface not flat across the channel at arc {arc}, offset {off}"
                );
            }
        }
        assert!(
            (level(90.0, 0.0) - level(330.0, 0.0) - slope * 240.0).abs() < 0.03,
            "the water must fall {slope} per metre downstream"
        );
    }

    #[test]
    fn channel_is_as_deep_as_stated_away_from_the_ford() {
        let d = river_course();
        let c = generate(&d).expect("river");
        let (p, _, _) = river_frame(&d, 100.0);
        let depth = c.world.water_surface_m(p.0, p.1).expect("water") - c.world.height_m(p.0, p.1);
        // The ground detail (a few centimetres RMS) rides on the carved bed.
        assert!((depth - d.rivers[0].depth_m.v).abs() < 0.25, "depth {depth} vs stated {}", d.rivers[0].depth_m.v);
    }

    #[test]
    fn ford_is_wadable_across_its_whole_width() {
        let d = river_course();
        let c = generate(&d).expect("river");
        let (r, f) = (&d.rivers[0], &d.rivers[0].fords[0]);
        let arc = f.at_fraction.v * river_total(&d);
        let mut deepest = 0.0f64;
        for k in -8..=8 {
            let (p, _, nrm) = river_frame(&d, arc);
            let (x, z) = (p.0 + nrm.0 * k as f64, p.1 + nrm.1 * k as f64);
            if let Some(s) = c.world.water_surface_m(x, z) {
                deepest = deepest.max(s - c.world.height_m(x, z));
            }
        }
        assert!(deepest > 0.3, "there must still be water at the ford ({deepest} m)");
        assert!(
            deepest <= f.depth_m.v + 0.1 && f.depth_m.v < r.depth_m.v,
            "ford depth {deepest} vs stated {}",
            f.depth_m.v
        );
    }

    #[test]
    fn river_banks_climb_out_at_the_stated_grade() {
        let d = river_course();
        let c = generate(&d).expect("river");
        let r = &d.rivers[0];
        let (p, _, nrm) = river_frame(&d, 100.0);
        let h = |off: f64| c.world.height_m(p.0 + nrm.0 * off, p.1 + nrm.1 * off);
        let (from, to) = (r.width_m.v * 0.5 + 1.0, r.width_m.v * 0.5 + r.freeboard_m.v / r.bank_grade.v - 1.0);
        let grade = (h(to) - h(from)) / (to - from);
        assert!((grade - r.bank_grade.v).abs() < 0.07, "bank grade {grade} vs stated {}", r.bank_grade.v);
    }

    #[test]
    fn no_road_enters_the_water() {
        let d = river_course();
        let c = generate(&d).expect("river");
        for &(x, z, _) in &c.road {
            for (dx, dz) in [(0.0, 0.0), (3.0, 0.0), (-3.0, 0.0), (0.0, 3.0), (0.0, -3.0)] {
                assert!(c.world.water_surface_m(x + dx, z + dz).is_none(), "water on the road at ({x}, {z})");
            }
        }
    }

    #[test]
    fn water_exists_only_where_the_ground_is_below_the_surface_and_lies_on_a_riverbed() {
        let d = river_course();
        let c = generate(&d).expect("river");
        let mut rng = Pcg32::new(4, 4);
        let (mut wet, mut dry) = (0, 0);
        for _ in 0..20000 {
            let (x, z) = (rng.range_f64(-199.0, 199.0), rng.range_f64(-199.0, 199.0));
            match c.world.water_surface_m(x, z) {
                Some(s) => {
                    wet += 1;
                    assert!(c.world.height_m(x, z) < s);
                    assert_eq!(
                        c.world.material_at(x, z).name,
                        "riverbed",
                        "the bed under the water is riverbed at ({x}, {z})"
                    );
                }
                None => dry += 1,
            }
        }
        assert!(wet > 200 && dry > 15000, "wet {wet}, dry {dry}");
    }

    #[test]
    fn river_course_is_deterministic_for_a_seed() {
        let d = river_course();
        let (a, b) = (generate(&d).expect("a"), generate(&d).expect("b"));
        assert_eq!(hash(&a), hash(&b));
        assert_eq!(hash(&a), GOLDEN_RIVER_HASH, "river hash was {:#018x}", hash(&a));
    }

    #[test]
    fn a_river_valley_narrower_than_its_banks_is_refused() {
        let mut d = river_course();
        d.rivers[0].valley_half_m = Param::spec(5.0, "test");
        assert!(generate(&d).err().expect("refused").contains("narrower than the channel"));
        let mut d = river_course();
        d.rivers[0].width_m = Param::spec(8.0, "test: too narrow for its depth");
        assert!(generate(&d).err().expect("refused").contains("at least"));
    }

    fn crossing() -> CourseDef {
        CourseDef::from_ron(include_str!("../../../content/world/courses/crossing.ron")).expect("crossing.ron parses")
    }

    fn bridge(c: &Course, kind: BridgeKind) -> &PlacedBridge {
        c.bridges.iter().find(|b| b.kind == kind).expect("bridge of this kind")
    }

    /// Plan point at fraction `f` of the way from `a` to `b` of a bridge.
    fn on_bridge(b: &PlacedBridge, f: f64) -> (f64, f64) {
        (b.a.0 + (b.b.0 - b.a.0) * f, b.a.1 + (b.b.1 - b.a.1) * f)
    }

    #[test]
    fn bridge_deck_is_level_with_the_banks_and_the_river_runs_beneath_it() {
        let d = crossing();
        let c = generate(&d).expect("crossing");
        let fb = d.rivers[0].freeboard_m.v;
        for kind in [BridgeKind::Road, BridgeKind::Wooden] {
            let b = bridge(&c, kind);
            let mid = on_bridge(b, 0.5);
            // The deck is dry and at deck level; the water beside it is below by about the freeboard.
            assert!(c.world.water_surface_m(mid.0, mid.1).is_none(), "{kind:?}: water on the deck");
            assert!((c.world.height_m(mid.0, mid.1) - b.deck_y_m).abs() < 0.05, "{kind:?}: deck height");
            let dir = (
                (b.b.0 - b.a.0) / scalar::hypot(b.b.0 - b.a.0, b.b.1 - b.a.1),
                (b.b.1 - b.a.1) / scalar::hypot(b.b.0 - b.a.0, b.b.1 - b.a.1),
            );
            let side = (-dir.1, dir.0);
            let beside = (mid.0 + side.0 * (b.width_m * 0.5 + 1.5), mid.1 + side.1 * (b.width_m * 0.5 + 1.5));
            let level = c.world.water_surface_m(beside.0, beside.1).expect("the river runs beside the deck");
            assert!(
                (b.deck_y_m - level - fb).abs() < 0.1,
                "{kind:?}: deck {} m above the water, expected {fb}",
                b.deck_y_m - level
            );
        }
    }

    #[test]
    fn bridge_deck_is_as_wide_as_stated_and_drops_away_beside_it() {
        let d = crossing();
        let c = generate(&d).expect("crossing");
        for kind in [BridgeKind::Road, BridgeKind::Wooden] {
            let b = bridge(&c, kind);
            let mid = on_bridge(b, 0.5);
            let len = scalar::hypot(b.b.0 - b.a.0, b.b.1 - b.a.1);
            let side = (-(b.b.1 - b.a.1) / len, (b.b.0 - b.a.0) / len);
            let h = |off: f64| c.world.height_m(mid.0 + side.0 * off, mid.1 + side.1 * off);
            // The deck's edge is accurate to a grid cell, so look one and a half cells inside it.
            assert!(
                (h((b.width_m * 0.5 - 1.5).max(0.0)) - b.deck_y_m).abs() < 0.05,
                "{kind:?}: the deck reaches its edges"
            );
            assert!(b.deck_y_m - h(b.width_m * 0.5 + 1.5) > 1.0, "{kind:?}: a drop into the water beside the deck");
        }
    }

    #[test]
    fn the_wooden_bridge_is_narrower_and_lower_rated_than_the_road_bridge() {
        let c = generate(&crossing()).expect("crossing");
        let (w, r) = (bridge(&c, BridgeKind::Wooden), bridge(&c, BridgeKind::Road));
        assert!(w.width_m < r.width_m && w.load_limit_kg < r.load_limit_kg);
        assert_eq!(c.world.material_at(on_bridge(w, 0.5).0, on_bridge(w, 0.5).1).name, "planks");
        assert_eq!(c.world.material_at(on_bridge(r, 0.5).0, on_bridge(r, 0.5).1).name, "asphalt");
    }

    #[test]
    fn both_roads_cross_the_river_over_their_bridges_and_never_through_the_water() {
        let d = crossing();
        let c = generate(&d).expect("crossing");
        let line = d.rivers[0].centreline(d.seed);
        let side_of = |p: (f64, f64)| {
            let (k, _) = line
                .iter()
                .enumerate()
                .min_by(|a, b| {
                    scalar::hypot(a.1 .0 - p.0, a.1 .1 - p.1).total_cmp(&scalar::hypot(b.1 .0 - p.0, b.1 .1 - p.1))
                })
                .expect("line");
            let (a, b) = (line[k.saturating_sub(1)], line[(k + 1).min(line.len() - 1)]);
            ((b.0 - a.0) * (p.1 - line[k].1) - (b.1 - a.1) * (p.0 - line[k].0)).signum()
        };
        for road in std::iter::once(&c.road).chain(&c.extra_roads) {
            let mut flips = 0;
            for w in road.windows(2) {
                if side_of((w[0].0, w[0].1)) * side_of((w[1].0, w[1].1)) < 0.0 {
                    flips += 1;
                }
            }
            assert_eq!(flips, 1, "a road must cross the river exactly once");
            for p in road {
                assert!(c.world.water_surface_m(p.0, p.1).is_none(), "road wades at ({}, {})", p.0, p.1);
            }
        }
    }

    #[test]
    fn roads_keep_their_own_grade_limit_across_a_bridge() {
        let d = crossing();
        let c = generate(&d).expect("crossing");
        for (road, limit) in [(&c.road, d.road.max_grade.v), (&c.extra_roads[0], d.extra_roads[0].max_grade.v)] {
            for w in road.windows(2) {
                let run = scalar::hypot(w[1].0 - w[0].0, w[1].1 - w[0].1);
                assert!(
                    (w[1].2 - w[0].2).abs() / run <= limit + 1e-4,
                    "grade over {limit} at ({}, {})",
                    w[0].0,
                    w[0].1
                );
            }
        }
    }

    #[test]
    fn bridge_rails_stand_on_both_edges_and_only_the_wooden_ones_break() {
        let c = generate(&crossing()).expect("crossing");
        let walls: Vec<_> = c.world.props().iter().filter(|p| p.kind == PropKind::Wall).collect();
        assert_eq!(walls.len(), 4, "two rails per bridge");
        assert_eq!(walls.iter().filter(|p| p.break_impulse_ns.is_finite()).count(), 2);
        for p in &walls {
            let PropShape::Box { half_m } = p.shape else { panic!("rails are boxes") };
            assert!(half_m.z > 5.0, "a rail runs the length of the deck");
        }
    }

    #[test]
    fn crossing_course_is_deterministic_for_a_seed() {
        let d = crossing();
        let (a, b) = (generate(&d).expect("a"), generate(&d).expect("b"));
        assert_eq!(hash(&a), hash(&b));
        assert_eq!(hash(&a), GOLDEN_CROSSING_HASH, "crossing hash was {:#018x}", hash(&a));
    }

    #[test]
    fn a_bridge_over_a_river_that_does_not_exist_is_refused() {
        let mut d = crossing();
        d.road.bridges[0].river = 3;
        assert!(generate(&d).err().expect("refused").contains("does not exist"));
    }

    #[test]
    fn soft_patch_is_sand_inside_its_disc_only_and_never_on_the_road() {
        let d = def();
        let c = generate(&d).expect("course");
        let sp = &d.soft_patches[0];
        let (mut sand, mut inside) = (0, 0);
        for k in 0..c.world.n() * c.world.n() {
            let (i, j) = (k % c.world.n(), k / c.world.n());
            let (x, z) = c.world.node_xz(i, j);
            let is_sand =
                c.world.materials().get(MaterialId(u16::from(c.world.splat_at_node(i, j)))).name == sp.surface;
            let in_disc = scalar::hypot(x - sp.x_m, z - sp.z_m) <= sp.radius_m.v;
            if is_sand {
                sand += 1;
                assert!(in_disc, "sand outside the patch at ({x}, {z})");
                assert!(c.world.material_at(x, z).soil.is_some(), "sand carries soil parameters");
            }
            if in_disc {
                inside += 1;
            }
        }
        assert!(sand > inside / 2, "the patch should be mostly sand ({sand} of {inside} cells)");
        for &(x, z, _) in &c.road {
            assert_ne!(c.world.material_at(x, z).name, sp.surface, "sand on the road");
        }
    }

    #[test]
    fn a_soft_patch_of_hard_ground_is_refused() {
        let mut d = def();
        d.soft_patches[0].surface = "gravel".into();
        assert!(generate(&d).err().expect("refused").contains("not soft ground"));
    }

    #[test]
    fn mud_pit_is_as_deep_as_stated_in_the_middle_and_leaves_the_road_alone_outside() {
        let d = def();
        let (with, without) = (generate(&d).expect("slice"), smooth_slice());
        let m = &d.road.mud_crossings[0];
        let depth = m.pit_depth_m.as_ref().expect("the slice has a pit").v;
        let total = road_length(&with);
        let diff = |s: f64| {
            let (p, _) = road_at(&with, s);
            without.world.height_m(p.0, p.1) - with.world.height_m(p.0, p.1)
        };
        let mid = m.at_fraction.v * total;
        assert!((diff(mid) - depth).abs() < 0.08, "the pit is {} m deep in the middle, stated {depth}", diff(mid));
        assert!(diff(mid - m.length_m.v * 0.5 - 4.0).abs() < 0.02, "no dip before the pit");
        assert!(diff(mid + m.length_m.v * 0.5 + 4.0).abs() < 0.02, "no dip after the pit");
        // The walls are eased: no steeper than the bound the cells' limits were given.
        let wall = (m.length_m.v / 3.0).min(PIT_WALL_M);
        let mut worst = 0.0f64;
        for k in 0..400 {
            let s = mid - m.length_m.v * 0.5 + k as f64 * m.length_m.v / 400.0;
            worst = worst.max((diff(s + 0.5) - diff(s)).abs() / 0.5);
        }
        assert!(worst <= depth * SMOOTHSTEP_PEAK / wall * 1.1, "pit wall slope {worst}");
        assert!(
            with.world.material_at(road_at(&with, mid).0 .0, road_at(&with, mid).0 .1).soil.is_some(),
            "the pit is filled with mud"
        );
    }
}
