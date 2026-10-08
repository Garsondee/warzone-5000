//! Time trials: one vehicle driving the course.
//!
//! [`run`] simulates a vehicle with the physics of [`crate::mover`]; [`parade`] is the physics-free reference lap (hold the
//! rated speed). The parade is useful after the physics exists too: the difference between the two is what the course cost the
//! vehicle ("how much did the hill and the launch take?").

use w5k_math::{Fx, StateHasher};

use crate::course::Course;
use crate::mover::{Mover, MoverState, Step, STUCK_SPEED};
use crate::replay::{limit, run_state, Cause, Frame, Outcome, Run, HZ};
use crate::spec::{GearClass, MoverSpec};

/// One tick in seconds as Q32.32 (2^32 / 20 = 214748364.8, rounded). The tick *count* is the clock; this only scales
/// the integration, so the rounding never accumulates into the reported times.
pub const DT: Fx = Fx::from_raw(214_748_365);
/// Sub-steps per tick (60 Hz): finer than the tick, so a hard launch or a sharp change of grade is integrated stably.
pub const SUBSTEPS: i64 = 3;
/// One sub-step in seconds.
pub const SUB_DT: Fx = Fx::from_raw(214_748_365 / SUBSTEPS);
/// Ticks of being stuck (no speed, nothing left to push with) before the run is called.
const STUCK_TICKS: u32 = 3 * HZ;

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

/// A force as a percentage of the weight (rounded).
fn pct(force: Fx, weight: Fx) -> i32 {
    if weight <= Fx::ZERO {
        0
    } else {
        to_i32((force * Fx::from_int(100)) / weight)
    }
}

/// Encode a vehicle's state as a replay frame (its sinkage and slip come from the forces of the last step).
pub fn frame(course: &Course, s: Fx, v: Fx, span: Fx, run: u8, lim: u8, forces: &Step) -> Frame {
    let (y, pitch) = pose(course, s, span);
    let (sink, slip) = (forces.sink, forces.slip);
    let w = forces.weight;
    Frame {
        s_mm: to_i32(s.mul_int(1000)),
        y_mm: to_i32(y.mul_int(1000)),
        pitch_mrad: to_i32(pitch.mul_int(1000)).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
        v_cms: to_i32(v.mul_int(100)).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
        sink_mm: to_i32(sink.mul_int(1000)).clamp(0, u16::MAX as i32) as u16,
        slip_pct: to_i32(slip.mul_int(100)).clamp(0, 100) as u8,
        state: (lim << 4) | (run & 0x0f),
        thrust_pct: pct(forces.thrust, w).clamp(0, 255) as u8,
        grip_pct: pct(forces.cap, w).clamp(0, 255) as u8,
        grade_pct: pct(forces.grade, w).clamp(-128, 127) as i8,
        resist_pct: pct(forces.roll + forces.drag + forces.soil(), w).clamp(0, 255) as u8,
    }
}

/// Brake after the finish line, at the rate that stops exactly at the end of the run-out (v^2 / 2d), but never gently.
/// For presentation only: the time was taken at the line.
fn brake(v: Fx, s: Fx, s_end: Fx) -> Fx {
    let remaining = (s_end - s).max(Fx::ONE);
    ((v * v) / remaining.mul_int(2)).max(Fx::TWO)
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
    let no_forces = Step::default();

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
        frames.push(frame(course, s, v, span, run, lim, &no_forces));
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
            v = (v - brake(v, s, s_end) * DT).max(Fx::ZERO);
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

/// Why a vehicle that is not moving cannot go on: the cause and the numbers. In soft ground it is *bogged* (the soil took more than
/// the vehicle could push); anywhere else it *stalled* (not enough engine or grip for the slope).
fn stuck_detail(last: &Step, class: GearClass) -> (Cause, String) {
    let kn = |x: Fx| x.to_f64();
    let resist = last.grade + last.roll + last.drag + last.soil();
    if last.soft >= Fx::from_ratio(1, 5) {
        let gear = match class {
            GearClass::Legs => "feet",
            GearClass::Wheels => "tyres",
            _ => "tracks",
        };
        // Which side of the contest was short: the ground (it gave all it could) or the engine (the ground had more to give).
        let ground_limited = last.thrust >= last.cap * Fx::from_ratio(95, 100);
        let advice = if ground_limited {
            "all the soil would give: a longer or wider footprint would push harder and sink less".to_string()
        } else {
            format!("while the soil would have given {:.0} kN: it needs more engine power", kn(last.cap))
        };
        return (
            Cause::Bogged,
            format!(
                "sunk {:.2} m into soft ground and stopped: the soil and the slope resisted with {:.0} kN (compaction {:.0}, hull dragging {:.0}, slope {:.0}, rolling {:.0}, air {:.0}) but the {gear} could push with only {:.0} kN, {advice}",
                kn(last.sink),
                kn(resist),
                kn(last.compaction),
                kn(last.hull),
                kn(last.grade),
                kn(last.roll),
                kn(last.drag),
                kn(last.thrust),
            ),
        );
    }
    let who = if last.limit == limit::GRIP || last.limit == limit::THRUST { "the ground (or the thrust limit)" } else { "the engine" };
    (
        Cause::Stalled,
        format!(
            "{} could not push harder: {:.0} kN of thrust against {:.0} kN of resistance (slope {:.0}, rolling {:.0}, air {:.0}) on a {:.1}% grade; the most the gear can push with here is {:.0} kN",
            who,
            kn(last.thrust),
            kn(resist),
            kn(last.grade),
            kn(last.roll),
            kn(last.drag),
            last.slope.to_f64() * 100.0,
            kn(last.cap)
        ),
    )
}

/// Simulate one vehicle on the course.
///
/// A standing start with the reference point on the start line. The clock stops when the reference point crosses the finish
/// line; the vehicle then brakes to a halt on the run-out (presentation only). The run ends early if the vehicle is
/// stuck for three seconds (`Stalled`) or the time limit passes (`TimedOut`).
pub fn run(course: &Course, spec: &MoverSpec) -> Run {
    if let Some(why) = &spec.dns {
        return Run::did_not_start(&spec.id, why);
    }
    if spec.class.grounded() && spec.contact_area_m2 <= 0.0 && course.has_soft_ground() {
        return Run::did_not_start(&spec.id, "no ground-contact data: the running gear does not say how big its footprint is, so the soil cannot be modelled");
    }
    let mover = match Mover::new(spec) {
        Ok(m) => m,
        Err(why) => return Run::did_not_start(&spec.id, &why),
    };
    let span = mover.span();
    let finish = course.finish();
    let s_end = course.s_max() - Fx::from_int(2);
    let gates = course.checkpoints();

    let mut st = MoverState::default();
    let mut last = Step::default();
    let mut hasher = StateHasher::new();
    let mut checks = Vec::new();
    let mut frames = Vec::new();
    let mut splits: Vec<u32> = Vec::new();
    let mut tick: u32 = 0;
    let mut since_check: u32 = 0;
    let mut finished: Option<u32> = None;
    let mut stuck: u32 = 0;
    let mut dnf: Option<(Cause, String)> = None;

    loop {
        let (run, lim) = match finished {
            Some(_) => (run_state::FINISHED, limit::NONE),
            None => (run_state::RUNNING, last.limit),
        };
        frames.push(frame(course, st.s, st.v, span, run, lim, &last));
        hasher.write_fx(st.s);
        hasher.write_fx(st.v);
        hasher.write_u32(tick);
        since_check += 1;
        if since_check > HZ {
            since_check = 1;
            checks.push(hasher.finish());
        }
        if finished.is_some() && (st.v <= Fx::ZERO || st.s >= s_end) {
            break;
        }
        if finished.is_none() && (dnf.is_some() || tick >= course.time_limit_ticks()) {
            break;
        }

        tick += 1;
        if finished.is_none() {
            for _ in 0..SUBSTEPS {
                last = mover.step(course, &mut st, SUB_DT);
            }
            if st.v < STUCK_SPEED && last.accel <= Fx::ZERO {
                stuck += 1;
                if stuck >= STUCK_TICKS {
                    dnf = Some(stuck_detail(&last, spec.class));
                }
            } else {
                stuck = 0;
            }
            while splits.len() + 1 < gates.len() && st.s >= gates[splits.len() + 1] {
                splits.push(tick);
            }
            if st.s >= finish {
                finished = Some(tick);
                last = Step { weight: last.weight, ..Step::default() };
            }
        } else {
            st.v = (st.v - brake(st.v, st.s, s_end) * DT).max(Fx::ZERO);
            st.s += st.v * DT;
            if st.s >= s_end {
                st.s = s_end;
                st.v = Fx::ZERO;
            }
        }
    }

    let outcome = match (finished, dnf) {
        (Some(ticks), _) => Outcome::Finished { ticks, splits },
        (None, Some((cause, detail))) => Outcome::Dnf { cause, s_mm: to_i32(st.s.mul_int(1000)), ticks: tick, detail },
        (None, None) => Outcome::Dnf {
            cause: Cause::TimedOut,
            s_mm: to_i32(st.s.mul_int(1000)),
            ticks: tick,
            detail: format!("still driving at the time limit, {:.0} m from the finish", (finish - st.s).to_f64()),
        },
    };
    Run { id: spec.id.clone(), outcome, frames, checkpoint_hashes: checks, final_hash: hasher.finish() }
}
