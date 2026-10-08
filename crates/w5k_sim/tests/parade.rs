//! The parade lap: a constant-speed lap over the course, in a replay any viewer can play.

use w5k_math::Fx;
use w5k_sim::replay::{limit, run_state, Frame};
use w5k_sim::trial::{parade, ParadeSpec};
use w5k_sim::{Cause, Course, CourseDef, Outcome, SegmentDef, HZ};

fn course() -> Course {
    let seg = |length_m: f64, grade: f64, surface: &str| SegmentDef { length_m, grade, surface: surface.to_string() };
    Course::bake(&CourseDef {
        name: "test".into(),
        pad_behind_m: 40.0,
        run_out_m: 100.0,
        blend_m: 12.0,
        time_limit_s: 180.0,
        segments: vec![seg(30.0, 0.0, "concrete"), seg(80.0, -0.08, "concrete"), seg(140.0, 0.0, "soft_earth"), seg(80.0, 0.06, "soft_earth"), seg(40.0, 0.06, "concrete")],
        checkpoints_m: vec![0.0, 110.0, 250.0, 370.0],
    })
    .unwrap()
}

fn spec(rated: f64) -> ParadeSpec {
    ParadeSpec { rated_ms: rated, span_m: 4.0, accel_ms2: 4.0 }
}

#[test]
fn a_parade_lap_takes_the_distance_over_the_speed() {
    let c = course();
    let run = parade(&c, "tank", &spec(20.0));
    let Outcome::Finished { ticks, splits } = &run.outcome else { panic!("{:?}", run.outcome) };
    // 5 s to reach 20 m/s covers 50 m, the other 320 m take 16 s: 21 s in all (within a tick or two).
    let t = *ticks as f64 / HZ as f64;
    assert!((t - 21.0).abs() < 0.15, "finished in {t} s");
    // Splits come in order, one per checkpoint after the start, ending at the finish.
    assert_eq!(splits.len(), 3);
    assert!(splits.windows(2).all(|w| w[0] < w[1]) && splits[2] == *ticks);
    // After the finish the vehicle brakes to a halt on the run-out.
    let last = run.frames.last().unwrap();
    assert_eq!(last.run_state(), run_state::FINISHED);
    assert_eq!(last.v_cms, 0);
    assert!((last.s_mm as f64 / 1000.0) < c.s_max().to_f64());
}

#[test]
fn the_vehicle_pitches_with_the_ground() {
    let c = course();
    let run = parade(&c, "tank", &spec(20.0));
    let at = |s_m: f64| -> Frame { *run.frames.iter().find(|f| f.s_mm as f64 / 1000.0 >= s_m).unwrap() };
    assert!(at(10.0).pitch_mrad.abs() < 3, "level on the pad");
    assert!(at(70.0).pitch_mrad < -70, "nose down on the descent: {}", at(70.0).pitch_mrad);
    assert!(at(180.0).pitch_mrad.abs() < 3, "level in the valley");
    assert!(at(290.0).pitch_mrad > 50, "nose up on the climb: {}", at(290.0).pitch_mrad);
    // Heights follow the course: the valley floor is about 6.4 m below the pad.
    assert!((at(180.0).y_mm as f64 / 1000.0 + 6.4).abs() < 0.3);
    assert!(run.frames.iter().all(|f| f.state >> 4 == limit::NONE || f.state >> 4 == limit::RATING));
}

#[test]
fn a_slow_vehicle_times_out() {
    let c = course();
    let run = parade(&c, "snail", &spec(1.0));
    let Outcome::Dnf { cause, s_mm, ticks, .. } = &run.outcome else { panic!("{:?}", run.outcome) };
    assert_eq!(*cause, Cause::TimedOut);
    assert_eq!(*ticks, 3600);
    assert!(*s_mm > 150_000 && *s_mm < 190_000, "got as far as {s_mm} mm");
}

#[test]
fn no_running_gear_does_not_start() {
    let run = parade(&course(), "brick", &spec(0.0));
    assert!(matches!(run.outcome, Outcome::Dns { .. }));
    assert!(run.frames.is_empty());
}

#[test]
fn the_run_is_deterministic() {
    let c = course();
    let a = parade(&c, "a", &spec(18.5));
    let b = parade(&c, "a", &spec(18.5));
    assert_eq!(a.final_hash, b.final_hash);
    assert_eq!(a.checkpoint_hashes, b.checkpoint_hashes);
    assert_eq!(a.frame_bytes(), b.frame_bytes());
    assert_ne!(parade(&c, "a", &spec(18.6)).final_hash, a.final_hash);
}

#[test]
fn frames_round_trip_through_bytes() {
    let f = Frame { s_mm: -1234, y_mm: 987_654, pitch_mrad: -321, v_cms: 1999, sink_mm: 4321, slip_pct: 37, state: 0x41 };
    assert_eq!(Frame::from_bytes(&f.to_bytes()), f);
    assert_eq!(f.run_state(), 1);
    assert_eq!(f.limit(), 4);
    let c = course();
    let run = parade(&c, "x", &spec(20.0));
    assert_eq!(run.frame_bytes().len(), run.frames.len() * 16);
    let _ = Fx::ZERO;
}
