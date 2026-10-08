//! The mover against closed-form answers: energy conservation, constant power, terminal speed, the steepest climb.

use w5k_sim::trial::run;
use w5k_sim::{Cause, Course, CourseDef, GearClass, MoverSpec, Outcome, SegmentDef, HZ};

fn seg(length_m: f64, grade: f64) -> SegmentDef {
    SegmentDef { length_m, grade, surface: "concrete".to_string() }
}

fn course(pad: f64, segments: Vec<SegmentDef>) -> Course {
    let finish: f64 = segments.iter().map(|s| s.length_m).sum();
    Course::bake(&CourseDef { name: "test".into(), pad_behind_m: pad, run_out_m: 100.0, blend_m: 12.0, time_limit_s: 180.0, segments, checkpoints_m: vec![0.0, finish] }).unwrap()
}

fn hill_valley() -> Course {
    course(40.0, vec![seg(30.0, 0.0), seg(80.0, -0.08), seg(140.0, 0.0), seg(80.0, 0.06), seg(40.0, 0.06)])
}

/// A plain ground vehicle: 20 t, 400 kW at the wheels, grips at 0.8, rated 40 m/s.
fn base() -> MoverSpec {
    MoverSpec {
        id: "test".into(),
        class: GearClass::Tracks,
        mass_t: 20.0,
        drive_kw: 400.0,
        rated_ms: 40.0,
        c_roll: 0.02,
        c_internal: 0.0,
        grip_mu: 0.8,
        thrust_w: 0.0,
        skirt_drag: false,
        cd_a_m2: 4.0,
        span_m: 4.0,
        altitude_m: 0.0,
        launch_floor: 0.2,
        dns: None,
    }
}

fn speed_at(r: &w5k_sim::Run, t: f64) -> f64 {
    r.frames[(t * HZ as f64).round() as usize].v_cms as f64 / 100.0
}

/// Solve P = (c_roll W + k CdA v^2) v for v by bisection (kW, kN).
fn terminal_speed(spec: &MoverSpec) -> f64 {
    let w = spec.mass_t * 9.81;
    let power = |v: f64| (spec.c_roll * w + 0.5 * 1.225 / 1000.0 * spec.cd_a_m2 * v * v) * v;
    let (mut lo, mut hi) = (0.0, 500.0);
    for _ in 0..100 {
        let mid = 0.5 * (lo + hi);
        if power(mid) > spec.drive_kw { hi = mid } else { lo = mid }
    }
    lo
}

/// Height of a constant-grade ramp that starts at s = 0, in the course's own terms (flat before it).
fn ramp_height(grade: f64, s: f64) -> f64 {
    if s <= 0.0 { 0.0 } else { grade * s }
}

/// The ground as the vehicle feels it: the height averaged over its 4 m span (a box filter). For a ramp of `grade` that starts
/// at s = 0 with flat ground behind it, this is exact and piecewise quadratic.
fn felt_height(grade: f64, s: f64) -> f64 {
    let half = 2.0;
    if s >= half {
        grade * s
    } else if s <= -half {
        0.0
    } else {
        grade * (s + half) * (s + half) / 8.0
    }
}

#[test]
fn a_frictionless_slide_matches_an_independent_integration() {
    // No engine to speak of, no rolling resistance, no drag: gravity alone. The vehicle feels the slope across its span
    // (here 4 m, so the first 2 m of the ramp are felt at half strength); an f64 integration of the same law is the oracle.
    let grade = -0.08;
    let c = course(0.0, vec![seg(300.0, grade), seg(60.0, 0.0)]);
    let spec = MoverSpec { drive_kw: 0.001, grip_mu: 0.0, thrust_w: 0.0, c_roll: 0.0, cd_a_m2: 0.0, rated_ms: 200.0, ..base() };
    let r = run(&c, &spec);
    let (mut s, mut v, dt) = (0.0f64, 0.0f64, 1.0 / 2000.0);
    let mut t = 0.0;
    for check in [4.0, 8.0, 12.0] {
        while t < check {
            let half = 2.0;
            let g = (ramp_height(grade, s + half) - ramp_height(grade, s - half)) / 4.0;
            let cos = 1.0 / (1.0 + g * g).sqrt();
            v += 9.81 * (-g) * cos * dt; // grade force / mass = g sin(theta), downhill
            s += v * cos * dt;
            t += dt;
        }
        let f = &r.frames[(check * HZ as f64) as usize];
        let (sv, vv) = (f.s_mm as f64 / 1000.0, f.v_cms as f64 / 100.0);
        assert!((vv - v).abs() < 0.01 * v + 0.02, "t={check}: sim v = {vv}, oracle {v}");
        assert!((sv - s).abs() < 0.01 * s + 0.05, "t={check}: sim s = {sv}, oracle {s}");
        // Energy: the force is the slope of the felt height, so it is conservative with that potential, whatever the path:
        // v^2 / 2 = g (felt height at the start - felt height now). (Horizontal distance, so no cosine appears.)
        let want = (2.0 * 9.81 * (felt_height(grade, 0.0) - felt_height(grade, sv))).sqrt();
        assert!((vv - want).abs() < 0.01 * want + 0.02, "t={check}: v = {vv}, energy says {want}");
    }
}

#[test]
fn a_constant_power_launch_follows_the_square_root_of_time() {
    // F = P / v with no resistance and plenty of grip: m v dv/dt = P, so v^2 = 2 P t / m.
    let c = course(40.0, vec![seg(2000.0, 0.0)]);
    let spec = MoverSpec { drive_kw: 500.0, mass_t: 10.0, grip_mu: 50.0, c_roll: 0.0, cd_a_m2: 0.0, rated_ms: 100.0, launch_floor: 0.01, ..base() };
    let r = run(&c, &spec);
    for t in [4.0, 8.0, 12.0] {
        let want = (2.0 * 500.0 * t / 10.0_f64).sqrt();
        let v = speed_at(&r, t);
        assert!((v - want).abs() < 0.015 * want, "t={t}: v = {v}, expected {want}");
    }
}

#[test]
fn the_terminal_speed_is_the_power_balance() {
    // Flat, rolling resistance and drag, governor out of the way: the engine's power balances the resistance. (A light
    // vehicle, so the approach to the balance takes seconds, not minutes: the time constant is m / (P/v^2 + 2 k v).)
    let c = course(40.0, vec![seg(6000.0, 0.0)]);
    let spec = MoverSpec { mass_t: 2.0, rated_ms: 500.0, launch_floor: 0.01, ..base() };
    let want = terminal_speed(&spec);
    let r = run(&c, &spec);
    let v = speed_at(&r, 45.0);
    assert!((v - want).abs() < 0.01 * want, "sim {v}, power balance {want}");
}

#[test]
fn a_standing_start_on_a_slope_is_limited_by_grip() {
    // The steepest slope a vehicle can start on satisfies grip x cos = sin + C_rr cos, so tan(theta) = grip - C_rr.
    let spec = MoverSpec { drive_kw: 20_000.0, grip_mu: 0.9, c_roll: 0.06, cd_a_m2: 0.0, ..base() };
    let can = run(&course(0.0, vec![seg(100.0, 0.80), seg(20.0, 0.0)]), &spec);
    assert!(matches!(can.outcome, Outcome::Finished { .. }), "0.80 < 0.84 should climb: {:?}", can.outcome);
    let cannot = run(&course(0.0, vec![seg(100.0, 0.90), seg(20.0, 0.0)]), &spec);
    let Outcome::Dnf { cause, detail, .. } = &cannot.outcome else { panic!("0.90 > 0.84 must stall: {:?}", cannot.outcome) };
    assert_eq!(*cause, Cause::Stalled);
    assert!(detail.contains("kN") && detail.contains("grade"), "the failure explains itself: {detail}");
}

#[test]
fn too_little_power_stalls_on_the_climb() {
    // 40 kW: in the lowest gear it pushes 5 kN, enough to get off the line (rolling resistance is 3.9 kN), down the hill and
    // across the valley at ~9 m/s, but the 6% climb asks for 11.8 kN of slope plus 3.9 kN of rolling: the ~0.9 MJ of
    // momentum is gone after ~85 m of it, and the vehicle stalls on the climb. (With 60 kW the momentum carries it over.)
    let r = run(&hill_valley(), &MoverSpec { drive_kw: 40.0, ..base() });
    let Outcome::Dnf { cause, s_mm, detail, .. } = &r.outcome else { panic!("{:?}", r.outcome) };
    assert_eq!(*cause, Cause::Stalled, "{detail}");
    assert!(*s_mm > 250_000 && *s_mm < 370_000, "down the hill and across the valley, into the climb, but not up it: {s_mm}");
    // And with so little power that it cannot move its own rolling resistance, it never leaves the line.
    let r = run(&hill_valley(), &MoverSpec { drive_kw: 30.0, ..base() });
    let Outcome::Dnf { cause, s_mm, .. } = &r.outcome else { panic!("{:?}", r.outcome) };
    assert_eq!(*cause, Cause::Stalled);
    assert!(*s_mm < 20_000, "stuck on the line: {s_mm}");
}

#[test]
fn more_power_is_never_slower_and_more_mass_is_never_faster() {
    let c = hill_valley();
    let time = |spec: &MoverSpec| match run(&c, spec).outcome {
        Outcome::Finished { ticks, .. } => ticks,
        other => panic!("{other:?}"),
    };
    let by_power: Vec<u32> = [200.0, 400.0, 800.0, 1600.0].iter().map(|&p| time(&MoverSpec { drive_kw: p, ..base() })).collect();
    assert!(by_power.windows(2).all(|w| w[0] >= w[1]), "power: {by_power:?}");
    assert!(by_power[0] > by_power[3], "power matters: {by_power:?}");
    let by_mass: Vec<u32> = [10.0, 20.0, 40.0, 80.0].iter().map(|&m| time(&MoverSpec { mass_t: m, ..base() })).collect();
    assert!(by_mass.windows(2).all(|w| w[0] <= w[1]), "mass: {by_mass:?}");
}

/// A walker: 40 t, 300 kW, rated at 10 m/s (a gait limit well above what its power allows), so its launch floor is 0.5 m/s.
fn walker() -> MoverSpec {
    MoverSpec { class: GearClass::Legs, c_roll: 0.0, c_internal: 0.6, grip_mu: 0.8, drive_kw: 300.0, mass_t: 40.0, rated_ms: 10.0, cd_a_m2: 0.0, launch_floor: 0.05, ..base() }
}

#[test]
fn legs_pay_their_cost_of_transport_inside_the_machine() {
    // c_internal comes off the engine's force before grip is considered: the steady speed solves P = c W v + drag.
    let want = 300.0 / (0.6 * 40.0 * 9.81);
    let r = run(&course(40.0, vec![seg(1500.0, 0.0)]), &walker());
    let v = speed_at(&r, 120.0);
    assert!((v - want).abs() < 0.02 * want, "sim {v}, expected {want}");
}

#[test]
fn a_walker_is_not_a_free_rolling_cart() {
    // Downhill the slope pays part of the gait's cost, but it does not simply accelerate the walker: the steady speed
    // solves P = W v (c cos(theta) - sin(theta)). With c = 0.6 on an 8% descent that is about 15% faster than on the flat,
    // not the 2.4 times a free-rolling body would reach.
    let grade: f64 = -0.08;
    let spec = walker();
    let w = spec.mass_t * 9.81;
    let theta = grade.atan();
    let want = spec.drive_kw / (w * (0.6 * theta.cos() - (-theta).sin()));
    let flat = spec.drive_kw / (w * 0.6);
    let r = run(&course(40.0, vec![seg(2500.0, grade), seg(100.0, 0.0)]), &spec);
    let v = speed_at(&r, 150.0);
    assert!((v - want).abs() < 0.02 * want, "downhill: sim {v}, energy balance {want}");
    assert!(v > flat && v < 1.25 * flat, "a little faster than the flat speed {flat}: {v}");
    // And uphill the grade is paid in full: P = W v (c cos(theta) + sin(theta)).
    let up = 0.06f64;
    let theta = up.atan();
    let want = spec.drive_kw / (w * (0.6 * theta.cos() + theta.sin()));
    let r = run(&course(40.0, vec![seg(2500.0, up), seg(100.0, 0.0)]), &spec);
    let v = speed_at(&r, 150.0);
    assert!((v - want).abs() < 0.02 * want, "uphill: sim {v}, energy balance {want}");
}

#[test]
fn a_cushion_can_start_and_a_vehicle_cannot_outrun_its_rating() {
    let hover = MoverSpec { class: GearClass::Cushion, grip_mu: 0.0, thrust_w: 0.12, c_roll: 0.03, skirt_drag: true, drive_kw: 800.0, mass_t: 17.0, rated_ms: 25.0, ..base() };
    let r = run(&course(40.0, vec![seg(1500.0, 0.0)]), &hover);
    let v = speed_at(&r, 40.0);
    assert!(v > 5.0 && v <= 25.0 + 1e-9, "a cushion gets going, never past its rating: {v}");
    let fast = run(&course(40.0, vec![seg(3000.0, 0.0)]), &MoverSpec { drive_kw: 50_000.0, rated_ms: 30.0, ..base() });
    assert!(speed_at(&fast, 60.0) <= 30.0 + 1e-9);
}

#[test]
fn runs_are_deterministic_and_pinned() {
    let c = hill_valley();
    let a = run(&c, &base());
    let b = run(&c, &base());
    assert_eq!(a.final_hash, b.final_hash);
    assert_eq!(a.frame_bytes(), b.frame_bytes());
    assert_ne!(run(&c, &MoverSpec { drive_kw: 401.0, ..base() }).final_hash, a.final_hash);
    // A golden value: if this changes, the simulation changed (see docs/design/02-determinism-rules.md before updating it).
    assert_eq!(a.final_hash, GOLDEN, "final hash of the baseline run is {:#018x}", a.final_hash);
}

/// Final state hash of the baseline run on the hill-and-valley shape (all concrete). Terms added later (soil, ruts) must leave it
/// unchanged for hard ground: they multiply by zero where there is no soft ground.
const GOLDEN: u64 = 0xc0d6_2a27_6ce8_46c5;
