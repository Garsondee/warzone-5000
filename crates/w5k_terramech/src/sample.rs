//! One contact sample of a track: a massless patch of belt on the ground (a [`ContactElement`]).
//!
//! Vertical: the wheel and belt in series with the soil. The sample is handed the *geometric* penetration `delta` of the belt into the undeformed
//! ground; it splits into soil sinkage `z` and deflection `delta - z` of the wheel contact, which carries the same force as the soil:
//! `k_s (delta - z) = A k z^n`. Horizontal: a shear *vector* displacement `j` (Janosi-Hanamoto) that is carried along the footprint by the belt
//! (an upwind advection from the neighbouring sample, set by the running gear) and grows with the slip velocity `u = v_hull - v_belt`.
//! Compaction resistance is not here: it needs the rut ahead, which only the gear knows (see [`TrackSample::compaction_force_n`]).

use w5k_contract::{ContactElement, ContactInput, ContactOutput, Material, SoilParams};
use w5k_math::scalar;

use crate::soil;
use crate::tuning::Tuning;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SampleGeom {
    /// Track width `b`, m.
    pub width_m: f64,
    /// Length of belt this sample stands for, m.
    pub length_m: f64,
    /// Vertical stiffness of the wheel contact in series with the soil, N/m (this sample's share).
    pub stiffness_n_m: f64,
    pub damping_ns_m: f64,
    /// `TrackDef::shoe_mu_scale` and `shoe_mu_scale_soft`.
    pub shoe_mu_scale: f64,
    pub shoe_mu_scale_soft: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct TrackSample {
    pub geom: SampleGeom,
    tuning: Tuning,
    jx_m: f64,
    jy_m: f64,
    inflow_x_m: f64,
    inflow_y_m: f64,
    sinkage_m: f64,
}

impl TrackSample {
    pub fn new(geom: SampleGeom, tuning: Tuning) -> TrackSample {
        TrackSample { geom, tuning, jx_m: 0.0, jy_m: 0.0, inflow_x_m: 0.0, inflow_y_m: 0.0, sinkage_m: 0.0 }
    }

    /// The shear displacement of the neighbouring sample the shoe is arriving from (zero at the ends of the footprint).
    pub fn set_inflow_shear_m(&mut self, jx_m: f64, jy_m: f64) {
        self.inflow_x_m = jx_m;
        self.inflow_y_m = jy_m;
    }

    pub fn shear_displacement_m(&self) -> (f64, f64) {
        (self.jx_m, self.jy_m)
    }

    pub fn sinkage_m(&self) -> f64 {
        self.sinkage_m
    }

    /// Compaction resistance of this sample deepening the rut from `z_ahead_m` (what the sample ahead left) to its own sinkage, N (>= 0).
    pub fn compaction_force_n(&self, ground: &Material, z_ahead_m: f64) -> f64 {
        match &ground.soil {
            // `b * integral p dz` is the work per metre of travel for the whole width; this sample's share of the track's one rut is all of it
            // at the leading sample and the extra depth only elsewhere (the gear passes `z_ahead_m`).
            Some(s) => soil::compaction_resistance_n(s, self.geom.width_m, z_ahead_m, self.sinkage_m),
            None => 0.0,
        }
    }

    /// Shear strength `tau_max` (Pa) and displacement scale `K` (m) for this ground at normal pressure `p`.
    fn shear_limits(&self, ground: &Material, p_pa: f64) -> (f64, f64) {
        match &ground.soil {
            Some(s) => {
                let scale = self.geom.shoe_mu_scale_soft.unwrap_or(1.0); // const-ok: no grouser advantage unless declared
                (soil::shear_strength_pa(s, p_pa) * scale, s.shear_k_m)
            }
            None => (ground.mu_peak * self.geom.shoe_mu_scale * p_pa.max(0.0), self.tuning.firm_shear_k_m),
        }
    }
}

/// Solve `k_s (delta - z) = a k z^n` for `z` in `[0, delta]` (bracketed Newton, fixed iteration cap: deterministic).
fn series_sinkage_m(ks_n_m: f64, area_m2: f64, soil: &SoilParams, b_m: f64, delta_m: f64) -> f64 {
    let ak = area_m2 * soil::modulus_pa_m_n(soil, b_m);
    let (mut lo, mut hi) = (0.0, delta_m);
    let mut z = 0.5 * delta_m; // const-ok: bracket midpoint
    for _ in 0..40 {
        // const-ok: iteration cap; the bracket halves at worst, 2^-40 of the penetration
        let zn = scalar::pow(z, soil.n);
        let g = ks_n_m * (delta_m - z) - ak * zn;
        if g > 0.0 {
            lo = z;
        } else {
            hi = z;
        }
        if hi - lo <= 1e-13 * delta_m {
            // const-ok: convergence tolerance
            break;
        }
        let dg = -ks_n_m - ak * soil.n * zn / z;
        let zt = z - g / dg;
        z = if zt > lo && zt < hi { zt } else { 0.5 * (lo + hi) }; // const-ok: bisect when Newton leaves the bracket
    }
    z
}

impl ContactElement for TrackSample {
    fn step(&mut self, input: &ContactInput) -> ContactOutput {
        let g = &self.geom;
        let t = &self.tuning;
        let delta = input.penetration_m;
        if delta <= 0.0 {
            self.reset();
            return ContactOutput::default();
        }
        let area = g.width_m * g.length_m;

        // Vertical: soil and wheel contact in series.
        let (z, f_spring) = match &input.ground.soil {
            Some(s) => {
                let z = series_sinkage_m(g.stiffness_n_m, area, s, g.width_m, delta);
                (z, g.stiffness_n_m * (delta - z))
            }
            None => (0.0, g.stiffness_n_m * delta),
        };
        let fz = (f_spring + g.damping_ns_m * input.penetration_rate_m_s).max(0.0);
        self.sinkage_m = z;
        let p = fz / area;

        // Horizontal: shear displacement carried by the belt and grown by the slip velocity.
        let (tau_max, k) = self.shear_limits(input.ground, p);
        let ux = input.vel_long_m_s - input.surface_speed_m_s;
        let uy = input.vel_lat_m_s;
        let a = input.dt_s * input.surface_speed_m_s.abs() / g.length_m;
        let cap = t.shear_cap_k * k;
        let mut jx = (self.jx_m + input.dt_s * ux + a * self.inflow_x_m) / (1.0 + a);
        let mut jy = (self.jy_m + input.dt_s * uy + a * self.inflow_y_m) / (1.0 + a);
        let jn = scalar::hypot(jx, jy);
        if jn > cap {
            jx *= cap / jn;
            jy *= cap / jn;
        }
        self.jx_m = scalar::flush_tiny(jx);
        self.jy_m = scalar::flush_tiny(jy);
        // The state is the displacement at the sample's exit face; the stress belongs to the cell centre, half a cell upstream. With a belt
        // moving (advection dominates) take the mean of entry and exit faces, with a stopped belt the sample's own value.
        let theta = input.surface_speed_m_s.abs() / (input.surface_speed_m_s.abs() + t.damping_speed_m_s);
        let jcx = self.jx_m - 0.5 * theta * (self.jx_m - self.inflow_x_m); // const-ok: cell-centre mean
        let jcy = self.jy_m - 0.5 * theta * (self.jy_m - self.inflow_y_m); // const-ok: cell-centre mean
        let s = scalar::hypot(jcx, jcy) / k;
        let spring = tau_max * soil::saturation_over_s(s) / k;
        let (mut tx, mut ty) = (spring * jcx, spring * jcy);
        // Creep damping: a viscous stress sized from the shear stiffness so that a parked tank is critically damped, fading out above u_c.
        let un = scalar::hypot(ux, uy);
        if p > 0.0 && tau_max > 0.0 {
            let mu_eff = tau_max / p;
            let kappa = 2.0 * t.damping_ratio * scalar::sqrt(mu_eff / (t.gravity_m_s2 * k)); // const-ok: critical damping 2 zeta sqrt(k m)
            let fade = 1.0 / (1.0 + (un / t.damping_speed_m_s) * (un / t.damping_speed_m_s));
            tx += p * kappa * ux * fade;
            ty += p * kappa * uy * fade;
        }
        let tn = scalar::hypot(tx, ty);
        if tn > tau_max {
            let r = tau_max / tn;
            tx *= r;
            ty *= r;
        }
        let mobilisation = if tau_max > 0.0 { scalar::hypot(tx, ty) / tau_max } else { 0.0 };
        let (fx, fy) = (-tx * area, -ty * area);

        let vb = input.surface_speed_m_s;
        let slip = if vb.abs() > 1e-6 { scalar::clamp(1.0 - input.vel_long_m_s / vb, -1.0, 1.0) } else { 0.0 }; // const-ok: belt-stopped guard
        ContactOutput {
            fx_n: fx,
            fy_n: fy,
            fz_n: fz,
            mz_nm: 0.0,
            shaft_reaction_nm: 0.0,
            sinkage_m: z,
            slip_ratio: slip,
            slip_angle_rad: scalar::atan2(input.vel_lat_m_s, input.vel_long_m_s.abs().max(1e-6)), // const-ok: atan2 guard
            saturated: mobilisation > 0.95, // const-ok: reporting threshold
        }
    }

    fn reset(&mut self) {
        self.jx_m = 0.0;
        self.jy_m = 0.0;
        self.inflow_x_m = 0.0;
        self.inflow_y_m = 0.0;
        self.sinkage_m = 0.0;
    }
}
