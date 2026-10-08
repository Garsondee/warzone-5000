//! Time trials: one vehicle driving the course.
//!
//! The first mover is a **parade lap**: the vehicle accelerates to its rated speed and holds it, following the terrain. It
//! proves the whole pipeline (course, replay, viewer) before any physics exists, and it stays useful afterwards as the
//! reference every physical run is compared with ("how much did the valley cost?").

use w5k_math::{Fx, StateHasher};

use crate::course::Course;
use crate::replay::{limit, run_state, Cause, Frame, Outcome, Run, HZ};

/// One tick in seconds as Q32.32 (2^32 / 20 = 214748364.8, rounded). The tick *count* is the clock; this only scales
/// the integration, so the rounding never accumulates into the reported times.
pub const DT: Fx = Fx::from_raw(214_748_365);

/// Pose of a rigid vehicle resting on two support points `span` metres apart, centred on `s`: the height of the chord's
/// midpoint and its nose-up pitch (radians). On a crest the chord sinks below the ground (the belly is about to scrape);
/// in a hollow the vehicle bridges it.
pub fn pose(course: &Course, s: Fx, span: Fx) -> (Fx, Fx) {
    let half = span * Fx::HALF;
    let (front, rear) = (course.height(s + half), course.height(s - half));
    ((front + rear) * Fx::HALF, Fx::atan2(front - rear, span))
}

fn to_i32(x: Fx) -> i32 {
    x.round().floor_int().clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

/// Encode a vehicle's state as a replay frame.
#[allow(clippy::too_many_arguments)]
pub fn frame(course: &Course, s: Fx, v: Fx, span: Fx, sink: Fx, slip: Fx, run: u8, lim: u8) -> Frame {
    let (y, pitch) = pose(course, s, span);
    Frame {
        s_mm: to_i32(s.mul_int(1000)),
        y_mm: to_i32(y.mul_int(1000)),
        pitch_mrad: to_i32(pitch.mul_int(1000)).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
        v_cms: to_i32(v.mul_int(100)).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
        sink_mm: to_i32(sink.mul_int(1000)).clamp(0, u16::MAX as i32) as u16,
        slip_pct: to_i32(slip.mul_int(100)).clamp(0, 100) as u8,
        state: (lim << 4) | (run & 0x0f),
    }
}

/// What a parade lap needs to know about a vehicle.
#[derive(Clone, Debug)]
pub struct ParadeSpec {
    /// Speed it holds (m/s).
    pub rated_ms: f64,
    /// Distance between the two support points that set its pitch (m).
    pub span_m: f64,
    /// How quickly it gets up to speed (m/s^2).
    pub accel_ms2: f64,
}

/// Drive the course at a fixed speed. Deterministic and physics-free: a standing start with the reference point on the
/// start line, a constant acceleration to the rated speed, then a brake to a stop on the run-out after the finish.
pub fn parade(course: &Course, id: &str, spec: &ParadeSpec) -> Run {
    let rated = Fx::from_f64(spec.rated_ms);
    if rated <= Fx::ZERO {
        return Run::did_not_start(id, "no running gear");
    }
    let span = Fx::from_f64(spec.span_m).clamp(Fx::ONE, Fx::from_int(30));
    let accel = Fx::from_f64(spec.accel_ms2).max(Fx::from_ratio(1, 10));
    let finish = course.finish();
    let s_end = course.s_max() - Fx::from_int(2);
    let gates = course.checkpoints();

    let mut hasher = StateHasher::new();
    let mut checks = Vec::new();
    let mut frames = Vec::new();
    let mut splits: Vec<u32> = Vec::new();
    let (mut s, mut v) = (Fx::ZERO, Fx::ZERO);
    let mut tick: u32 = 0;
    let mut finished: Option<u32> = None;
    let mut since_check: u32 = 0;

    loop {
        let (run, lim) = match finished {
            Some(_) => (run_state::FINISHED, limit::NONE),
            None if v >= rated => (run_state::RUNNING, limit::RATING),
            None => (run_state::RUNNING, limit::NONE),
        };
        frames.push(frame(course, s, v, span, Fx::ZERO, Fx::ZERO, run, lim));
        hasher.write_fx(s);
        hasher.write_fx(v);
        hasher.write_u32(tick);
        since_check += 1;
        if since_check > HZ {
            since_check = 1;
            checks.push(hasher.finish());
        }
        if finished.is_some() && (v <= Fx::ZERO || s >= s_end) {
            break;
        }
        if finished.is_none() && tick >= course.time_limit_ticks() {
            break;
        }

        tick += 1;
        if finished.is_none() {
            v = (v + accel * DT).min(rated);
        } else {
            // Brake at the rate that stops exactly at the end of the run-out (v^2 / 2d), but never gently.
            let remaining = (s_end - s).max(Fx::ONE);
            let brake = (v * v / remaining.mul_int(2)).max(Fx::TWO);
            v = (v - brake * DT).max(Fx::ZERO);
        }
        s += v * DT;
        if finished.is_some() && s >= s_end {
            s = s_end;
            v = Fx::ZERO;
        }
        if finished.is_none() {
            while splits.len() + 1 < gates.len() && s >= gates[splits.len() + 1] {
                splits.push(tick);
            }
            if s >= finish {
                finished = Some(tick);
            }
        }
    }

    let outcome = match finished {
        Some(ticks) => Outcome::Finished { ticks, splits },
        None => Outcome::Dnf { cause: Cause::TimedOut, s_mm: to_i32(s.mul_int(1000)), ticks: tick, detail: "still driving at the time limit".into() },
    };
    Run { id: id.to_string(), outcome, frames, checkpoint_hashes: checks, final_hash: hasher.finish() }
}
