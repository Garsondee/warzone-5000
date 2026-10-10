//! Course features laid on the terrain: mud where water collects, Poisson-disc tree stands and a barricade across the road.
//! Every function is a pure function of its inputs (randomness comes from `Pcg32::derive(seed, [stage])`, never from query order).

use serde::{Deserialize, Serialize};
use w5k_contract::param::Param;
use w5k_contract::world::{PropId, PropKind, PropRef, PropShape};
use w5k_math::{scalar, Pcg32, Quat, Transform, Vec3};

use crate::grid::CELL_M;

/// Mud where the drainage rule says water collects: a cell with at least `min_drainage_cells` upslope cells draining through it
/// (D8 flow accumulation) and a local slope no steeper than `max_slope` (water stands only on gentle ground).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MudDef {
    pub min_drainage_cells: Param,
    pub max_slope: Param,
    /// A channel cell wets the ground this far around it (a bog is wider than the one-cell line the flow rule finds), m.
    pub spread_m: Param,
}

/// Tree stands: Poisson-disc sampling (no two trunks closer than `min_spacing_m`), thinned by `density`, on ground gentler than
/// `max_slope` and at least `road_clearance_m` from the road.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreesDef {
    pub min_spacing_m: Param,
    /// Acceptance probability of a candidate, 0 to 1.
    pub density: Param,
    pub max_slope: Param,
    pub road_clearance_m: Param,
    /// Size of the clumps: wavelength of the noise that thins the density, m.
    pub stand_wavelength_m: Param,
    pub trunk_radius_m: Param,
    pub height_m: Param,
    pub break_impulse_ns: Param,
}

/// Two blocks across the road leaving a gap in the middle (a chicane the vehicle must thread).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BarricadeDef {
    /// Position along the road centreline, 0 = start, 1 = finish. PROVISIONAL(C-004): fixed by hand until chokepoint detection lands.
    pub at_fraction: Param,
    pub gap_m: Param,
    pub block_height_m: Param,
    pub block_depth_m: Param,
}

/// A stone-riddled field: a disc of half-buried rocks with a guaranteed minimum gap between them (the lane a vehicle must find),
/// sizes skewed to many small and few large. The ground under it becomes `surface` (gravel).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RockFieldDef {
    pub x_m: f64,
    pub z_m: f64,
    pub radius_m: Param,
    /// Closest two rock centres may be, m (Poisson-disc radius). The free gap between the biggest rocks is `min_spacing - 2 r_max`.
    pub min_spacing_m: Param,
    /// Acceptance probability of a candidate, 0 to 1.
    pub density: Param,
    pub rock_radius_min_m: Param,
    pub rock_radius_max_m: Param,
    pub road_clearance_m: Param,
    pub surface: String,
}

/// Cells within `r_m` of any cell flagged in `mask` (a disc dilation).
pub fn dilate(mask: &[bool], n: usize, r_m: f64) -> Vec<bool> {
    let r = (r_m / CELL_M).ceil() as i64;
    let mut out = vec![false; n * n];
    for c in (0..n * n).filter(|&c| mask[c]) {
        let (i, j) = ((c % n) as i64, (c / n) as i64);
        for dj in -r..=r {
            for di in -r..=r {
                let (ni, nj) = (i + di, j + dj);
                if ni >= 0
                    && nj >= 0
                    && ni < n as i64
                    && nj < n as i64
                    && scalar::hypot(di as f64, dj as f64) * CELL_M <= r_m
                {
                    out[nj as usize * n + ni as usize] = true;
                }
            }
        }
    }
    out
}

/// D8 flow accumulation: for every cell, the number of cells (itself included) whose water reaches it, each cell draining to its
/// steepest lower neighbour. Cells are processed from high to low (ties by index), so the result is order-independent and exact.
pub fn drainage_area(h: &[f64], n: usize) -> Vec<u32> {
    let mut order: Vec<usize> = (0..n * n).collect();
    order.sort_by(|&a, &b| h[b].total_cmp(&h[a]).then(a.cmp(&b)));
    let mut acc = vec![1u32; n * n];
    for &c in &order {
        let (i, j) = ((c % n) as i64, (c / n) as i64);
        let mut best: Option<(f64, usize)> = None;
        for dj in -1i64..=1 {
            for di in -1i64..=1 {
                let (ni, nj) = (i + di, j + dj);
                if (di == 0 && dj == 0) || ni < 0 || nj < 0 || ni >= n as i64 || nj >= n as i64 {
                    continue;
                }
                let nb = nj as usize * n + ni as usize;
                let drop = (h[c] - h[nb]) / (scalar::hypot(di as f64, dj as f64) * CELL_M);
                if drop > 0.0 && best.is_none_or(|(d, _)| drop > d) {
                    best = Some((drop, nb));
                }
            }
        }
        if let Some((_, nb)) = best {
            acc[nb] += acc[c];
        }
    }
    acc
}

/// Local slope (rise over run) by central differences, edges one-sided.
pub fn cell_slope(h: &[f64], n: usize, i: usize, j: usize) -> f64 {
    let at = |i: usize, j: usize| h[j * n + i];
    let (i0, i1, j0, j1) = (i.saturating_sub(1), (i + 1).min(n - 1), j.saturating_sub(1), (j + 1).min(n - 1));
    let gx = (at(i1, j) - at(i0, j)) / ((i1 - i0) as f64 * CELL_M);
    let gz = (at(i, j1) - at(i, j0)) / ((j1 - j0) as f64 * CELL_M);
    scalar::hypot(gx, gz)
}

/// Cells that satisfy the drainage rule itself (before spreading and before the road is excluded by the caller).
pub fn drainage_rule_mask(h: &[f64], n: usize, def: &MudDef) -> Vec<bool> {
    let acc = drainage_area(h, n);
    (0..n * n)
        .map(|c| f64::from(acc[c]) >= def.min_drainage_cells.v && cell_slope(h, n, c % n, c / n) <= def.max_slope.v)
        .collect()
}

/// Mud cells: the rule cells plus everything within `spread_m` of one.
pub fn mud_mask(h: &[f64], n: usize, def: &MudDef) -> Vec<bool> {
    let rule = drainage_rule_mask(h, n, def);
    let r = (def.spread_m.v / CELL_M).ceil() as i64;
    let mut out = rule.clone();
    for c in (0..n * n).filter(|&c| rule[c]) {
        let (i, j) = ((c % n) as i64, (c / n) as i64);
        for dj in -r..=r {
            for di in -r..=r {
                let (ni, nj) = (i + di, j + dj);
                if ni >= 0
                    && nj >= 0
                    && ni < n as i64
                    && nj < n as i64
                    && scalar::hypot(di as f64, dj as f64) * CELL_M <= def.spread_m.v
                {
                    out[nj as usize * n + ni as usize] = true;
                }
            }
        }
    }
    out
}

/// Bridson's Poisson-disc sampling over `[-half, half]^2`: `accept(x, z)` is the probability that a candidate may join (0 rejects it).
pub fn poisson_disc(rng: &mut Pcg32, half: f64, r: f64, accept: &dyn Fn(f64, f64) -> f64) -> Vec<(f64, f64)> {
    poisson_disc_in(rng, (0.0, 0.0), half, r, accept)
}

/// As [`poisson_disc`] over the square of half side `half` centred on `centre`.
pub fn poisson_disc_in(
    rng: &mut Pcg32,
    centre: (f64, f64),
    half: f64,
    r: f64,
    accept: &dyn Fn(f64, f64) -> f64,
) -> Vec<(f64, f64)> {
    let k = 30; // const-ok: Bridson's candidates per active point
    let cell = r / std::f64::consts::SQRT_2;
    let side = (2.0 * half / cell).ceil() as usize + 1;
    let mut grid = vec![u32::MAX; side * side];
    let mut pts: Vec<(f64, f64)> = Vec::new();
    let mut active: Vec<usize> = Vec::new();
    let gi = |p: (f64, f64)| (((p.0 - centre.0 + half) / cell) as usize, ((p.1 - centre.1 + half) / cell) as usize);
    let ok = |pts: &Vec<(f64, f64)>, grid: &Vec<u32>, p: (f64, f64)| -> bool {
        if (p.0 - centre.0).abs() > half || (p.1 - centre.1).abs() > half {
            return false;
        }
        let (ci, cj) = gi(p);
        for j in cj.saturating_sub(2)..=(cj + 2).min(side - 1) {
            for i in ci.saturating_sub(2)..=(ci + 2).min(side - 1) {
                let q = grid[j * side + i];
                if q != u32::MAX {
                    let o = pts[q as usize];
                    if scalar::hypot(o.0 - p.0, o.1 - p.1) < r {
                        return false;
                    }
                }
            }
        }
        true
    };
    // Seed points: a fixed number of random tries, so a course with little valid ground still ends.
    for _ in 0..200 {
        // const-ok: seeding attempts
        let p = (centre.0 + rng.range_f64(-half, half), centre.1 + rng.range_f64(-half, half));
        if rng.next_f64() < accept(p.0, p.1) && ok(&pts, &grid, p) {
            let (ci, cj) = gi(p);
            grid[cj * side + ci] = pts.len() as u32;
            active.push(pts.len());
            pts.push(p);
        }
    }
    while !active.is_empty() {
        let a = rng.below(active.len() as u32) as usize;
        let base = pts[active[a]];
        let mut found = false;
        for _ in 0..k {
            let (ang, rad) = (rng.range_f64(0.0, scalar::TAU), rng.range_f64(r, 2.0 * r));
            let p = (base.0 + rad * scalar::cos(ang), base.1 + rad * scalar::sin(ang));
            if rng.next_f64() < accept(p.0, p.1) && ok(&pts, &grid, p) {
                let (ci, cj) = gi(p);
                grid[cj * side + ci] = pts.len() as u32;
                active.push(pts.len());
                pts.push(p);
                found = true;
                break;
            }
        }
        if !found {
            active.swap_remove(a);
        }
    }
    pts
}

/// Two barricade blocks across the road at `centre` (plan `x, z`, ground height `y`), the road running along `tangent` (unit, plan).
/// The blocks cover the road width plus a metre each side and leave `gap_m` open in the middle.
pub fn barricade_blocks(
    first_id: u32,
    centre: (f64, f64, f64),
    tangent: (f64, f64),
    road_half_width_m: f64,
    def: &BarricadeDef,
    ground_y: &dyn Fn(f64, f64) -> f64,
) -> Vec<PropRef> {
    let (gap, depth, height) = (def.gap_m.v, def.block_depth_m.v, def.block_height_m.v);
    let reach = road_half_width_m + 1.0; // const-ok: blocks overhang the road edge by a metre
    let block_len = (reach - gap * 0.5) * 0.5; // half length of each block along the cross axis
    let side = (-tangent.1, tangent.0); // plan perpendicular
    let yaw = scalar::atan2(-tangent.0, -tangent.1); // the box's local -Z points along the tangent
    [-1.0, 1.0]
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let off = s * (gap * 0.5 + block_len);
            let (x, z) = (centre.0 + side.0 * off, centre.1 + side.1 * off);
            PropRef {
                id: PropId(first_id + k as u32),
                kind: PropKind::Barricade,
                shape: PropShape::Box { half_m: Vec3::new(block_len, height * 0.5, depth * 0.5) },
                transform: Transform::new(
                    Vec3::new(x, ground_y(x, z).max(centre.2) + height * 0.5, z),
                    Quat::from_yaw(yaw),
                ),
                break_impulse_ns: f64::INFINITY,
            }
        })
        .collect()
}
