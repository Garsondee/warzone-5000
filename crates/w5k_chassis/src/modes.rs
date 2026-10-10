//! The stiffest resolved modes of a wheeled rig and the substep count they need (`CONTRACTS.md`, spike S1).
//!
//! Per station: the **wheel hop** (the unsprung mass between its spring and its tyre, `sqrt((k_s + k_t) / m_u)`) at rest and with
//! the bump stop fully engaged at full bump travel; for the hull, the **heave** on the series spring-tyre rates. The slip mode is
//! excluded: the tyre's relaxation update is exact (spike S1). The rule: `substeps * TICK_HZ >= SAMPLES_PER_PERIOD * f_max_hz`.

use w5k_contract::rig::{PhysRig, SAMPLES_PER_PERIOD, TICK_HZ};

use crate::suspension::Suspension;
use crate::tuning::ChassisTuning;

#[derive(Clone, Debug, PartialEq)]
pub struct StationModes {
    pub name: String,
    /// Spring wheel rate at rest and the bump stop's incremental rate at full bump travel, N/m.
    pub spring_rate_n_m: f64,
    pub stop_rate_n_m: f64,
    pub tyre_rate_n_m: f64,
    pub hop_hz: f64,
    pub hop_on_stop_hz: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModeReport {
    pub stations: Vec<StationModes>,
    pub heave_hz: f64,
    /// The stiffest mode the integrator must resolve, Hz.
    pub f_max_hz: f64,
    pub required_substeps: u32,
    pub declared_substeps: u32,
}

fn hz(k_n_m: f64, m_kg: f64) -> f64 {
    w5k_math::scalar::sqrt(k_n_m / m_kg) / (2.0 * core::f64::consts::PI)
}

/// The modes of `rig` (stations without a tyre or without unsprung mass are skipped: they have no hop).
pub fn modes(rig: &PhysRig, tuning: &ChassisTuning) -> ModeReport {
    let mut stations = Vec::new();
    let mut k_series_sum = 0.0;
    for s in &rig.stations {
        let Some(t) = &s.wheel.tyre else { continue };
        let susp = Suspension::new(&s.suspension, s.bump_travel_m, tuning.suspension());
        let h = 1e-6; // const-ok: finite-difference step for the spring's local rate
        let k_s = (susp.spring_force_n(h) - susp.spring_force_n(-h)) / (2.0 * h);
        // incremental bump-stop rate at full bump: d/dx [k0 (1 + p x / e) x] = k0 (1 + 2 p x / e)
        let b = &s.suspension.bump_stop;
        let pen = (s.bump_travel_m - b.engage_m).max(0.0);
        let k_stop = if b.engage_m > 0.0 { b.rate_n_m * (1.0 + 2.0 * b.progression * pen / b.engage_m) } else { 0.0 };
        let k_spring_full =
            (susp.spring_force_n(s.bump_travel_m + h) - susp.spring_force_n(s.bump_travel_m - h)) / (2.0 * h);
        let k_t = t.vertical_stiffness_n_m;
        if k_s + k_t > 0.0 {
            k_series_sum += k_s * k_t / (k_s + k_t);
        }
        if s.unsprung_mass_kg <= 0.0 {
            continue;
        }
        stations.push(StationModes {
            name: s.name.clone(),
            spring_rate_n_m: k_s,
            stop_rate_n_m: k_stop,
            tyre_rate_n_m: k_t,
            hop_hz: hz(k_s + k_t, s.unsprung_mass_kg),
            hop_on_stop_hz: hz(k_spring_full + k_stop + k_t, s.unsprung_mass_kg),
        });
    }
    let heave_hz = hz(k_series_sum, rig.hull.mass_kg);
    let f_max_hz = stations.iter().map(|m| m.hop_on_stop_hz.max(m.hop_hz)).fold(heave_hz, f64::max);
    let required_substeps = (SAMPLES_PER_PERIOD * f_max_hz / TICK_HZ).ceil() as u32;
    ModeReport {
        stations,
        heave_hz,
        f_max_hz,
        required_substeps: required_substeps.max(1),
        declared_substeps: rig.integration.substeps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::testing::box_truck;

    fn tuning() -> ChassisTuning {
        ChassisTuning::from_ron(include_str!("../../../content/physics/chassis/tuning.ron")).unwrap()
    }

    #[test]
    fn wheel_hop_matches_sqrt_spring_plus_tyre_over_unsprung_mass() {
        let rig = box_truck().0;
        let r = modes(&rig, &tuning());
        let s = &rig.stations[0];
        let w5k_contract::rig::SpringKind::Linear { rate_n_m } = s.suspension.spring else {
            panic!("box_truck has linear springs")
        };
        let k_t = s.wheel.tyre.as_ref().unwrap().vertical_stiffness_n_m;
        let expect = w5k_math::scalar::sqrt((rate_n_m + k_t) / s.unsprung_mass_kg) / (2.0 * core::f64::consts::PI);
        assert!((r.stations[0].hop_hz / expect - 1.0).abs() < 1e-6);
        assert!(r.stations[0].hop_on_stop_hz > r.stations[0].hop_hz);
        // the substep rule covers the stiffest mode with 20 samples per period
        assert!(f64::from(r.required_substeps) * TICK_HZ >= SAMPLES_PER_PERIOD * r.f_max_hz);
        assert!(f64::from(r.required_substeps - 1) * TICK_HZ < SAMPLES_PER_PERIOD * r.f_max_hz);
        println!("{r:#?}");
    }
}
