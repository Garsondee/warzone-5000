//! The tracked compile (design note `docs/lanes/forge/tracked-design-note.md`, T1): stations in loop order, the `TrackDef` of each side, the
//! road-wheel springs, the steer unit and the sprocket brakes. The render rig follows in T3.

use w5k_contract::combat::CombatDef;
use w5k_contract::def::{SuspensionKind, TrackedDef, VehicleDef};
use w5k_contract::rig::*;
use w5k_math::{scalar, Transform, Vec3};

use crate::budget::{hull_body, sprung};
use crate::compile::{axle_loads, need, powertrain_parts, Compiled, J_PER_KJ, MAX_SUBSTEPS};
use crate::extras::Extras;

const PITCH_TOLERANCE: f64 = 0.01; // const-ok: coherence tolerance between the sprocket diameter and its teeth count
const HULL_SAMPLES: usize = 720; // const-ok: points per circle for the belt-length hull (error about 3e-6 relative)

/// Perimeter of the convex hull of circles `(u, v, radius)`: the length of a taut belt round them (monotone chain over a dense polygon).
pub fn belt_perimeter(circles: &[(f64, f64, f64)]) -> f64 {
    let mut pts: Vec<(f64, f64)> = Vec::new();
    for &(u, v, r) in circles {
        for k in 0..HULL_SAMPLES {
            let a = scalar::TAU * k as f64 / HULL_SAMPLES as f64;
            pts.push((u + r * scalar::cos(a), v + r * scalar::sin(a)));
        }
    }
    pts.sort_by(|p, q| p.0.total_cmp(&q.0).then(p.1.total_cmp(&q.1)));
    let cross = |o: (f64, f64), a: (f64, f64), b: (f64, f64)| (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0);
    let mut hull: Vec<(f64, f64)> = Vec::new();
    for pass in 0..2 {
        let start = hull.len();
        let iter: Box<dyn Iterator<Item = &(f64, f64)>> =
            if pass == 0 { Box::new(pts.iter()) } else { Box::new(pts.iter().rev()) };
        for &p in iter {
            while hull.len() >= start + 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0 {
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
    }
    (0..hull.len())
        .map(|i| scalar::hypot(hull[(i + 1) % hull.len()].0 - hull[i].0, hull[(i + 1) % hull.len()].1 - hull[i].1))
        .sum()
}

pub(crate) fn tracked(def: &VehicleDef, t: &TrackedDef, ex: &Extras) -> Result<Compiled, String> {
    let tx = ex.tracked.as_ref().ok_or("a tracked def needs the `tracked` section of its extras sidecar")?;
    let (h, su, pt) = (&def.hull, &def.suspension, &def.powertrain);
    let g = scalar::G;
    let mut report = Vec::new();

    // ---- frame, as in the wheeled compile (datum at the hull box centre, +Z back)
    let z_of = |from_front: f64| from_front - 0.5 * h.length_m.v;
    let ride_height = h.ground_clearance_m.v + 0.5 * h.height_m.v;
    let (n, d_rw, th) = (usize::from(t.road_wheels_per_side), t.road_wheel_diameter_m.v, tx.belt_thickness_m.v);
    let r_rw = 0.5 * d_rw;
    let (z_spr, z_idl) = (z_of(tx.sprocket_from_front_m.v), z_of(tx.idler_from_front_m.v));
    let dir = if z_spr < z_idl { -1.0 } else { 1.0 }; // the ground run goes from the idler end toward the sprocket end
    let spacing = (t.ground_contact_length_m.v - d_rw) / (n - 1) as f64;
    if spacing < d_rw {
        return Err(format!("road wheels overlap: spacing {spacing:.3} m is below the wheel diameter {d_rw:.3} m for a contact length of {:.2} m", t.ground_contact_length_m.v));
    }
    let z_c = 0.5 * (z_spr + z_idl);
    let z_road: Vec<f64> = (0..n).map(|k| z_c + dir * (k as f64 - 0.5 * (n - 1) as f64) * spacing).collect();
    let sprung_mass = sprung(def, ex, ride_height, &[z_spr, z_spr])?; // one final drive per sprocket
    let (m_sprung, com) = (sprung_mass.mass_kg, sprung_mass.com_m);
    let loads = axle_loads(&z_road, com.z, m_sprung * g)?;

    // ---- the wheels of one side in loop order: (kind, name, z, centre height above ground, radius)
    let teeth = tx.sprocket_teeth.v.round();
    let r_spr = 0.5 * t.sprocket_diameter_m.v;
    let r_pitch = t.pitch_m.v / (2.0 * scalar::sin(scalar::PI / teeth));
    if (r_pitch - r_spr).abs() > PITCH_TOLERANCE * r_spr {
        return Err(format!("the sprocket pitch radius {r_pitch:.4} m from {teeth} teeth at pitch {:.3} m disagrees with the diameter {:.3} m (1%)", t.pitch_m.v, t.sprocket_diameter_m.v));
    }
    let (r_idl, y_spr, y_idl) = (0.5 * tx.idler_diameter_m.v, tx.sprocket_height_m.v, tx.idler_height_m.v);
    let rollers = tx.return_rollers.v.round() as usize;
    let r_roll = 0.5 * tx.roller_diameter_m.v;
    // Rollers touch the belt line from the idler's top to the sprocket's top (belt thickness allowed for).
    let (top_spr, top_idl) = (y_spr + r_spr + th, y_idl + r_idl + th);
    let mut wheels: Vec<(WheelKind, String, f64, f64, f64)> =
        vec![(WheelKind::Sprocket, "spr".into(), z_spr, y_spr, r_spr)];
    for j in 0..rollers {
        let f = (j + 1) as f64 / (rollers + 1) as f64;
        let line = top_spr + f * (top_idl - top_spr);
        wheels.push((
            WheelKind::ReturnRoller,
            format!("ret{j}"),
            z_spr + f * (z_idl - z_spr),
            line - th - r_roll,
            r_roll,
        ));
    }
    wheels.push((WheelKind::Idler, "idl".into(), z_idl, y_idl, r_idl));
    for (k, z) in z_road.iter().enumerate() {
        wheels.push((WheelKind::RoadWheel, format!("r{}", k + 1), *z, r_rw + th, r_rw));
    }
    let per_side = wheels.len();

    // ---- stations
    let (fa, fb) = (su.front_ride_frequency_hz.v, su.rear_ride_frequency_hz.v);
    let (rebound, engage, stop_rate) = (
        need(&su.rebound_to_bump, "suspension.rebound_to_bump")?,
        need(&su.bump_stop_engage_frac, "suspension.bump_stop_engage_frac")?,
        need(&su.bump_stop_rate_n_m, "suspension.bump_stop_rate_n_m")?,
    );
    let (k_wc, m_u) = (tx.wheel_contact_stiffness_n_m.v, tx.road_wheel_unsprung_kg.v);
    let (mut omega_max, mut k_series_sum) = (0.0f64, 0.0f64);
    let mut stations = Vec::new();
    for (side, sx, prefix) in [(Side::Left, -0.5 * t.track_gauge_m.v, "l"), (Side::Right, 0.5 * t.track_gauge_m.v, "r")]
    {
        for (k, (kind, name, z, y, r)) in wheels.iter().enumerate() {
            let road = *kind == WheelKind::RoadWheel;
            let suspension = if road {
                let i = k + 1 - (per_side - n); // index among the road wheels
                let m_corner = 0.5 * loads[i - 1] / g;
                let f_hz = fa + (fb - fa) * (i - 1) as f64 / (n - 1) as f64;
                let k_s = m_corner * (scalar::TAU * f_hz) * (scalar::TAU * f_hz); // no tyre in series: the belt contact is its own number
                if k_s >= k_wc {
                    return Err(format!("road wheel {i}: the spring rate {k_s:.0} N/m is not below the belt contact stiffness {k_wc:.0} N/m"));
                }
                let f0 = m_corner * g; // the vertical force the spring carries at rest
                let spring = if su.kind == SuspensionKind::TorsionBar {
                    // The wheel rate k at rest from the slider; the bar rate K that gives it, F = T / (L cos phi), sin(phi) = sin(phi0) - c / L:
                    // k = K / (L cos phi0)^2 - F0 sin(phi0) / (L cos^2 phi0).
                    let (l, phi0) = (tx.torsion_arm_length_m.v, tx.torsion_rest_angle_rad.v);
                    let (sp, cp) = (scalar::sin(phi0), scalar::cos(phi0));
                    let rate_nm_rad = (k_s + f0 * sp / (l * cp * cp)) * (l * cp) * (l * cp);
                    SpringKind::Torsion { rate_nm_rad, arm_length_m: l, rest_arm_angle_rad: phi0 }
                } else {
                    SpringKind::Linear { rate_n_m: k_s }
                };
                let c_mean = 2.0 * su.damping_ratio.v * scalar::sqrt(k_s * m_corner);
                let (c_bump, c_reb) = (2.0 * c_mean / (1.0 + rebound), 2.0 * c_mean * rebound / (1.0 + rebound));
                let engage_m = engage * su.bump_travel_m.v;
                let pen = su.bump_travel_m.v - engage_m;
                let k_stop = stop_rate * (1.0 + 2.0 * ex.susp.bump_stop_progression.v * pen / engage_m);
                omega_max = omega_max.max(scalar::sqrt((k_wc + k_s + k_stop) / m_u));
                k_series_sum += 2.0 * k_s * k_wc / (k_s + k_wc);
                SuspensionDef {
                    spring,
                    preload_n: m_corner * g,
                    damper: DamperDef {
                        bump_ns_m: c_bump,
                        rebound_ns_m: c_reb,
                        knee_speed_m_s: 0.0,
                        post_knee_ratio: 1.0,
                        friction_n: 0.0,
                    },
                    bump_stop: BumpStopDef {
                        engage_m,
                        rate_n_m: stop_rate,
                        progression: ex.susp.bump_stop_progression.v,
                        damping_ns_m: ex.susp.bump_stop_damping_ns_m.v,
                        hard_limit: false,
                        restitution: 0.0,
                    },
                }
            } else {
                SuspensionDef {
                    spring: SpringKind::Rigid,
                    preload_n: 0.0,
                    damper: DamperDef {
                        bump_ns_m: 0.0,
                        rebound_ns_m: 0.0,
                        knee_speed_m_s: 0.0,
                        post_knee_ratio: 1.0,
                        friction_n: 0.0,
                    },
                    bump_stop: BumpStopDef {
                        engage_m: 0.0,
                        rate_n_m: 0.0,
                        progression: 0.0,
                        damping_ns_m: 0.0,
                        hard_limit: false,
                        restitution: 0.0,
                    },
                }
            };
            // The belt contact carries the wheel's whole load at rest (the spring's preload plus the unsprung weight): the wheel stands that
            // much into the belt, which is the design pose the solver starts from (static penetration, validated within the contract's tolerance).
            let pen = if road { (suspension.preload_n + m_u * g) / k_wc } else { 0.0 };
            let rest = Vec3::new(sx, y - pen - ride_height, *z);
            let arm_pivot_m = match suspension.spring {
                SpringKind::Torsion { arm_length_m, rest_arm_angle_rad, .. } => Some(Vec3::new(
                    sx,
                    rest.y + arm_length_m * scalar::sin(rest_arm_angle_rad),
                    rest.z - arm_length_m * scalar::cos(rest_arm_angle_rad),
                )),
                _ => None,
            };
            stations.push(StationDef {
                name: format!("{prefix}_{name}"),
                side,
                axle: k as u8,
                rest_pos_m: rest,
                bump_dir: Vec3::Y,
                bump_travel_m: if road { su.bump_travel_m.v } else { 0.0 },
                droop_travel_m: if road { su.droop_travel_m.v } else { 0.0 },
                // A rigid (spin-only) station is part of the hull: its mass is in `hull.mass_kg`.
                unsprung_mass_kg: if road { m_u } else { 0.0 },
                suspension,
                steer: None,
                wheel: WheelDef {
                    kind: *kind,
                    radius_m: *r,
                    width_m: if road { tx.road_wheel_width_m.v } else { t.track_width_m.v },
                    inertia_kg_m2: if road { tx.road_wheel_inertia_kg_m2.v } else { tx.rigid_wheel_inertia_kg_m2.v },
                    tyre: None,
                    patches_x_m: vec![],
                },
                drive_output: (*kind == WheelKind::Sprocket).then_some(usize::from(side == Side::Right)),
                arm_pivot_m,
            });
        }
    }
    let f_max_hz = scalar::sqrt((omega_max * omega_max).max(k_series_sum / m_sprung)) / scalar::TAU;
    let substeps = ((SAMPLES_PER_PERIOD * f_max_hz / TICK_HZ).ceil() as u32).max(1);
    if substeps > MAX_SUBSTEPS {
        return Err(format!("numerically unstable design: the stiffest mode ({f_max_hz:.1} Hz) needs {substeps} substeps per tick, above {MAX_SUBSTEPS}"));
    }
    report.push(format!("stiffest mode {f_max_hz:.1} Hz (road-wheel hop on the belt contact with the bump stop engaged) -> {substeps} substeps per tick"));

    // ---- the belt: perimeter of the loop round the sprocket, the idler and the road wheels (rollers lie on the top line)
    let circle = |w: &(WheelKind, String, f64, f64, f64)| (w.2, w.3, w.4 + 0.5 * th);
    let hull_circles: Vec<_> = wheels.iter().filter(|w| w.0 != WheelKind::ReturnRoller).map(circle).collect();
    let belt_length = belt_perimeter(&hull_circles);
    if belt_length < t.ground_contact_length_m.v {
        return Err(format!(
            "the belt ({belt_length:.2} m) is shorter than its ground contact length {:.2} m",
            t.ground_contact_length_m.v
        ));
    }
    let tracks: Vec<TrackDef> = [(Side::Left, 0usize, "track_l"), (Side::Right, per_side, "track_r")]
        .iter()
        .map(|&(side, first, name)| TrackDef {
            name: name.into(),
            side,
            stations: (first..first + per_side).collect(), // already in loop order from the sprocket
            sprocket: first,
            idler: first + 1 + rollers,
            width_m: t.track_width_m.v,
            pitch_m: t.pitch_m.v,
            contact_length_m: t.ground_contact_length_m.v,
            mass_per_m_kg: t.track_mass_per_side_kg.v / belt_length,
            samples: tx.samples.v.round() as u16,
            shoe_mu_scale: t.shoe_mu_scale.v,
            shoe_mu_scale_soft: None,
            tension_n: tx.tension_n.v,
            belt_length_m: belt_length,
            thickness_m: th,
            grouser_height_m: 0.0,
            belt_stiffness_n_m: 0.0,
            resist_c0: tx.resist_c0.v,
            resist_c1_s_m: 0.0,
            sprocket_teeth: teeth as u8,
            wheel_contact: WheelContact {
                vertical_stiffness_n_m: k_wc,
                vertical_damping_ns_m: tx.wheel_contact_damping_ns_m.v,
            },
        })
        .collect();
    report.push(format!("belt: {per_side} wheels a side in loop order, perimeter {belt_length:.2} m (contact {:.2} m), {:.0} links of {:.3} m", t.ground_contact_length_m.v, belt_length / t.pitch_m.v, t.pitch_m.v));

    // ---- drivetrain: engine, coupling, gearbox as wheeled; a steer unit over the two sprockets; brakes on the sprockets
    let (hull, size) = hull_body(h, &sprung_mass);
    let (engine, coupling, gearbox) = powertrain_parts(def, ex)?;
    let su_def = pt.steering_unit.as_ref().ok_or("a tracked def needs `powertrain.steering_unit` (kind and ratio)")?;
    let sl = &tx.steer_law;
    let steer_law = SteerLaw {
        diff_ratio_by_gear: sl.diff_ratio_by_gear.iter().map(|p| p.v).collect(),
        detents: sl.detents.iter().map(|p| p.v).collect(),
        diff_speed_rad_s: sl.diff_speed_rad_s.as_ref().map(|p| p.v),
        works_in_neutral: sl.works_in_neutral,
        max_steer_torque_nm: sl.max_steer_torque_nm.as_ref().map_or(0.0, |p| p.v),
        // Clutch-brake and controlled-differential units steer with the sprocket brakes (indices into `brakes`: [left, right]).
        steer_brakes: matches!(su_def.kind, SteerUnitKind::ClutchBrake | SteerUnitKind::ControlledDifferential)
            .then_some([0, 1]),
    };
    let outputs = [0, per_side]
        .iter()
        .map(|&s| OutputDef {
            station: s,
            final_drive_ratio: pt.final_drive_ratio.v,
            efficiency: pt.driveline_efficiency.v,
        })
        .collect();
    let total_mass =
        m_sprung + m_u * (2 * n) as f64 + t.track_mass_per_side_kg.v * 2.0 * t.ground_contact_length_m.v / belt_length;
    let (br, bx) = (&def.brakes, &ex.brake);
    let pair_torque = match bx.axle_torque_nm.as_slice() {
        [] => total_mass * br.service_decel_g.v * g * r_spr, // sized from the design deceleration at the sprocket pitch radius
        [one] => one.v,
        more => {
            return Err(format!(
                "brake.axle_torque_nm has {} entries; a tracked vehicle brakes one sprocket pair",
                more.len()
            ))
        }
    };
    let brakes: Vec<BrakeDef> = [0, per_side]
        .iter()
        .map(|&s| BrakeDef {
            station: s,
            max_torque_nm: pair_torque / 2.0,
            thermal_mass_j_k: br.thermal_mass_kj_k.v * J_PER_KJ,
            cooling_w_k: bx.cooling_w_k.v,
            cooling_per_ms_w_k: bx.cooling_per_ms_w_k.v,
            fade_start_k: bx.fade_start_k.v,
            fade_end_k: bx.fade_end_k.v,
            fade_floor: bx.fade_floor.v,
            parking: br.parking_brake,
            service: true,
            location: BrakeLocation::AtStation,
            site: BrakeSite::Wheel,
            steering: false,
            reverse_torque_factor: 1.0,
            apply_time_s: 0.0,
            release_time_s: 0.0,
            circuit: 0,
        })
        .collect();
    report.push(format!(
        "brakes: {pair_torque:.0} N m on the sprocket pair -> {:.2} g on {total_mass:.0} kg",
        pair_torque / r_spr / total_mass / g
    ));

    let rig = PhysRig {
        id: def.id.clone(),
        ride_height_m: ride_height,
        hull,
        stations,
        linkages: vec![],
        anti_roll: vec![],
        tracks,
        drivetrain: DrivetrainDef {
            engine,
            coupling,
            gearbox,
            driveline: DriveNode::SteerUnit {
                kind: su_def.kind,
                ratio: su_def.ratio.v,
                law: steer_law,
                children: vec![DriveNode::Output(0), DriveNode::Output(1)],
            },
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
        proxies: vec![
            CollisionProxy {
                name: "belly".into(),
                // The belly plate: between the tracks, underside exactly `ground_clearance_m` above the ground (the shape TRACKS' belly drag needs).
                shape: ProxyShape::Box {
                    half_m: Vec3::new(
                        0.5 * (t.track_gauge_m.v - t.track_width_m.v),
                        0.5 * tx.belly_thickness_m.v,
                        0.5 * tx.belly_length_m.v,
                    ),
                },
                pose: Transform::from_pos(Vec3::new(
                    0.0,
                    h.ground_clearance_m.v + 0.5 * tx.belly_thickness_m.v - ride_height,
                    z_c,
                )),
                attached_to: None,
                attached_station: None,
                role: ProxyRole::Belly,
            },
            CollisionProxy {
                name: "hull".into(),
                shape: ProxyShape::Box { half_m: 0.5 * size },
                pose: Transform::IDENTITY,
                attached_to: None,
                attached_station: None,
                role: ProxyRole::Hull,
            },
        ],
        muzzles: vec![],
        combat: CombatDef::default(),
        integration: IntegrationDef { substeps, f_max_hz: Some(f_max_hz) },
    };
    rig.validate().map_err(|e| format!("the compiled rig fails PhysRig::validate(): {e:?}"))?;
    Ok(Compiled { rig, report, hull_size_m: size, mass_items: sprung_mass.items })
}
