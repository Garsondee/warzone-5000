//! The ladder: the same tank, heavier by a step at a time, on one soil. Where does it start to sink into its belly, and where does it bog?
//!
//! A row is a static equilibrium (the weight carried by the two gears and, once the sinkage passes the clearance, the belly) followed by a
//! drive at full slip: the net drawbar pull (shear thrust minus compaction resistance and belly drag) says whether it can still pull itself.
//! **Bogged means the net drawbar pull at 50% slip is below `bog_pull_fraction` of the weight** (PROVISIONAL, card TRACKS-D1; a fraction of 0 is
//! Wong's zero-drawbar-pull definition), or that the ground cannot carry the weight within the search bound.

use serde::Deserialize;
use w5k_contract::{Material, Param};

use crate::belly::BellyGeom;
use crate::gear::GearConfig;
use crate::plan::PlanVehicle;
use crate::soil;
use crate::tuning::Tuning;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LadderRow {
    pub mass_kg: f64,
    /// Weight over the two footprints, Pa.
    pub ground_pressure_pa: f64,
    /// `z = (p / (kc/b + kphi))^(1/n)` for that pressure: what the soil theory says a rigid footprint sinks.
    pub predicted_sinkage_m: f64,
    /// Mean sinkage of the track samples in the static equilibrium, m (the search bound when the ground cannot carry the tank).
    pub sinkage_m: f64,
    pub belly_engaged: bool,
    pub net_pull_n: f64,
    pub bogged: bool,
}

/// One rung. `gauge_m` is the track spacing; `max_penetration_m` bounds the search for the equilibrium.
pub fn ladder_row(
    cfg: &GearConfig,
    belly: BellyGeom,
    gauge_m: f64,
    tuning: Tuning,
    ground: &Material,
    mass_kg: f64,
    max_penetration_m: f64,
) -> LadderRow {
    let weight_n = mass_kg * tuning.gravity_m_s2;
    let area_m2 = 2.0 * cfg.width_m * cfg.contact_length_m; // const-ok: two tracks
    let pressure = weight_n / area_m2;
    let predicted = ground.soil.as_ref().map_or(0.0, |s| soil::sinkage_m(s, cfg.width_m, pressure));
    let Some(mut v) =
        PlanVehicle::new_with_belly(cfg.clone(), tuning, gauge_m, weight_n, ground, max_penetration_m, Some(belly))
    else {
        return LadderRow {
            mass_kg,
            ground_pressure_pa: pressure,
            predicted_sinkage_m: predicted,
            sinkage_m: max_penetration_m,
            belly_engaged: true,
            net_pull_n: 0.0,
            bogged: true,
        };
    };
    let (full_slip, speed, dt) = (0.5, 1.0, 0.002); // const-ok: bench drive: 50% slip at 1 m/s, 500 Hz
    v.motion.vx_m_s = speed;
    let belt = speed / (1.0 - full_slip);
    let mut pull = 0.0;
    for _ in 0..3000 {
        // const-ok: 6 s, several belt crossings of the footprint
        pull = v.step((belt, belt), dt).fx_n;
    }
    let samples = v.left.samples();
    let sinkage = samples.iter().map(|s| s.sinkage_m()).sum::<f64>() / samples.len() as f64;
    LadderRow {
        mass_kg,
        ground_pressure_pa: pressure,
        predicted_sinkage_m: predicted,
        sinkage_m: sinkage,
        belly_engaged: v.static_penetration_m() > belly.clearance_m,
        net_pull_n: pull,
        bogged: pull < tuning.bog_pull_fraction * weight_n,
    }
}

/// The belly of the reference tank the benches use, from `content/physics/tracks/ladder_tank.ron`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LadderTank {
    pub clearance_m: Param,
    pub belly_width_m: Param,
    pub belly_length_m: Param,
}

impl LadderTank {
    pub fn shipped() -> LadderTank {
        ron::from_str(include_str!("../../../content/physics/tracks/ladder_tank.ron")).expect("ladder_tank.ron parses")
    }

    pub fn belly(&self, tuning: &Tuning) -> BellyGeom {
        BellyGeom {
            clearance_m: self.clearance_m.v,
            width_m: self.belly_width_m.v,
            length_m: self.belly_length_m.v,
            stiffness_n_m: tuning.belly_stiffness_n_m,
        }
    }
}
