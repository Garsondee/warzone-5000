//! PROVISIONAL(CCR-forge): the numbers the rig needs that `VehicleDef` still cannot state (contract 0.2 took W1 to W6; what is left here
//! is bump-stop damping and progression, the drive and brake detail, the aero centre and the reference surface). They live in a sidecar
//! `content/vehicles/<id>.extras.ron` until a later contract version moves them into `def.rs`; then this module disappears. Every number is a `Param`, so provenance is kept.

use serde::{Deserialize, Serialize};
use w5k_contract::param::Param;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Extras {
    pub susp: SuspensionExtras,
    pub drive: DriveExtras,
    pub brake: BrakeExtras,
    /// Height of the centre of aerodynamic pressure above the datum, m.
    pub aero_cop_height_m: Param,
    /// Peak friction of the terrain table's dry hard reference surface (the denominator of `mu_scale`).
    pub ref_surface_mu_peak: Param,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuspensionExtras {
    pub bump_stop_damping_ns_m: Param,
    pub bump_stop_progression: Param,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DriveExtras {
    pub gearbox_inertia_kg_m2: Param,
    pub converter_k_factor_rpm_per_sqrt_nm: Param,
    pub converter_lockup_speed_ratio: Param,
    /// Clutch capacity over the engine's torque peak (used when the def says `Clutch`).
    pub clutch_capacity_factor: Param,
    pub clutch_engage_rpm: Param,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrakeExtras {
    pub cooling_w_k: Param,
    pub cooling_per_ms_w_k: Param,
    pub fade_start_k: Param,
    pub fade_end_k: Param,
    pub fade_floor: Param,
}

impl Extras {
    pub fn visit_params(&self, f: &mut dyn FnMut(&str, &Param)) {
        let (s, d, b) = (&self.susp, &self.drive, &self.brake);
        for (n, p) in [
            ("susp.bump_stop_damping_ns_m", &s.bump_stop_damping_ns_m),
            ("susp.bump_stop_progression", &s.bump_stop_progression),
            ("drive.gearbox_inertia_kg_m2", &d.gearbox_inertia_kg_m2),
            ("drive.converter_k_factor_rpm_per_sqrt_nm", &d.converter_k_factor_rpm_per_sqrt_nm),
            ("drive.converter_lockup_speed_ratio", &d.converter_lockup_speed_ratio),
            ("drive.clutch_capacity_factor", &d.clutch_capacity_factor),
            ("drive.clutch_engage_rpm", &d.clutch_engage_rpm),
            ("brake.cooling_w_k", &b.cooling_w_k),
            ("brake.cooling_per_ms_w_k", &b.cooling_per_ms_w_k),
            ("brake.fade_start_k", &b.fade_start_k),
            ("brake.fade_end_k", &b.fade_end_k),
            ("brake.fade_floor", &b.fade_floor),
            ("aero_cop_height_m", &self.aero_cop_height_m),
            ("ref_surface_mu_peak", &self.ref_surface_mu_peak),
        ] {
            f(n, p);
        }
    }

    pub fn check(&self) -> Vec<String> {
        let mut e = Vec::new();
        self.visit_params(&mut |n, p| {
            if let Err(m) = p.check(&format!("extras.{n}")) {
                e.push(m);
            }
        });
        e
    }
}
