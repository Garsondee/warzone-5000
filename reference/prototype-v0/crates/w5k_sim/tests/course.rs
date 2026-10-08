//! The baked course against an independent f64 reference, and its basic geometry.

use w5k_math::Fx;
use w5k_sim::{Course, CourseDef, SegmentDef};

fn hill_valley() -> CourseDef {
    let seg = |length_m: f64, grade: f64, surface: &str| SegmentDef { length_m, grade, surface: surface.to_string() };
    CourseDef {
        name: "test".into(),
        pad_behind_m: 40.0,
        run_out_m: 100.0,
        blend_m: 12.0,
        time_limit_s: 180.0,
        segments: vec![seg(30.0, 0.0, "concrete"), seg(80.0, -0.08, "concrete"), seg(140.0, 0.0, "soft_earth"), seg(80.0, 0.06, "soft_earth"), seg(40.0, 0.06, "concrete")],
        checkpoints_m: vec![0.0, 110.0, 250.0, 370.0],
    }
}

/// Reference slope in f64: piecewise grades with linear easing across each knot.
fn ref_slope(def: &CourseDef, s: f64) -> f64 {
    let mut starts = vec![-def.pad_behind_m];
    let mut grades = vec![0.0];
    let mut at = 0.0;
    for seg in &def.segments {
        starts.push(at);
        grades.push(seg.grade);
        at += seg.length_m;
    }
    starts.push(at);
    grades.push(0.0);
    let mut g = grades[0];
    for (k, &st) in starts.iter().enumerate() {
        if s >= st {
            g = grades[k];
        }
    }
    let half = def.blend_m / 2.0;
    for j in 1..starts.len() {
        let k = starts[j];
        if grades[j] != grades[j - 1] && s > k - half && s < k + half {
            return grades[j - 1] + (grades[j] - grades[j - 1]) * (s - (k - half)) / def.blend_m;
        }
    }
    g
}

fn ref_height(def: &CourseDef, s: f64) -> f64 {
    // Fine trapezoid integration of the reference slope from the back of the pad.
    let (mut h, mut x) = (0.0, -def.pad_behind_m);
    let dx = 0.01;
    while x + dx <= s {
        h += 0.5 * (ref_slope(def, x) + ref_slope(def, x + dx)) * dx;
        x += dx;
    }
    h + ref_slope(def, x) * (s - x)
}

#[test]
fn heights_match_the_reference() {
    let def = hill_valley();
    let c = Course::bake(&def).unwrap();
    for s in [-40.0, -10.0, 0.0, 29.0, 30.0, 31.5, 60.0, 109.0, 110.0, 111.0, 180.0, 249.0, 250.0, 251.0, 300.0, 329.0, 330.0, 369.0, 370.0, 371.0, 420.0, 470.0] {
        let got = c.height(Fx::from_f64(s)).to_f64();
        let want = ref_height(&def, s);
        assert!((got - want).abs() < 2e-3, "h({s}) = {got}, reference {want}");
    }
}

#[test]
fn the_valley_is_where_it_should_be() {
    let c = Course::bake(&hill_valley()).unwrap();
    let h = |s: f64| c.height(Fx::from_f64(s)).to_f64();
    // A flat pad, a drop of about 6.4 m (80 m at 8 %), a flat valley floor, then a climb of about 7.2 m (120 m at 6 %).
    assert!(h(-40.0).abs() < 1e-9 && h(0.0).abs() < 1e-3 && h(20.0).abs() < 1e-3);
    let floor = h(180.0);
    assert!((floor + 6.4).abs() < 0.2, "valley floor {floor}");
    assert!((h(200.0) - floor).abs() < 1e-3);
    let top = h(380.0);
    assert!((top - (floor + 7.2)).abs() < 0.2, "finish plateau {top}");
    assert!((h(470.0) - top).abs() < 1e-3, "the run-out is flat");
}

#[test]
fn slopes_are_the_grades_away_from_the_curves() {
    let c = Course::bake(&hill_valley()).unwrap();
    let g = |s: f64| c.slope(Fx::from_f64(s)).to_f64();
    assert!(g(10.0).abs() < 1e-6);
    assert!((g(70.0) + 0.08).abs() < 1e-6);
    assert!(g(180.0).abs() < 1e-6);
    assert!((g(290.0) - 0.06).abs() < 1e-6);
    assert!((g(350.0) - 0.06).abs() < 1e-6);
    // Mid-way through the curve at the foot of the descent the slope is half-way between -8 % and 0.
    assert!((g(110.0) + 0.04).abs() < 1e-3, "{}", g(110.0));
}

#[test]
fn the_profile_is_continuous() {
    let c = Course::bake(&hill_valley()).unwrap();
    let mut prev = c.height(c.s_min()).to_f64();
    let mut s = c.s_min().to_f64() + 0.05;
    while s < c.s_max().to_f64() {
        let h = c.height(Fx::from_f64(s)).to_f64();
        assert!((h - prev).abs() <= 0.08 * 0.05 + 1e-6, "jump at {s}: {prev} -> {h}");
        prev = h;
        s += 0.05;
    }
}

#[test]
fn surfaces_and_fractions() {
    let c = Course::bake(&hill_valley()).unwrap();
    let concrete = c.surface_id("concrete").unwrap();
    let soft = c.surface_id("soft_earth").unwrap();
    assert_eq!(c.surface(Fx::from_f64(-30.0)), concrete);
    assert_eq!(c.surface(Fx::from_f64(20.0)), concrete);
    assert_eq!(c.surface(Fx::from_f64(80.0)), concrete);
    assert_eq!(c.surface(Fx::from_f64(150.0)), soft);
    assert_eq!(c.surface(Fx::from_f64(300.0)), soft);
    assert_eq!(c.surface(Fx::from_f64(350.0)), concrete);
    // A 20 m footprint centred on the valley entry (110 m) is half soft; the fraction slides smoothly.
    let f = |a: f64, b: f64| c.fraction(soft, Fx::from_f64(a), Fx::from_f64(b)).to_f64();
    assert!((f(100.0, 120.0) - 0.5).abs() < 1e-6);
    assert!((f(105.0, 125.0) - 0.75).abs() < 1e-6);
    assert!(f(0.0, 50.0).abs() < 1e-9 && (f(150.0, 200.0) - 1.0).abs() < 1e-9);
}

#[test]
fn checkpoints_finish_and_limit() {
    let c = Course::bake(&hill_valley()).unwrap();
    assert_eq!(c.finish().to_f64(), 370.0);
    let g: Vec<f64> = c.checkpoints().iter().map(|x| x.to_f64()).collect();
    assert_eq!(g, vec![0.0, 110.0, 250.0, 370.0]);
    assert_eq!(c.time_limit_ticks(), 3600);
}

#[test]
fn baking_is_deterministic() {
    let a = Course::bake(&hill_valley()).unwrap();
    let b = Course::bake(&hill_valley()).unwrap();
    assert_eq!(a.table_hash(), b.table_hash());
}

#[test]
fn bad_courses_are_refused() {
    let mut d = hill_valley();
    d.segments[1].length_m = 8.0; // shorter than the vertical curve
    assert!(Course::bake(&d).is_err());
    let mut d = hill_valley();
    d.segments.clear();
    assert!(Course::bake(&d).is_err());
}
