//! `VehicleDef`: the designer-level description of a vehicle, authored as RON in `content/vehicles/`. Every number is a [`Param`]
//! (value, band, provenance, source), so a reference vehicle that mimics a real one carries the evidence for each figure.
//! Lane FORGE compiles a `VehicleDef` into a [`crate::rig::PhysRig`] and a [`crate::render::RenderRig`]; lane GEOMETRY reads the
//! same parameters to shape the meshes; lane VALIDATION links it to a dossier through `reference`.
//!
//! This is the **v0 draft**: the shape of the sections is settled, the field list is expected to grow through CCRs from FORGE,
//! GEOMETRY and VALIDATION in the settling round. There are deliberately no `Default`s: a physical number is never implicit.
//!
//! Designer sliders (ride frequency, damping ratio, gear spread) are what a player turns; FORGE turns them into spring rates,
//! damper coefficients and gearbox ratios using the measured sprung mass.

use serde::{Deserialize, Serialize};

use crate::param::Param;
use crate::rig::{EngineKind, SteerUnitKind};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VehicleDef {
    pub id: String,
    pub name: String,
    /// Id of the dossier of the real vehicle this design mimics, if any (`content/dossier/<id>.ron`).
    #[serde(default)]
    pub reference: Option<String>,
    pub hull: HullDef,
    pub running_gear: RunningGearDef,
    pub suspension: SuspensionSliders,
    pub powertrain: PowertrainDef,
    pub brakes: BrakesDef,
    pub aero: AeroSliders,
    #[serde(default)]
    pub turret: Option<TurretDef>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HullDef {
    pub length_m: Param,
    pub width_m: Param,
    pub height_m: Param,
    /// Mass in the stated loading (combat or curb, with crew and fuel) excluding the turret and the unsprung mass, kg.
    pub mass_kg: Param,
    /// Centre-of-mass height above the ground at the design ride height, m.
    pub com_height_m: Param,
    /// Centre-of-mass position measured back from the front of the hull, m.
    pub com_from_front_m: Param,
    pub ground_clearance_m: Param,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RunningGearDef {
    Wheeled(WheeledDef),
    Tracked(TrackedDef),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WheeledDef {
    pub axles: Vec<AxleDef>,
    pub tyre: TyreSliders,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AxleDef {
    /// Axle position measured back from the front of the hull, m.
    pub from_front_m: Param,
    /// Distance between the wheel centres of this axle, m.
    pub track_width_m: Param,
    pub steered: bool,
    pub driven: bool,
    /// Anti-roll bar stiffness at the wheels, N/m of differential travel; `None` = no bar.
    #[serde(default)]
    pub anti_roll_n_m: Option<Param>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TyreSliders {
    pub outer_diameter_m: Param,
    pub section_width_m: Param,
    pub inflation_pa: Param,
    /// Peak friction coefficient on dry hard ground.
    pub mu_peak_ref: Param,
    pub cornering_stiffness_per_rad: Param,
    pub rolling_coeff: Param,
    /// Mass of wheel, tyre, hub, brake and the moving part of the suspension, per wheel, kg.
    pub unsprung_mass_kg: Param,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackedDef {
    pub road_wheels_per_side: u8,
    pub road_wheel_diameter_m: Param,
    pub sprocket_diameter_m: Param,
    pub sprocket_at_front: bool,
    /// Belt width, m.
    pub track_width_m: Param,
    /// Distance between the two belt centre lines, m.
    pub track_gauge_m: Param,
    /// Length of belt on firm ground, m.
    pub ground_contact_length_m: Param,
    pub pitch_m: Param,
    pub track_mass_per_side_kg: Param,
    /// Shoe grip relative to rubber (grousers on soft ground > 1, steel on road < 1).
    pub shoe_mu_scale: Param,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuspensionKind {
    Coil,
    LeafSpring,
    TorsionBar,
    Hydropneumatic,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuspensionSliders {
    pub kind: SuspensionKind,
    /// Natural frequency of the sprung mass on its suspension, Hz (cars 1-1.5, trucks 1.5-2.5, tanks 1.2-2).
    pub front_ride_frequency_hz: Param,
    pub rear_ride_frequency_hz: Param,
    /// Damping ratio zeta of the sprung mass (0.2-0.4 typical).
    pub damping_ratio: Param,
    pub bump_travel_m: Param,
    pub droop_travel_m: Param,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PowertrainDef {
    pub engine: EngineSliders,
    pub coupling: CouplingSliders,
    pub gearbox: GearboxSliders,
    pub final_drive_ratio: Param,
    #[serde(default)]
    pub transfer_case_ratio: Option<Param>,
    #[serde(default)]
    pub steering_unit: Option<SteeringUnitSliders>,
    /// Efficiency of the whole driveline from gearbox output to the wheels (0..1).
    pub driveline_efficiency: Param,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EngineSliders {
    pub kind: EngineKind,
    pub peak_power_w: Param,
    pub peak_power_rpm: Param,
    pub peak_torque_nm: Param,
    pub peak_torque_rpm: Param,
    pub idle_rpm: Param,
    pub redline_rpm: Param,
    pub inertia_kg_m2: Param,
    pub bsfc_best_g_kwh: Param,
    /// A published full-load curve `(rpm, N m)`; when present it replaces the shape derived from the peaks.
    #[serde(default)]
    pub torque_curve: Option<Vec<(f64, f64)>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CouplingSliders {
    Clutch,
    TorqueConverter { stall_ratio: Param, lockup: bool },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GearboxSliders {
    pub forward_ratios: Vec<Param>,
    pub reverse_ratios: Vec<Param>,
    pub efficiency: Param,
    pub automatic: bool,
    pub upshift_rpm: Param,
    pub downshift_rpm: Param,
    pub shift_time_s: Param,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SteeringUnitSliders {
    pub kind: SteerUnitKind,
    pub ratio: Param,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrakesDef {
    /// Service-brake capability as deceleration on dry hard ground with all wheels at the limit, in g.
    pub service_decel_g: Param,
    /// Share of the braking torque on the front axle (wheeled) or ignored (tracked).
    pub front_share: Param,
    /// Heat capacity of one brake, kJ/K.
    pub thermal_mass_kj_k: Param,
    pub parking_brake: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AeroSliders {
    pub drag_coeff: Param,
    pub frontal_area_m2: Param,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurretDef {
    pub ring_diameter_m: Param,
    /// Mass of the turret structure with its contents, excluding the gun, kg.
    pub mass_kg: Param,
    pub traverse_rate_deg_s: Param,
    pub traverse_accel_deg_s2: Param,
    pub gun: GunDef,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GunDef {
    pub caliber_mm: Param,
    pub barrel_length_m: Param,
    /// Mass of barrel, breech and mantlet that elevate together, kg.
    pub mass_kg: Param,
    pub muzzle_velocity_m_s: Param,
    pub depression_deg: Param,
    pub elevation_deg: Param,
    pub elevation_rate_deg_s: Param,
    pub recoil_stroke_m: Param,
    /// Fraction of hull motion the stabiliser rejects (0 = unstabilised, WW2 style).
    pub stabiliser_rejection: Param,
}

impl VehicleDef {
    /// Visit every `Param` with its dotted path (for validation, provenance reports and the dossier link).
    pub fn visit_params(&self, f: &mut dyn FnMut(&str, &Param)) {
        macro_rules! each {
            ($prefix:expr, $obj:expr, $($name:ident),+) => { $( f(&format!("{}.{}", $prefix, stringify!($name)), &$obj.$name); )+ };
        }
        let h = &self.hull;
        each!("hull", h, length_m, width_m, height_m, mass_kg, com_height_m, com_from_front_m, ground_clearance_m);
        match &self.running_gear {
            RunningGearDef::Wheeled(w) => {
                for (i, a) in w.axles.iter().enumerate() {
                    f(&format!("running_gear.axles[{i}].from_front_m"), &a.from_front_m);
                    f(&format!("running_gear.axles[{i}].track_width_m"), &a.track_width_m);
                    if let Some(p) = &a.anti_roll_n_m {
                        f(&format!("running_gear.axles[{i}].anti_roll_n_m"), p);
                    }
                }
                each!("running_gear.tyre", w.tyre, outer_diameter_m, section_width_m, inflation_pa, mu_peak_ref, cornering_stiffness_per_rad, rolling_coeff, unsprung_mass_kg);
            }
            RunningGearDef::Tracked(t) => {
                each!(
                    "running_gear",
                    t,
                    road_wheel_diameter_m,
                    sprocket_diameter_m,
                    track_width_m,
                    track_gauge_m,
                    ground_contact_length_m,
                    pitch_m,
                    track_mass_per_side_kg,
                    shoe_mu_scale
                );
            }
        }
        let s = &self.suspension;
        each!("suspension", s, front_ride_frequency_hz, rear_ride_frequency_hz, damping_ratio, bump_travel_m, droop_travel_m);
        let p = &self.powertrain;
        let e = &p.engine;
        each!("powertrain.engine", e, peak_power_w, peak_power_rpm, peak_torque_nm, peak_torque_rpm, idle_rpm, redline_rpm, inertia_kg_m2, bsfc_best_g_kwh);
        if let CouplingSliders::TorqueConverter { stall_ratio, .. } = &p.coupling {
            f("powertrain.coupling.stall_ratio", stall_ratio);
        }
        let g = &p.gearbox;
        for (i, r) in g.forward_ratios.iter().enumerate() {
            f(&format!("powertrain.gearbox.forward_ratios[{i}]"), r);
        }
        for (i, r) in g.reverse_ratios.iter().enumerate() {
            f(&format!("powertrain.gearbox.reverse_ratios[{i}]"), r);
        }
        each!("powertrain.gearbox", g, efficiency, upshift_rpm, downshift_rpm, shift_time_s);
        f("powertrain.final_drive_ratio", &p.final_drive_ratio);
        f("powertrain.driveline_efficiency", &p.driveline_efficiency);
        if let Some(t) = &p.transfer_case_ratio {
            f("powertrain.transfer_case_ratio", t);
        }
        if let Some(u) = &p.steering_unit {
            f("powertrain.steering_unit.ratio", &u.ratio);
        }
        each!("brakes", self.brakes, service_decel_g, front_share, thermal_mass_kj_k);
        each!("aero", self.aero, drag_coeff, frontal_area_m2);
        if let Some(t) = &self.turret {
            each!("turret", t, ring_diameter_m, mass_kg, traverse_rate_deg_s, traverse_accel_deg_s2);
            each!(
                "turret.gun",
                t.gun,
                caliber_mm,
                barrel_length_m,
                mass_kg,
                muzzle_velocity_m_s,
                depression_deg,
                elevation_deg,
                elevation_rate_deg_s,
                recoil_stroke_m,
                stabiliser_rejection
            );
        }
    }

    /// Check every parameter and the structural basics. Returns every problem found.
    pub fn check(&self) -> Result<(), Vec<String>> {
        let mut errs = Vec::new();
        self.visit_params(&mut |name, p| {
            if let Err(e) = p.check(name) {
                errs.push(e);
            }
        });
        if self.id.trim().is_empty() {
            errs.push("id must not be empty".into());
        }
        match &self.running_gear {
            RunningGearDef::Wheeled(w) if w.axles.len() < 2 => errs.push("a wheeled vehicle needs at least two axles".into()),
            RunningGearDef::Tracked(t) if t.road_wheels_per_side < 2 => errs.push("a tracked vehicle needs at least two road wheels per side".into()),
            _ => {}
        }
        if self.powertrain.gearbox.forward_ratios.is_empty() {
            errs.push("the gearbox needs at least one forward ratio".into());
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs)
        }
    }

    /// Counts of parameters by provenance `(spec, measured, estimate, tuned)`: the honesty meter of a design.
    pub fn provenance_counts(&self) -> (usize, usize, usize, usize) {
        let mut c = (0, 0, 0, 0);
        self.visit_params(&mut |_, p| match p.prov {
            crate::param::Provenance::Spec => c.0 += 1,
            crate::param::Provenance::Measured => c.1 += 1,
            crate::param::Provenance::Estimate => c.2 += 1,
            crate::param::Provenance::Tuned => c.3 += 1,
        });
        c
    }
}
