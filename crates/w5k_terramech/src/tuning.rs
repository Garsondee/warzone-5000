//! Model constants of the track contact, loaded from `content/physics/tracks/tuning.ron` (every number a `Param`).

use serde::Deserialize;
use w5k_contract::Param;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TracksTuning {
    pub firm_shear_k_m: Param,
    pub shear_cap_k: Param,
    pub damping_ratio: Param,
    pub advection_speed_m_s: Param,
    pub damping_time_s: Param,
    pub gravity_m_s2: Param,
    pub direction_speed_m_s: Param,
    pub belly_ramp_m: Param,
    pub bog_pull_fraction: Param,
    pub belly_stiffness_n_m: Param,
}

/// The plain numbers the contact code reads every substep.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tuning {
    pub firm_shear_k_m: f64,
    pub shear_cap_k: f64,
    pub damping_ratio: f64,
    pub advection_speed_m_s: f64,
    pub damping_time_s: f64,
    pub gravity_m_s2: f64,
    pub direction_speed_m_s: f64,
    pub belly_ramp_m: f64,
    pub bog_pull_fraction: f64,
    pub belly_stiffness_n_m: f64,
}

impl TracksTuning {
    pub fn from_ron(text: &str) -> Result<TracksTuning, ron::error::SpannedError> {
        ron::from_str(text)
    }

    pub fn bake(&self) -> Tuning {
        Tuning {
            firm_shear_k_m: self.firm_shear_k_m.v,
            shear_cap_k: self.shear_cap_k.v,
            damping_ratio: self.damping_ratio.v,
            advection_speed_m_s: self.advection_speed_m_s.v,
            damping_time_s: self.damping_time_s.v,
            gravity_m_s2: self.gravity_m_s2.v,
            direction_speed_m_s: self.direction_speed_m_s.v,
            belly_ramp_m: self.belly_ramp_m.v,
            bog_pull_fraction: self.bog_pull_fraction.v,
            belly_stiffness_n_m: self.belly_stiffness_n_m.v,
        }
    }
}

impl Tuning {
    /// The tuning shipped in `content/physics/tracks/tuning.ron`.
    pub fn shipped() -> Tuning {
        TracksTuning::from_ron(include_str!("../../../content/physics/tracks/tuning.ron"))
            .expect("tracks tuning.ron parses")
            .bake()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_tuning_file_loads_and_every_value_is_inside_its_band() {
        let t = TracksTuning::from_ron(include_str!("../../../content/physics/tracks/tuning.ron")).unwrap();
        for p in [
            &t.firm_shear_k_m,
            &t.shear_cap_k,
            &t.damping_ratio,
            &t.advection_speed_m_s,
            &t.damping_time_s,
            &t.gravity_m_s2,
            &t.direction_speed_m_s,
            &t.belly_ramp_m,
            &t.bog_pull_fraction,
            &t.belly_stiffness_n_m,
        ] {
            let (lo, hi) = p.band();
            assert!(p.v >= lo && p.v <= hi && !p.src.is_empty());
        }
    }
}
