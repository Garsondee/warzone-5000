//! PROVISIONAL(CCR-forge): the numbers the rig needs that `VehicleDef` still cannot state (contract 0.2 took W1 to W6; what is left here
//! is bump-stop damping and progression, the drive and brake detail, the aero centre and the reference surface). They live in a sidecar
//! `content/vehicles/<id>.extras.ron` until a later contract version moves them into `def.rs`; then this module disappears. Every number is a `Param`, so provenance is kept.

use serde::{Deserialize, Serialize};
use w5k_contract::param::Param;
use w5k_contract::rig::DiffKind;

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
    /// PROVISIONAL(CCR-forge, tracked): what `TrackedDef` cannot state yet (design note `tracked-design-note.md`, section 8).
    #[serde(default)]
    pub tracked: Option<TrackedExtras>,
    /// PROVISIONAL(CCR-forge): the tyre's load sensitivity (contract 0.3, CHASSIS CCR-4). Absent = linear tyres (0).
    #[serde(default)]
    pub tyre_load: Option<TyreLoadExtras>,
    /// PROVISIONAL(C-020): the component mass budget (design note `mass-budget-note.md`). Absent = the def's `hull.mass_kg` is the whole
    /// sprung mass at the def's COM, as before.
    #[serde(default)]
    pub mass_budget: Option<MassBudgetExtras>,
}

/// A position in the hull as a designer states it: back from the front of the hull, height above the ground at the design pose, and
/// sideways (+ right), metres.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartPos {
    pub from_front_m: Param,
    pub height_m: Param,
    pub lateral_m: Param,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Loading {
    /// Tanks at the `fill_curb` fraction.
    Curb,
    /// Tanks at the `fill_combat` fraction.
    #[default]
    Combat,
}

/// Every part's mass follows from the choice that sizes it; the structure is authored (D3 will derive it from armour and the loft). Linear
/// laws on purpose (PROVISIONAL): the simplest honest dependence, each coefficient an ESTIMATE with a band and a source.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MassBudgetExtras {
    /// Hull, body, armour and everything not listed below: mass and where it sits.
    pub structure_mass_kg: Param,
    pub structure_pos: PartPos,
    /// Installed engine (with cooling and accessories), kg per kW of peak power, and where it sits.
    pub engine_kg_per_kw: Param,
    pub engine_pos: PartPos,
    /// Gearbox, transfer case and steer unit, kg per N m of peak engine torque.
    pub transmission_kg_per_nm: Param,
    pub transmission_pos: PartPos,
    /// Final drives: kg per N m of the torque each driven axle (or sprocket) carries (peak torque x first gear x final drive / drives), and
    /// their height above the ground (they sit at the axles or sprockets).
    pub final_drive_kg_per_nm: Param,
    pub final_drive_height_m: Param,
    pub tank_litres: Param,
    pub fuel_density_kg_l: Param,
    pub tank_kg_per_litre: Param,
    pub fill_curb: Param,
    pub fill_combat: Param,
    pub fuel_pos: PartPos,
    pub crew_count: Param,
    pub crew_kg_each: Param,
    pub crew_pos: PartPos,
    #[serde(default)]
    pub loading: Loading,
}

impl MassBudgetExtras {
    pub fn visit_params(&self, f: &mut dyn FnMut(&str, &Param)) {
        let pos = |f: &mut dyn FnMut(&str, &Param), n: &str, p: &PartPos| {
            f(&format!("mass_budget.{n}.from_front_m"), &p.from_front_m);
            f(&format!("mass_budget.{n}.height_m"), &p.height_m);
            f(&format!("mass_budget.{n}.lateral_m"), &p.lateral_m);
        };
        for (n, p) in [
            ("structure_mass_kg", &self.structure_mass_kg),
            ("engine_kg_per_kw", &self.engine_kg_per_kw),
            ("transmission_kg_per_nm", &self.transmission_kg_per_nm),
            ("final_drive_kg_per_nm", &self.final_drive_kg_per_nm),
            ("final_drive_height_m", &self.final_drive_height_m),
            ("tank_litres", &self.tank_litres),
            ("fuel_density_kg_l", &self.fuel_density_kg_l),
            ("tank_kg_per_litre", &self.tank_kg_per_litre),
            ("fill_curb", &self.fill_curb),
            ("fill_combat", &self.fill_combat),
            ("crew_count", &self.crew_count),
            ("crew_kg_each", &self.crew_kg_each),
        ] {
            f(&format!("mass_budget.{n}"), p);
        }
        pos(f, "structure_pos", &self.structure_pos);
        pos(f, "engine_pos", &self.engine_pos);
        pos(f, "transmission_pos", &self.transmission_pos);
        pos(f, "fuel_pos", &self.fuel_pos);
        pos(f, "crew_pos", &self.crew_pos);
    }
}

/// `s(k) = 1 / (1 + k (Fz / Fz0 - 1))` scales the friction coefficient and the slip and cornering stiffnesses (Pacejka, ch. 4); `Fz0`
/// is the tyre's static load (the solver's default while the rig's `nominal_load_n` is 0).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TyreLoadExtras {
    pub mu_load_sensitivity: Param,
    pub stiffness_load_sensitivity: Param,
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
    /// PROVISIONAL(CCR-forge, drivetrain): the centre differential (several driven axles) and the axle differentials. Absent = open.
    #[serde(default = "open_diff")]
    pub centre_diff: DiffSpec,
    #[serde(default = "open_diff")]
    pub axle_diff: DiffSpec,
}

/// A differential's kind and, for a limited slip, its torque-bias ratio (>= 1; ignored for open and locked).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffSpec {
    pub kind: DiffKind,
    pub bias: Param,
}

// const-ok: the band of an unused placeholder bias (an open differential has none)
const UNUSED_BIAS_HI: f64 = 8.0;

fn open_diff() -> DiffSpec {
    DiffSpec {
        kind: DiffKind::Open,
        bias: Param::estimate(1.0, 1.0, UNUSED_BIAS_HI, "an open differential: the bias is unused"),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrakeExtras {
    pub cooling_w_k: Param,
    pub cooling_per_ms_w_k: Param,
    pub fade_start_k: Param,
    pub fade_end_k: Param,
    pub fade_floor: Param,
    /// PROVISIONAL(CCR-forge, drivetrain): brake torque at the wheel pair of each axle, N m, front first. When present it is THE spec
    /// (a real brake has a torque; the deceleration is derived and reported, and falls with mass); when empty the torque is derived
    /// from `BrakesDef::service_decel_g` as before.
    #[serde(default)]
    pub axle_torque_nm: Vec<Param>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackedExtras {
    /// Positions of the sprocket and the idler measured back from the front of the hull, m, and the heights of their centres above the ground.
    pub sprocket_from_front_m: Param,
    pub idler_from_front_m: Param,
    pub sprocket_height_m: Param,
    pub idler_height_m: Param,
    pub idler_diameter_m: Param,
    /// Return rollers on the top run (a count) and their diameter.
    pub return_rollers: Param,
    pub roller_diameter_m: Param,
    pub belt_thickness_m: Param,
    pub sprocket_teeth: Param,
    /// Cells along the ground run for the soil solver.
    pub samples: Param,
    pub tension_n: Param,
    pub resist_c0: Param,
    pub road_wheel_width_m: Param,
    pub road_wheel_unsprung_kg: Param,
    pub road_wheel_inertia_kg_m2: Param,
    /// Spin inertia of the sprocket, idler and rollers about their axles (rigid stations), kg m^2.
    pub rigid_wheel_inertia_kg_m2: Param,
    /// Road-wheel contact with the belt: stiffness and damping per wheel (the belt itself is massless).
    pub wheel_contact_stiffness_n_m: Param,
    pub wheel_contact_damping_ns_m: Param,
    /// Torsion-bar arm (used when the suspension kind is `TorsionBar`): length, m, and rest angle below the horizontal, rad. The arm trails:
    /// its pivot is ahead of the wheel.
    pub torsion_arm_length_m: Param,
    pub torsion_rest_angle_rad: Param,
    /// The belly plate (a Belly collision proxy between the tracks): its length along the hull and its thickness, m.
    pub belly_length_m: Param,
    pub belly_thickness_m: Param,
    /// The steering law of the steer unit (empty = the kind's default).
    #[serde(default)]
    pub steer_law: SteerLawExtras,
}

/// The `SteerLaw` numbers (design note section 5); the brakes that realise clutch-brake and controlled-differential steering are the two
/// sprocket brakes, set by the compile.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SteerLawExtras {
    /// `(v_outer - v_inner) / (v_outer + v_inner)` at full demand, per forward gear.
    #[serde(default)]
    pub diff_ratio_by_gear: Vec<Param>,
    #[serde(default)]
    pub detents: Vec<Param>,
    #[serde(default)]
    pub diff_speed_rad_s: Option<Param>,
    #[serde(default)]
    pub works_in_neutral: bool,
    #[serde(default)]
    pub max_steer_torque_nm: Option<Param>,
}

impl TrackedExtras {
    pub fn visit_params(&self, f: &mut dyn FnMut(&str, &Param)) {
        for (n, p) in [
            ("sprocket_from_front_m", &self.sprocket_from_front_m),
            ("idler_from_front_m", &self.idler_from_front_m),
            ("sprocket_height_m", &self.sprocket_height_m),
            ("idler_height_m", &self.idler_height_m),
            ("idler_diameter_m", &self.idler_diameter_m),
            ("return_rollers", &self.return_rollers),
            ("roller_diameter_m", &self.roller_diameter_m),
            ("belt_thickness_m", &self.belt_thickness_m),
            ("sprocket_teeth", &self.sprocket_teeth),
            ("samples", &self.samples),
            ("tension_n", &self.tension_n),
            ("resist_c0", &self.resist_c0),
            ("road_wheel_width_m", &self.road_wheel_width_m),
            ("road_wheel_unsprung_kg", &self.road_wheel_unsprung_kg),
            ("road_wheel_inertia_kg_m2", &self.road_wheel_inertia_kg_m2),
            ("rigid_wheel_inertia_kg_m2", &self.rigid_wheel_inertia_kg_m2),
            ("wheel_contact_stiffness_n_m", &self.wheel_contact_stiffness_n_m),
            ("wheel_contact_damping_ns_m", &self.wheel_contact_damping_ns_m),
            ("torsion_arm_length_m", &self.torsion_arm_length_m),
            ("torsion_rest_angle_rad", &self.torsion_rest_angle_rad),
            ("belly_length_m", &self.belly_length_m),
            ("belly_thickness_m", &self.belly_thickness_m),
        ] {
            f(&format!("tracked.{n}"), p);
        }
        let l = &self.steer_law;
        for (i, p) in l.diff_ratio_by_gear.iter().enumerate() {
            f(&format!("tracked.steer_law.diff_ratio_by_gear[{i}]"), p);
        }
        for (i, p) in l.detents.iter().enumerate() {
            f(&format!("tracked.steer_law.detents[{i}]"), p);
        }
        for (n, p) in [("diff_speed_rad_s", &l.diff_speed_rad_s), ("max_steer_torque_nm", &l.max_steer_torque_nm)] {
            if let Some(p) = p {
                f(&format!("tracked.steer_law.{n}"), p);
            }
        }
    }
}

impl Extras {
    pub fn visit_params(&self, f: &mut dyn FnMut(&str, &Param)) {
        let (s, d, b) = (&self.susp, &self.drive, &self.brake);
        if let Some(t) = &self.tracked {
            t.visit_params(f);
        }
        if let Some(m) = &self.mass_budget {
            m.visit_params(f);
        }
        if let Some(l) = &self.tyre_load {
            f("tyre_load.mu_load_sensitivity", &l.mu_load_sensitivity);
            f("tyre_load.stiffness_load_sensitivity", &l.stiffness_load_sensitivity);
        }
        f("drive.centre_diff.bias", &d.centre_diff.bias);
        f("drive.axle_diff.bias", &d.axle_diff.bias);
        for (i, p) in b.axle_torque_nm.iter().enumerate() {
            f(&format!("brake.axle_torque_nm[{i}]"), p);
        }
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
