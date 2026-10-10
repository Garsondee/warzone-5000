//! The course generator: a `CourseDef` (RON, every number a `Param`) and a seed give a `GridWorld`, the same way every time.
//!
//! Stages (docs/lanes/world/design-note.md section 6): hills (domain-warped fBm plus an optional named hill, then a maximum-grade
//! clamp), then the road (slope-cost A* between waypoints, a graded profile, stamped into terrain and splat map). Every stage is a pure
//! function of the definition; the only containers are `Vec`s and a heap with a total order, so iteration order cannot leak in.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use serde::{Deserialize, Serialize};
use w5k_contract::param::Param;
use w5k_contract::world::MaterialId;
use w5k_math::scalar;

use crate::grid::{warped_fbm, GridWorld, CELL_M};
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
    pub ground: MaterialId,
    pub road_material: MaterialId,
}

/// The profile is graded to this fraction of the stated road grade; sampled bilinearly the road can add a sliver of slope.
const PROFILE_MARGIN: f64 = 0.9; // const-ok: safety factor on the stated grade

/// One exact two-pass sweep of `v[p] = op(v[p], v[q] + sign * L)` over the 4-neighbour grid (forward then backward raster order):
/// the distance transform that turns a field into the tightest L-Lipschitz field above (`min`, sign +) or below (`max`, sign -) it.
fn sweep(v: &mut [f64], n: usize, l: f64, lower: bool) {
    let step = |a: f64, b: f64| if lower { a.min(b + l) } else { a.max(b - l) };
    for j in 0..n {
        for i in 0..n {
            let c = j * n + i;
            if i > 0 {
                v[c] = step(v[c], v[c - 1]);
            }
            if j > 0 {
                v[c] = step(v[c], v[c - n]);
            }
        }
    }
    for j in (0..n).rev() {
        for i in (0..n).rev() {
            let c = j * n + i;
            if i + 1 < n {
                v[c] = step(v[c], v[c + 1]);
            }
            if j + 1 < n {
                v[c] = step(v[c], v[c + n]);
            }
        }
    }
}

/// Make the field L-Lipschitz along the grid axes (no two axis neighbours differ by more than `axis_limit_m`), exactly and in O(n^2):
/// peaks are shaved by a min-envelope and pits filled by a max-envelope. `anchors` (cell, fixed height) are never moved: every free cell
/// is first clamped into the cone each anchor allows, which makes the two envelopes leave the anchors alone (provided the anchors are
/// mutually consistent).
pub fn limit_grades(h: &mut [f64], n: usize, axis_limit_m: f64, anchors: &[bool]) {
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

/// Slope-cost A* over the 8-neighbour grid between two nodes. The cost of a step is `length * (1 + (slope / g)^2)`, so flat routes win
/// and the road looks for the saddle, but nothing is forbidden: the profile stage grades whatever path comes out.
fn astar(h: &[f64], n: usize, from: (usize, usize), to: (usize, usize), g: f64) -> Option<Vec<(usize, usize)>> {
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
                let step = (len * (1.0 + (slope / g) * (slope / g)) * scale) as u64;
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
        ("road.width_m", &def.road.width_m),
        ("road.shoulder_m", &def.road.shoulder_m),
        ("road.max_grade", &def.road.max_grade),
    ] {
        p.check(&format!("{}.{l}", def.name))?;
    }
    if def.road.waypoints.len() < 2 {
        return Err("road needs at least two waypoints".into());
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
    // Axis limit g / sqrt(2): the bilinear gradient magnitude is at most sqrt(2) x the largest axis difference.
    let axis_terrain = hd.max_grade.v * CELL_M * std::f64::consts::FRAC_1_SQRT_2;
    limit_grades(&mut h, n, axis_terrain, &vec![false; n * n]);

    // 2. Road path, profile, stamp.
    let snap = |p: (f64, f64)| -> Result<(usize, usize), String> {
        let (i, j) = (((p.0 + half) / CELL_M).round(), ((p.1 + half) / CELL_M).round());
        if i < 0.0 || j < 0.0 || i >= n as f64 || j >= n as f64 {
            return Err(format!("waypoint ({}, {}) is outside the course", p.0, p.1));
        }
        Ok((i as usize, j as usize))
    };
    let mut nodes: Vec<(usize, usize)> = Vec::new();
    for pair in def.road.waypoints.windows(2) {
        let leg = astar(&h, n, snap(pair[0])?, snap(pair[1])?, def.road.max_grade.v)
            .ok_or("no road path between waypoints")?;
        nodes.extend(if nodes.is_empty() { &leg[..] } else { &leg[1..] });
    }
    // The A* staircase has tight kinks that a real road would not: average the path over a window (symmetric, so the ends stay put).
    let smooth = 16usize; // const-ok: moving-average half width in samples, a smoothing choice (radius of curvature of about 10 m)
    let pts: Vec<(f64, f64)> = (0..nodes.len())
        .map(|k| {
            let w = smooth.min(k).min(nodes.len() - 1 - k);
            let (mut sx, mut sz) = (0.0, 0.0);
            for &(i, j) in &nodes[k - w..=k + w] {
                sx += i as f64;
                sz += j as f64;
            }
            let m = (2 * w + 1) as f64;
            (sx / m, sz / m)
        })
        .collect();
    let mut y: Vec<f64> = nodes.iter().map(|&(i, j)| h[j * n + i]).collect();
    let ds: Vec<f64> = pts.windows(2).map(|w| scalar::hypot(w[1].0 - w[0].0, w[1].1 - w[0].1) * CELL_M).collect();
    // Grade a little under the stated limit: the stamped cells are sampled bilinearly, which can add a sliver of slope.
    grade_profile(&mut y, &ds, def.road.max_grade.v * PROFILE_MARGIN);

    let (w2, shoulder) = (def.road.width_m.v * 0.5, def.road.shoulder_m.v);
    let reach = w2 + shoulder;
    let mut weight = vec![0.0f64; n * n];
    let mut target = vec![0.0f64; n * n];
    let mut best_d = vec![f64::INFINITY; n * n];
    let r_cells = (reach / CELL_M).ceil() as i64;
    // Distance to the centreline polyline, with the profile height interpolated at the projection (so neighbouring cells get
    // nearly equal targets even at a bend); the nearest segment wins and ties keep the earlier one.
    for k in 0..nodes.len() - 1 {
        let ((ax, az), (bx, bz)) = (pts[k], pts[k + 1]);
        let len2 = (bx - ax) * (bx - ax) + (bz - az) * (bz - az);
        for j in (az.min(bz).floor() as i64 - r_cells).max(0)..=(az.max(bz).ceil() as i64 + r_cells).min(n as i64 - 1) {
            for i in
                (ax.min(bx).floor() as i64 - r_cells).max(0)..=(ax.max(bx).ceil() as i64 + r_cells).min(n as i64 - 1)
            {
                let t = (((i as f64 - ax) * (bx - ax) + (j as f64 - az) * (bz - az)) / len2).clamp(0.0, 1.0);
                let d = scalar::hypot(i as f64 - (ax + t * (bx - ax)), j as f64 - (az + t * (bz - az))) * CELL_M;
                let c = j as usize * n + i as usize;
                if d < reach && d < best_d[c] {
                    best_d[c] = d;
                    target[c] = y[k] + t * (y[k + 1] - y[k]);
                    weight[c] = 1.0 - scalar::smoothstep(w2, reach, d);
                }
            }
        }
    }
    let mut splat = vec![ground.0 as u8; n * n];
    let mut frozen = vec![false; n * n];
    for c in 0..n * n {
        if best_d[c] <= w2 {
            splat[c] = road_material.0 as u8;
            frozen[c] = true;
        }
        if weight[c] > 0.0 {
            h[c] += (target[c] - h[c]) * weight[c];
        }
    }
    // The centreline cells are exactly the profile (they have weight 1); keep the shoulders inside the terrain grade limit.
    limit_grades(&mut h, n, axis_terrain, &frozen);
    let road = pts.iter().zip(&y).map(|(&(i, j), &yy)| (i * CELL_M - half, j * CELL_M - half, yy)).collect();

    let heights: Vec<f32> = h.iter().map(|&v| v as f32).collect();
    let world = GridWorld::from_arrays(n, heights, splat, materials);
    Ok(Course { world, road, ground, road_material })
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::world::WorldQuery;
    use w5k_math::{Pcg32, StateHasher};

    fn def() -> CourseDef {
        CourseDef::from_ron(include_str!("../../../content/world/courses/slice.ron")).expect("slice.ron parses")
    }

    fn hash(c: &Course) -> u64 {
        let mut s = StateHasher::new();
        let w = &c.world;
        for j in 0..w.n() {
            for i in 0..w.n() {
                s.write_u32((w.height_at_node(i, j) as f32).to_bits());
                s.write_u8(w.splat_at_node(i, j));
            }
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
        assert!(generate(&bad).err().expect("refused").contains("road.width_m"));
    }

    #[test]
    fn course_generator_is_deterministic_for_a_seed() {
        let d = def();
        let (a, b) = (generate(&d).expect("a"), generate(&d).expect("b"));
        assert_eq!(hash(&a), hash(&b));
        let mut other = d;
        other.seed += 1;
        assert_ne!(hash(&a), hash(&generate(&other).expect("c")));
    }

    #[test]
    fn terrain_never_exceeds_the_stated_maximum_grade() {
        let d = def();
        let c = generate(&d).expect("course");
        let mut rng = Pcg32::new(5, 5);
        for _ in 0..4000 {
            let (x, z) = (rng.range_f64(-199.0, 199.0), rng.range_f64(-199.0, 199.0));
            let nrm = c.world.normal(x, z);
            let grade = scalar::hypot(nrm.x, nrm.z) / nrm.y;
            assert!(grade <= d.hills.max_grade.v * 1.02, "grade {grade} at ({x}, {z})");
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
        let c = generate(&d).expect("course");
        for &(x, z, y) in &c.road {
            assert_eq!(c.world.material_id_at(x, z), c.road_material);
            assert!((c.world.height_m(x, z) - y).abs() < 0.05, "centreline sits on the graded profile");
            // bilinear sampling between stamped nodes
        }
        let (first, last) = (c.road[0], c.road[c.road.len() - 1]);
        assert!(scalar::hypot(first.0 - d.road.waypoints[0].0, first.1 - d.road.waypoints[0].1) < 1.0);
        let wl = d.road.waypoints[d.road.waypoints.len() - 1];
        assert!(scalar::hypot(last.0 - wl.0, last.1 - wl.1) < 1.0);
    }
}
