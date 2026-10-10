//! The whole-vehicle running gear on the ladder tank: both tracks and the belly behind one step.

use w5k_contract::testing::box_tank;
use w5k_contract::Material;
use w5k_math::scalar;
use w5k_terramech::ladder::LadderTank;
use w5k_terramech::plan::PlanVehicle;
use w5k_terramech::reference::reference_soils;
use w5k_terramech::vehicle::{BellyStepInput, TrackStepInput, TrackedVehicleGear};
use w5k_terramech::{gear::GearConfig, Tuning};

struct Setup {
    gear: TrackedVehicleGear,
    pen: Vec<f64>,
    d: f64,
    weight_n: f64,
}

fn setup(ground: &Material, mass_t: f64) -> Setup {
    let (rig, _) = box_tank();
    let t = Tuning::shipped();
    let belly = LadderTank::shipped().belly(&t);
    let weight_n = mass_t * 1000.0 * t.gravity_m_s2;
    let v =
        PlanVehicle::new_with_belly(GearConfig::from_rig(&rig, 0), t, 2.7, weight_n, ground, 1.0, Some(belly)).unwrap();
    let d = v.static_penetration_m();
    Setup { gear: TrackedVehicleGear::new(&rig, t, Some(belly)), pen: vec![d; 5], d, weight_n }
}

fn run(s: &mut Setup, g: &Material, belts: (f64, f64), v: f64, steps: usize) {
    let rate = vec![0.0; 5];
    let grounds: Vec<&Material> = vec![g; 12];
    for _ in 0..steps {
        let mk = |omega: f64| TrackStepInput {
            wheel_penetration_m: &s.pen,
            wheel_penetration_rate_m_s: &rate,
            vel_long_m_s: v,
            vel_lat_m_s: 0.0,
            yaw_rate_rad_s: 0.0,
            sprocket_omega_rad_s: omega,
            ground: &grounds,
        };
        let inputs = [mk(belts.0 / 0.4), mk(belts.1 / 0.4)];
        let b = BellyStepInput { belt_penetration_m: s.d, vel_long_m_s: v, vel_lat_m_s: 0.0, ground: g };
        s.gear.step(&inputs, Some(&b), 0.001);
    }
}

#[test]
fn both_tracks_carry_the_weight_between_them_when_the_belly_is_clear() {
    let clay = reference_soils()[1].clone();
    let mut s = setup(&clay, 20.0);
    run(&mut s, &clay, (0.0, 0.0), 0.0, 3);
    let wheels: f64 = (0..2).map(|i| s.gear.track(i).wheel_force_n().iter().sum::<f64>()).sum();
    assert!(scalar::approx_eq(wheels, s.weight_n, 1e-6 * s.weight_n), "{wheels} vs {}", s.weight_n);
    assert!(s.gear.belly_output().fz_n.abs() < 1e-12);
}

#[test]
fn the_belly_shares_the_weight_once_the_hull_sits_in_the_snow() {
    let snow = reference_soils()[2].clone();
    let mut s = setup(&snow, 60.0);
    run(&mut s, &snow, (1.0, 1.0), 0.5, 2000);
    let tracks: f64 = s.gear.totals().iter().map(|t| t.fz_n).sum();
    let belly = s.gear.belly_output();
    assert!(belly.fz_n > 0.0 && belly.fx_n < 0.0, "the belly bears and drags: {belly:?}");
    assert!(scalar::approx_eq(tracks + belly.fz_n, s.weight_n, 1e-6 * s.weight_n));
}

#[test]
fn a_straight_drive_loads_both_sprockets_equally_and_a_pivot_loads_them_oppositely() {
    let dirt = {
        let t = w5k_contract::testing::standard_materials();
        t.get(w5k_contract::MaterialId(1)).clone()
    };
    let mut s = setup(&dirt, 20.0);
    run(&mut s, &dirt, (1.2, 1.2), 1.0, 3000);
    let r = |s: &Setup| (s.gear.totals()[0].shaft_reaction_nm, s.gear.totals()[1].shaft_reaction_nm);
    let (l, rr) = r(&s);
    assert!(scalar::approx_eq(l, rr, 1e-9 * l.abs()) && l > 0.0);
    s.gear.reset();
    run(&mut s, &dirt, (-1.0, 1.0), 0.0, 3000);
    let (l, rr) = r(&s);
    assert!(l < 0.0 && rr > 0.0, "the backward track is driven backward: {l} {rr}");
}
