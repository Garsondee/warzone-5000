//! What the proving-ground results echo as `inputs`: the numbers the simulation actually used, read from the rig and from the settled
//! chassis (never from a spec sheet), so VALIDATION's oracle judges the run by its own numbers.

use w5k_contract::rig::PhysRig;

use super::sim::Sim;

/// Total vehicle mass: the sprung hull plus every unsprung mass, kg.
pub fn mass_kg(rig: &PhysRig) -> f64 {
    rig.hull.mass_kg + rig.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>()
}

/// Facts of the settled, parked truck: static loads decide the friction.
pub struct Facts {
    pub mass_kg: f64,
    /// Peak tyre friction on the test surface, weighted by static wheel load: `sum(mu_i Fz_i) / sum(Fz_i)` with
    /// `mu_i = material mu_peak * tyre mu_scale`, exactly the product the tyre model uses.
    pub mu: f64,
}

impl Facts {
    pub fn read(sim: &Sim) -> Facts {
        let (mut fz_sum, mut mu_fz) = (0.0, 0.0);
        for (st, def) in sim.chassis.stations.iter().zip(&sim.rig.stations) {
            let fz = st.report.contact.fz_n;
            let scale = def.wheel.tyre.as_ref().map_or(1.0, |t| t.mu_scale);
            fz_sum += fz;
            mu_fz += fz * sim.world.materials().get(st.report.material).mu_peak * scale;
        }
        let share = |x: f64| if fz_sum > 0.0 { x / fz_sum } else { 0.0 };
        Facts { mass_kg: mass_kg(sim.rig), mu: share(mu_fz) }
    }
}
