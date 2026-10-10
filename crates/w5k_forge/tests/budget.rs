//! The component mass budget (D2a): mass is a consequence of the configuration; the baseline is unchanged.

use w5k_contract::def::VehicleDef;
use w5k_contract::rig::*;
use w5k_forge::compile::{compile, parse_def, parse_extras, Compiled};
use w5k_forge::extras::{Extras, Loading};
use w5k_forge::levers::apply_both;
use w5k_math::scalar::G;
use w5k_math::{Mat3, Vec3};

const IDS: [&str; 4] = ["scout_4x4", "mule_4x4", "hauler_4x4", "carrier_tracked"];

fn load(id: &str) -> (VehicleDef, Extras) {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/vehicles/game/");
    (
        parse_def(&std::fs::read_to_string(format!("{dir}{id}.ron")).unwrap()).unwrap(),
        parse_extras(&std::fs::read_to_string(format!("{dir}{id}.extras.ron")).unwrap()).unwrap(),
    )
}

fn built(d: &VehicleDef, x: &Extras) -> Compiled {
    compile(d, x).unwrap_or_else(|e| panic!("{e:?}"))
}

fn mass_of(c: &Compiled, name: &str) -> f64 {
    c.mass_items.iter().find(|m| m.name == name).unwrap_or_else(|| panic!("no mass item {name}")).mass_kg
}

fn preloads(c: &Compiled) -> Vec<f64> {
    c.rig.stations.iter().map(|s| s.suspension.preload_n).collect()
}

#[test]
fn baseline_total_mass_com_axle_loads_and_springs_are_unchanged_by_the_budget() {
    for id in IDS {
        let (d, x) = load(id);
        let with = built(&d, &x);
        let mut lump = x.clone();
        lump.mass_budget = None;
        let without = built(&d, &lump);
        assert!(!with.mass_items.is_empty() && without.mass_items.is_empty(), "{id}");
        assert!(
            (with.rig.hull.mass_kg - without.rig.hull.mass_kg).abs() < 1e-9 * with.rig.hull.mass_kg,
            "{id}: total mass"
        );
        assert!((with.rig.hull.com_m - without.rig.hull.com_m).length() < 1e-9, "{id}: COM");
        for (a, b) in preloads(&with).iter().zip(preloads(&without)) {
            assert!((a - b).abs() < 1e-6 * b.max(1.0), "{id}: preload");
        }
        for (a, b) in with.rig.stations.iter().zip(&without.rig.stations) {
            let rate = |s: &SpringKind| match s {
                SpringKind::Linear { rate_n_m } => *rate_n_m,
                SpringKind::Torsion { rate_nm_rad, .. } => *rate_nm_rad,
                _ => 0.0, // rigid stations carry no spring
            };
            let (ra, rb) = (rate(&a.suspension.spring), rate(&b.suspension.spring));
            assert!((ra - rb).abs() <= 1e-9 * rb, "{id}: spring rates follow the sprung mass");
        }
        assert_eq!(with.rig.integration.substeps, without.rig.integration.substeps, "{id}");
    }
}

#[test]
fn the_budget_sums_to_the_def_mass_at_its_design_point() {
    for id in IDS {
        let (d, x) = load(id);
        let c = built(&d, &x);
        let sum: f64 = c.mass_items.iter().map(|m| m.mass_kg).sum();
        assert!(
            (sum - d.hull.mass_kg.v).abs() < 1e-6 * sum,
            "{id}: parts sum to {sum}, the def says {}",
            d.hull.mass_kg.v
        );
    }
}

#[test]
fn engine_power_moves_mass_com_axle_loads_and_spring_rates() {
    let (d, x) = load("mule_4x4");
    let base = built(&d, &x);
    let (d2, x2) = apply_both(&d, &x, "engine_peak_power", 1.3).unwrap();
    let big = built(&d2, &x2);
    // Engine, transmission and final drives all scale with the engine (power and torque scale together).
    for name in ["engine", "transmission", "final_drive0"] {
        assert!((mass_of(&big, name) / mass_of(&base, name) - 1.3).abs() < 1e-9, "{name}");
    }
    assert!(big.rig.hull.mass_kg > base.rig.hull.mass_kg + 100.0, "total mass rises with the engine");
    // The engine is in the front: the COM moves forward (-Z) and the front axle takes more of the weight, so its springs stiffen.
    assert!(big.rig.hull.com_m.z < base.rig.hull.com_m.z);
    let front =
        |c: &Compiled| c.rig.stations.iter().filter(|s| s.axle == 0).map(|s| s.suspension.preload_n).sum::<f64>();
    let total = |c: &Compiled| preloads(c).iter().sum::<f64>();
    assert!(front(&big) / total(&big) > front(&base) / total(&base));
    let rate = |c: &Compiled| match c.rig.stations[0].suspension.spring {
        SpringKind::Linear { rate_n_m } => rate_n_m,
        _ => panic!("linear"),
    };
    assert!(rate(&big) > rate(&base), "spring rates follow the sprung mass at a fixed ride frequency");
}

#[test]
fn fuel_tank_size_and_loading_move_mass_and_com() {
    let (d, x) = load("mule_4x4");
    let base = built(&d, &x);
    let mut big = x.clone();
    let b = big.mass_budget.as_mut().unwrap();
    b.tank_litres.v *= 1.5;
    b.tank_litres.hi = Some(b.tank_litres.v);
    let c = built(&d, &big);
    let bx = x.mass_budget.as_ref().unwrap();
    let want = 0.5 * bx.tank_litres.v * (bx.fuel_density_kg_l.v * bx.fill_combat.v + bx.tank_kg_per_litre.v);
    assert!((mass_of(&c, "fuel") - mass_of(&base, "fuel") - want).abs() < 1e-9);
    assert!(c.rig.hull.com_m.z > base.rig.hull.com_m.z, "the tank is behind the COM: it moves rearward");
    let mut curb = x.clone();
    curb.mass_budget.as_mut().unwrap().loading = Loading::Curb;
    let light = built(&d, &curb);
    let saved = bx.tank_litres.v * bx.fuel_density_kg_l.v * (bx.fill_combat.v - bx.fill_curb.v);
    assert!((base.rig.hull.mass_kg - light.rig.hull.mass_kg - saved).abs() < 1e-9);
}

#[test]
fn parallel_axis_matches_a_hand_calculation_with_parts() {
    for id in IDS {
        let (d, x) = load(id);
        let c = built(&d, &x);
        let h = &d.hull;
        let size = Vec3::new(h.width_m.v, h.height_m.v, h.length_m.v);
        let com = c.rig.hull.com_m;
        let shift = |m: f64, v: Vec3| {
            Mat3::diagonal(1.0, 1.0, 1.0).scaled(v.dot(v)).add(&Mat3::outer(v, v).scaled(-1.0)).scaled(m)
        };
        let ms = c.mass_items[0].mass_kg;
        let k = ms / 12.0;
        let box_i = Mat3::diagonal(
            k * (size.y * size.y + size.z * size.z),
            k * (size.x * size.x + size.z * size.z),
            k * (size.x * size.x + size.y * size.y),
        );
        let mut want = box_i.add(&shift(ms, com));
        for m in c.mass_items.iter().skip(1) {
            want = want.add(&shift(m.mass_kg, m.pos_m - com));
        }
        for i in 0..3 {
            for j in 0..3 {
                assert!(
                    (c.rig.hull.inertia_kg_m2.m[i][j] - want.m[i][j]).abs() < 1e-9 * want.m[0][0],
                    "{id} [{i}][{j}]"
                );
            }
        }
        // Positive definite, and the principal moments obey the triangle inequalities.
        let m = c.rig.hull.inertia_kg_m2.m;
        assert!(
            m[0][0] > 0.0 && m[0][0] * m[1][1] - m[0][1] * m[1][0] > 0.0 && c.rig.hull.inertia_kg_m2.det() > 0.0,
            "{id}"
        );
        assert!(m[0][0] + m[1][1] >= m[2][2] && m[1][1] + m[2][2] >= m[0][0] && m[0][0] + m[2][2] >= m[1][1], "{id}");
    }
}

#[test]
fn the_com_is_the_mass_weighted_mean_of_the_parts_and_axle_loads_follow_it() {
    let (d, x) = load("hauler_4x4");
    let c = built(&d, &x);
    let total: f64 = c.mass_items.iter().map(|m| m.mass_kg).sum();
    let mean = c.mass_items.iter().fold(Vec3::ZERO, |a, m| a + m.pos_m * m.mass_kg) / total;
    assert!((mean - c.rig.hull.com_m).length() < 1e-9);
    let weight = c.rig.hull.mass_kg * G;
    let preload: f64 = preloads(&c).iter().sum();
    assert!((preload - weight).abs() < 1e-6 * weight);
}

#[test]
fn a_part_that_cannot_exist_is_rejected_with_its_name_and_a_heavy_engine_on_a_light_hull_still_builds() {
    let (d, x) = load("scout_4x4");
    let mut bad = x.clone();
    let b = bad.mass_budget.as_mut().unwrap();
    b.crew_kg_each.v = -10.0;
    b.crew_kg_each.lo = Some(-20.0);
    let e = compile(&d, &bad).err().expect("rejected");
    assert!(e.iter().any(|r| r.reason.contains("crew") && r.reason.contains("positive mass")), "{e:?}");
    // Wacky but representable: three times the engine on the scout builds (it will be flagged by the audit, not refused).
    let (d3, x3) = apply_both(&d, &x, "engine_peak_power", 3.0).unwrap();
    let c = built(&d3, &x3);
    c.rig.validate().unwrap();
    assert!(mass_of(&c, "engine") > 2.9 * mass_of(&built(&d, &x), "engine"));
}
