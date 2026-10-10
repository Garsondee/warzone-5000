//! Spike S-W: query cost and determinism of a 2001 x 2001 heightfield world (throwaway; see docs/lanes/world/spike-w.md).
//!
//! Heights are stored as f32 (16 MB) and all arithmetic is f64. The surface is the bilinear patch through the four corner
//! heights of each 1 m cell, its normal is the analytic gradient of that same patch, the material is a u8 splat map read
//! at the nearest cell, and props live in a uniform grid (CSR layout, no allocation on query).

use w5k_contract::world::{MaterialId, MaterialTable, PropId, PropKind, PropRef, PropShape, RayHit, WorldQuery};
use w5k_math::{Pcg32, Quat, StateHasher, Transform, Vec3};

pub const SPIKE_N: usize = 2001;
pub const CELL_M: f64 = 1.0;
const PROP_CELL_M: f64 = 16.0; // const-ok: spike placeholder, the generator reads these from CourseDef
const MUD_BELOW_M: f64 = -12.0; // const-ok: spike placeholder for the drainage rule

// ---- noise: value noise from an integer hash, a pure function of (seed, lattice point) ----

fn mix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15); // const-ok: splitmix64 constants
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Uniform in [-1, 1), a pure function of the lattice point.
fn lattice(seed: u64, ix: i64, iz: i64) -> f64 {
    let h = mix64(seed ^ mix64(ix as u64) ^ mix64((iz as u64).rotate_left(32)));
    ((h >> 11) as f64) * (1.0 / 4_503_599_627_370_496.0) - 1.0 // const-ok: 2^-52 scaling of 53 bits to [0,2) then shift
}

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0) // const-ok: Perlin's quintic fade
}

pub fn value_noise(seed: u64, x: f64, z: f64) -> f64 {
    let (fx, fz) = (x.floor(), z.floor());
    let (u, v) = (fade(x - fx), fade(z - fz));
    let (ix, iz) = (fx as i64, fz as i64);
    let a = lattice(seed, ix, iz);
    let b = lattice(seed, ix + 1, iz);
    let c = lattice(seed, ix, iz + 1);
    let d = lattice(seed, ix + 1, iz + 1);
    let top = a + (b - a) * u;
    let bot = c + (d - c) * u;
    top + (bot - top) * v
}

/// Domain-warped fractal Brownian motion, normalised to roughly [-1, 1].
pub fn warped_fbm(seed: u64, x: f64, z: f64, base_wavelength_m: f64, warp_m: f64, octaves: u32) -> f64 {
    let f = 1.0 / base_wavelength_m;
    let wx = warp_m * value_noise(seed ^ 0x51, x * f, z * f);
    let wz = warp_m * value_noise(seed ^ 0xA7, x * f, z * f);
    let (px, pz) = ((x + wx) * f, (z + wz) * f);
    let (mut sum, mut amp, mut norm, mut freq) = (0.0, 1.0, 0.0, 1.0);
    for o in 0..octaves {
        sum += amp * value_noise(seed.wrapping_add(o as u64), px * freq, pz * freq);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// Micro-roughness: a pure function of position, so the same bump is there every time a wheel visits.
pub fn micro_roughness_m(seed: u64, x: f64, z: f64, rms_m: f64) -> f64 {
    rms_m * 1.7 * value_noise(seed ^ 0xB1, x * 2.0, z * 2.0) // const-ok: 2 cycles/m; 1.7 ~ 1/rms of uniform value noise
}

// ---- the world ----

pub struct GridWorld {
    n: usize,
    half_m: f64,
    prop_n: usize,
    y_range: (f64, f64),
    heights: Vec<f32>,
    splat: Vec<u8>,
    materials: MaterialTable,
    props: Vec<PropRef>,
    cell_start: Vec<u32>,
    cell_items: Vec<u32>,
}

fn prop_n_for(half_m: f64) -> usize {
    ((2.0 * half_m) / PROP_CELL_M).ceil() as usize + 1
}

impl GridWorld {
    /// A world from raw baked arrays: `n` x `n` heights and splat ids, centred on the origin (`CELL_M` spacing).
    pub fn from_arrays(n: usize, heights: Vec<f32>, splat: Vec<u8>, materials: MaterialTable) -> GridWorld {
        assert!(n >= 2 && heights.len() == n * n && splat.len() == n * n, "grid arrays must be n x n");
        let half_m = (n - 1) as f64 * CELL_M * 0.5;
        let prop_n = prop_n_for(half_m);
        let y_range = heights
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| (lo.min(v as f64), hi.max(v as f64)));
        GridWorld {
            n,
            half_m,
            prop_n,
            y_range,
            heights,
            splat,
            materials,
            props: Vec::new(),
            cell_start: vec![0; prop_n * prop_n + 1],
            cell_items: Vec::new(),
        }
    }

    /// The 2 km spike terrain (seed-driven fBm, test road stripe, low-ground mud rule, scattered trees and boxes).
    pub fn generate_spike(seed: u64) -> GridWorld {
        let n = SPIKE_N;
        let half = (n - 1) as f64 * 0.5;
        let mut heights = vec![0f32; n * n];
        let mut splat = vec![0u8; n * n];
        for j in 0..n {
            for i in 0..n {
                let (x, z) = (i as f64 - half, j as f64 - half);
                let h = 25.0 * warped_fbm(seed, x, z, 500.0, 120.0, 6); // const-ok: spike amplitude / wavelength
                heights[j * n + i] = h as f32;
                let road = (x - 0.3 * z).abs() < 4.0; // const-ok: spike stripe
                splat[j * n + i] = if road {
                    2
                } else if h < MUD_BELOW_M {
                    1
                } else {
                    0
                };
            }
        }
        let mut w = GridWorld::from_arrays(n, heights, splat, MaterialTable::default());
        w.scatter_props(seed);
        w
    }

    /// A prop-free spike-sized world from a height function (tests and benches).
    pub fn from_fn(h: impl Fn(f64, f64) -> f64) -> GridWorld {
        GridWorld::from_fn_sized(SPIKE_N, h)
    }

    pub fn from_fn_sized(n: usize, h: impl Fn(f64, f64) -> f64) -> GridWorld {
        let half = (n - 1) as f64 * 0.5;
        let mut heights = vec![0f32; n * n];
        for j in 0..n {
            for i in 0..n {
                heights[j * n + i] = h(i as f64 - half, j as f64 - half) as f32;
            }
        }
        GridWorld::from_arrays(n, heights, vec![0; n * n], MaterialTable::default())
    }

    pub fn n(&self) -> usize {
        self.n
    }

    pub fn props(&self) -> &[PropRef] {
        &self.props
    }

    /// Heights as f64, row-major (z rows), for exporters and plots.
    pub fn height_at_node(&self, i: usize, j: usize) -> f64 {
        self.at(i, j)
    }

    pub fn splat_at_node(&self, i: usize, j: usize) -> u8 {
        self.splat[j * self.n + i]
    }

    /// Plan position of grid node (i, j).
    pub fn node_xz(&self, i: usize, j: usize) -> (f64, f64) {
        (i as f64 * CELL_M - self.half_m, j as f64 * CELL_M - self.half_m)
    }

    pub fn add_prop(&mut self, p: PropRef) {
        self.props.push(p);
        self.rebuild_grid();
    }

    /// Replace all props at once (one grid rebuild).
    pub fn set_props(&mut self, props: Vec<PropRef>) {
        self.props = props;
        self.rebuild_grid();
    }

    fn scatter_props(&mut self, seed: u64) {
        let mut rng = Pcg32::derive(seed, &[1]);
        let mut props = Vec::new();
        for k in 0..20_000u32 {
            let (x, z) = (rng.range_f64(-990.0, 990.0), rng.range_f64(-990.0, 990.0)); // const-ok: spike placeholder, the generator reads these from CourseDef
            let y = self.height_m(x, z);
            props.push(PropRef {
                id: PropId(k),
                kind: PropKind::Tree,
                shape: PropShape::Cylinder { radius_m: rng.range_f64(0.15, 0.5), height_m: 8.0 }, // const-ok: spike placeholder, the generator reads these from CourseDef
                transform: Transform::from_pos(Vec3::new(x, y, z)),
                break_impulse_ns: 4.0e4, // const-ok: spike
            });
        }
        for k in 0..200u32 {
            let (x, z) = (rng.range_f64(-990.0, 990.0), rng.range_f64(-990.0, 990.0)); // const-ok: spike placeholder, the generator reads these from CourseDef
            let y = self.height_m(x, z) + 2.0;
            props.push(PropRef {
                id: PropId(20_000 + k),
                kind: PropKind::Building,
                shape: PropShape::Box { half_m: Vec3::new(5.0, 2.0, 4.0) }, // const-ok: spike placeholder, the generator reads these from CourseDef
                transform: Transform::new(Vec3::new(x, y, z), Quat::from_yaw(rng.range_f64(0.0, 3.0))),
                break_impulse_ns: f64::INFINITY,
            });
        }
        self.props = props;
        self.rebuild_grid();
    }

    /// CSR build: count, prefix-sum, fill. A prop sits in every grid cell its AABB overlaps.
    fn rebuild_grid(&mut self) {
        for p in &self.props {
            self.y_range.1 = self.y_range.1.max(prop_aabb(p).1.y);
        }
        let props = &self.props;
        let mut counts = vec![0u32; self.prop_n * self.prop_n + 1];
        for p in props {
            let (lo, hi) = prop_aabb(p);
            let ((i0, j0), (i1, j1)) =
                (prop_cell(self.half_m, self.prop_n, lo.x, lo.z), prop_cell(self.half_m, self.prop_n, hi.x, hi.z));
            for j in j0..=j1 {
                for i in i0..=i1 {
                    counts[j * self.prop_n + i] += 1;
                }
            }
        }
        let mut start = vec![0u32; self.prop_n * self.prop_n + 1];
        for c in 0..self.prop_n * self.prop_n {
            start[c + 1] = start[c] + counts[c];
        }
        let mut fill = start.clone();
        let mut items = vec![0u32; start[self.prop_n * self.prop_n] as usize];
        for (idx, p) in props.iter().enumerate() {
            let (lo, hi) = prop_aabb(p);
            let ((i0, j0), (i1, j1)) =
                (prop_cell(self.half_m, self.prop_n, lo.x, lo.z), prop_cell(self.half_m, self.prop_n, hi.x, hi.z));
            for j in j0..=j1 {
                for i in i0..=i1 {
                    let c = j * self.prop_n + i;
                    items[fill[c] as usize] = idx as u32;
                    fill[c] += 1;
                }
            }
        }
        self.cell_start = start;
        self.cell_items = items;
    }

    /// Hash of the whole baked output (heights by bit pattern, splat, props): the determinism check.
    pub fn content_hash(&self) -> u64 {
        let mut h = StateHasher::new();
        for &v in &self.heights {
            h.write_u32(v.to_bits());
        }
        h.write_bytes(&self.splat);
        for p in &self.props {
            h.write_u32(p.id.0);
            h.write_vec3(p.transform.pos);
        }
        h.finish()
    }

    fn at(&self, i: usize, j: usize) -> f64 {
        self.heights[j * self.n + i] as f64
    }

    /// Cell index and local coordinates (u, v in [0, 1]) of a plan position, clamped to the grid.
    fn locate(&self, x: f64, z: f64) -> (usize, usize, f64, f64) {
        let max = (self.n - 1) as f64; // const-ok: grid extent in cells
        let fx = ((x + self.half_m) / CELL_M).clamp(0.0, max);
        let fz = ((z + self.half_m) / CELL_M).clamp(0.0, max);
        let i = (fx.floor() as usize).min(self.n - 2);
        let j = (fz.floor() as usize).min(self.n - 2);
        (i, j, fx - i as f64, fz - j as f64)
    }

    /// Bilinear coefficients h = a + b u + c v + d u v of cell (i, j).
    fn patch(&self, i: usize, j: usize) -> (f64, f64, f64, f64) {
        let (h00, h10, h01, h11) = (self.at(i, j), self.at(i + 1, j), self.at(i, j + 1), self.at(i + 1, j + 1));
        (h00, h10 - h00, h01 - h00, h00 - h10 - h01 + h11)
    }

    /// Terrain part of the ray: DDA over cells, a quadratic per cell. `None` if nothing within `max_m`.
    fn raycast_terrain(&self, o: Vec3, d: Vec3, max_m: f64) -> Option<(f64, Vec3)> {
        // Start already below the surface: the hit is at the origin.
        if o.y < self.height_m(o.x, o.z) && self.in_plan(o.x, o.z) {
            return Some((0.0, self.normal(o.x, o.z)));
        }
        let mut result = None;
        grid_walk(o, d, max_m, CELL_M, -self.half_m, self.n - 1, &mut |i, j, t_in, t_out| {
            let t_in = t_in.max(0.0);
            let (a, b, c, dd) = self.patch(i, j);
            let (u0, v0) = (
                (o.x + d.x * t_in + self.half_m) / CELL_M - i as f64,
                (o.z + d.z * t_in + self.half_m) / CELL_M - j as f64,
            );
            let (ux, vz) = (d.x / CELL_M, d.z / CELL_M);
            let y0 = o.y + d.y * t_in;
            let qa = dd * ux * vz;
            let qb = b * ux + c * vz + dd * (u0 * vz + v0 * ux) - d.y;
            let qc = a + b * u0 + c * v0 + dd * u0 * v0 - y0;
            if let Some(s) = smallest_root(qa, qb, qc, t_out - t_in) {
                let t = t_in + s;
                result = Some((t, self.normal(o.x + d.x * t, o.z + d.z * t)));
                return true;
            }
            false
        });
        result
    }

    fn in_plan(&self, x: f64, z: f64) -> bool {
        x.abs() <= self.half_m && z.abs() <= self.half_m
    }
}

/// Smallest root s in [0, s_max] of qa s^2 + qb s + qc (qc <= 0 means the ray starts above the surface).
fn smallest_root(qa: f64, qb: f64, qc: f64, s_max: f64) -> Option<f64> {
    if qc > 0.0 {
        return Some(0.0);
    }
    let eps = 1e-12; // const-ok: degenerate-quadratic tolerance
    let mut best: Option<f64> = None;
    let mut consider = |s: f64| {
        if (0.0..=s_max).contains(&s) && best.is_none_or(|b| s < b) {
            best = Some(s);
        }
    };
    if qa.abs() < eps {
        if qb.abs() > eps {
            consider(-qc / qb);
        }
    } else {
        let disc = qb * qb - 4.0 * qa * qc;
        if disc >= 0.0 {
            let sq = disc.sqrt();
            let q = -0.5 * (qb + if qb >= 0.0 { sq } else { -sq });
            consider(q / qa);
            if q.abs() > eps {
                consider(qc / q);
            }
        }
    }
    best
}

/// Visit the grid cells (cell size `cell`, origin `org` on both axes, `n` cells per side) crossed by the ray's plan
/// projection in order of distance. `f(i, j, t_in, t_out)` returns true to stop.
fn grid_walk(
    o: Vec3,
    d: Vec3,
    max_m: f64,
    cell: f64,
    org: f64,
    n: usize,
    f: &mut dyn FnMut(usize, usize, f64, f64) -> bool,
) {
    let (mut t, inf) = (0.0f64, f64::INFINITY);
    // Enter the grid's plan box first (rays may start outside it).
    let (lo, hi) = (org, org + cell * n as f64);
    let mut t_enter = 0.0f64;
    let mut t_exit = max_m;
    for (p, dp) in [(o.x, d.x), (o.z, d.z)] {
        if dp.abs() < 1e-15 {
            // const-ok: parallel to the slab
            if p < lo || p > hi {
                return;
            }
        } else {
            let (t0, t1) = ((lo - p) / dp, (hi - p) / dp);
            t_enter = t_enter.max(t0.min(t1));
            t_exit = t_exit.min(t0.max(t1));
        }
    }
    if t_enter > t_exit {
        return;
    }
    t = t.max(t_enter);
    let (px, pz) = (o.x + d.x * t, o.z + d.z * t);
    let mut i = (((px - org) / cell).floor() as i64).clamp(0, n as i64 - 1);
    let mut j = (((pz - org) / cell).floor() as i64).clamp(0, n as i64 - 1);
    let step_i = if d.x >= 0.0 { 1 } else { -1 };
    let step_j = if d.z >= 0.0 { 1 } else { -1 };
    let next = |idx: i64, step: i64, p: f64, dp: f64| -> f64 {
        if dp.abs() < 1e-15 {
            // const-ok: parallel to the slab
            return inf;
        }
        let edge = org + cell * (idx + if step > 0 { 1 } else { 0 }) as f64;
        (edge - p) / dp
    };
    let (mut tx, mut tz) = (next(i, step_i, o.x, d.x), next(j, step_j, o.z, d.z));
    while t < t_exit {
        let t_next = tx.min(tz).min(t_exit);
        if f(i as usize, j as usize, t, t_next) {
            return;
        }
        t = t_next;
        if tx <= tz {
            i += step_i;
            tx = next(i, step_i, o.x, d.x);
        } else {
            j += step_j;
            tz = next(j, step_j, o.z, d.z);
        }
        if i < 0 || j < 0 || i >= n as i64 || j >= n as i64 {
            return;
        }
    }
}

fn prop_cell(half_m: f64, prop_n: usize, x: f64, z: f64) -> (usize, usize) {
    let c = |v: f64| (((v + half_m) / PROP_CELL_M).floor().max(0.0) as usize).min(prop_n - 1);
    (c(x), c(z))
}

fn prop_aabb(p: &PropRef) -> (Vec3, Vec3) {
    let c = p.transform.pos;
    match p.shape {
        PropShape::Sphere { radius_m } => (c - Vec3::splat(radius_m), c + Vec3::splat(radius_m)),
        PropShape::Cylinder { radius_m, height_m } => {
            (c + Vec3::new(-radius_m, 0.0, -radius_m), c + Vec3::new(radius_m, height_m, radius_m))
        }
        PropShape::Box { half_m } => {
            let r = p.transform.rot;
            let e = r.rotate(Vec3::X * half_m.x).abs()
                + r.rotate(Vec3::Y * half_m.y).abs()
                + r.rotate(Vec3::Z * half_m.z).abs();
            (c - e, c + e)
        }
    }
}

/// Ray against one prop shape; returns (distance, outward normal).
fn ray_prop(p: &PropRef, o: Vec3, d: Vec3, max_m: f64) -> Option<(f64, Vec3)> {
    let inv = p.transform.inverse();
    let (lo, ld) = (inv.apply_point(o), inv.apply_dir(d));
    let (t, n) = match p.shape {
        PropShape::Box { half_m } => {
            let (mut t0, mut t1, mut n0) = (0.0f64, max_m, Vec3::ZERO);
            for (k, (po, pd, h)) in
                [(lo.x, ld.x, half_m.x), (lo.y, ld.y, half_m.y), (lo.z, ld.z, half_m.z)].into_iter().enumerate()
            {
                if pd.abs() < 1e-15 {
                    // const-ok: parallel to the slab
                    if po.abs() > h {
                        return None;
                    }
                    continue;
                }
                let (a, b) = ((-h - po) / pd, (h - po) / pd);
                let (near, far, sign) = if a < b { (a, b, -1.0) } else { (b, a, 1.0) };
                if near > t0 {
                    t0 = near;
                    n0 = Vec3::ZERO;
                    match k {
                        0 => n0.x = sign,
                        1 => n0.y = sign,
                        _ => n0.z = sign,
                    }
                }
                t1 = t1.min(far);
                if t0 > t1 {
                    return None;
                }
            }
            (t0, n0)
        }
        PropShape::Sphere { radius_m } => {
            let b = lo.dot(ld);
            let disc = b * b - ld.length_sq() * (lo.length_sq() - radius_m * radius_m);
            if disc < 0.0 {
                return None;
            }
            let t = (-b - disc.sqrt()) / ld.length_sq();
            (t, (lo + ld * t) / radius_m)
        }
        PropShape::Cylinder { radius_m, height_m } => {
            // Side wall only (a tree trunk is hit from the side); the cap is the terrain/prop top and not needed by the spike.
            let a = ld.x * ld.x + ld.z * ld.z;
            if a < 1e-15 {
                // const-ok: ray parallel to the axis
                return None;
            }
            let b = lo.x * ld.x + lo.z * ld.z;
            let disc = b * b - a * (lo.x * lo.x + lo.z * lo.z - radius_m * radius_m);
            if disc < 0.0 {
                return None;
            }
            let t = (-b - disc.sqrt()) / a;
            let y = lo.y + ld.y * t;
            if !(0.0..=height_m).contains(&y) {
                return None;
            }
            (t, Vec3::new(lo.x + ld.x * t, 0.0, lo.z + ld.z * t) / radius_m)
        }
    };
    if (0.0..=max_m).contains(&t) {
        Some((t, p.transform.apply_dir(n)))
    } else {
        None
    }
}

impl WorldQuery for GridWorld {
    fn height_m(&self, x: f64, z: f64) -> f64 {
        let (i, j, u, v) = self.locate(x, z);
        let (a, b, c, d) = self.patch(i, j);
        a + b * u + c * v + d * u * v
    }

    fn normal(&self, x: f64, z: f64) -> Vec3 {
        let (i, j, u, v) = self.locate(x, z);
        let (_, b, c, d) = self.patch(i, j);
        let (gx, gz) = ((b + d * v) / CELL_M, (c + d * u) / CELL_M);
        Vec3::new(-gx, 1.0, -gz).normalized_or_zero()
    }

    fn material_id_at(&self, x: f64, z: f64) -> MaterialId {
        let max = (self.n - 1) as f64; // const-ok: grid extent in cells
        let i = (((x + self.half_m) / CELL_M).round().clamp(0.0, max)) as usize;
        let j = (((z + self.half_m) / CELL_M).round().clamp(0.0, max)) as usize;
        MaterialId(self.splat[j * self.n + i] as u16)
    }

    fn materials(&self) -> &MaterialTable {
        &self.materials
    }

    fn raycast(&self, origin: Vec3, dir: Vec3, max_m: f64) -> Option<RayHit> {
        let terrain = self.raycast_terrain(origin, dir, max_m);
        let limit = terrain.map_or(max_m, |(t, _)| t);
        let mut best: Option<(f64, Vec3, PropId)> = None;
        grid_walk(origin, dir, limit, PROP_CELL_M, -self.half_m, self.prop_n, &mut |i, j, _, t_out| {
            let c = j * self.prop_n + i;
            for &idx in &self.cell_items[self.cell_start[c] as usize..self.cell_start[c + 1] as usize] {
                let p = &self.props[idx as usize];
                if let Some((t, n)) = ray_prop(p, origin, dir, limit) {
                    if best.is_none_or(|b| t < b.0) {
                        best = Some((t, n, p.id));
                    }
                }
            }
            // Cells are visited front to back: a hit inside this cell cannot be beaten by a later cell.
            best.is_some_and(|b| b.0 <= t_out)
        });
        let (distance_m, normal, prop) = match (best, terrain) {
            (Some((t, n, id)), _) => (t, n, Some(id)),
            (None, Some((t, n))) => (t, n, None),
            (None, None) => return None,
        };
        let point = origin + dir * distance_m;
        Some(RayHit { distance_m, point, normal, material: self.material_id_at(point.x, point.z), prop })
    }

    fn props_in_aabb(&self, min: Vec3, max: Vec3, out: &mut Vec<PropRef>) {
        let ((i0, j0), (i1, j1)) =
            (prop_cell(self.half_m, self.prop_n, min.x, min.z), prop_cell(self.half_m, self.prop_n, max.x, max.z));
        for j in j0..=j1 {
            for i in i0..=i1 {
                let c = j * self.prop_n + i;
                for &idx in &self.cell_items[self.cell_start[c] as usize..self.cell_start[c + 1] as usize] {
                    let p = &self.props[idx as usize];
                    let (lo, hi) = prop_aabb(p);
                    // Report a prop from the first query cell it shares, so a prop spanning cells appears once.
                    let (pi, pj) = prop_cell(self.half_m, self.prop_n, lo.x, lo.z);
                    if pi.max(i0) != i || pj.max(j0) != j {
                        continue;
                    }
                    let overlap = lo.x <= max.x
                        && hi.x >= min.x
                        && lo.y <= max.y
                        && hi.y >= min.y
                        && lo.z <= max.z
                        && hi.z >= min.z;
                    if overlap {
                        out.push(*p);
                    }
                }
            }
        }
    }

    fn bounds(&self) -> (Vec3, Vec3) {
        // The y range is the lowest terrain to the highest terrain or prop top (CCR W-4 asks to write this into the contract).
        (Vec3::new(-self.half_m, self.y_range.0, -self.half_m), Vec3::new(self.half_m, self.y_range.1, self.half_m))
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods, clippy::float_cmp)] // exact float equality is the point of the determinism tests; the ignored bench reads a wall clock to report ns per query; it never feeds the simulation
mod tests {
    use super::*;
    use std::sync::OnceLock;

    fn world() -> &'static GridWorld {
        static W: OnceLock<GridWorld> = OnceLock::new();
        W.get_or_init(|| GridWorld::generate_spike(7))
    }

    #[test]
    fn flat_heightfield_returns_the_constant_height() {
        let w = GridWorld::from_fn(|_, _| 3.5);
        for (x, z) in [(0.0, 0.0), (123.4, -567.8), (-1000.0, 1000.0), (2000.0, 0.0)] {
            assert_eq!(w.height_m(x, z), 3.5);
            assert!((w.normal(x, z) - Vec3::Y).length() < 1e-15);
        }
    }

    #[test]
    fn bilinear_height_is_continuous_across_cell_boundaries() {
        let w = world();
        let eps = 1e-9;
        for k in 0..2000 {
            let (x, z) = (-900.0 + k as f64 * 0.37, 100.0 + k as f64 * 0.11);
            let (bx, bz) = (x.round(), z.round()); // a lattice line in each axis
            assert!((w.height_m(bx - eps, z) - w.height_m(bx + eps, z)).abs() < 1e-6);
            assert!((w.height_m(x, bz - eps) - w.height_m(x, bz + eps)).abs() < 1e-6);
        }
    }

    #[test]
    fn normal_matches_the_finite_difference_gradient() {
        let w = world();
        let h = 1e-4;
        for k in 0..500 {
            let (x, z) = (-800.0 + k as f64 * 3.0 + 0.3 + 0.1 * (k % 5) as f64, 200.0 - k as f64 * 2.0 + 0.6); // cell interiors
            let gx = (w.height_m(x + h, z) - w.height_m(x - h, z)) / (2.0 * h);
            let gz = (w.height_m(x, z + h) - w.height_m(x, z - h)) / (2.0 * h);
            let n = Vec3::new(-gx, 1.0, -gz).normalized_or_zero();
            assert!((w.normal(x, z) - n).length() < 1e-5, "k={k}");
        }
    }

    #[test]
    fn planar_ramp_has_exactly_its_slope() {
        let w = GridWorld::from_fn(|x, _| 0.25 * x);
        let n = w.normal(10.3, -40.7);
        assert!((n.x / n.y + 0.25).abs() < 1e-6 && n.z.abs() < 1e-12);
        assert!((w.height_m(100.25, 5.0) - 25.0625).abs() < 1e-5);
    }

    #[test]
    fn raycast_hits_the_analytic_plane_at_the_expected_distance() {
        let w = GridWorld::from_fn(|x, _| 0.25 * x); // plane y = 0.25 x
        let o = Vec3::new(-300.0, 50.0, 20.0);
        let d = Vec3::new(0.6, -0.8, 0.0);
        // o.y + d.y t = 0.25 (o.x + d.x t)  =>  t = (o.y - 0.25 o.x) / (0.25 d.x - d.y)
        let t = (50.0 + 75.0) / (0.15 + 0.8);
        let hit = w.raycast(o, d, 1000.0).expect("hit");
        assert!((hit.distance_m - t).abs() < 1e-4, "{} vs {t}", hit.distance_m);
        assert!(hit.prop.is_none());
    }

    #[test]
    fn raycast_on_rough_terrain_lands_on_the_surface() {
        let w = world();
        let mut rng = Pcg32::new(3, 3);
        for _ in 0..300 {
            let o = Vec3::new(rng.range_f64(-800.0, 800.0), 200.0, rng.range_f64(-800.0, 800.0));
            let d = Vec3::new(rng.range_f64(-0.5, 0.5), -1.0, rng.range_f64(-0.5, 0.5)).normalized_or_zero();
            let hit = w.raycast(o, d, 1000.0);
            if let Some(h) = hit.filter(|h| h.prop.is_none()) {
                assert!((h.point.y - w.height_m(h.point.x, h.point.z)).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn raycast_through_a_box_prop_hits_the_face() {
        let mut w = GridWorld::from_fn(|_, _| 0.0);
        w.add_prop(PropRef {
            id: PropId(1),
            kind: PropKind::Building,
            shape: PropShape::Box { half_m: Vec3::new(2.0, 2.0, 3.0) },
            transform: Transform::from_pos(Vec3::new(50.0, 2.0, 0.0)),
            break_impulse_ns: f64::INFINITY,
        });
        let hit = w.raycast(Vec3::new(0.0, 1.0, 0.0), Vec3::X, 200.0).expect("hit");
        assert!((hit.distance_m - 48.0).abs() < 1e-9);
        assert!((hit.normal - (-Vec3::X)).length() < 1e-12);
        assert_eq!(hit.prop, Some(PropId(1)));
        let mut out = Vec::new();
        w.props_in_aabb(Vec3::new(40.0, 0.0, -5.0), Vec3::new(60.0, 5.0, 5.0), &mut out);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn generator_is_deterministic_for_a_seed() {
        let (a, b, c) = (
            GridWorld::generate_spike(7).content_hash(),
            world().content_hash(),
            GridWorld::generate_spike(8).content_hash(),
        );
        assert_eq!(a, b);
        assert_ne!(a, c);
        // Bit-identical on every platform: this constant is checked on Linux and Windows CI.
        assert_eq!(a, GOLDEN_HASH_SEED_7, "hash was {a:#018x}");
    }

    #[test]
    fn micro_roughness_is_a_pure_function_of_position() {
        assert_eq!(micro_roughness_m(1, 3.3, 4.4, 0.02), micro_roughness_m(1, 3.3, 4.4, 0.02));
    }

    /// Report: `cargo test -p w5k_world --release -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn query_cost_is_under_100_ns_per_height_lookup() {
        use std::hint::black_box;
        use std::time::Instant;
        let w = world();
        let mut rng = Pcg32::new(9, 9);
        let pts: Vec<(f64, f64)> =
            (0..1 << 16).map(|_| (rng.range_f64(-1000.0, 1000.0), rng.range_f64(-1000.0, 1000.0))).collect();
        let reps = 200;
        let t0 = Instant::now();
        let mut acc = 0.0;
        for _ in 0..reps {
            for &(x, z) in &pts {
                acc += w.height_m(black_box(x), black_box(z));
            }
        }
        let h_ns = t0.elapsed().as_nanos() as f64 / (reps * pts.len()) as f64;
        let t0 = Instant::now();
        for _ in 0..reps {
            for &(x, z) in &pts {
                let s = w.sample(black_box(x), black_box(z));
                acc += s.normal.y + s.material.0 as f64;
            }
        }
        let s_ns = t0.elapsed().as_nanos() as f64 / (reps * pts.len()) as f64;
        let rays: Vec<(Vec3, Vec3)> = pts
            .iter()
            .take(1 << 12)
            .map(|&(x, z)| (Vec3::new(x, 30.0, z), Vec3::new(0.8, -0.05, 0.6).normalized_or_zero()))
            .collect();
        let t0 = Instant::now();
        let mut hits = 0;
        for &(o, d) in &rays {
            hits += black_box(w.raycast(black_box(o), black_box(d), 200.0)).is_some() as u32;
        }
        let r_ns = t0.elapsed().as_nanos() as f64 / rays.len() as f64;
        let mut out = Vec::with_capacity(64);
        let t0 = Instant::now();
        for &(x, z) in pts.iter().take(1 << 14) {
            out.clear();
            w.props_in_aabb(Vec3::new(x - 3.0, -50.0, z - 3.0), Vec3::new(x + 3.0, 50.0, z + 3.0), &mut out);
            acc += out.len() as f64;
        }
        let p_ns = t0.elapsed().as_nanos() as f64 / (1 << 14) as f64;
        println!("S-W: height {h_ns:.1} ns | sample(height+normal+material) {s_ns:.1} ns | raycast(200 m) {r_ns:.0} ns ({hits} hits) | props_in_aabb(6 m) {p_ns:.0} ns | acc {acc:.1}");
        assert!(h_ns < 100.0, "height lookup {h_ns} ns");
    }

    /// Report: slopes of the spike terrain and how far a bilinear cell strays from its two-triangle mesh.
    #[test]
    #[ignore]
    fn spike_terrain_statistics() {
        let w = world();
        let (mut max_grade, mut max_dev, mut hist) = (0.0f64, 0.0f64, [0u32; 6]);
        for j in (0..SPIKE_N - 1).step_by(3) {
            for i in (0..SPIKE_N - 1).step_by(3) {
                let (_, b, c, d) = w.patch(i, j);
                let g = (b * b + c * c).sqrt();
                max_grade = max_grade.max(g);
                max_dev = max_dev.max(d.abs() / 4.0);
                hist[((g * 20.0) as usize).min(5)] += 1;
            }
        }
        println!("S-W terrain: max grade {max_grade:.3} | max bilinear-vs-triangle deviation {max_dev:.4} m | grade histogram (0.05 bins) {hist:?}");
    }

    /// Writes a 500 x 500 hill-shaded top-down map as a binary PPM to $S_W_PPM (docs image; converted to PNG by a script).
    #[test]
    #[ignore]
    fn spike_top_down_map() {
        let Ok(path) = std::env::var("S_W_PPM") else { return };
        let w = world();
        let light = Vec3::new(-0.5, 0.8, -0.4).normalized_or_zero();
        let mut buf = b"P6\n500 500\n255\n".to_vec();
        for r in 0..500 {
            for c in 0..500 {
                let (x, z) = (c as f64 * 4.0 - 998.0, r as f64 * 4.0 - 998.0);
                let shade = (w.normal(x, z).dot(light) * 0.8 + 0.2).clamp(0.0, 1.0);
                let base = match w.material_id_at(x, z).0 {
                    1 => [110.0, 85.0, 55.0],
                    2 => [160.0, 160.0, 165.0],
                    _ => [90.0 + (w.height_m(x, z) + 25.0) * 1.5, 140.0, 80.0],
                };
                buf.extend(base.iter().map(|b| (b * shade) as u8));
            }
        }
        std::fs::write(path, buf).expect("write");
    }

    const GOLDEN_HASH_SEED_7: u64 = 0x754c_32db_53b5_882a;
}
