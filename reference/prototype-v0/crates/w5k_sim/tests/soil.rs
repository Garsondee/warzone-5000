//! Soft ground: the laws in `soil.rs` against closed forms, and what they do to a vehicle on a course.

use std::collections::BTreeMap;

use w5k_math::Fx;
use w5k_sim::replay::run_state;
use w5k_sim::soil::{Footprint, Soil};
use w5k_sim::trial::run;
use w5k_sim::{Cause, Course, CourseDef, GearClass, MoverSpec, Outcome, SegmentDef, SurfaceDef, TerrainDef, HZ};

/// Soft earth as in `content/terrain.ron`.
fn earth() -> Soil {
    Soil::from_def(60.0, 170.0, 4.0, 22.0, 0.02).unwrap()
}

fn fx(x: f64) -> Fx {
    Fx::from_f64(x)
}

fn terrain() -> TerrainDef {
    let mut surfaces = BTreeMap::new();
    surfaces.insert("concrete".to_string(), SurfaceDef::Rigid);
    surfaces.insert("soft_earth".to_string(), SurfaceDef::Soft { kc: 60.0, kphi: 170.0, cohesion: 4.0, friction_deg: 22.0, shear_k: 0.02 });
    TerrainDef { surfaces }
}

fn seg(length_m: f64, grade: f64, surface: &str) -> SegmentDef {
    SegmentDef { length_m, grade, surface: surface.to_string() }
}

/// Concrete, then a long flat stretch of soft earth, then concrete.
fn soft_course() -> Course {
    let def = CourseDef {
        name: "soft".into(),
        pad_behind_m: 40.0,
        run_out_m: 100.0,
        blend_m: 12.0,
        time_limit_s: 180.0,
        segments: vec![seg(60.0, 0.0, "concrete"), seg(240.0, 0.0, "soft_earth"), seg(60.0, 0.0, "concrete")],
        checkpoints_m: vec![0.0, 360.0],
    };
    Course::bake_with(&def, &terrain()).unwrap()
}

/// A 30 tonne tracked vehicle: two 0.6 m x 4.5 m tracks (50 kPa), 0.5 m of hull clearance, 500 kW.
fn tank() -> MoverSpec {
    MoverSpec {
        id: "tank".into(),
        class: GearClass::Tracks,
        mass_t: 30.0,
        drive_kw: 500.0,
        rated_ms: 18.0,
        c_roll: 0.06,
        grip_mu: 0.9,
        cd_a_m2: 5.0,
        span_m: 4.5,
        contact_units: 2,
        contact_width_m: 0.6,
        contact_len_m: 4.5,
        contact_area_m2: 2.0 * 0.6 * 4.5,
        clearance_m: 0.5,
        ..MoverSpec::default()
    }
}

fn finished_ticks(r: &w5k_sim::Run) -> u32 {
    match &r.outcome {
        Outcome::Finished { ticks, .. } => *ticks,
        other => panic!("{other:?}"),
    }
}

#[test]
fn sinkage_is_linear_in_pressure_and_stiffer_under_narrow_footprints() {
    let soil = earth();
    // z = p / (kc / b + kphi): exactly proportional to the pressure (n = 1) ...
    let (z1, z2) = (soil.sinkage(fx(40.0), fx(0.6)).to_f64(), soil.sinkage(fx(80.0), fx(0.6)).to_f64());
    assert!((z2 / z1 - 2.0).abs() < 1e-6, "{z1} {z2}");
    // ... and equal to the formula in floats.
    assert!((z1 - 40.0 / (60.0 / 0.6 + 170.0)).abs() < 1e-6, "{z1}");
    // The soil beside a narrow footprint helps to carry it: the same pressure sinks less.
    let narrow = soil.sinkage(fx(100.0), fx(0.15)).to_f64();
    let wide = soil.sinkage(fx(100.0), fx(1.5)).to_f64();
    assert!(narrow < wide, "narrow {narrow} m, wide {wide} m");
}

#[test]
fn the_thrust_law_starts_like_the_exponential_and_saturates() {
    let soil = earth();
    // x / (x + 2) against 1 - (1 - e^-x) / x: the same slope at the start, a few per cent apart, both below 1 and rising.
    let (mut prev_h, mut prev_e) = (0.0, 0.0);
    for l in [0.01, 0.05, 0.1, 0.4, 1.0, 3.0, 5.0, 20.0] {
        let h = soil.thrust_fraction(fx(l)).to_f64();
        let x = 0.6 * l / 0.02;
        let exact = 1.0 - (1.0 - (-x).exp()) / x;
        assert!(h > prev_h && h < 1.0, "L {l}: {h}");
        assert!(exact > prev_e);
        assert!((h - exact).abs() < 0.13, "L {l}: hyperbola {h}, exponential {exact}");
        if x < 0.1 {
            assert!((h - exact).abs() < 0.01 * exact + 1e-4, "same initial slope: {h} {exact}");
        }
        (prev_h, prev_e) = (h, exact);
    }
}

#[test]
fn the_soil_forces_follow_their_formulas() {
    let soil = earth();
    let fp = Footprint { units: fx(2.0), width: fx(0.6), length: fx(4.5), area: fx(5.4), stride: Fx::ZERO, clearance: fx(0.5) };
    let (w, n) = (300.0, 300.0);
    let a = soil.act(&fp, fx(n), fx(w));
    let p = n / 5.4;
    let z = p / (60.0 / 0.6 + 170.0);
    assert!((a.sink.to_f64() - z).abs() < 1e-6);
    // Compaction: R_c = 1/2 B p z for rolling footprints of total width B.
    assert!((a.compaction.to_f64() - 0.5 * 1.2 * p * z).abs() < 1e-4, "{}", a.compaction.to_f64());
    // Hull drag: a cubic ramp reaching a quarter of the weight at a sinkage equal to the clearance.
    assert!((a.hull.to_f64() - 0.25 * w * (z / 0.5).powi(3)).abs() < 1e-4);
    let at_clearance = soil.act(&Footprint { clearance: fx(z), ..fp }, fx(n), fx(w));
    assert!((at_clearance.hull.to_f64() - 0.25 * w).abs() < 0.01, "{}", at_clearance.hull.to_f64());
    // Strength A c + N tan(phi), of which the working slip gives x / (x + 2).
    let strength = 5.4 * 4.0 + n * 22.0f64.to_radians().tan();
    assert!((a.saturated.to_f64() - strength).abs() < 1e-3);
    let x = 0.6 * 4.5 / 0.02;
    assert!((a.shear.to_f64() - strength * x / (x + 2.0)).abs() < 0.05);
    // Feet that step: the energy of pressing every foot in once per stride is N z.
    let feet = Footprint { stride: fx(5.0), ..fp };
    let legs = soil.act(&feet, fx(n), fx(w));
    assert!((legs.compaction.to_f64() - n * z / 5.0).abs() < 1e-4);
}

#[test]
fn a_vehicle_sinks_to_the_depth_the_pressure_says_and_slows_down() {
    let course = soft_course();
    let r = run(&course, &tank());
    let ticks = finished_ticks(&r);
    // Mid-valley the sinkage equals p / k for the tank's own footprint.
    let mid = r.frames.iter().find(|f| f.s_mm > 180_000).expect("reaches the middle of the valley");
    let p = 30.0 * 9.81 / (2.0 * 0.6 * 4.5);
    let z = p / (60.0 / 0.6 + 170.0);
    assert!((mid.sink_mm as f64 / 1000.0 - z).abs() < 0.002, "sunk {} mm, expected {:.0} mm", mid.sink_mm, z * 1000.0);
    // The same vehicle on the same shape of course with every surface rigid is faster.
    let rigid = Course::bake(&CourseDef {
        name: "rigid".into(),
        pad_behind_m: 40.0,
        run_out_m: 100.0,
        blend_m: 12.0,
        time_limit_s: 180.0,
        segments: vec![seg(60.0, 0.0, "concrete"), seg(240.0, 0.0, "soft_earth"), seg(60.0, 0.0, "concrete")],
        checkpoints_m: vec![0.0, 360.0],
    })
    .unwrap();
    let dry = finished_ticks(&run(&rigid, &tank()));
    assert!(ticks > dry, "soil costs time: {ticks} against {dry} ticks");
}

#[test]
fn rigid_ground_is_exactly_what_it_was() {
    // A course whose surfaces are all rigid must run bit for bit as one baked without a terrain table: the soil terms are
    // multiplied by a fraction that is exactly zero.
    let def = CourseDef {
        name: "hard".into(),
        pad_behind_m: 40.0,
        run_out_m: 100.0,
        blend_m: 12.0,
        time_limit_s: 180.0,
        segments: vec![seg(60.0, 0.0, "concrete"), seg(80.0, -0.06, "concrete"), seg(100.0, 0.05, "concrete")],
        checkpoints_m: vec![0.0, 240.0],
    };
    let (with, without) = (Course::bake_with(&def, &terrain()).unwrap(), Course::bake(&def).unwrap());
    let (a, b) = (run(&with, &tank()), run(&without, &tank()));
    assert_eq!(a.final_hash, b.final_hash);
    assert_eq!(a.frame_bytes(), b.frame_bytes());
    // And the same vehicle with no footprint at all (a spec from before soils existed) is the same run too.
    let bare = MoverSpec { contact_units: 0, contact_width_m: 0.0, contact_len_m: 0.0, contact_area_m2: 0.0, clearance_m: 0.0, ..tank() };
    assert_eq!(run(&without, &bare).final_hash, b.final_hash);
}

#[test]
fn wider_tracks_are_never_slower_in_the_valley_at_the_same_weight() {
    let course = soft_course();
    // Same mass, same engine; only the footprint changes (the length is fixed, the width grows).
    let ticks: Vec<u32> = [0.3, 0.45, 0.6, 0.9, 1.2, 1.8]
        .iter()
        .map(|&w| {
            let spec = MoverSpec { contact_width_m: w, contact_area_m2: 2.0 * w * 4.5, ..tank() };
            match run(&course, &spec).outcome {
                Outcome::Finished { ticks, .. } => ticks,
                Outcome::Dnf { .. } => u32::MAX,
                Outcome::Dns { cause } => panic!("{cause}"),
            }
        })
        .collect();
    assert!(ticks.windows(2).all(|p| p[0] >= p[1]), "{ticks:?}");
    assert!(ticks[0] > ticks[5], "the footprint matters: {ticks:?}");
}

#[test]
fn what_does_not_touch_the_ground_does_not_care_about_it() {
    let hard = Course::bake_with(
        &CourseDef {
            name: "all hard".into(),
            pad_behind_m: 40.0,
            run_out_m: 100.0,
            blend_m: 12.0,
            time_limit_s: 180.0,
            segments: vec![seg(60.0, 0.0, "concrete"), seg(240.0, 0.0, "concrete"), seg(60.0, 0.0, "concrete")],
            checkpoints_m: vec![0.0, 360.0],
        },
        &terrain(),
    )
    .unwrap();
    let soft = soft_course();
    let cushion = MoverSpec { class: GearClass::Cushion, grip_mu: 0.0, thrust_w: 0.12, c_roll: 0.03, skirt_drag: true, drive_kw: 800.0, mass_t: 17.0, rated_ms: 30.0, cd_a_m2: 8.0, span_m: 10.0, ..MoverSpec::default() };
    let rotor = MoverSpec { class: GearClass::Rotor, grip_mu: 0.0, thrust_w: 0.3, c_roll: 0.0, drive_kw: 300.0, mass_t: 6.0, rated_ms: 80.0, cd_a_m2: 3.0, span_m: 8.0, altitude_m: 6.0, ..MoverSpec::default() };
    for spec in [cushion, rotor] {
        assert_eq!(run(&hard, &spec).final_hash, run(&soft, &spec).final_hash, "{:?}", spec.class);
    }
}

#[test]
fn a_footprint_entering_soft_ground_sinks_in_smoothly() {
    let course = soft_course();
    let r = run(&course, &tank());
    let sinks: Vec<f64> = r.frames.iter().take_while(|f| f.run_state() == run_state::RUNNING).map(|f| f.sink_mm as f64).collect();
    let first = sinks.iter().position(|&z| z > 0.0).expect("it sinks");
    assert!(first > 20, "starts to sink only after the soft ground begins");
    let deepest = sinks.iter().cloned().fold(0.0, f64::max);
    assert!(deepest > 100.0 && deepest < 300.0, "{deepest}");
    // It does not drop in: the sinkage follows the share of the footprint on the soil, which builds up over the length of the
    // footprint (4.5 m, about six ticks at 14 m/s), rising all the way.
    let (a, b) = (sinks.iter().position(|&z| z >= 0.1 * deepest).unwrap(), sinks.iter().position(|&z| z >= 0.9 * deepest).unwrap());
    assert!(b - a >= 4, "from 10% to 90% of the depth in {} ticks", b - a);
    assert!(sinks[first..=b].windows(2).all(|w| w[1] >= w[0]), "rising: {:?}", &sinks[first..=b]);
}

#[test]
fn heavy_narrow_footprints_bog_and_say_so() {
    let course = soft_course();
    // 80 tonnes on 0.3 m tracks: it sinks past its clearance, and the same engine cannot pull it out.
    let spec = MoverSpec { mass_t: 80.0, contact_width_m: 0.3, contact_area_m2: 2.0 * 0.3 * 4.5, ..tank() };
    let r = run(&course, &spec);
    let Outcome::Dnf { cause, s_mm, detail, .. } = &r.outcome else { panic!("{:?}", r.outcome) };
    assert_eq!(*cause, Cause::Bogged, "{detail}");
    assert!(*s_mm > 60_000, "it gets into the valley before it stops: {s_mm}");
    assert!(detail.contains("soft ground") && detail.contains("kN") && detail.contains("compaction"), "{detail}");
    // Bogged means stopped: for the last three seconds of the run it did not move.
    let tail = &r.frames[r.frames.len() - 3 * HZ as usize..];
    assert!(tail.iter().all(|f| f.v_cms <= 15), "still moving: {:?}", tail.iter().map(|f| f.v_cms).collect::<Vec<_>>());
    // And the same vehicle on concrete only is merely slow, or stalled: never *bogged*.
    let hard = Course::bake_with(
        &CourseDef {
            name: "all hard".into(),
            pad_behind_m: 40.0,
            run_out_m: 100.0,
            blend_m: 12.0,
            time_limit_s: 180.0,
            segments: vec![seg(60.0, 0.0, "concrete"), seg(240.0, 0.0, "concrete"), seg(60.0, 0.0, "concrete")],
            checkpoints_m: vec![0.0, 360.0],
        },
        &terrain(),
    )
    .unwrap();
    assert!(matches!(run(&hard, &spec).outcome, Outcome::Finished { .. }));
}

#[test]
fn more_engine_gets_an_engine_limited_vehicle_out_but_not_a_ground_limited_one() {
    let course = soft_course();
    // 55 t on 0.5 m tracks sinks 0.4 m, short of its clearance: the soil would give 240 kN and the slope of its resistance is
    // under that, so it is the engine that is short at low speed. More engine gets it through.
    let heavy = MoverSpec { mass_t: 55.0, contact_width_m: 0.5, contact_area_m2: 2.0 * 0.5 * 4.5, ..tank() };
    let r = run(&course, &MoverSpec { drive_kw: 300.0, ..heavy.clone() });
    let Outcome::Dnf { cause: Cause::Bogged, detail, .. } = &r.outcome else { panic!("{:?}", r.outcome) };
    assert!(detail.contains("more engine power"), "the engine was the limit: {detail}");
    assert!(matches!(run(&course, &MoverSpec { drive_kw: 3000.0, ..heavy }).outcome, Outcome::Finished { .. }));
    // 70 t on 0.4 m tracks sinks past its clearance: the hull drags with more than the soil can give, and no engine helps.
    let hopeless = MoverSpec { mass_t: 70.0, contact_width_m: 0.4, contact_area_m2: 2.0 * 0.4 * 4.5, drive_kw: 3000.0, ..tank() };
    let Outcome::Dnf { cause: Cause::Bogged, detail, .. } = run(&course, &hopeless).outcome else { panic!("a hull on the ground is not pulled out by power") };
    assert!(detail.contains("footprint"), "the ground was the limit: {detail}");
}

#[test]
fn a_ground_vehicle_with_no_footprint_cannot_run_a_course_with_soft_ground() {
    let course = soft_course();
    let spec = MoverSpec { contact_units: 0, contact_width_m: 0.0, contact_len_m: 0.0, contact_area_m2: 0.0, ..tank() };
    let Outcome::Dns { cause } = run(&course, &spec).outcome else { panic!("it has no footprint to sink") };
    assert!(cause.contains("contact"), "{cause}");
}

#[test]
fn unknown_surfaces_are_refused_when_there_is_a_terrain_table() {
    let def = CourseDef {
        name: "oops".into(),
        pad_behind_m: 0.0,
        run_out_m: 10.0,
        blend_m: 12.0,
        time_limit_s: 60.0,
        segments: vec![seg(100.0, 0.0, "quicksand")],
        checkpoints_m: vec![],
    };
    assert!(Course::bake_with(&def, &terrain()).unwrap_err().contains("quicksand"));
    assert!(Course::bake(&def).is_ok(), "without a table every surface is rigid");
}
