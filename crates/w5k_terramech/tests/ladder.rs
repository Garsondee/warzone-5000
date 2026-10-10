//! The ladder: heavier tank, deeper sinkage, belly drag, bog. Plus the belly's smooth onset and the width/weight effects against a tyre.

use w5k_contract::testing::box_tank;
use w5k_contract::Material;
use w5k_math::scalar;
use w5k_terramech::belly::{Belly, BellyGeom};
use w5k_terramech::gear::GearConfig;
use w5k_terramech::ladder::{ladder_row, LadderRow};
use w5k_terramech::reference::reference_soils;
use w5k_terramech::{soil, Tuning};

fn belly() -> BellyGeom {
    BellyGeom { clearance_m: 0.4, width_m: 2.0, length_m: 4.0, stiffness_n_m: 2e7 }
}

fn rows(ground: &Material, masses_t: &[f64]) -> Vec<LadderRow> {
    let (rig, _) = box_tank();
    let cfg = GearConfig::from_rig(&rig, 0);
    masses_t.iter().map(|&m| ladder_row(&cfg, belly(), 2.7, Tuning::shipped(), ground, m * 1000.0, 1.0)).collect()
}

#[test]
fn belly_drag_ramps_smoothly_with_sinkage_over_clearance() {
    let snow = reference_soils()[2].clone();
    let t = Tuning::shipped();
    let (clear, ramp) = (belly().clearance_m, t.belly_ramp_m);
    let eval = |pen: f64| {
        let mut b = Belly::new(belly(), t);
        let mut o = Default::default();
        for _ in 0..300 {
            o = b.step(pen, 1.0, 0.0, &snow, 0.001);
        }
        o
    };
    // Nothing until the sinkage passes the clearance.
    let below = eval(clear - 0.01);
    assert_eq!((below.fz_n, below.fx_n), (0.0, 0.0));
    // Zero slope at the onset: a millimetre over is still a rounding error next to a full ramp over.
    let (onset, full) = (eval(clear + 0.001), eval(clear + ramp));
    assert!(onset.fz_n < 1e-3 * full.fz_n, "{} vs {}", onset.fz_n, full.fz_n);
    // Monotone and without a jump: no step of 0.2 mm in sinkage moves the load or the drag by more than 2% of the value at 0.15 m over.
    let reference = eval(clear + 0.15);
    let (mut prev, mut worst) = (eval(clear), 0.0f64);
    for i in 1..=750 {
        let o = eval(clear + 0.0002 * f64::from(i));
        assert!(o.fz_n >= prev.fz_n - 1e-9 && -o.fx_n >= -prev.fx_n - 1e-9, "monotone at {i}");
        worst = worst.max((o.fz_n - prev.fz_n) / reference.fz_n).max((o.fx_n - prev.fx_n).abs() / reference.fx_n.abs());
        prev = o;
    }
    println!("largest step in load or drag: {:.3}% of the value at 0.15 m over", worst * 100.0);
    assert!(worst < 0.02, "a cliff: {worst}");
    assert!(reference.fx_n < 0.0, "the belly drags: {}", reference.fx_n);
}

#[test]
fn heavier_tank_sinks_deeper_and_bogs_beyond_a_ground_pressure_threshold() {
    let snow = reference_soils()[2].clone();
    let masses: Vec<f64> = (1..=30).map(|i| 5.0 * f64::from(i)).collect();
    let rows = rows(&snow, &masses);
    assert!(rows.windows(2).all(|w| w[1].sinkage_m >= w[0].sinkage_m), "heavier sinks deeper");
    // Until the belly bears, the model is the soil theory: z = (p / (kc/b + kphi))^(1/n).
    for r in rows.iter().filter(|r| !r.belly_engaged) {
        assert!(scalar::approx_eq(r.sinkage_m, r.predicted_sinkage_m, 0.02 * r.predicted_sinkage_m), "{r:?}");
    }
    // The hull starts to drag at the pressure that sinks the footprint to the clearance (Bekker, solved for p).
    let s = snow.soil.unwrap();
    let p_theory = soil::pressure_pa(&s, 0.55, belly().clearance_m);
    let first = rows.iter().position(|r| r.belly_engaged).expect("the belly engages");
    let (lo, hi) = (rows[first.saturating_sub(1)].ground_pressure_pa, rows[first].ground_pressure_pa);
    println!(
        "belly threshold: model between {:.1} and {:.1} kPa, theory {:.1} kPa",
        lo / 1e3,
        hi / 1e3,
        p_theory / 1e3
    );
    assert!(lo * 0.75 <= p_theory && p_theory <= hi * 1.25, "theory outside the bracket");
    // Bogged is a threshold: light tanks go, heavy tanks do not, and it happens after the belly bears.
    let bog = rows.iter().position(|r| r.bogged).expect("a heavy enough tank bogs");
    assert!(bog > first && rows[..bog].iter().all(|r| !r.bogged) && rows[bog..].iter().all(|r| r.bogged));
    println!(
        "bogs from {} t ({:.0} kPa); hull drags from {} t",
        rows[bog].mass_kg / 1e3,
        rows[bog].ground_pressure_pa / 1e3,
        rows[first].mass_kg / 1e3
    );
}

#[test]
fn track_floats_where_a_tyre_of_the_same_weight_sinks() {
    // 11 t on clay: two 0.55 m tracks 4.8 m long against four tyres (D 1.0 m, b 0.4 m) on the rigid-wheel formula.
    let clay = reference_soils()[1].clone();
    let s = clay.soil.unwrap();
    let w = 11_000.0 * 9.80665;
    let track = rows(&clay, &[11.0])[0];
    let tyre = soil::rigid_wheel_sinkage_m(&s, w / 4.0, 0.4, 1.0);
    println!("track sinks {:.1} mm, tyre {:.1} mm", track.sinkage_m * 1e3, tyre * 1e3);
    assert!(tyre > 10.0 * track.sinkage_m);
}

#[test]
fn width_helps_through_pressure_not_through_the_kc_over_b_term() {
    // Bekker's law has p = (kc/b + kphi) z^n: at the SAME pressure a narrow plate is the stiffer one (kc/b is larger), so it sinks a little LESS.
    let s = reference_soils()[1].soil.unwrap();
    let p = 30_000.0;
    let ratio = soil::sinkage_m(&s, 0.3, p) / soil::sinkage_m(&s, 0.6, p);
    let by_hand = scalar::pow((s.kc_pa_m_n1 / 0.6 + s.kphi_pa_m_n) / (s.kc_pa_m_n1 / 0.3 + s.kphi_pa_m_n), 1.0 / s.n);
    assert!(scalar::approx_eq(ratio, by_hand, 1e-12));
    assert!(ratio < 1.0 && ratio > 0.85, "a small effect on clay, where kphi dominates: {ratio}");
    // The same WEIGHT on a wider footprint is less pressure, and that wins by far: the width of a track floats it.
    let w = 100_000.0;
    let z = |b: f64| soil::sinkage_m(&s, b, w / (2.0 * b * 4.0));
    assert!(z(0.6) < 0.5 * z(0.3), "{} vs {}", z(0.6), z(0.3));
}

#[test]
fn a_belly_proxy_in_the_rig_sets_clearance_and_plan_area() {
    use w5k_contract::rig::{CollisionProxy, ProxyRole, ProxyShape};
    use w5k_math::{Transform, Vec3};
    let (mut rig, _) = box_tank();
    rig.ride_height_m = 0.9;
    assert!(BellyGeom::from_rig(&rig, &Tuning::shipped()).is_none());
    rig.proxies.push(CollisionProxy {
        name: "belly".into(),
        shape: ProxyShape::Box { half_m: Vec3::new(0.9, 0.05, 2.1) },
        pose: Transform::from_pos(Vec3::new(0.0, -0.45, 0.0)),
        attached_to: None,
        attached_station: None,
        role: ProxyRole::Belly,
    });
    let b = BellyGeom::from_rig(&rig, &Tuning::shipped()).unwrap();
    // The lowest face is at y = -0.50 in the hull frame; the ground plane is at y = -0.9: 0.40 m of clearance.
    assert!(scalar::approx_eq(b.clearance_m, 0.40, 1e-12));
    assert!(scalar::approx_eq(b.width_m, 1.8, 1e-12) && scalar::approx_eq(b.length_m, 4.2, 1e-12));
}
