//! What the proving-ground results echo as `inputs`: the numbers the simulation actually used, read from the rig and from the settled
//! chassis (never from a spec sheet), so VALIDATION's oracle judges the run by its own numbers.

use w5k_contract::rig::PhysRig;
use w5k_math::scalar;

use super::sim::Sim;

/// Sampling of each torque-curve segment when looking for the power peak (power is quadratic between curve points, so the peak can sit
/// between them).
const POWER_SAMPLES_PER_SEGMENT: u32 = 64; // const-ok: numerical resolution of the peak search

/// Total vehicle mass: the sprung hull plus every unsprung mass, kg.
pub fn mass_kg(rig: &PhysRig) -> f64 {
    rig.hull.mass_kg + rig.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>()
}

/// Peak engine shaft power over the torque curve up to the redline, W.
fn peak_engine_power_w(rig: &PhysRig) -> f64 {
    let e = &rig.drivetrain.engine;
    let mut peak = 0.0_f64;
    for w in e.torque_curve.windows(2) {
        for k in 0..=POWER_SAMPLES_PER_SEGMENT {
            let t = f64::from(k) / f64::from(POWER_SAMPLES_PER_SEGMENT);
            let (rpm, torque_nm) = (scalar::lerp(w[0].0, w[1].0, t), scalar::lerp(w[0].1, w[1].1, t));
            if rpm <= e.redline_rpm {
                peak = peak.max(torque_nm * scalar::rpm_to_rad_s(rpm));
            }
        }
    }
    peak
}

/// Peak engine power that can reach the wheels, W: the engine's peak times the gearbox efficiency. An upper bound (the converter, the
/// differentials and the final drives only lose more), which is what the `t_min` lower bound of the acceleration test needs.
pub fn wheel_power_w(rig: &PhysRig) -> f64 {
    peak_engine_power_w(rig) * rig.drivetrain.gearbox.efficiency
}

/// Facts of the settled, parked truck: static loads decide the friction and the share of the weight on driven wheels.
pub struct Facts {
    pub mass_kg: f64,
    /// Peak tyre friction on the test surface, weighted by static wheel load: `sum(mu_i Fz_i) / sum(Fz_i)` with
    /// `mu_i = material mu_peak * tyre mu_scale`, exactly the product the tyre model uses.
    pub mu: f64,
    pub driven_load_fraction: f64,
    pub power_w: f64,
}

impl Facts {
    pub fn read(sim: &Sim) -> Facts {
        let (mut fz_sum, mut mu_fz, mut driven_fz) = (0.0, 0.0, 0.0);
        for (st, def) in sim.chassis.stations.iter().zip(&sim.rig.stations) {
            let fz = st.report.contact.fz_n;
            let scale = def.wheel.tyre.as_ref().map_or(1.0, |t| t.mu_scale);
            fz_sum += fz;
            mu_fz += fz * sim.world.materials().get(st.report.material).mu_peak * scale;
            if def.drive_output.is_some() {
                driven_fz += fz;
            }
        }
        let share = |x: f64| if fz_sum > 0.0 { x / fz_sum } else { 0.0 };
        Facts {
            mass_kg: mass_kg(sim.rig),
            mu: share(mu_fz),
            driven_load_fraction: share(driven_fz),
            power_w: wheel_power_w(sim.rig),
        }
    }
}
