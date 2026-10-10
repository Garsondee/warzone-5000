//! PROVISIONAL(CCR-forge): the numbers the rig needs that `VehicleDef` cannot yet state (CCR items W1 to W4 in
//! `docs/swarm/requests/forge-ccr-vehicledef.md`). They live in a sidecar `content/vehicles/<id>.extras.ron` until contract-v0.2 puts
//! them into `def.rs`; then this module disappears. Every number is a `Param`, so provenance is kept.

use serde::{Deserialize, Serialize};
use w5k_contract::param::Param;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Extras {
    pub tyre: TyreExtras,
    pub susp: SuspensionExtras,
    pub steer: SteerExtras,
    pub engine: EngineExtras,
    pub drive: DriveExtras,
    pub brake: BrakeExtras,
    /// Height of the centre of aerodynamic pressure above the datum, m.
    pub aero_cop_height_m: Param,
    /// Peak friction of the terrain table's dry hard reference surface (the denominator of `mu_scale`).
    pub ref_surface_mu_peak: Param,
    /// PROVISIONAL(S1): substeps are `ceil(omega_max * dt / limit)`; CHASSIS' spike S1 settles the limit.
    pub substep_omega_dt_limit: Param,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TyreExtras {
    pub vertical_stiffness_n_m: Param,
    pub vertical_damping_ns_m: Param,
    pub slip_stiffness: Param,
    pub relaxation_length_m: Param,
    /// Wheel, tyre and brake disc spin inertia about the axle, kg m^2.
    pub wheel_inertia_kg_m2: Param,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuspensionExtras {
    pub rebound_to_bump: Param,
    pub bump_stop_engage_frac: Param,
    pub bump_stop_rate_n_m: Param,
    pub bump_stop_damping_ns_m: Param,
    pub bump_stop_progression: Param,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SteerExtras {
    pub max_steer_rad: Param,
    pub ackermann: Param,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineExtras {
    /// Full-load torque at idle as a fraction of the torque peak.
    pub idle_torque_frac: Param,
    pub drag_const_nm: Param,
    pub drag_per_rpm_nm: Param,
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
        let (t, s, st, e, d, b) = (&self.tyre, &self.susp, &self.steer, &self.engine, &self.drive, &self.brake);
        for (n, p) in [
            ("tyre.vertical_stiffness_n_m", &t.vertical_stiffness_n_m),
            ("tyre.vertical_damping_ns_m", &t.vertical_damping_ns_m),
            ("tyre.slip_stiffness", &t.slip_stiffness),
            ("tyre.relaxation_length_m", &t.relaxation_length_m),
            ("tyre.wheel_inertia_kg_m2", &t.wheel_inertia_kg_m2),
            ("susp.rebound_to_bump", &s.rebound_to_bump),
            ("susp.bump_stop_engage_frac", &s.bump_stop_engage_frac),
            ("susp.bump_stop_rate_n_m", &s.bump_stop_rate_n_m),
            ("susp.bump_stop_damping_ns_m", &s.bump_stop_damping_ns_m),
            ("susp.bump_stop_progression", &s.bump_stop_progression),
            ("steer.max_steer_rad", &st.max_steer_rad),
            ("steer.ackermann", &st.ackermann),
            ("engine.idle_torque_frac", &e.idle_torque_frac),
            ("engine.drag_const_nm", &e.drag_const_nm),
            ("engine.drag_per_rpm_nm", &e.drag_per_rpm_nm),
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
            ("substep_omega_dt_limit", &self.substep_omega_dt_limit),
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
