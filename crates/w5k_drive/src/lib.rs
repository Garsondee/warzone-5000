//! `w5k_drive`: lane DRIVE.
//!
//! Powertrain and brakes: engine map, clutch or converter, gearbox, diffs, tracked steering units, brakes with heat, fuel.
//! Read the design note: `docs/lanes/drive/design-note.md`. Only lane DRIVE edits this crate (`docs/swarm/ownership.toml`).

pub mod bench;
pub mod brakes;
pub mod coupling;
pub mod driveline;
pub mod engine;
pub mod gearbox;
pub mod powertrain;

/// Brake temperature before the first step (replaced by `DriveInputs::ambient_k` from the first step on): ISA sea level.
pub(crate) const AMBIENT_FALLBACK_K: f64 = w5k_contract::ports::DEFAULT_AMBIENT_K;

#[cfg(test)]
mod testkit {
    use crate::coupling::CouplingTuning;
    use crate::engine::{Engine, EngineTuning};
    use crate::gearbox::ShiftTuning;
    use w5k_contract::rig::{EngineDef, EngineKind};

    pub fn engine_tuning() -> EngineTuning {
        ron::from_str(include_str!("../../../content/physics/drive/engine_tuning.ron"))
            .expect("engine_tuning.ron parses")
    }

    pub fn coupling_tuning() -> CouplingTuning {
        ron::from_str(include_str!("../../../content/physics/drive/coupling_tuning.ron"))
            .expect("coupling_tuning.ron parses")
    }

    pub fn shift_tuning() -> ShiftTuning {
        ron::from_str(include_str!("../../../content/physics/drive/shift_tuning.ron")).expect("shift_tuning.ron parses")
    }

    fn def(curve: Vec<(f64, f64)>) -> EngineDef {
        EngineDef {
            kind: EngineKind::Diesel,
            torque_curve: curve,
            idle_rpm: 700.0,
            redline_rpm: 4000.0,
            inertia_kg_m2: 0.4,
            drag_const_nm: 12.0,
            drag_per_rpm_nm: 0.006,
            bsfc_best_g_kwh: 220.0,
            free_output: false,
            response_time_s: 0.0,
            idle_fuel_kg_s: 0.0,
        }
    }

    pub fn diesel_engine() -> Engine {
        Engine::new(&def(vec![(700.0, 300.0), (1800.0, 400.0), (3000.0, 350.0), (4000.0, 250.0)]), &engine_tuning())
            .unwrap()
    }

    /// 350 N m at every speed, so closed-form checks do not depend on the curve.
    pub fn flat_engine() -> Engine {
        Engine::new(&def(vec![(700.0, 350.0), (4000.0, 350.0)]), &engine_tuning()).unwrap()
    }
}
