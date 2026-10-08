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

fn content() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content")
}

fn hill_valley(with_soil: bool) -> Course {
    let def: CourseDef = ron::from_str(&std::fs::read_to_string(content().join("courses/hill_valley.ron")).unwrap()).unwrap();
    if with_soil {
        let terrain: w5k_sim::TerrainDef = ron::from_str(&std::fs::read_to_string(content().join("terrain.ron")).unwrap()).unwrap();
        Course::bake_with(&def, &terrain).expect("the course and the terrain table agree")
    } else {
        Course::bake(&def).unwrap()
    }
}

#[test]
fn every_ground_design_has_a_real_footprint() {
    let forge = forge();
    for (id, design) in &forge.designs {
        let (asm, sheet) = forge.quick_design(design);
        let spec = forge.mover_spec(id, &asm, &sheet);
        if spec.dns.is_some() || !spec.class.grounded() {
            assert_eq!(spec.contact_area_m2, 0.0, "{id}: gear that is not on the ground has no footprint");
            continue;
        }
        assert!(spec.contact_units >= 2, "{id}: {} contact units", spec.contact_units);
        assert!(spec.contact_width_m > 0.05 && spec.contact_len_m > 0.05, "{id}: {} x {} m", spec.contact_width_m, spec.contact_len_m);
        // The footprint adds up: units x width x length is the contact area the sheet divides the weight by.
        let each = spec.contact_area_m2 / spec.contact_units as f64;
        assert!((each / (spec.contact_width_m * spec.contact_len_m)).clamp(0.3, 1.5) == each / (spec.contact_width_m * spec.contact_len_m), "{id}: {each} m2 against {} x {}", spec.contact_width_m, spec.contact_len_m);
        let kpa = sheet.mass_kg * 9.81 / spec.contact_area_m2 / 1000.0;
        assert!((kpa - sheet.ground_pressure_kpa).abs() < 1e-6 * kpa.max(1.0), "{id}: {kpa} against the sheet's {}", sheet.ground_pressure_kpa);
        assert!(spec.clearance_m > 0.1 && spec.clearance_m < 10.0, "{id}: hull clearance {} m", spec.clearance_m);
        if spec.class == GearClass::Legs {
            assert!(spec.stride_m > 1.0, "{id}: legs have a stride");
        }
    }
}

#[test]
fn soft_earth_sorts_the_army() {
    let forge = forge();
    let (dry, wet) = (hill_valley(false), hill_valley(true));
    let mut bogged = Vec::new();
    for (id, design) in &forge.designs {
        let (asm, sheet) = forge.quick_design(design);
        let spec = forge.mover_spec(id, &asm, &sheet);
        if spec.dns.is_some() {
            continue;
        }
        let (a, b) = (run(&dry, &spec), run(&wet, &spec));
        match spec.class {
            // What does not touch the ground does not care about it: the very same run, bit for bit.
            GearClass::Cushion | GearClass::AntiGrav | GearClass::Rotor => assert_eq!(a.final_hash, b.final_hash, "{id} ({})", spec.class.name()),
            _ => {
                if let Outcome::Dnf { cause, detail, .. } = &b.outcome {
                    // Too slow for the clock (a giant walker the quick path weighs a little heavy) is a fair end; stalling is not.
                    assert!(matches!(cause, Cause::Bogged | Cause::TimedOut), "{id}: {cause:?}: {detail}");
                    if *cause == Cause::Bogged {
                        assert!(detail.contains("soft ground"), "{id}: {detail}");
                        bogged.push(id.clone());
                    }
                }
                // The soil never makes a vehicle faster.
                if let (Some(t0), Some(t1)) = (a.outcome.time_s(), b.outcome.time_s()) {
                    assert!(t1 >= t0 - 0.05, "{id}: {t1} s on soft earth against {t0} s on concrete");
                }
            }
        }
    }
    // The course is worth running: it stops some designs and not all of them.
    assert!(!bogged.is_empty() && bogged.len() <= 6, "bogged: {bogged:?}");
    // The owner's own examples: a narrow-tyred scout (110 kPa on 0.15 m tyres) bogs; a light tank (20 kPa) does not notice.
    assert!(bogged.iter().any(|b| b == "wheel_scout"), "the narrow-tyred scout bogs: {bogged:?}");
    let scout = forge.designs.get("lancer_scout").expect("lancer_scout");
    let (asm, sheet) = forge.quick_design(scout);
    let spec = forge.mover_spec("lancer_scout", &asm, &sheet);
    let (t0, t1) = (run(&dry, &spec).outcome.time_s().unwrap(), run(&wet, &spec).outcome.time_s().unwrap());
    assert!(t1 < 1.05 * t0, "the light scout does not slow down in the valley: {t0} s then {t1} s");
}
