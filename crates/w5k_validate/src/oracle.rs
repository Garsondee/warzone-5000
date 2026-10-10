//! Closed-form oracles for the proving-ground tests: braking, side-slope rollover, step climb.
//! Each oracle is evaluated on the **inputs the runner echoed**, so the simulation is judged against its own numbers.
//! Spec and the reasoning behind every band: `docs/validation/proving-ground.md` section 3.

use w5k_math::scalar::{atan, G};

use crate::proving::ProvingResult;
use crate::verdict::Light;

/// ADR-0007 has no class for an exact analytic oracle. PROVISIONAL(oracle-class card): green within 10%, amber within 20%.
pub const ORACLE_GREEN: f64 = 0.10; // const-ok: oracle tolerance class, provisional default
pub const ORACLE_AMBER: f64 = 0.20; // const-ok: oracle tolerance class, provisional default
/// A result may touch an upper bound by this much (numerical slack) before it counts as having beaten physics.
const BOUND_SLACK: f64 = 0.01; // const-ok: numerical slack on a hard physical bound

#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    pub name: String,
    pub measured: f64,
    pub expected: f64,
    pub light: Light,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TestScore {
    pub test: String,
    pub vehicle: String,
    pub checks: Vec<Check>,
    /// Worst of the checks; red for the whole test when the run ended early.
    pub light: Light,
}

fn severity(l: Light) -> u8 {
    match l {
        Light::Green => 0,
        Light::NotMeasured => 1,
        Light::Amber => 2,
        Light::Red => 3,
    }
}

fn finish(r: &ProvingResult, checks: Vec<Check>) -> TestScore {
    let worst = checks.iter().map(|c| c.light).max_by_key(|l| severity(*l)).unwrap_or(Light::NotMeasured);
    let light = if r.ended_early.is_some() { Light::Red } else { worst };
    TestScore { test: r.test.clone(), vehicle: r.vehicle.clone(), checks, light }
}

fn check(name: &str, measured: f64, expected: f64, light: Light, note: &str) -> Check {
    Check { name: name.into(), measured, expected, light, note: note.into() }
}

/// A run that ended early is red with its reason; nothing else is scored.
fn early(r: &ProvingResult) -> Option<TestScore> {
    let why = r.ended_early.as_ref()?;
    Some(finish(r, vec![check("ended_early", f64::NAN, f64::NAN, Light::Red, why)]))
}

/// Light for a measured/oracle ratio where the oracle is a **lower** bound the model may not beat (braking distance).
fn light_above_floor(ratio: f64) -> Light {
    if !ratio.is_finite() || ratio < 1.0 - BOUND_SLACK {
        Light::Red
    } else if ratio <= 1.0 + ORACLE_GREEN {
        Light::Green
    } else if ratio <= 1.0 + ORACLE_AMBER {
        Light::Amber
    } else {
        Light::Red
    }
}

/// b. Braking from speed `v` on friction `mu`: `d = v^2 / (2 mu g)`. Shorter than that beats friction (red); much longer is a soft brake.
pub fn score_braking(r: &ProvingResult) -> Result<TestScore, String> {
    r.check()?;
    if let Some(s) = early(r) {
        return Ok(s);
    }
    let (v, mu) = (r.inputs["speed_m_s"], r.inputs["mu"]);
    let expected = v * v / (2.0 * mu * G);
    let d = r.measured["stop_distance_m"];
    let note =
        if d < expected { "stopped shorter than friction allows: the tyre force exceeds its own peak" } else { "" };
    let mut checks = vec![check("stop_distance_m", d, expected, light_above_floor(d / expected), note)];
    let a = r.measured["peak_decel_g"];
    let cap = mu * (1.0 + BOUND_SLACK);
    checks.push(check(
        "peak_decel_g",
        a,
        mu,
        if a <= cap { Light::Green } else { Light::Red },
        "peak deceleration cannot exceed mu g",
    ));
    Ok(finish(r, checks))
}

/// e. Side slope: tips at `atan(t / 2h)` (rigid; compliance only lowers it), slides at `atan(mu)`; the smaller happens first.
pub fn score_side_slope(r: &ProvingResult) -> Result<TestScore, String> {
    r.check()?;
    if let Some(s) = early(r) {
        return Ok(s);
    }
    let (t, h, mu) = (r.inputs["track_m"], r.inputs["cg_height_m"], r.inputs["mu"]);
    let (tip, slide) = (atan(t / (2.0 * h)), atan(mu));
    let bound = tip.min(slide);
    let a = r.measured["slope_angle_rad"];
    let ratio = a / bound;
    let light = if !ratio.is_finite() || ratio > 1.0 + 3.0 * BOUND_SLACK {
        Light::Red // const-ok: 3% slack ADR-0007 static class
    } else if ratio >= 0.75 {
        Light::Green // const-ok: ESTIMATE band for roll compliance, proving-ground.md (e)
    } else if ratio >= 0.60 {
        Light::Amber // const-ok: ESTIMATE band for roll compliance, proving-ground.md (e)
    } else {
        Light::Red
    };
    let note = if ratio > 1.0 { "above the rigid bound: tilt or centre-of-mass height is wrong" } else { "" };
    let mut checks = vec![check("slope_angle_rad", a, bound, light, note)];
    let expected_mode = if tip < slide { "roll" } else { "slide" };
    if let Some(mode) = r.labels.get("mode") {
        let ok = mode == expected_mode;
        checks.push(check(
            "mode",
            f64::NAN,
            f64::NAN,
            if ok { Light::Green } else { Light::Red },
            &format!("sim `{mode}`, oracle `{expected_mode}`"),
        ));
    } else {
        checks.push(check("mode", f64::NAN, f64::NAN, Light::NotMeasured, "runner did not report roll or slide"));
    }
    Ok(finish(r, checks))
}

/// g. Step climb: a rigid wheel cannot clear a step higher than its own radius. Only the kinematic bound is scored (spec (g)).
pub fn score_step(r: &ProvingResult) -> Result<TestScore, String> {
    r.check()?;
    if let Some(s) = early(r) {
        return Ok(s);
    }
    let radius = r.inputs["wheel_radius_m"];
    let h = r.measured["step_height_m"];
    let ratio = h / radius;
    let light = if !ratio.is_finite() || ratio > 1.0 + BOUND_SLACK {
        Light::Red
    } else if ratio >= 0.5 {
        Light::Green // const-ok: ESTIMATE band, proving-ground.md (g)
    } else if ratio >= 0.3 {
        Light::Amber // const-ok: ESTIMATE band, proving-ground.md (g)
    } else {
        Light::Red
    };
    let note = if ratio > 1.0 { "higher than one wheel radius: contact is not a rigid wheel on a corner" } else { "" };
    Ok(finish(r, vec![check("step_height_m", h, radius, light, note)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(test: &str, inputs: &[(&str, f64)], measured: &[(&str, f64)]) -> ProvingResult {
        ProvingResult {
            schema: crate::proving::SCHEMA.into(),
            test: test.into(),
            vehicle: "t".into(),
            contract_pin: String::new(),
            inputs: inputs.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            labels: Default::default(),
            measured: measured.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            ended_early: None,
            replay: None,
        }
    }

    fn braking(d: f64, a: f64) -> ProvingResult {
        result("braking_50kmh", &[("speed_m_s", 13.89), ("mu", 0.8)], &[("stop_distance_m", d), ("peak_decel_g", a)])
    }

    #[test]
    fn braking_distance_matches_v2_over_2mu_g() {
        let oracle = 13.89_f64 * 13.89 / (2.0 * 0.8 * G);
        assert_eq!(score_braking(&braking(oracle * 1.05, 0.76)).expect("score").light, Light::Green);
        assert_eq!(score_braking(&braking(oracle * 1.15, 0.7)).expect("score").light, Light::Amber);
        assert_eq!(score_braking(&braking(oracle * 1.5, 0.5)).expect("score").light, Light::Red);
    }

    #[test]
    fn a_model_that_stops_shorter_than_friction_allows_is_red() {
        // negative control: a tyre that beats its own mu
        let oracle = 13.89_f64 * 13.89 / (2.0 * 0.8 * G);
        let s = score_braking(&braking(oracle * 0.6, 1.3)).expect("score");
        assert_eq!(s.light, Light::Red);
        assert!(s.checks.iter().all(|c| c.light == Light::Red));
    }

    fn side(angle: f64, mode: Option<&str>) -> ProvingResult {
        let mut r = result(
            "side_slope_rollover",
            &[("track_m", 1.8), ("cg_height_m", 0.9), ("mu", 0.8)],
            &[("slope_angle_rad", angle)],
        );
        if let Some(m) = mode {
            r.labels.insert("mode".into(), m.into());
        }
        r
    }

    #[test]
    fn side_slope_rollover_is_bounded_by_atan_t_over_2h() {
        // t/2h = 1.0 (45 degrees) is higher than mu 0.8 (38.7 degrees): this vehicle slides first
        let slide = atan(0.8);
        assert_eq!(score_side_slope(&side(0.9 * slide, Some("slide"))).expect("score").light, Light::Green);
        assert_eq!(score_side_slope(&side(0.9 * slide, Some("roll"))).expect("score").light, Light::Red);
        assert_eq!(score_side_slope(&side(1.2 * slide, Some("slide"))).expect("score").light, Light::Red);
        assert_eq!(score_side_slope(&side(0.3 * slide, Some("slide"))).expect("score").light, Light::Red);
    }

    #[test]
    fn step_climb_above_one_wheel_radius_is_red() {
        let step = |h: f64| result("step_climb", &[("wheel_radius_m", 0.47), ("mu", 0.8)], &[("step_height_m", h)]);
        assert_eq!(score_step(&step(0.35)).expect("score").light, Light::Green);
        assert_eq!(score_step(&step(0.20)).expect("score").light, Light::Amber);
        assert_eq!(score_step(&step(0.60)).expect("score").light, Light::Red);
    }

    #[test]
    fn an_early_end_makes_the_whole_test_red_with_the_reason() {
        let mut r = braking(10.0, 0.8);
        r.ended_early = Some("rolled over".into());
        let s = score_braking(&r).expect("score");
        assert_eq!(s.light, Light::Red);
        assert_eq!(s.checks[0].note, "rolled over");
    }
}
