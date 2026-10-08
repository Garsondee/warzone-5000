//! The time trial's physics against the vehicle sheet: the same vehicle must be the same vehicle in both.

use std::path::PathBuf;

use w5k_forge::Forge;
use w5k_sim::trial::run;
use w5k_sim::{Cause, Course, CourseDef, GearClass, Outcome, SegmentDef, HZ};

fn forge() -> Forge {
    Forge::load(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content")).expect("content loads")
}

/// A long flat concrete road: nothing but the vehicle and the air. Long, and with a long clock, because a 400 tonne vehicle
/// takes minutes to get up to speed (its time constant is its mass over the slope of its resistance).
fn flat_road() -> Course {
    Course::bake(&CourseDef {
        name: "flat".into(),
        pad_behind_m: 60.0,
        run_out_m: 100.0,
        blend_m: 12.0,
        time_limit_s: 1200.0,
        segments: vec![SegmentDef { length_m: 80_000.0, grade: 0.0, surface: "concrete".into() }],
        checkpoints_m: vec![0.0, 80_000.0],
    })
    .expect("flat road")
}

/// Fastest speed over a run (km/h), counting only while it was racing.
fn top_speed_kmh(r: &w5k_sim::Run) -> f64 {
    r.frames.iter().filter(|f| f.run_state() == w5k_sim::replay::run_state::RUNNING).map(|f| f.v_cms as f64 * 0.036).fold(0.0, f64::max)
}

#[test]
fn every_design_bakes_to_a_runnable_mover_or_says_why_not() {
    let forge = forge();
    let mut runnable = 0;
    for (id, design) in &forge.designs {
        let (asm, sheet) = forge.quick_design(design);
        let spec = forge.mover_spec(id, &asm, &sheet);
        match &spec.dns {
            Some(why) => assert!(spec.class == GearClass::Rail || sheet.locomotion.is_empty() || !sheet.problems.is_empty(), "{id} cannot start: {why}"),
            None => {
                runnable += 1;
                assert!(spec.mass_t > 0.0 && spec.drive_kw > 0.0 && spec.rated_ms > 0.0, "{id}: {spec:?}");
                assert!(spec.span_m >= 1.0 && spec.span_m <= 60.0, "{id}: span {}", spec.span_m);
                assert!(spec.cd_a_m2 > 0.0, "{id}: no drag area");
                // Ground gear grips; fliers and cushions push with thrust. Never both, never neither.
                assert!((spec.grip_mu > 0.0) != (spec.thrust_w > 0.0), "{id}: grip {} thrust {}", spec.grip_mu, spec.thrust_w);
            }
        }
    }
    assert!(runnable >= 15, "most of the army runs: {runnable}");
    // The rail vehicle is the one that must refuse this course, and say so.
    let rail = forge.designs.get("rail_battery").expect("the rail battery is in the army");
    let (asm, sheet) = forge.quick_design(rail);
    let r = run(&flat_road(), &forge.mover_spec("rail_battery", &asm, &sheet));
    let Outcome::Dns { cause } = r.outcome else { panic!("a rail vehicle cannot run a course without rails") };
    assert!(cause.contains("rail"), "{cause}");
}

#[test]
fn the_simulated_top_speed_agrees_with_the_sheet() {
    // The sheet solves the power balance for a speed; the simulation reaches it by integrating Newton's second law with a
    // governor at the gear's rating. On a long flat road the two must meet. (Cushions drag the skirt harder as they speed
    // up, where the sheet charges a flat rolling resistance, so they may run a little faster than it says.)
    let forge = forge();
    let road = flat_road();
    let mut table = Vec::new();
    for (id, design) in &forge.designs {
        let (asm, sheet) = forge.quick_design(design);
        let spec = forge.mover_spec(id, &asm, &sheet);
        if spec.dns.is_some() {
            continue;
        }
        let r = run(&road, &spec);
        let (sim, want) = (top_speed_kmh(&r), sheet.top_speed_kmh);
        table.push(format!("{id:<20} {:<12} sheet {want:6.1} km/h  sim {sim:6.1} km/h  ratio {:.3}", spec.class.name(), sim / want));
        let hi = if spec.class == GearClass::Cushion { 1.25 } else { 1.03 };
        assert!(sim >= 0.88 * want && sim <= hi * want, "{id}: simulated {sim:.1} km/h against the sheet's {want:.1} km/h\n{}", table.join("\n"));
    }
    println!("{}", table.join("\n"));
}

#[test]
fn more_engine_never_slows_a_real_design_down() {
    // Double the engine of one tracked design: it must not get slower over the hill-and-valley course.
    let forge = forge();
    let course = Course::bake(&ron::from_str::<CourseDef>(&std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/courses/hill_valley.ron")).unwrap()).unwrap()).unwrap();
    let design = forge.designs.get("lancer_mk1").expect("lancer_mk1");
    let (asm, sheet) = forge.quick_design(design);
    let base = forge.mover_spec("lancer_mk1", &asm, &sheet);
    let ticks = |drive: f64| match run(&course, &w5k_sim::MoverSpec { drive_kw: drive, ..base.clone() }).outcome {
        Outcome::Finished { ticks, .. } => ticks,
        Outcome::Dnf { cause, .. } => {
            assert_ne!(cause, Cause::Bogged);
            u32::MAX
        }
        Outcome::Dns { .. } => u32::MAX,
    };
    let times: Vec<u32> = [0.5, 1.0, 2.0].iter().map(|k| ticks(base.drive_kw * k)).collect();
    assert!(times.windows(2).all(|w| w[0] >= w[1]), "{times:?} ({} ticks per second)", HZ);
}
