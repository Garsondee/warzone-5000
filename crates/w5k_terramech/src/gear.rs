//! The running gear of one track: a row of [`TrackSample`]s along the ground run, coupled to the road wheels (loads and penetrations) and to the
//! sprocket (belt speed in, shaft reaction out).
//!
//! * **Wheels to samples.** The glue gives the penetration of each road wheel's belt bottom into the undeformed ground. A sample's penetration is
//!   the linear interpolation between its two neighbouring wheels minus the sag of the belt bridging the span (`m' g s^2 / 8T`, from the belt
//!   weight and tension), so pressure peaks under the wheels on firm ground and evens out as the wheels sink. The force on each wheel is the same
//!   interpolation weights applied to the sample forces (partition of unity: Newton's third law holds exactly).
//! * **Belt to ground.** All samples share the belt speed `omega r`; shear displacement is passed from sample to sample against the belt motion.
//! * **Compaction.** The rut: each sample's extra sinkage beyond the sample ahead, in the direction of travel, costs `b integral p dz`.
//! * **Sprocket.** Reaction torque = sprocket radius x (total shear thrust + internal running resistance); compaction is a hull load, not a belt one.
//!
//! Frame: x along the track, forward positive, origin at the contact centre; y left. The glue maps it to the hull.

use w5k_contract::rig::TrackDef;
use w5k_contract::{ContactElement, ContactInput, ContactOutput, Material};

use crate::sample::{SampleGeom, TrackSample};
use crate::tuning::Tuning;

#[derive(Clone, Debug, PartialEq)]
pub struct GearConfig {
    pub width_m: f64,
    pub contact_length_m: f64,
    pub samples: usize,
    /// Road-wheel positions along the ground run, ascending (rear to front), relative to the contact centre, m.
    pub wheel_x_m: Vec<f64>,
    pub sprocket_radius_m: f64,
    pub wheel_stiffness_n_m: f64,
    pub wheel_damping_ns_m: f64,
    pub mass_per_m_kg: f64,
    pub tension_n: f64,
    pub shoe_mu_scale: f64,
    pub shoe_mu_scale_soft: Option<f64>,
    pub resist_c0: f64,
    pub resist_c1_s_m: f64,
}

impl GearConfig {
    /// From a rig's `TrackDef`; the glue supplies the road-wheel positions and the sprocket's pitch radius.
    pub fn from_track_def(def: &TrackDef, wheel_x_m: Vec<f64>, sprocket_radius_m: f64) -> GearConfig {
        GearConfig {
            width_m: def.width_m,
            contact_length_m: def.contact_length_m,
            samples: usize::from(def.samples),
            wheel_x_m,
            sprocket_radius_m,
            wheel_stiffness_n_m: def.wheel_contact.vertical_stiffness_n_m,
            wheel_damping_ns_m: def.wheel_contact.vertical_damping_ns_m,
            mass_per_m_kg: def.mass_per_m_kg,
            tension_n: def.tension_n,
            shoe_mu_scale: def.shoe_mu_scale,
            shoe_mu_scale_soft: def.shoe_mu_scale_soft,
            resist_c0: def.resist_c0,
            resist_c1_s_m: def.resist_c1_s_m,
        }
    }
}

/// How one sample reads the wheels: `penetration = (1 - t) d[lo] + t d[lo + 1] - sag`, and its force goes back by the same weights.
#[derive(Clone, Copy, Debug)]
struct Link {
    lo: usize,
    t: f64,
    sag_m: f64,
}

pub struct GearInput<'a> {
    /// Penetration of each road wheel's belt bottom into the undeformed ground, m (negative = clear of the ground).
    pub wheel_penetration_m: &'a [f64],
    pub wheel_penetration_rate_m_s: &'a [f64],
    /// Hull velocity at the contact centre in the track frame, m/s, and the yaw rate (about +up), rad/s.
    pub vel_long_m_s: f64,
    pub vel_lat_m_s: f64,
    pub yaw_rate_rad_s: f64,
    pub sprocket_omega_rad_s: f64,
    /// The surface under each sample.
    pub ground: &'a [&'a Material],
    pub dt_s: f64,
}

/// Sums over the whole track for one step, in the track frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GearTotals {
    pub fx_n: f64,
    pub fy_n: f64,
    pub fz_n: f64,
    /// Moment of the forces about the contact centre (about +up): `sum(x fy)`, N m.
    pub mz_nm: f64,
    /// Torque the sprocket must overcome, N m (positive resists forward rolling).
    pub shaft_reaction_nm: f64,
    pub shear_thrust_n: f64,
    pub compaction_n: f64,
    /// Deepest sinkage along the track, m.
    pub max_sinkage_m: f64,
}

pub struct TrackedRunningGear {
    cfg: GearConfig,
    tuning: Tuning,
    samples: Vec<TrackSample>,
    x_m: Vec<f64>,
    links: Vec<Link>,
    out: Vec<ContactOutput>,
    wheel_force_n: Vec<f64>,
}

impl TrackedRunningGear {
    pub fn new(cfg: GearConfig, tuning: Tuning) -> TrackedRunningGear {
        let n = cfg.samples.max(1);
        let dx = cfg.contact_length_m / n as f64;
        let x_m: Vec<f64> = (0..n).map(|k| -0.5 * cfg.contact_length_m + (k as f64 + 0.5) * dx).collect(); // const-ok: cell centres
        let nw = cfg.wheel_x_m.len();
        let mut links = Vec::with_capacity(n);
        for &x in &x_m {
            let link = if nw == 0 || x <= cfg.wheel_x_m[0] {
                Link { lo: 0, t: 0.0, sag_m: 0.0 }
            } else if x >= cfg.wheel_x_m[nw - 1] {
                Link { lo: nw - 1, t: 0.0, sag_m: 0.0 }
            } else {
                let i = cfg.wheel_x_m.partition_point(|&w| w <= x) - 1;
                let span = cfg.wheel_x_m[i + 1] - cfg.wheel_x_m[i];
                let t = (x - cfg.wheel_x_m[i]) / span;
                let sag = cfg.mass_per_m_kg * tuning.gravity_m_s2 * span * span / (8.0 * cfg.tension_n.max(1.0)); // const-ok: taut string sag q s^2 / 8T
                Link { lo: i, t, sag_m: sag * 4.0 * t * (1.0 - t) } // const-ok: parabola with the maximum at mid-span
            };
            links.push(link);
        }
        // Each wheel's stiffness is spread over the stretch of footprint it serves (half the distance to each neighbour, the end wheels out to
        // the ends), so the stiffness per sample is smooth and does not alias with the sample spacing.
        let half = 0.5 * cfg.contact_length_m; // const-ok: half length
        let served: Vec<f64> = (0..nw)
            .map(|w| {
                let left =
                    if w == 0 { cfg.wheel_x_m[0] + half } else { 0.5 * (cfg.wheel_x_m[w] - cfg.wheel_x_m[w - 1]) }; // const-ok: half spans
                let right =
                    if w + 1 == nw { half - cfg.wheel_x_m[w] } else { 0.5 * (cfg.wheel_x_m[w + 1] - cfg.wheel_x_m[w]) }; // const-ok: half spans
                (left + right).max(dx)
            })
            .collect();
        let samples = links
            .iter()
            .map(|l| {
                let mut ks = 0.0;
                let mut cs = 0.0;
                let mut add = |w: usize, weight: f64| {
                    ks += weight * cfg.wheel_stiffness_n_m * dx / served[w];
                    cs += weight * cfg.wheel_damping_ns_m * dx / served[w];
                };
                add(l.lo, 1.0 - l.t);
                if l.t > 0.0 {
                    add(l.lo + 1, l.t);
                }
                TrackSample::new(
                    SampleGeom {
                        width_m: cfg.width_m,
                        length_m: dx,
                        stiffness_n_m: ks,
                        damping_ns_m: cs,
                        shoe_mu_scale: cfg.shoe_mu_scale,
                        shoe_mu_scale_soft: cfg.shoe_mu_scale_soft,
                    },
                    tuning,
                )
            })
            .collect();
        TrackedRunningGear {
            out: vec![ContactOutput::default(); n],
            wheel_force_n: vec![0.0; nw],
            cfg,
            tuning,
            samples,
            x_m,
            links,
        }
    }

    pub fn config(&self) -> &GearConfig {
        &self.cfg
    }

    /// Sample positions along the track, m.
    pub fn sample_x_m(&self) -> &[f64] {
        &self.x_m
    }

    pub fn samples(&self) -> &[TrackSample] {
        &self.samples
    }

    /// The per-sample outputs of the last step (compaction included in `fx_n`).
    pub fn outputs(&self) -> &[ContactOutput] {
        &self.out
    }

    /// The vertical force the soil returns on each road wheel, N, from the last step.
    pub fn wheel_force_n(&self) -> &[f64] {
        &self.wheel_force_n
    }

    pub fn reset(&mut self) {
        for s in &mut self.samples {
            s.reset();
        }
        self.out.iter_mut().for_each(|o| *o = ContactOutput::default());
        self.wheel_force_n.iter_mut().for_each(|f| *f = 0.0);
    }

    pub fn step(&mut self, input: &GearInput) -> GearTotals {
        let n = self.samples.len();
        let vb = input.sprocket_omega_rad_s * self.cfg.sprocket_radius_m;
        // The shoe arrives from the front of the footprint when the belt runs rearward relative to the hull (vb > 0).
        for k in 0..n {
            let upstream = if vb > 0.0 { k.checked_add(1).filter(|&i| i < n) } else { k.checked_sub(1) };
            let (jx, jy) = upstream.map_or((0.0, 0.0), |i| self.samples[i].shear_displacement_m());
            self.samples[k].set_inflow_shear_m(jx, jy);
        }
        self.wheel_force_n.iter_mut().for_each(|f| *f = 0.0);
        for k in 0..n {
            let l = self.links[k];
            let wpen = |w: usize| input.wheel_penetration_m.get(w).copied().unwrap_or(0.0);
            let wrate = |w: usize| input.wheel_penetration_rate_m_s.get(w).copied().unwrap_or(0.0);
            let (p_lo, p_hi) = (wpen(l.lo), if l.t > 0.0 { wpen(l.lo + 1) } else { wpen(l.lo) });
            let (r_lo, r_hi) = (wrate(l.lo), if l.t > 0.0 { wrate(l.lo + 1) } else { wrate(l.lo) });
            // The belt sags between wheels only where it clears the ground: it can never be pushed below the chord.
            let delta = (1.0 - l.t) * p_lo + l.t * p_hi - l.sag_m;
            let rate = (1.0 - l.t) * r_lo + l.t * r_hi;
            let ci = ContactInput {
                penetration_m: delta,
                penetration_rate_m_s: rate,
                vel_long_m_s: input.vel_long_m_s,
                vel_lat_m_s: input.vel_lat_m_s + input.yaw_rate_rad_s * self.x_m[k],
                surface_speed_m_s: vb,
                ground: input.ground[k],
                dt_s: input.dt_s,
            };
            self.out[k] = self.samples[k].step(&ci);
            if !self.wheel_force_n.is_empty() {
                self.wheel_force_n[l.lo] += (1.0 - l.t) * self.out[k].fz_n;
                if l.t > 0.0 {
                    self.wheel_force_n[l.lo + 1] += l.t * self.out[k].fz_n;
                }
            }
        }

        // Compaction: deepening the rut beyond what the sample ahead left, in the direction of travel.
        let u0 = self.tuning.direction_speed_m_s;
        let dir = input.vel_long_m_s / (input.vel_long_m_s.abs() + u0);
        let (fwd, back) = (dir.max(0.0), (-dir).max(0.0));
        let mut totals = GearTotals::default();
        let mut shear_x = 0.0;
        for k in 0..n {
            let z_front = if k + 1 < n { self.samples[k + 1].sinkage_m() } else { 0.0 };
            let z_rear = if k > 0 { self.samples[k - 1].sinkage_m() } else { 0.0 };
            let g = input.ground[k];
            let rc = fwd * self.samples[k].compaction_force_n(g, z_front)
                - back * self.samples[k].compaction_force_n(g, z_rear);
            shear_x += self.out[k].fx_n;
            self.out[k].fx_n -= rc;
            totals.compaction_n += rc;
            totals.fx_n += self.out[k].fx_n;
            totals.fy_n += self.out[k].fy_n;
            totals.fz_n += self.out[k].fz_n;
            totals.mz_nm += self.x_m[k] * self.out[k].fy_n;
            totals.max_sinkage_m = totals.max_sinkage_m.max(self.samples[k].sinkage_m());
        }
        totals.shear_thrust_n = shear_x;
        let belt_dir = vb / (vb.abs() + u0);
        let internal = (self.cfg.resist_c0 + self.cfg.resist_c1_s_m * vb.abs()) * totals.fz_n * belt_dir;
        totals.shaft_reaction_nm = self.cfg.sprocket_radius_m * (shear_x + internal);
        for o in &mut self.out {
            o.shaft_reaction_nm = totals.shaft_reaction_nm * o.fz_n / totals.fz_n.max(f64::MIN_POSITIVE);
            // const-ok: division guard
        }
        totals
    }
}
