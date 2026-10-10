//! A plan-view skid-steer vehicle: two running gears under a rigid hull moving in the ground plane (surge, sway, yaw). Heave is frozen at the
//! static equilibrium, so what moves is only the horizontal contact: the model of spike S3 and the bench behind `w5k tracks bench`.
//!
//! Frame: x forward, y left, yaw positive turns the nose left (a left turn); the left track is at `y = +gauge/2`.

use w5k_contract::Material;

use w5k_math::scalar;

use crate::gear::{GearConfig, GearInput, GearTotals, TrackedRunningGear};
use crate::tuning::Tuning;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlanMotion {
    pub vx_m_s: f64,
    pub vy_m_s: f64,
    pub yaw_rate_rad_s: f64,
}

/// Forces on the hull from both tracks, about the centre of mass, with the sprocket loads.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlanWrench {
    pub fx_n: f64,
    pub fy_n: f64,
    pub mz_nm: f64,
    pub left: GearTotals,
    pub right: GearTotals,
    /// Mechanical power into the two sprockets, W.
    pub sprocket_power_w: f64,
}

pub struct PlanVehicle {
    pub left: TrackedRunningGear,
    pub right: TrackedRunningGear,
    pub gauge_m: f64,
    pub mass_kg: f64,
    pub yaw_inertia_kg_m2: f64,
    pub motion: PlanMotion,
    ground: Material,
    penetration_m: Vec<f64>,
    rate_m_s: Vec<f64>,
    grounds: Vec<Material>,
}

impl PlanVehicle {
    /// Two identical gears on `ground`, carrying `weight_n` in total; the wheels are set to the static penetration for that load. `None` if the
    /// ground cannot carry it within `max_penetration_m`.
    pub fn new(
        cfg: GearConfig,
        tuning: Tuning,
        gauge_m: f64,
        weight_n: f64,
        ground: &Material,
        max_penetration_m: f64,
    ) -> Option<PlanVehicle> {
        let d = uniform_penetration_for_load_m(&cfg, tuning, ground, 0.5 * weight_n, max_penetration_m)?; // const-ok: half the weight per track
        let mass_kg = weight_n / tuning.gravity_m_s2;
        let nw = cfg.wheel_x_m.len();
        let l = cfg.contact_length_m;
        Some(PlanVehicle {
            yaw_inertia_kg_m2: mass_kg * (l * l + gauge_m * gauge_m) / 12.0, // const-ok: uniform plate
            left: TrackedRunningGear::new(cfg.clone(), tuning),
            right: TrackedRunningGear::new(cfg.clone(), tuning),
            gauge_m,
            mass_kg,
            motion: PlanMotion::default(),
            ground: ground.clone(),
            penetration_m: vec![d; nw],
            rate_m_s: vec![0.0; nw],
            grounds: vec![ground.clone(); cfg.samples.max(1)],
        })
    }

    pub fn ground(&self) -> &Material {
        &self.ground
    }

    /// Belt speeds `(left, right)`, m/s, positive = the vehicle driven forward.
    pub fn step(&mut self, belts_m_s: (f64, f64), dt_s: f64) -> PlanWrench {
        let m = self.motion;
        let refs: Vec<&Material> = self.grounds.iter().collect();
        let run = |gear: &mut TrackedRunningGear, y_m: f64, vb: f64| {
            let r = gear.config().sprocket_radius_m;
            gear.step(&GearInput {
                wheel_penetration_m: &self.penetration_m,
                wheel_penetration_rate_m_s: &self.rate_m_s,
                vel_long_m_s: m.vx_m_s - m.yaw_rate_rad_s * y_m,
                vel_lat_m_s: m.vy_m_s,
                yaw_rate_rad_s: m.yaw_rate_rad_s,
                sprocket_omega_rad_s: vb / r,
                ground: &refs,
                dt_s,
            })
        };
        let h = 0.5 * self.gauge_m; // const-ok: half gauge
        let l = run(&mut self.left, h, belts_m_s.0);
        let r = run(&mut self.right, -h, belts_m_s.1);
        let power = l.shaft_reaction_nm * belts_m_s.0 / self.left.config().sprocket_radius_m
            + r.shaft_reaction_nm * belts_m_s.1 / self.right.config().sprocket_radius_m;
        PlanWrench {
            fx_n: l.fx_n + r.fx_n,
            fy_n: l.fy_n + r.fy_n,
            mz_nm: l.mz_nm + r.mz_nm - h * l.fx_n + h * r.fx_n,
            left: l,
            right: r,
            sprocket_power_w: power,
        }
    }

    /// One dynamic step: the belts are driven at the given speeds, `external_n` (body frame) is added to the contact forces, and the hull
    /// moves under Newton's laws in the rotating body frame.
    pub fn step_dynamic(&mut self, belts_m_s: (f64, f64), external_n: (f64, f64), dt_s: f64) -> PlanWrench {
        let w = self.step(belts_m_s, dt_s);
        let m = &mut self.motion;
        let ax = (w.fx_n + external_n.0) / self.mass_kg + m.yaw_rate_rad_s * m.vy_m_s;
        let ay = (w.fy_n + external_n.1) / self.mass_kg - m.yaw_rate_rad_s * m.vx_m_s;
        m.vx_m_s += ax * dt_s;
        m.vy_m_s += ay * dt_s;
        m.yaw_rate_rad_s += w.mz_nm / self.yaw_inertia_kg_m2 * dt_s;
        w
    }
}

/// The belt's penetration needed so that a uniform set of wheel penetrations carries `load_n` on `ground` (a bisection on the gear's own law);
/// used by benches and tests to start a rig in vertical equilibrium. Returns `None` if even `max_m` cannot carry the load.
pub fn uniform_penetration_for_load_m(
    cfg: &GearConfig,
    tuning: Tuning,
    ground: &Material,
    load_n: f64,
    max_m: f64,
) -> Option<f64> {
    let wheels = cfg.wheel_x_m.len();
    let grounds: Vec<&Material> = vec![ground; cfg.samples.max(1)];
    let load_at = |d: f64| {
        let mut g = TrackedRunningGear::new(cfg.clone(), tuning);
        let pen = vec![d; wheels];
        let rate = vec![0.0; wheels];
        g.step(&GearInput {
            wheel_penetration_m: &pen,
            wheel_penetration_rate_m_s: &rate,
            vel_long_m_s: 0.0,
            vel_lat_m_s: 0.0,
            yaw_rate_rad_s: 0.0,
            sprocket_omega_rad_s: 0.0,
            ground: &grounds,
            dt_s: 0.0,
        })
        .fz_n
    };
    if load_at(max_m) < load_n {
        return None;
    }
    let (mut lo, mut hi) = (0.0, max_m);
    for _ in 0..60 {
        // const-ok: bisection depth
        let mid = 0.5 * (lo + hi); // const-ok: midpoint
        if load_at(mid) < load_n {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(scalar::clamp(0.5 * (lo + hi), 0.0, max_m)) // const-ok: midpoint
}
