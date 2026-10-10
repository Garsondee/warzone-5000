//! The tracked hull on the chassis integrator, against closed-form answers. Tests are physics sentences.

use w5k_chassis::tracked::TrackedChassis;
use w5k_chassis::tuning::ChassisTuning;
use w5k_contract::rig::{PhysRig, WheelKind, TICK_HZ};
use w5k_contract::testing::{box_tank, ConstantTorquePowertrain, FlatPlane};
use w5k_contract::{DriveInputs, ForceLedger, GearRequest, WorldQuery};
use w5k_math::{scalar, Vec3};
use w5k_terramech::ladder::LadderTank;

fn tuning() -> ChassisTuning {
    ChassisTuning::from_ron(include_str!("../../../content/physics/chassis/tuning.ron")).unwrap()
}

/// `box_tank()` with its road-wheel preloads set by statics: the springs carry the hull's weight, `P_i = a + b z_i` with `sum P = m g` and
/// `sum P z = m g z_com` (the contract stand-in's preloads add up to about 44 t against a 28 t hull, so it would rest on its droop stops).
fn tank() -> PhysRig {
    let (mut r, _) = box_tank();
    let w = r.hull.mass_kg * scalar::G;
    let zs: Vec<f64> =
        r.stations.iter().filter(|s| s.wheel.kind == WheelKind::RoadWheel).map(|s| s.rest_pos_m.z).collect();
    let n = zs.len() as f64;
    let (sz, szz) = (zs.iter().sum::<f64>(), zs.iter().map(|z| z * z).sum::<f64>());
    let b = (w * r.hull.com_m.z - w * sz / n) / (szz - sz * sz / n);
    let a = (w - b * sz) / n;
    for s in r.stations.iter_mut().filter(|s| s.wheel.kind == WheelKind::RoadWheel) {
        s.suspension.preload_n = a + b * s.rest_pos_m.z;
    }
    r
}

fn chassis(r: &PhysRig, w: &dyn WorldQuery) -> TrackedChassis {
    let t = w5k_terramech::Tuning::shipped();
    let belly = LadderTank::shipped().belly(&t);
    TrackedChassis::new(r, &tuning(), t, Some(belly), w, 0.0, 0.0, 0.0).unwrap()
}

fn drive(r: &PhysRig) -> ConstantTorquePowertrain {
    ConstantTorquePowertrain::new(r.drivetrain.outputs.len(), 8000.0, 40000.0)
}

#[test]
fn tank_rests_at_its_design_ride_height_less_the_static_belt_penetration_within_5mm() {
    let (r, w) = (tank(), FlatPlane::new());
    let mut c = chassis(&r, &w);
    let mut d = drive(&r);
    let parked = DriveInputs { gear: GearRequest::Neutral, parking_brake: true, ..DriveInputs::default() };
    for _ in 0..(5.0 * TICK_HZ) as usize {
        c.tick(1.0 / TICK_HZ, &parked, &w, &mut d);
        assert!(c.is_finite());
    }
    let sag = c.datum_m().y - r.ride_height_m;
    let pen: Vec<f64> = c.wheels.iter().map(|w| w.penetration_m).collect();
    let travel: Vec<f64> = c.wheels.iter().map(|w| w.travel_m).collect();
    let carried: f64 = c.wheels.iter().map(|w| w.contact_force_n).sum();
    let weight = (r.hull.mass_kg + r.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>()) * scalar::G;
    // TRACKS' own static solve (`PlanVehicle`): the belt penetration that carries the weight on both gears. The rig's ride height does not
    // include it (unlike a tyre's rest pose, which carries the static deflection), so the datum rests that much lower.
    let gauge = 2.0 * r.stations[r.tracks[0].stations[1]].rest_pos_m.x.abs();
    let ground = w.materials().get(w.material_id_at(0.0, 0.0));
    let plan = w5k_terramech::plan::PlanVehicle::new(
        w5k_terramech::gear::GearConfig::from_rig(&r, 0),
        w5k_terramech::Tuning::shipped(),
        gauge,
        weight,
        ground,
        1.0,
    )
    .unwrap();
    let d_static = plan.static_penetration_m();
    println!(
        "datum {sag:.4} m off the ride height; TRACKS' static belt penetration {d_static:.4} m; per wheel {pen:.4?}; travel {travel:.4?}; carried {carried:.0} N of {weight:.0} N"
    );
    assert!((sag + d_static).abs() < 0.005, "datum {sag} m off, against {} expected", -d_static);
    assert!(travel.iter().all(|t| t.abs() < 0.005), "the springs sit at their preload");
    assert!((carried / weight - 1.0).abs() < 1e-3, "the belts carry the weight");
    assert!(c.hull.vel_m_s.length() < 1e-3);
}

#[test]
fn tracked_ledger_net_force_equals_mass_times_acceleration() {
    let (r, w) = (tank(), FlatPlane::new());
    let mut c = chassis(&r, &w);
    c.ledger = ForceLedger::on();
    let mut d = drive(&r);
    let h = 1.0 / TICK_HZ / f64::from(c.substeps());
    let mut worst: f64 = 0.0;
    let scale = r.hull.mass_kg * scalar::G;
    for n in 0..(4.0 * TICK_HZ) as usize {
        // settle, then drive off, then brake
        let inp = match n {
            0..=59 => DriveInputs { gear: GearRequest::Neutral, ..DriveInputs::default() },
            60..=179 => DriveInputs { gear: GearRequest::Auto, throttle: 0.6, ..DriveInputs::default() },
            _ => DriveInputs { gear: GearRequest::Neutral, brake: 1.0, ..DriveInputs::default() },
        };
        for _ in 0..c.substeps() {
            c.substep(h, &inp, &w, &mut d);
            let ef = (c.ledger.net_force_on(0) - c.hull.acc_m_s2 * r.hull.mass_kg).length() / scale;
            let torque: Vec3 = c.ledger.totals().iter().fold(Vec3::ZERO, |a, t| a + t.1);
            let et = (torque - c.hull.last_torque_nm).length() / scale;
            worst = worst.max(ef).max(et);
            for (i, wheel) in c.wheels.iter().enumerate() {
                // net force m_u x travel acceleration along the travel direction the substep used (booked in the hull's frame)
                let def = r.stations.iter().find(|s| s.name == wheel.name).unwrap();
                let expect = wheel.strut_dir * (def.unsprung_mass_kg * c.travel_acc_m_s2[i]);
                worst = worst.max((c.ledger.net_force_on(1 + i as u16) - expect).length() / scale);
            }
        }
        assert!(c.is_finite());
    }
    println!("worst relative ledger error {worst:e}; speed at the end {:.2} m/s", c.forward_speed_m_s());
    assert!(worst < 1e-9);
}

#[test]
fn tank_accelerates_at_twice_the_sprocket_force_over_its_effective_mass() {
    // two sprockets each pushing T / r through the belt, less the tracks' running resistance on hard ground (c0 + c1 v) N; the mass includes
    // every spinning part at its speed ratio (J_eff / r^2 per track). Measured over the second second, once the belts run well past the
    // speed over which the gear fades its resistance in; belt slip takes a little: within 10%.
    let (r, w) = (tank(), FlatPlane::new());
    let mut c = chassis(&r, &w);
    let mut d = drive(&r);
    let parked = DriveInputs { gear: GearRequest::Neutral, ..DriveInputs::default() };
    for _ in 0..(2.0 * TICK_HZ) as usize {
        c.tick(1.0 / TICK_HZ, &parked, &w, &mut d);
    }
    let go = DriveInputs { gear: GearRequest::Auto, throttle: 1.0, ..DriveInputs::default() };
    for _ in 0..(1.0 * TICK_HZ) as usize {
        c.tick(1.0 / TICK_HZ, &go, &w, &mut d);
    }
    let (v0, t0) = (c.forward_speed_m_s(), c.time_s);
    for _ in 0..(1.0 * TICK_HZ) as usize {
        c.tick(1.0 / TICK_HZ, &go, &w, &mut d);
    }
    let a = (c.forward_speed_m_s() - v0) / (c.time_s - t0);
    let r_s = r.stations[r.tracks[0].sprocket].wheel.radius_m;
    let j: f64 = r.tracks.iter().map(|t| w5k_chassis::tracked::reflected_sprocket_inertia_kg_m2(&r, t)).sum();
    let m = r.hull.mass_kg + r.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>() + j / (r_s * r_s);
    let weight = (r.hull.mass_kg + r.stations.iter().map(|s| s.unsprung_mass_kg).sum::<f64>()) * scalar::G;
    let v_mid = c.forward_speed_m_s() - 0.5 * a * (c.time_s - t0);
    let resist = (r.tracks[0].resist_c0 + r.tracks[0].resist_c1_s_m * v_mid) * weight;
    let expect = (2.0 * 8000.0 / r_s - resist) / m;
    println!("a {a:.3} m/s2 against (2 T / r - (c0 + c1 v) W) / m_eff = {expect:.3} (m_eff {m:.0} kg, resistance {resist:.0} N, v {v_mid:.2} m/s)");
    assert!(a > 0.0 && (a / expect - 1.0).abs() < 0.1);
}
