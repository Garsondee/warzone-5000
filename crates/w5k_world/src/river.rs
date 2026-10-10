//! A river: a meandering channel cut into a levelled valley, with banks that climb out at a stated grade, optional fords (a raised
//! stretch of bed that can be waded) and a water surface that is flat across the channel and falls gently downstream.
//!
//! The ground is a heightfield, so the river is *ground below a water level*: the channel bed is carved, and the water is a surface
//! height stored per node (`GridWorld::water_surface_m`). There is no flow simulation (NOT-MODELLED): the surface is a plane across
//! the channel and a straight slope along it, which is all a fording or swimming vehicle model needs.
//!
//! Cross-section (distance `d` from the centreline): the bed is a trapezoid, `S - min(depth, (w/2 - d) * bank_grade)`, reaching the water
//! surface `S` at the waterline `d = w/2`; the bank then climbs at `bank_grade` to the floodplain `S + freeboard`, which is flat out to
//! `valley_half_m` and blends into the natural ground over `valley_blend_m`.

use serde::{Deserialize, Serialize};
use w5k_contract::param::Param;
use w5k_math::scalar;

use crate::grid::{value_noise, CELL_M};

/// A wadable stretch: the bed rises to `depth_m` over `length_m` of the river, centred `at_fraction` of the way along it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FordDef {
    pub at_fraction: Param,
    pub length_m: Param,
    pub depth_m: Param,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiverDef {
    /// Where the river enters and leaves, plan `(x, z)`, m.
    pub start: (f64, f64),
    pub end: (f64, f64),
    pub meander_amplitude_m: Param,
    pub meander_wavelength_m: Param,
    /// Width of the water at the surface, m.
    pub width_m: Param,
    /// Depth of the deepest part of the channel, m.
    pub depth_m: Param,
    /// Slope of the banks, rise over run: what a vehicle must climb to leave the water.
    pub bank_grade: Param,
    /// Height of the floodplain above the water, m.
    pub freeboard_m: Param,
    pub valley_half_m: Param,
    pub valley_blend_m: Param,
    /// Fall of the water surface per metre of river.
    pub surface_slope: Param,
    #[serde(default)]
    pub fords: Vec<FordDef>,
}

/// Salt separating the meander noise from the hills noise.
const RIVER_SALT: u64 = 0x21FE; // const-ok: noise stream label
/// The meander fades to the straight line over this distance at each end, m.
const END_TAPER_M: f64 = 60.0; // const-ok: where the river meets the map edge
/// The bank grade limit applies this far beyond the top of the bank, m.
const BANK_ZONE_MARGIN_M: f64 = 1.0; // const-ok: width of the relaxed zone around a bank
/// Length over which the bed rises into a ford, m.
const FORD_RAMP_M: f64 = 8.0; // const-ok: shape of a ford's approaches

impl RiverDef {
    pub fn check(&self, name: &str) -> Result<(), String> {
        for (l, p) in [
            ("meander_amplitude_m", &self.meander_amplitude_m),
            ("meander_wavelength_m", &self.meander_wavelength_m),
            ("width_m", &self.width_m),
            ("depth_m", &self.depth_m),
            ("bank_grade", &self.bank_grade),
            ("freeboard_m", &self.freeboard_m),
            ("valley_half_m", &self.valley_half_m),
            ("valley_blend_m", &self.valley_blend_m),
            ("surface_slope", &self.surface_slope),
        ] {
            p.check(&format!("{name}.{l}"))?;
        }
        let min_width = 2.0 * self.depth_m.v / self.bank_grade.v;
        if self.width_m.v < min_width {
            return Err(format!("{name}: a {} m deep channel with banks at grade {} needs water at least {min_width:.1} m wide (it is {})", self.depth_m.v, self.bank_grade.v, self.width_m.v));
        }
        let bank_run = self.freeboard_m.v / self.bank_grade.v;
        let need = self.width_m.v * 0.5 + bank_run;
        if self.valley_half_m.v < need {
            return Err(format!(
                "{name}: valley_half_m {} is narrower than the channel and its banks ({need:.1} m)",
                self.valley_half_m.v
            ));
        }
        for (k, f) in self.fords.iter().enumerate() {
            f.at_fraction.check(&format!("{name}.fords[{k}].at_fraction"))?;
            f.length_m.check(&format!("{name}.fords[{k}].length_m"))?;
            f.depth_m.check(&format!("{name}.fords[{k}].depth_m"))?;
            if f.depth_m.v >= self.depth_m.v {
                return Err(format!("{name}.fords[{k}]: a ford must be shallower than the channel"));
            }
        }
        Ok(())
    }

    /// The centreline, one point per metre of arc: `(x, z, arc_m)`.
    pub fn centreline(&self, seed: u64) -> Vec<(f64, f64, f64)> {
        let (dx, dz) = (self.end.0 - self.start.0, self.end.1 - self.start.1);
        let len = scalar::hypot(dx, dz);
        let (tx, tz) = (dx / len, dz / len);
        let steps = (len / CELL_M).ceil() as usize;
        let mut pts: Vec<(f64, f64, f64)> = Vec::with_capacity(steps + 1);
        let mut arc = 0.0;
        for k in 0..=steps {
            let s = len * k as f64 / steps as f64;
            let taper = scalar::smoothstep(0.0, END_TAPER_M, s) * scalar::smoothstep(0.0, END_TAPER_M, len - s);
            let off = self.meander_amplitude_m.v
                * taper
                * value_noise(seed ^ RIVER_SALT, s / self.meander_wavelength_m.v, 0.0);
            let p = (self.start.0 + tx * s - tz * off, self.start.1 + tz * s + tx * off);
            if let Some(&(px, pz, _)) = pts.last() {
                arc += scalar::hypot(p.0 - px, p.1 - pz);
            }
            pts.push((p.0, p.1, arc));
        }
        pts
    }

    /// Depth of the channel at river distance `arc_m` (the full depth except in a ford).
    pub fn depth_at(&self, arc_m: f64, total_m: f64) -> f64 {
        let mut d = self.depth_m.v;
        for f in &self.fords {
            let centre = f.at_fraction.v * total_m;
            let half = f.length_m.v * 0.5;
            let w = 1.0 - scalar::smoothstep(half, half + FORD_RAMP_M, (arc_m - centre).abs());
            d = d.min(self.depth_m.v - (self.depth_m.v - f.depth_m.v) * w);
        }
        d
    }

    /// Ground height at distance `d_m` from the centreline given the water level `s_m` and the local channel depth.
    pub fn ground(&self, d_m: f64, s_m: f64, depth_m: f64) -> f64 {
        let (half, g) = (self.width_m.v * 0.5, self.bank_grade.v);
        if d_m <= half {
            s_m - depth_m.min((half - d_m) * g)
        } else if d_m <= half + self.freeboard_m.v / g {
            s_m + (d_m - half) * g
        } else {
            s_m + self.freeboard_m.v
        }
    }

    /// Distance from the centreline out to which the river shapes the ground, m.
    pub fn reach_m(&self) -> f64 {
        self.valley_half_m.v + self.valley_blend_m.v
    }

    /// Distance from the centreline to the top of the bank, m.
    pub fn bank_top_m(&self) -> f64 {
        self.width_m.v * 0.5 + self.freeboard_m.v / self.bank_grade.v
    }
}

/// What carving left behind: the water level at every node the river shapes (`NAN` elsewhere).
pub struct Carved {
    pub surface: Vec<f64>,
    /// Nodes on the channel or its banks (up to the bank top plus a metre): where the bank grade, not the hills limit, applies.
    pub bank_zone: Vec<bool>,
    /// Length of the centreline, m.
    pub total_m: f64,
}

/// Cut the river into `h` (n x n nodes, centred, `CELL_M` apart): flatten the valley, carve the channel, blend into the ground.
pub fn carve(def: &RiverDef, seed: u64, h: &mut [f64], n: usize) -> Carved {
    let half = (n - 1) as f64 * 0.5 * CELL_M;
    let line = def.centreline(seed);
    let total = line[line.len() - 1].2;
    // The water level at the start: the valley floor sits at the mean height of the ground along the river.
    let mean = line
        .iter()
        .map(|p| {
            let (i, j) = (
                ((p.0 + half) / CELL_M).round().clamp(0.0, (n - 1) as f64) as usize,
                ((p.1 + half) / CELL_M).round().clamp(0.0, (n - 1) as f64) as usize,
            );
            h[j * n + i]
        })
        .sum::<f64>()
        / line.len() as f64;
    let s0 = mean - def.freeboard_m.v + def.surface_slope.v * total * 0.5;
    let reach = def.reach_m();
    let r_cells = (reach / CELL_M).ceil() as i64;
    let mut best_d = vec![f64::INFINITY; n * n];
    let mut best_arc = vec![0.0; n * n];
    for w in line.windows(2) {
        let ((ax, az, a_arc), (bx, bz, b_arc)) = (w[0], w[1]);
        let (gax, gaz, gbx, gbz) =
            ((ax + half) / CELL_M, (az + half) / CELL_M, (bx + half) / CELL_M, (bz + half) / CELL_M);
        let len2 = (gbx - gax) * (gbx - gax) + (gbz - gaz) * (gbz - gaz);
        for j in
            (gaz.min(gbz).floor() as i64 - r_cells).max(0)..=(gaz.max(gbz).ceil() as i64 + r_cells).min(n as i64 - 1)
        {
            for i in (gax.min(gbx).floor() as i64 - r_cells).max(0)
                ..=(gax.max(gbx).ceil() as i64 + r_cells).min(n as i64 - 1)
            {
                let t = (((i as f64 - gax) * (gbx - gax) + (j as f64 - gaz) * (gbz - gaz)) / len2).clamp(0.0, 1.0);
                let d = scalar::hypot(i as f64 - (gax + t * (gbx - gax)), j as f64 - (gaz + t * (gbz - gaz))) * CELL_M;
                let c = j as usize * n + i as usize;
                if d < best_d[c] {
                    best_d[c] = d;
                    best_arc[c] = a_arc + t * (b_arc - a_arc);
                }
            }
        }
    }
    let mut surface = vec![f64::NAN; n * n];
    let mut bank_zone = vec![false; n * n];
    let top = def.bank_top_m();
    for c in 0..n * n {
        let d = best_d[c];
        if d >= reach {
            continue;
        }
        let s = s0 - def.surface_slope.v * best_arc[c];
        let g = def.ground(d, s, def.depth_at(best_arc[c], total));
        let vw = 1.0 - scalar::smoothstep(def.valley_half_m.v, reach, d);
        h[c] += (g - h[c]) * vw;
        if d <= top {
            surface[c] = s;
        }
        bank_zone[c] = d <= top + BANK_ZONE_MARGIN_M;
    }
    Carved { surface, bank_zone, total_m: total }
}
