//! `VehicleDef`: the designer-level description of a vehicle, authored as RON in `content/vehicles/`. Every number is a [`Param`]
//! (value, band, provenance, source), so a reference vehicle that mimics a real one carries the evidence for each figure.
//! Lane FORGE compiles a `VehicleDef` into a [`crate::rig::PhysRig`] and a [`crate::render::RenderRig`]; lane GEOMETRY reads the
//! same parameters to shape the meshes; lane VALIDATION links it to a dossier through `reference`.
//!
//! This is the **v0 draft**: the shape of the sections is settled, the field list is expected to grow through CCRs from FORGE,
//! GEOMETRY and VALIDATION in the settling round. There are deliberately no `Default`s: a physical number is never implicit.
//!
//! **Contract 0.2** adds the wheeled sliders of FORGE's CCR (W1 to W6) as `Option`s or `#[serde(default)]` fields, so every 0.1 RON file still
//! loads; an absent optional means "FORGE derives it, or reports it missing". Tracked (T), gun and mount (G) and armour (A) sliders follow in a later version.
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
    // Datum (0.2, W5): the hull-frame origin is the centre of the hull box (`length_m` x `width_m` x `height_m`); +X right, +Y up, +Z
    // **back** (the nose is at z = -length/2). A position measured "from the front" converts as z = -length_m/2 + from_front.
    pub length_m: Param,
    pub width_m: Param,
    pub height_m: Param,
    /// Mass in the stated loading (combat or curb, with crew and fuel) excluding the turret and the unsprung mass, kg.
    pub mass_kg: Param,
    /// Centre-of-mass height above the ground at the design ride height, m (a height above the ground, not a hull-frame y: FORGE converts
    /// with the ride height, which puts the ground at y = -ride_height_m in the hull frame).
    pub com_height_m: Param,
    /// Centre-of-mass position measured back from the front of the hull, m (so hull-frame z = -length_m/2 + com_from_front_m).
    pub com_from_front_m: Param,
    pub ground_clearance_m: Param,
}

// A designer description is built once per vehicle, never per tick; boxing would be a breaking change for every user of the variants.
#[allow(clippy::large_enum_variant)]
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
    /// 0.2 (W4). Steering angle of the outer wheel at full lock, degrees (RON edge); only read when `steered`. `None` = FORGE's default for the kind.
    #[serde(default)]
    pub max_steer_deg: Option<Param>,
    /// 0.2 (W4). 0 = parallel steer, 1 = full Ackermann geometry. `None` = FORGE's default.
    #[serde(default)]
    pub ackermann: Option<Param>,
    /// 0.2 (W4). Tyres on each side of this axle: 1 single, 2 dual (a dual wheel is one station with two patches). Default 1.
    #[serde(default = "one_u8")]
    pub tyres_per_side: u8,
    /// 0.2 (W4). Lateral spacing between the two tyres of a dual, m; read only when `tyres_per_side` > 1.
    #[serde(default)]
    pub dual_spacing_m: Option<Param>,
    /// 0.2 (W4). Hub (wheel-end) reduction, input / output, >= 1 (military trucks, portal axles). `None` = 1.
    #[serde(default)]
    pub hub_reduction: Option<Param>,
    /// 0.2 (W4). Drop of the wheel centre below the axle shaft line for a portal axle, m. `None` = 0.
    #[serde(default)]
    pub portal_drop_m: Option<Param>,
    /// 0.2 (W4). How the two wheels of the axle are suspended. Default `Independent`.
    #[serde(default)]
    pub layout: AxleLayout,
    /// 0.2 (W4). This axle's share of the braking torque; shares are normalised over the axles. `None` = from `BrakesDef::front_share` (two axles).
    #[serde(default)]
    pub brake_share: Option<Param>,
    /// 0.2 (W4). Tyre of this axle where it differs from `WheeledDef::tyre`.
    #[serde(default)]
    pub tyre: Option<TyreSliders>,
    /// 0.2 (W4). Unsprung mass per wheel of this axle, kg, where it differs from the tyre's. `None` = the tyre's.
    #[serde(default)]
    pub unsprung_mass_kg: Option<Param>,
}

fn one_u8() -> u8 {
    1
}

/// How the wheels of an axle are connected to the hull (0.2, W4).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum AxleLayout {
    /// Each wheel on its own linkage (double wishbone, strut, swing arm).
    #[default]
    Independent,
    /// A live (solid) axle on springs `spring_track_m` apart; the wheels share the axle's roll.
    Solid { spring_track_m: Param },
    /// An axle in a tandem group of `group` axles on a walking beam pivoted at `pivot_frac` of the way between the group's outer axles.
    Tandem { group: u8, pivot_frac: Param },
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
    /// 0.2 (W1). Vertical stiffness of the tyre, N/m. `None` = FORGE derives it from inflation and size. (`patch_length_m` is derived, not a field:
    /// load / (inflation x width).)
    #[serde(default)]
    pub vertical_stiffness_n_m: Option<Param>,
    /// 0.2 (W1). Vertical damping of the tyre, N s/m. `None` = FORGE's default for the stiffness.
    #[serde(default)]
    pub vertical_damping_ns_m: Option<Param>,
    /// 0.2 (W1). Longitudinal slip stiffness, per unit load per unit slip ratio. `None` = FORGE's default.
    #[serde(default)]
    pub slip_stiffness: Option<Param>,
    /// 0.2 (W1). Relaxation length, m. `None` = FORGE's default.
    #[serde(default)]
    pub relaxation_length_m: Option<Param>,
    /// 0.2 (W1). Inertia of wheel and tyre about the axle, kg m^2. `None` = FORGE estimates it from mass and radius.
    #[serde(default)]
    pub wheel_inertia_kg_m2: Option<Param>,
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
    /// 0.2 (W2). Volute (conical) spring, as on tracked vehicles: a progressive rate.
    Volute,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuspensionSliders {
    pub kind: SuspensionKind,
    /// Natural frequency of the sprung mass on its suspension, Hz (cars 1-1.5, trucks 1.5-2.5, tanks 1.2-2). This is the **ride** frequency:
    /// the tyre's vertical stiffness is in series with the spring (0.2, W2), so FORGE solves for the spring rate that gives this ride frequency.
    pub front_ride_frequency_hz: Param,
    pub rear_ride_frequency_hz: Param,
    /// Damping ratio zeta of the sprung mass (0.2-0.4 typical): the **mean of bump and rebound** (0.2, W2); `rebound_to_bump` splits it.
    pub damping_ratio: Param,
    pub bump_travel_m: Param,
    pub droop_travel_m: Param,
    /// 0.2 (W2). Rebound damping coefficient / bump damping coefficient (typically 1.5 to 3), keeping the mean damping ratio. `None` = 1 (symmetric).
    #[serde(default)]
    pub rebound_to_bump: Option<Param>,
    /// 0.2 (W2). Fraction of `bump_travel_m` at which the bump stop starts to act (0..1). `None` = FORGE's default.
    #[serde(default)]
    pub bump_stop_engage_frac: Option<Param>,
    /// 0.2 (W2). Stiffness of the engaged bump stop, N/m. `None` = FORGE's default.
    #[serde(default)]
    pub bump_stop_rate_n_m: Option<Param>,
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
    /// 0.2 (W3). Torque at idle as a fraction of `peak_torque_nm` (the template curve's low end). `None` = FORGE's default for the kind.
    #[serde(default)]
    pub idle_torque_frac: Option<Param>,
    /// 0.2 (W3). Engine drag (friction and pumping) at zero speed, N m. `None` = FORGE's default.
    #[serde(default)]
    pub drag_const_nm: Option<Param>,
    /// 0.2 (W3). Added drag per rpm, N m per rpm. `None` = FORGE's default.
    #[serde(default)]
    pub drag_per_rpm_nm: Option<Param>,
    /// 0.2 (W3). Time constant of torque build-up after a throttle change, s. `None` = FORGE's default for the kind.
    #[serde(default)]
    pub response_time_s: Option<Param>,
    /// 0.2 (W3). Fuel burnt at idle, kg/s. `None` = derived from `bsfc_best_g_kwh`.
    #[serde(default)]
    pub idle_fuel_kg_s: Option<Param>,
    /// 0.2 (W3). A gas turbine with a free (power) turbine: the output shaft is not tied to the gas generator (M2).
    #[serde(default)]
    pub free_turbine: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CouplingSliders {
    Clutch,
    TorqueConverter { stall_ratio: Param, lockup: bool },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GearboxSliders {
    /// Explicit forward ratios (input / output). Authoritative when `design` is `None` (reference vehicles with published ratios); when
    /// `design` is present the compile generates them and this may be left empty.
    #[serde(default)]
    pub forward_ratios: Vec<Param>,
    #[serde(default)]
    pub reverse_ratios: Vec<Param>,
    pub efficiency: Param,
    pub automatic: bool,
    pub upshift_rpm: Param,
    pub downshift_rpm: Param,
    pub shift_time_s: Param,
    /// 0.2 (W6). The designer form of the gearbox (the "gear spread" slider). When present the compile generates `forward_ratios` and
    /// `PowertrainDef::final_drive_ratio` from it (FORGE design note section 2) and ignores the explicit values; when absent the explicit
    /// lists stay authoritative.
    #[serde(default)]
    pub design: Option<GearDesign>,
}

/// Inputs from which FORGE generates a gearbox and final drive (0.2, W6).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GearDesign {
    /// Number of forward gears.
    pub gears: u8,
    /// Steepest grade the first gear must climb at full torque, as rise over run (tan of the slope).
    pub launch_grade: Param,
    /// Speed at the redline in top gear, m/s.
    pub top_speed_m_s: Param,
    /// Overall ratio of the top gear (1 direct, < 1 overdrive); the final drive takes up the rest.
    pub top_gear_ratio: Param,
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
                visit_tyre("running_gear.tyre", &w.tyre, f);
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
        self.visit_optional_params(f);
        let s = &self.suspension;
        each!(
            "suspension",
            s,
            front_ride_frequency_hz,
            rear_ride_frequency_hz,
            damping_ratio,
            bump_travel_m,
            droop_travel_m
        );
        let p = &self.powertrain;
        let e = &p.engine;
        each!(
            "powertrain.engine",
            e,
            peak_power_w,
            peak_power_rpm,
            peak_torque_nm,
            peak_torque_rpm,
            idle_rpm,
            redline_rpm,
            inertia_kg_m2,
            bsfc_best_g_kwh
        );
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

    /// The 0.2 optional sliders that are present (kept apart from `visit_params` so the 0.1 list reads as before).
    fn visit_optional_params(&self, f: &mut dyn FnMut(&str, &Param)) {
        fn opt(f: &mut dyn FnMut(&str, &Param), prefix: &str, name: &str, p: Option<&Param>) {
            if let Some(p) = p {
                f(&format!("{prefix}.{name}"), p);
            }
        }
        if let RunningGearDef::Wheeled(w) = &self.running_gear {
            for (i, a) in w.axles.iter().enumerate() {
                let pre = format!("running_gear.axles[{i}]");
                opt(f, &pre, "max_steer_deg", a.max_steer_deg.as_ref());
                opt(f, &pre, "ackermann", a.ackermann.as_ref());
                opt(f, &pre, "dual_spacing_m", a.dual_spacing_m.as_ref());
                opt(f, &pre, "hub_reduction", a.hub_reduction.as_ref());
                opt(f, &pre, "portal_drop_m", a.portal_drop_m.as_ref());
                opt(f, &pre, "brake_share", a.brake_share.as_ref());
                opt(f, &pre, "unsprung_mass_kg", a.unsprung_mass_kg.as_ref());
                match &a.layout {
                    AxleLayout::Independent => {}
                    AxleLayout::Solid { spring_track_m } => opt(f, &pre, "layout.spring_track_m", Some(spring_track_m)),
                    AxleLayout::Tandem { pivot_frac, .. } => opt(f, &pre, "layout.pivot_frac", Some(pivot_frac)),
                }
                if let Some(t) = &a.tyre {
                    visit_tyre(&format!("{pre}.tyre"), t, f);
                }
            }
        }
        let su = &self.suspension;
        opt(f, "suspension", "rebound_to_bump", su.rebound_to_bump.as_ref());
        opt(f, "suspension", "bump_stop_engage_frac", su.bump_stop_engage_frac.as_ref());
        opt(f, "suspension", "bump_stop_rate_n_m", su.bump_stop_rate_n_m.as_ref());
        let en = &self.powertrain.engine;
        opt(f, "powertrain.engine", "idle_torque_frac", en.idle_torque_frac.as_ref());
        opt(f, "powertrain.engine", "drag_const_nm", en.drag_const_nm.as_ref());
        opt(f, "powertrain.engine", "drag_per_rpm_nm", en.drag_per_rpm_nm.as_ref());
        opt(f, "powertrain.engine", "response_time_s", en.response_time_s.as_ref());
        opt(f, "powertrain.engine", "idle_fuel_kg_s", en.idle_fuel_kg_s.as_ref());
        if let Some(d) = &self.powertrain.gearbox.design {
            opt(f, "powertrain.gearbox.design", "launch_grade", Some(&d.launch_grade));
            opt(f, "powertrain.gearbox.design", "top_speed_m_s", Some(&d.top_speed_m_s));
            opt(f, "powertrain.gearbox.design", "top_gear_ratio", Some(&d.top_gear_ratio));
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
            RunningGearDef::Wheeled(w) if w.axles.len() < 2 => {
                errs.push("a wheeled vehicle needs at least two axles".into())
            }
            RunningGearDef::Tracked(t) if t.road_wheels_per_side < 2 => {
                errs.push("a tracked vehicle needs at least two road wheels per side".into())
            }
            _ => {}
        }
        let gb = &self.powertrain.gearbox;
        if gb.forward_ratios.is_empty() && gb.design.is_none() {
            errs.push("the gearbox needs at least one forward ratio (or a `design`)".into());
        }
        if gb.design.as_ref().is_some_and(|d| d.gears == 0) {
            errs.push("gearbox.design.gears must be at least 1".into());
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

/// Every `Param` of a tyre slider set (required and present optional ones) under `prefix`.
fn visit_tyre(prefix: &str, t: &TyreSliders, f: &mut dyn FnMut(&str, &Param)) {
    let mut one = |name: &str, p: &Param| f(&format!("{prefix}.{name}"), p);
    one("outer_diameter_m", &t.outer_diameter_m);
    one("section_width_m", &t.section_width_m);
    one("inflation_pa", &t.inflation_pa);
    one("mu_peak_ref", &t.mu_peak_ref);
    one("cornering_stiffness_per_rad", &t.cornering_stiffness_per_rad);
    one("rolling_coeff", &t.rolling_coeff);
    one("unsprung_mass_kg", &t.unsprung_mass_kg);
    let optional = [
        ("vertical_stiffness_n_m", &t.vertical_stiffness_n_m),
        ("vertical_damping_ns_m", &t.vertical_damping_ns_m),
        ("slip_stiffness", &t.slip_stiffness),
        ("relaxation_length_m", &t.relaxation_length_m),
        ("wheel_inertia_kg_m2", &t.wheel_inertia_kg_m2),
    ];
    for (name, p) in optional {
        if let Some(p) = p {
            one(name, p);
        }
    }
}
