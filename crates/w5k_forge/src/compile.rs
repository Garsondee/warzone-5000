//! The wheeled compile: `VehicleDef` (+ PROVISIONAL extras) to `PhysRig`, `RenderRig` and a report of what each slider became.
//! Formulas and rejection rules: `docs/lanes/forge/design-note.md`. Rejections carry a reason; nothing is silently clamped.

use std::fmt;

use w5k_contract::combat::CombatDef;
use w5k_contract::def::{CouplingSliders, RunningGearDef, SuspensionKind, VehicleDef, WheeledDef};
use w5k_contract::param::Param;
use w5k_contract::rig::*;
use w5k_math::{scalar, Transform, Vec3};

use crate::budget::{hull_body, inertia_report, sprung, MassItem};
use crate::curve::torque_curve_through_peaks;
use crate::extras::{DiffSpec, Extras};

/// The design tripwire: a rig that needs more substeps than this is a numerically unstable design.
pub(crate) const MAX_SUBSTEPS: u32 = 8; // const-ok: lane tripwire from the CHASSIS and DRIVE briefs
const RAD_PER_DEG: f64 = scalar::PI / 180.0; // const-ok: unit conversion
pub(crate) const J_PER_KJ: f64 = 1e3; // const-ok: unit conversion

#[derive(Clone, Debug, PartialEq)]
pub struct Rejection {
    pub field: String,
    pub reason: String,
}

impl fmt::Display for Rejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.reason)
    }
}

/// A slider that contract 0.2 made optional: absent means the design is incomplete, which is a rejection with the field's name.
pub(crate) fn need(p: &Option<Param>, field: &str) -> Result<f64, String> {
    p.as_ref()
        .map(|p| p.v)
        .ok_or_else(|| format!("{field} is missing: the compile derives nothing for it, state it in the def"))
}

/// The contract's `bias`: the torque-bias ratio of a limited slip, 0 for the other kinds (as the stand-in rigs write them).
fn diff_bias(d: &DiffSpec) -> f64 {
    if d.kind == DiffKind::LimitedSlip {
        d.bias.v
    } else {
        0.0
    }
}

pub(crate) fn rej(field: &str, reason: impl Into<String>) -> Vec<Rejection> {
    vec![Rejection { field: field.into(), reason: reason.into() }]
}

pub struct Compiled {
    pub rig: PhysRig,
    /// One line per slider or derived quantity: what it became.
    pub report: Vec<String>,
    /// Hull box size (width, height, length), m: the render rig builder needs it.
    pub hull_size_m: Vec3,
    /// The lumps of the sprung mass (structure, engine, ...): empty when the extras carry no mass budget.
    pub mass_items: Vec<MassItem>,
}

/// Static vertical load on each axle from statics: loads are linear in the axle position, `N_i = a + b z_i`, with `sum N = W` and
/// `sum N z = W z_com` (the minimum-norm solution; exact for two axles).
pub(crate) fn axle_loads(z: &[f64], z_com: f64, weight: f64) -> Result<Vec<f64>, String> {
    let (n, sz, szz) = (z.len() as f64, z.iter().sum::<f64>(), z.iter().map(|v| v * v).sum::<f64>());
    let det = n * szz - sz * sz;
    if det.abs() < 1e-9 {
        // const-ok: coincident axles
        return Err("the axles are at the same position".into());
    }
    let (a, b) = ((weight * szz - sz * weight * z_com) / det, (n * weight * z_com - sz * weight) / det);
    let loads: Vec<f64> = z.iter().map(|zi| a + b * zi).collect();
    if let Some(i) = loads.iter().position(|&l| l <= 0.0) {
        return Err(format!("statics put {:.0} N on axle {i}: the COM lies outside the axles' support", loads[i]));
    }
    Ok(loads)
}

pub fn compile(def: &VehicleDef, ex: &Extras) -> Result<Compiled, Vec<Rejection>> {
    let mut errs: Vec<Rejection> = Vec::new();
    if let Err(e) = def.check() {
        errs.extend(e.into_iter().map(|m| Rejection { field: "def".into(), reason: m }));
    }
    errs.extend(ex.check().into_iter().map(|m| Rejection { field: "extras".into(), reason: m }));
    if !errs.is_empty() {
        return Err(errs);
    }
    match &def.running_gear {
        RunningGearDef::Wheeled(w) => wheeled(def, w, ex),
        RunningGearDef::Tracked(t) => crate::tracked::tracked(def, t, ex),
    }
    .map_err(|e| rej("compile", e))
}

fn wheeled(def: &VehicleDef, w: &WheeledDef, ex: &Extras) -> Result<Compiled, String> {
    let (h, ty, su, pt) = (&def.hull, &w.tyre, &def.suspension, &def.powertrain);
    let g = scalar::G;
    let mut report = Vec::new();
    if su.kind != SuspensionKind::Coil {
        report.push(format!(
            "suspension kind {:?} is modelled as a linear spring (the kinds arrive with their solver support)",
            su.kind
        ));
    }

    // ---- frame: datum at the hull box centre, +Z back (the front of the hull is at z = -L/2)
    let z_of = |from_front: f64| from_front - 0.5 * h.length_m.v;
    let ride_height = h.ground_clearance_m.v + 0.5 * h.height_m.v;
    let axle_z: Vec<f64> = w.axles.iter().map(|a| z_of(a.from_front_m.v)).collect();
    let drive_z: Vec<f64> = w.axles.iter().zip(&axle_z).filter(|(a, _)| a.driven).map(|(_, z)| *z).collect();
    let sprung_mass = sprung(def, ex, ride_height, &drive_z)?;
    let (m_sprung, com) = (sprung_mass.mass_kg, sprung_mass.com_m);
    let loads = axle_loads(&axle_z, com.z, m_sprung * g)?;
    report.push(format!(
        "datum: hull box centre; ride height {ride_height:.3} m; COM {:.3} m above the ground, {:.3} m from the front",
        h.com_height_m.v, h.com_from_front_m.v
    ));

    // ---- geometry of obstacles: what ground clearance changes (the hull proxy's underside is exactly `ground_clearance_m` above the ground)
    let (a_first, a_last) = (w.axles[0].from_front_m.v, w.axles[w.axles.len() - 1].from_front_m.v);
    let tyre_r = 0.5 * w.tyre.outer_diameter_m.v;
    let clear = h.ground_clearance_m.v;
    let to_deg = 180.0 / scalar::PI; // const-ok: radians to degrees for display
    let angle = |overhang: f64| if overhang <= 0.0 { 90.0 } else { scalar::atan2(clear, overhang) * to_deg }; // const-ok: tyre-limited
    let wheelbase = a_last - a_first;
    report.push(format!(
        "obstacle geometry from the {clear:.2} m clearance: approach {:.0} deg, departure {:.0} deg (90 = the tyre limits), ramp breakover {:.0} deg",
        angle(a_first - tyre_r),
        angle(h.length_m.v - a_last - tyre_r),
        2.0 * scalar::atan2(2.0 * clear, wheelbase) * to_deg
    ));

    // ---- stations, tyres, springs
    let r = 0.5 * ty.outer_diameter_m.v;
    let tyre_k = need(&ty.vertical_stiffness_n_m, "running_gear.tyre.vertical_stiffness_n_m")?;
    let tyre_c = need(&ty.vertical_damping_ns_m, "running_gear.tyre.vertical_damping_ns_m")?;
    let slip_k = need(&ty.slip_stiffness, "running_gear.tyre.slip_stiffness")?;
    let relax = need(&ty.relaxation_length_m, "running_gear.tyre.relaxation_length_m")?;
    let wheel_j = need(&ty.wheel_inertia_kg_m2, "running_gear.tyre.wheel_inertia_kg_m2")?;
    let rebound = need(&su.rebound_to_bump, "suspension.rebound_to_bump")?;
    let engage = need(&su.bump_stop_engage_frac, "suspension.bump_stop_engage_frac")?;
    let stop_rate = need(&su.bump_stop_rate_n_m, "suspension.bump_stop_rate_n_m")?;
    let (k_t, e) = (tyre_k, &ex.susp);
    let mut stations = Vec::new();
    let mut anti_roll = Vec::new();
    let mut omega_max: f64 = 0.0;
    let mut k_series_sum = 0.0;
    for (ai, axle) in w.axles.iter().enumerate() {
        let m_corner = 0.5 * loads[ai] / g;
        let f_hz = if ai == 0 { su.front_ride_frequency_hz.v } else { su.rear_ride_frequency_hz.v };
        // The slider is the RIDE frequency: the sprung mass on the suspension in series with the tyre (decision D1).
        let k_ride = m_corner * (scalar::TAU * f_hz) * (scalar::TAU * f_hz);
        if k_ride >= k_t {
            return Err(format!(
                "axle {ai}: the ride rate {k_ride:.0} N/m is not below the tyre stiffness {k_t:.0} N/m"
            ));
        }
        let k_spring = k_ride * k_t / (k_t - k_ride);
        let c_mean = 2.0 * su.damping_ratio.v * scalar::sqrt(k_ride * m_corner); // mean of bump and rebound (D2)
        let rr = rebound;
        let (c_bump, c_reb) = (2.0 * c_mean / (1.0 + rr), 2.0 * c_mean * rr / (1.0 + rr));
        let load = (m_corner + ty.unsprung_mass_kg.v) * g;
        let deflection = load / k_t;
        let patch = load / (ty.inflation_pa.v * ty.section_width_m.v); // contact area = load / pressure
        let centre_above_ground = r - deflection;
        let half_track = 0.5 * axle.track_width_m.v;
        let m_u = ty.unsprung_mass_kg.v;
        // The stiffest mode: the unsprung mass on the tyre in parallel with the strut and the engaged bump stop (the slip mode is excluded).
        // The stop's incremental stiffness at full bump travel (d/dx of rate x (1 + p x / engage) x), as CHASSIS' mode analysis reads it.
        let (engage_m, pen) = (engage * su.bump_travel_m.v, su.bump_travel_m.v * (1.0 - engage));
        let k_stop = stop_rate * (1.0 + 2.0 * e.bump_stop_progression.v * pen / engage_m);
        let hop = scalar::sqrt((k_t + k_spring + k_stop) / m_u);
        k_series_sum += 2.0 * k_spring * k_t / (k_spring + k_t); // two wheels per axle
        omega_max = omega_max.max(hop);
        let steer = if axle.steered {
            let deg = need(&axle.max_steer_deg, &format!("running_gear.axles[{ai}].max_steer_deg"))?;
            let ackermann = need(&axle.ackermann, &format!("running_gear.axles[{ai}].ackermann"))?;
            Some(SteerDef { max_angle_rad: deg * RAD_PER_DEG, ackermann })
        // const-ok: degrees to radians
        } else {
            None
        };
        let names = ["l", "r"];
        for (k, side) in [Side::Left, Side::Right].into_iter().enumerate() {
            let x = if side == Side::Left { -half_track } else { half_track };
            stations.push(StationDef {
                name: format!("a{ai}{}", names[k]),
                side,
                axle: ai as u8,
                rest_pos_m: Vec3::new(x, centre_above_ground - ride_height, axle_z[ai]),
                bump_dir: Vec3::Y,
                bump_travel_m: su.bump_travel_m.v,
                droop_travel_m: su.droop_travel_m.v,
                unsprung_mass_kg: m_u,
                suspension: SuspensionDef {
                    spring: SpringKind::Linear { rate_n_m: k_spring },
                    preload_n: m_corner * g,
                    damper: DamperDef {
                        bump_ns_m: c_bump,
                        rebound_ns_m: c_reb,
                        knee_speed_m_s: 0.0,
                        post_knee_ratio: 1.0,
                        friction_n: 0.0,
                    },
                    bump_stop: BumpStopDef {
                        engage_m: engage * su.bump_travel_m.v,
                        rate_n_m: stop_rate,
                        progression: e.bump_stop_progression.v,
                        damping_ns_m: e.bump_stop_damping_ns_m.v,
                        hard_limit: false,
                        restitution: 0.0,
                    },
                },
                steer: steer.clone(),
                wheel: WheelDef {
                    kind: WheelKind::Tyre,
                    radius_m: r,
                    width_m: ty.section_width_m.v,
                    inertia_kg_m2: wheel_j,
                    tyre: Some(TyreDef {
                        vertical_stiffness_n_m: k_t,
                        vertical_damping_ns_m: tyre_c,
                        mu_scale: ty.mu_peak_ref.v / ex.ref_surface_mu_peak.v,
                        slip_stiffness: slip_k,
                        cornering_stiffness_per_rad: ty.cornering_stiffness_per_rad.v,
                        relaxation_length_m: relax,
                        rolling_coeff: ty.rolling_coeff.v,
                        inflation_pa: ty.inflation_pa.v,
                        patch_length_m: patch,
                        speed_floor_m_s: 0.0,
                        aligning_trail_frac: 0.0,
                        kappa_peak: 0.0,
                        alpha_peak_rad: 0.0,
                        mu_load_sensitivity: ex.tyre_load.as_ref().map_or(0.0, |l| l.mu_load_sensitivity.v),
                        stiffness_load_sensitivity: ex
                            .tyre_load
                            .as_ref()
                            .map_or(0.0, |l| l.stiffness_load_sensitivity.v),
                        nominal_load_n: 0.0, // 0 = the solver uses the tyre's static load
                    }),
                    patches_x_m: vec![],
                },
                drive_output: None,
                arm_pivot_m: None,
            });
        }
        if let Some(arb) = &axle.anti_roll_n_m {
            anti_roll.push(AntiRollDef { left_station: 2 * ai, right_station: 2 * ai + 1, rate_n_m: arb.v });
        }
        report.push(format!(
            "axle {ai}: load {:.0} N; sprung {m_corner:.0} kg/wheel; f {f_hz} Hz -> ride rate {k_ride:.0}, spring {k_spring:.0} N/m (tyre {k_t:.0}); zeta {} -> damper {c_bump:.0} bump / {c_reb:.0} rebound N s/m; tyre deflection {:.1} mm, patch {:.0} mm",
            loads[ai],
            su.damping_ratio.v,
            deflection * 1e3, // const-ok: m to mm for display
            patch * 1e3       // const-ok: m to mm for display
        ));
    }
    // The hull's heave on the series spring-tyre rates is the other candidate for the stiffest mode.
    omega_max = omega_max.max(scalar::sqrt(k_series_sum / m_sprung));
    let f_max_hz = omega_max / scalar::TAU;
    // The contract's rule: `substeps * TICK_HZ >= SAMPLES_PER_PERIOD * f_max`.
    let substeps = ((SAMPLES_PER_PERIOD * f_max_hz / TICK_HZ).ceil() as u32).max(1);
    if substeps > MAX_SUBSTEPS {
        return Err(format!("numerically unstable design: the stiffest mode ({f_max_hz:.1} Hz) needs {substeps} substeps per tick, above {MAX_SUBSTEPS}"));
    }
    report.push(format!(
        "stiffest mode {f_max_hz:.1} Hz (wheel hop with the bump stop fully engaged at full bump travel) -> {substeps} substeps per tick"
    ));

    let (hull, size) = hull_body(h, &sprung_mass);
    report.push(inertia_report(&hull, sprung_mass.items.len().saturating_sub(1), size));
    let total_mass = m_sprung + ty.unsprung_mass_kg.v * stations.len() as f64;

    let (engine, coupling, gearbox) = powertrain_parts(def, ex)?;
    let en = &pt.engine;
    let dx = &ex.drive;
    // Driveline: an open differential per driven axle (final drive), under an open centre differential when several axles drive.
    let mut outputs = Vec::new();
    let mut axle_diffs = Vec::new();
    for (ai, axle) in w.axles.iter().enumerate().filter(|(_, a)| a.driven) {
        let first = outputs.len();
        for s in [2 * ai, 2 * ai + 1] {
            stations[s].drive_output = Some(outputs.len());
            outputs.push(OutputDef { station: s, final_drive_ratio: 1.0, efficiency: pt.driveline_efficiency.v });
        }
        let _ = axle;
        axle_diffs.push(DriveNode::Diff {
            kind: dx.axle_diff.kind,
            ratio: pt.final_drive_ratio.v,
            bias: diff_bias(&dx.axle_diff),
            split: vec![],
            efficiency: 1.0,
            children: vec![DriveNode::Output(first), DriveNode::Output(first + 1)],
        });
    }
    let driveline = match axle_diffs.len() {
        0 => return Err("no axle is driven".into()),
        1 => axle_diffs.remove(0),
        _ => DriveNode::Diff {
            kind: dx.centre_diff.kind,
            ratio: pt.transfer_case_ratio.as_ref().map_or(1.0, |p| p.v),
            bias: diff_bias(&dx.centre_diff),
            split: vec![],
            efficiency: 1.0,
            children: axle_diffs,
        },
    };

    // ---- brakes: authored axle torque when the extras carry it (the deceleration is then derived), else sized from the design deceleration
    let br = &def.brakes;
    let b = &ex.brake;
    let axle_torque: Vec<f64> = if b.axle_torque_nm.is_empty() {
        if br.service_decel_g.v > ty.mu_peak_ref.v {
            return Err(format!(
                "service deceleration {} g exceeds the tyre's peak friction {}: the tyre cannot deliver it",
                br.service_decel_g.v, ty.mu_peak_ref.v
            ));
        }
        let force = total_mass * br.service_decel_g.v * g;
        let share: Vec<f64> = if w.axles.len() == 2 {
            vec![br.front_share.v, 1.0 - br.front_share.v]
        } else {
            loads.iter().map(|l| l / (m_sprung * g)).collect()
        };
        share.iter().map(|sh| sh * force * r).collect()
    } else if b.axle_torque_nm.len() == w.axles.len() {
        b.axle_torque_nm.iter().map(|p| p.v).collect()
    } else {
        return Err(format!("brake.axle_torque_nm has {} entries for {} axles", b.axle_torque_nm.len(), w.axles.len()));
    };
    let derived_decel_g = axle_torque.iter().sum::<f64>() / r / total_mass / g;
    if derived_decel_g > ty.mu_peak_ref.v {
        report.push(format!(
            "WARNING: the brake torque would give {derived_decel_g:.2} g, above the tyre's peak friction {}: the tyre limits the stop",
            ty.mu_peak_ref.v
        ));
    }
    let share0 = axle_torque[0] / axle_torque.iter().sum::<f64>();
    let brakes: Vec<BrakeDef> = (0..stations.len())
        .map(|s| {
            let ai = s / 2;
            BrakeDef {
                station: s,
                max_torque_nm: axle_torque[ai] / 2.0,
                thermal_mass_j_k: br.thermal_mass_kj_k.v * J_PER_KJ, // const-ok: kJ to J
                cooling_w_k: b.cooling_w_k.v,
                cooling_per_ms_w_k: b.cooling_per_ms_w_k.v,
                fade_start_k: b.fade_start_k.v,
                fade_end_k: b.fade_end_k.v,
                fade_floor: b.fade_floor.v,
                parking: br.parking_brake && ai + 1 == w.axles.len(),
                service: true,
                location: BrakeLocation::AtStation,
                site: BrakeSite::Wheel,
                steering: false,
                reverse_torque_factor: 1.0,
                apply_time_s: 0.0,
                release_time_s: 0.0,
                circuit: 0,
            }
        })
        .collect();
    report.push(format!(
        "brakes: axle torque {:.0} N m front, {:.0} N m rear -> {derived_decel_g:.2} g on {total_mass:.0} kg (front share {share0:.2})",
        axle_torque[0],
        axle_torque[axle_torque.len() - 1]
    ));
    report.push(format!(
        "engine: curve through {} N m at {} rpm and {} kW at {} rpm; {} forward gears",
        en.peak_torque_nm.v,
        en.peak_torque_rpm.v,
        en.peak_power_w.v / J_PER_KJ, // const-ok: W to kW for display
        en.peak_power_rpm.v,
        gearbox.forward_ratios.len()
    ));

    let rig = PhysRig {
        id: def.id.clone(),
        ride_height_m: ride_height,
        hull,
        stations,
        linkages: vec![],
        anti_roll,
        tracks: vec![],
        drivetrain: DrivetrainDef {
            engine,
            coupling,
            gearbox,
            driveline,
            outputs,
            brakes,
            modes: vec![],
            default_mode: 0,
        },
        articulation: vec![],
        aero: AeroDef {
            drag_coeff: def.aero.drag_coeff.v,
            frontal_area_m2: def.aero.frontal_area_m2.v,
            centre_of_pressure_m: Vec3::new(0.0, ex.aero_cop_height_m.v, 0.0),
        },
        proxies: vec![CollisionProxy {
            name: "hull".into(),
            shape: ProxyShape::Box { half_m: 0.5 * size },
            pose: Transform::IDENTITY,
            attached_to: None,
            attached_station: None,
            role: ProxyRole::Hull,
        }],
        muzzles: vec![],
        combat: CombatDef::default(),
        integration: IntegrationDef { substeps, f_max_hz: Some(f_max_hz) },
    };
    rig.validate().map_err(|e| format!("the compiled rig fails PhysRig::validate(): {e:?}"))?;
    Ok(Compiled { rig, report, hull_size_m: size, mass_items: sprung_mass.items })
}

/// Engine, coupling and gearbox from the sliders (shared by the wheeled and tracked compiles).
pub(crate) fn powertrain_parts(def: &VehicleDef, ex: &Extras) -> Result<(EngineDef, CouplingDef, GearboxDef), String> {
    let pt = &def.powertrain;
    let en = &pt.engine;
    let curve = torque_curve_through_peaks(
        en.peak_torque_nm.v,
        en.peak_torque_rpm.v,
        en.peak_power_w.v,
        en.peak_power_rpm.v,
        en.idle_rpm.v,
        en.redline_rpm.v,
        need(&en.idle_torque_frac, "powertrain.engine.idle_torque_frac")?,
    )?;
    let engine = EngineDef {
        kind: en.kind,
        torque_curve: curve,
        idle_rpm: en.idle_rpm.v,
        redline_rpm: en.redline_rpm.v,
        inertia_kg_m2: en.inertia_kg_m2.v,
        drag_const_nm: need(&en.drag_const_nm, "powertrain.engine.drag_const_nm")?,
        drag_per_rpm_nm: need(&en.drag_per_rpm_nm, "powertrain.engine.drag_per_rpm_nm")?,
        bsfc_best_g_kwh: en.bsfc_best_g_kwh.v,
        free_output: false,
        response_time_s: 0.0,
        idle_fuel_kg_s: 0.0,
    };
    let dx = &ex.drive;
    let coupling = match &pt.coupling {
        CouplingSliders::Clutch => CouplingDef::Clutch {
            max_torque_nm: dx.clutch_capacity_factor.v * en.peak_torque_nm.v,
            engage_rpm: dx.clutch_engage_rpm.v,
        },
        CouplingSliders::TorqueConverter { stall_ratio, lockup } => CouplingDef::TorqueConverter {
            stall_ratio: stall_ratio.v,
            k_factor_rpm_per_sqrt_nm: dx.converter_k_factor_rpm_per_sqrt_nm.v,
            lockup_speed_ratio: lockup.then_some(dx.converter_lockup_speed_ratio.v),
        },
    };
    let gb = &pt.gearbox;
    let gearbox = GearboxDef {
        forward_ratios: gb.forward_ratios.iter().map(|p| p.v).collect(),
        reverse_ratios: gb.reverse_ratios.iter().map(|p| p.v).collect(),
        efficiency: gb.efficiency.v,
        inertia_kg_m2: dx.gearbox_inertia_kg_m2.v,
        shift: ShiftDef {
            automatic: gb.automatic,
            upshift_rpm: gb.upshift_rpm.v,
            downshift_rpm: gb.downshift_rpm.v,
            shift_time_s: gb.shift_time_s.v,
        },
    };
    Ok((engine, coupling, gearbox))
}

/// Read a `VehicleDef` from RON text.
pub fn parse_def(text: &str) -> Result<VehicleDef, String> {
    ron::from_str(text).map_err(|e| format!("VehicleDef RON: {e}"))
}

/// Read the PROVISIONAL extras sidecar from RON text.
pub fn parse_extras(text: &str) -> Result<Extras, String> {
    ron::from_str(text).map_err(|e| format!("extras RON: {e}"))
}
