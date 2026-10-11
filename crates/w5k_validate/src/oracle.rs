//! Closed-form oracles for the proving-ground tests: braking, side-slope rollover, step climb.
//! Each oracle is evaluated on the **inputs the runner echoed**, so the simulation is judged against its own numbers.
//! Spec and the reasoning behind every band: `docs/validation/proving-ground.md` section 3.

use w5k_math::scalar::{asin, atan, sqrt, G};

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
    let note = if d < expected {
        "stopped shorter than friction allows: the tyre force exceeds its own peak".to_string()
    } else if d > expected * (1.0 + ORACLE_AMBER) {
        format!(
            "{:.0}% longer than v^2/(2 mu g): the brakes or the load distribution set the distance, not tyre friction",
            PERCENT_OF * (d / expected - 1.0)
        )
    } else {
        String::new()
    };
    let mut checks = vec![check("stop_distance_m", d, expected, light_above_floor(d / expected), &note)];
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

/// g. Step climb. Two references: the kinematic bound (a rigid wheel cannot clear a step higher than its own radius: red above it) and
/// the quasi-static traction limit at the corner, `h <= r (1 - 1/sqrt(1 + mu^2))` (the wheel's contact normal at the corner must stay
/// inside the friction cone, `tan(phi) <= mu` with `cos(phi) = (r - h) / r`; derivation in `docs/lanes/arch/proving.md`, used here as the
/// reference for the lower band, `docs/theory/validation.md`). Momentum lets a real climb exceed the quasi-static limit (the Mule clears
/// 0.8 r), so only a result well **below** it is suspect: green from half the limit, amber from 30% of it, red below that.
pub fn score_step(r: &ProvingResult) -> Result<TestScore, String> {
    r.check()?;
    if let Some(s) = early(r) {
        return Ok(s);
    }
    let (radius, mu) = (r.inputs["wheel_radius_m"], r.inputs["mu"]);
    let limit = radius.min(radius * (1.0 - 1.0 / sqrt(1.0 + mu * mu)));
    let h = r.measured["step_height_m"];
    let light = if !h.is_finite() || h > radius * (1.0 + BOUND_SLACK) {
        Light::Red
    } else if h >= STEP_GREEN_FRACTION * limit {
        Light::Green
    } else if h >= STEP_AMBER_FRACTION * limit {
        Light::Amber
    } else {
        Light::Red
    };
    let note = if h > radius {
        "higher than one wheel radius: contact is not a rigid wheel on a corner".to_string()
    } else if h < STEP_AMBER_FRACTION * limit {
        format!(
            "well below the quasi-static corner limit {limit:.3} m: the model hides a bug or the run-up is too slow"
        )
    } else {
        String::new()
    };
    Ok(finish(r, vec![check("step_height_m", h, limit, light, &note)]))
}

/// Shares of the quasi-static corner limit (ESTIMATE bands, proving-ground.md (g)).
const STEP_GREEN_FRACTION: f64 = 0.5; // const-ok: ESTIMATE band, proving-ground.md (g)
const STEP_AMBER_FRACTION: f64 = 0.3; // const-ok: ESTIMATE band, proving-ground.md (g)

/// 48 km/h, the speed the acceleration test times to.
const ACCEL_SPEED_M_S: f64 = 48.0 / 3.6; // const-ok: 48 km/h in m/s
/// ESTIMATE bands of proving-ground.md (a): gearing, shift time and drag put real trucks within this multiple of the physical bound.
const ACCEL_GREEN_X: f64 = 3.0; // const-ok: ESTIMATE band, proving-ground.md (a)
const ACCEL_AMBER_X: f64 = 5.0; // const-ok: ESTIMATE band, proving-ground.md (a)

/// a. 0-48 km/h: the vehicle cannot beat the energy it is given (`m v^2 / 2P`) or the traction it has (`v / (mu f g)`).
/// A time below that bound is impossible (red); within 3x green, within 5x amber, slower red.
pub fn score_accel(r: &ProvingResult) -> Result<TestScore, String> {
    r.check()?;
    if let Some(s) = early(r) {
        return Ok(s);
    }
    let (m, p, mu, f) = (r.inputs["mass_kg"], r.inputs["power_w"], r.inputs["mu"], r.inputs["driven_load_fraction"]);
    let v = ACCEL_SPEED_M_S;
    let t_min = (m * v * v / (2.0 * p)).max(v / (mu * f * G));
    let t = r.measured["t_0_48_s"];
    let ratio = t / t_min;
    let light = if !ratio.is_finite() || ratio < 1.0 - BOUND_SLACK {
        Light::Red
    } else if ratio <= ACCEL_GREEN_X {
        Light::Green
    } else if ratio <= ACCEL_AMBER_X {
        Light::Amber
    } else {
        Light::Red
    };
    let note = if ratio < 1.0 { "faster than the energy or the traction allows: energy is created" } else { "" };
    Ok(finish(r, vec![check("t_0_48_s", t, t_min, light, note)]))
}

/// The steepest grade the wheels can push against: `T / r = m g (sin a + c cos a)` solved for `a`, as rise over run.
fn torque_grade(m: f64, torque_nm: f64, wheel_radius_m: f64, rolling: f64) -> f64 {
    let k = torque_nm / (wheel_radius_m * m * G);
    let amp = sqrt(1.0 + rolling * rolling);
    let a = asin((k / amp).min(1.0)) - atan(rolling);
    let (s, c) = w5k_math::scalar::sin_cos(a);
    s / c
}

/// c. Gradeability: bound `min(mu f, torque grade)`; the torque input is an upper bound (converter stall multiplication at peak
/// torque). Above the bound is red (physics beaten); within 10% under it green, within 20% amber, further under red with the
/// reason that something other than traction or torque limits the climb.
pub fn score_gradeability(r: &ProvingResult) -> Result<TestScore, String> {
    r.check()?;
    if let Some(s) = early(r) {
        return Ok(s);
    }
    let i = |k: &str| r.inputs[k];
    let traction = i("mu") * i("driven_load_fraction");
    let torque =
        torque_grade(i("mass_kg"), i("wheel_torque_crawl_nm"), i("wheel_radius_m"), i("rolling_resistance_coeff"));
    let (bound, limit) = if traction <= torque { (traction, "traction") } else { (torque, "torque") };
    let g = r.measured["max_grade_ratio"];
    let ratio = g / bound;
    let light = if !ratio.is_finite() || ratio > 1.0 + BOUND_SLACK {
        Light::Red
    } else if ratio >= 1.0 - ORACLE_GREEN {
        Light::Green
    } else if ratio >= 1.0 - ORACLE_AMBER {
        Light::Amber
    } else {
        Light::Red
    };
    let note = if ratio > 1.0 {
        format!("above the {limit} bound: physics beaten")
    } else if ratio < 1.0 - ORACLE_GREEN {
        format!("{:.0}% of the {limit} bound: something other than traction or torque limits the climb (launch, converter, shift logic)", PERCENT_OF * ratio)
    } else {
        format!("{limit}-limited")
    };
    Ok(finish(r, vec![check("max_grade_ratio", g, bound, light, &note)]))
}

/// The published-bracket part of gradeability (`published <= sim <= 1.25 published`, ADR-0007): **provisional** while the published
/// figure is only a secondary source (dossier evidence); it is a separate check, never folded into the oracle light.
pub fn published_grade_bracket(r: &ProvingResult, published: &crate::dossier::Quantity) -> Check {
    let row = crate::verdict::judge_bracket(published, r.measured.get("max_grade_ratio").copied().unwrap_or(f64::NAN));
    check(
        "published_grade_bracket",
        row.simulated.unwrap_or(f64::NAN),
        published.param.v,
        row.light,
        "PROVISIONAL: the published figure is secondary (UNVERIFIED)",
    )
}

const PERCENT_OF: f64 = 100.0; // const-ok: unit conversion for display
/// A skidpad limit below this share of the bound is amber (proving-ground.md (d)).
const SKIDPAD_AMBER_FRACTION: f64 = 0.7; // const-ok: ESTIMATE band, proving-ground.md (d)
/// A front-load fraction this close to one half gives no expectation for the understeer sign.
const UNDERSTEER_DEADBAND: f64 = 0.02; // const-ok: judgement, proving-ground.md (d)

/// d. Skidpad: lateral acceleration is bounded by `min(mu, t / 2h)` (slide or roll first); above it is red, below 70% of it amber.
/// The understeer gradient's sign is checked against the one the axle loads suggest (heavy front: positive): UNVALIDATED, shown as such.
pub fn score_skidpad(r: &ProvingResult) -> Result<TestScore, String> {
    r.check()?;
    if let Some(s) = early(r) {
        return Ok(s);
    }
    let i = |k: &str| r.inputs[k];
    let bound = i("mu").min(i("track_m") / (2.0 * i("cg_height_m")));
    let a = r.measured["max_lat_accel_g"];
    let ratio = a / bound;
    let light = if !ratio.is_finite() || ratio > 1.0 + BOUND_SLACK {
        Light::Red
    } else if ratio >= SKIDPAD_AMBER_FRACTION {
        Light::Green
    } else {
        Light::Amber
    };
    let mut note = if ratio > 1.0 {
        "above min(mu, t/2h): the tyres or the roll limit are not enforced".to_string()
    } else {
        String::new()
    };
    if let Some(by) = r.labels.get("limited_by") {
        note = if note.is_empty() { format!("limited by: {by}") } else { format!("{note}; limited by: {by}") };
    }
    let mut checks = vec![check("max_lat_accel_g", a, bound, light, &note)];
    let (k, front) = (r.measured["understeer_gradient_rad_per_g"], i("front_load_fraction"));
    let sign = if (front - 0.5).abs() <= UNDERSTEER_DEADBAND {
        check(
            "understeer_sign",
            k,
            0.0,
            Light::NotMeasured,
            "axle loads are nearly equal: no expected sign (UNVALIDATED)",
        )
    } else {
        let want_positive = front > 0.5;
        let ok = (k > 0.0) == want_positive;
        check(
            "understeer_sign",
            k,
            if want_positive { 1.0 } else { -1.0 },
            if ok { Light::Green } else { Light::Red },
            "UNVALIDATED: expected sign from the axle load split (K_us = W_f/C_af - W_r/C_ar needs the tyre stiffness)",
        )
    };
    checks.push(sign);
    Ok(finish(r, checks))
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
        let slow = score_braking(&braking(oracle * 1.5, 0.5)).expect("score");
        assert_eq!(slow.light, Light::Red);
        assert!(
            slow.checks[0].note.contains("50% longer") && slow.checks[0].note.contains("brakes"),
            "{}",
            slow.checks[0].note
        );
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
        // r = 0.47, mu = 0.8: the quasi-static limit is r (1 - 1/sqrt(1.64)) = 0.103 m
        assert_eq!(
            score_step(&step(0.35)).expect("score").light,
            Light::Green,
            "momentum may exceed the quasi-static limit"
        );
        assert_eq!(score_step(&step(0.07)).expect("score").light, Light::Green);
        assert_eq!(score_step(&step(0.04)).expect("score").light, Light::Amber);
        assert_eq!(score_step(&step(0.02)).expect("score").light, Light::Red, "far below the corner limit");
        assert_eq!(score_step(&step(0.60)).expect("score").light, Light::Red, "higher than the wheel radius");
    }

    #[test]
    fn an_early_end_makes_the_whole_test_red_with_the_reason() {
        let mut r = braking(10.0, 0.8);
        r.ended_early = Some("rolled over".into());
        let s = score_braking(&r).expect("score");
        assert_eq!(s.light, Light::Red);
        assert_eq!(s.checks[0].note, "rolled over");
    }

    fn accel(t: f64) -> ProvingResult {
        result(
            "accel_0_48kmh",
            &[("mass_kg", 2620.0), ("power_w", 100000.0), ("mu", 0.9), ("driven_load_fraction", 1.0)],
            &[("t_0_48_s", t)],
        )
    }

    #[test]
    fn a_launch_cannot_beat_the_energy_it_is_given_or_the_traction_it_has() {
        // t_min = max(m v^2 / 2P, v / (mu f g)) = max(4.66 s, 1.51 s) = 4.66 s: power-limited
        let t_min = 2620.0 * ACCEL_SPEED_M_S * ACCEL_SPEED_M_S / (2.0 * 100000.0);
        assert_eq!(score_accel(&accel(t_min * 1.5)).expect("score").light, Light::Green);
        assert_eq!(score_accel(&accel(t_min * 4.0)).expect("score").light, Light::Amber);
        assert_eq!(score_accel(&accel(t_min * 6.0)).expect("score").light, Light::Red);
        // negative control: energy created from nothing
        let s = score_accel(&accel(t_min * 0.7)).expect("score");
        assert_eq!(s.light, Light::Red);
        assert!(s.checks[0].note.contains("energy is created"));
    }

    #[test]
    fn a_grip_limited_launch_is_bounded_by_mu_f_g() {
        // a huge engine: the bound becomes v / (mu f g) = 1.51 s and 0.5 s is impossible
        let big = |t: f64| {
            result(
                "accel_0_48kmh",
                &[("mass_kg", 1000.0), ("power_w", 1.0e7), ("mu", 0.9), ("driven_load_fraction", 1.0)],
                &[("t_0_48_s", t)],
            )
        };
        assert_eq!(score_accel(&big(0.5)).expect("score").light, Light::Red);
        assert_eq!(score_accel(&big(2.0)).expect("score").light, Light::Green);
    }

    fn grade(g: f64, torque: f64) -> ProvingResult {
        result(
            "gradeability",
            &[
                ("mass_kg", 2620.0),
                ("mu", 0.9),
                ("driven_load_fraction", 1.0),
                ("wheel_torque_crawl_nm", torque),
                ("wheel_radius_m", 0.41),
                ("rolling_resistance_coeff", 0.018),
            ],
            &[("max_grade_ratio", g), ("bracket_lo", g), ("bracket_hi", g)],
        )
    }

    #[test]
    fn the_steepest_grade_is_the_smaller_of_the_traction_and_the_torque_bound() {
        // plenty of torque: the 0.9 traction bound decides; little torque: the torque bound decides
        assert_eq!(score_gradeability(&grade(0.88, 20000.0)).expect("score").light, Light::Green);
        let torque_bound = torque_grade(2620.0, 4000.0, 0.41, 0.018);
        assert!(torque_bound < 0.9 && torque_bound > 0.1, "{torque_bound}");
        assert_eq!(score_gradeability(&grade(torque_bound * 0.95, 4000.0)).expect("score").light, Light::Green);
        // negative control: a climb above the bound beats physics
        assert_eq!(score_gradeability(&grade(torque_bound * 1.2, 4000.0)).expect("score").light, Light::Red);
        assert_eq!(score_gradeability(&grade(1.2, 20000.0)).expect("score").light, Light::Red);
    }

    #[test]
    fn a_climb_far_below_both_bounds_is_red_and_says_something_else_limits_it() {
        let s = score_gradeability(&grade(0.4, 20000.0)).expect("score");
        assert_eq!(s.light, Light::Red);
        assert!(s.checks[0].note.contains("something other than traction or torque"));
        assert_eq!(score_gradeability(&grade(0.75, 20000.0)).expect("score").light, Light::Amber);
    }

    #[test]
    fn the_torque_grade_matches_a_hand_solution() {
        // no rolling resistance: sin a = T / (r m g)
        let k = 4000.0 / (0.41 * 2620.0 * G);
        let want = k / sqrt(1.0 - k * k);
        assert!((torque_grade(2620.0, 4000.0, 0.41, 0.0) - want).abs() < 1e-9);
    }

    #[test]
    fn the_published_grade_bracket_is_a_separate_provisional_check() {
        let q = crate::dossier::Quantity {
            id: "mobility.max_grade_ratio".into(),
            category: crate::dossier::Category::Mobility,
            unit: "1".into(),
            param: w5k_contract::Param::spec(0.6, "UNVERIFIED: a search summary"),
            evidence: crate::dossier::Evidence::Secondary,
            notes: String::new(),
        };
        let c = published_grade_bracket(&grade(0.645, 20000.0), &q);
        assert_eq!(c.light, Light::Green);
        assert!(c.note.contains("PROVISIONAL"));
        assert_eq!(published_grade_bracket(&grade(0.3, 20000.0), &q).light, Light::Red);
    }

    fn skid(a: f64, k: f64, front: f64) -> ProvingResult {
        let mut r = result(
            "skidpad",
            &[("mu", 0.9), ("radius_m", 10.0), ("track_m", 1.8), ("cg_height_m", 0.9), ("front_load_fraction", front)],
            &[("max_lat_accel_g", a), ("understeer_gradient_rad_per_g", k)],
        );
        r.labels.insert("limited_by".into(), "grip plateau".into());
        r
    }

    #[test]
    fn lateral_acceleration_cannot_exceed_the_smaller_of_mu_and_t_over_2h() {
        // t/2h = 1.0, mu = 0.9: the bound is 0.9 g
        assert_eq!(score_skidpad(&skid(0.8, 0.01, 0.55)).expect("score").checks[0].light, Light::Green);
        assert_eq!(score_skidpad(&skid(0.5, 0.01, 0.55)).expect("score").checks[0].light, Light::Amber);
        // negative control: a tyre that beats mu, and a vehicle that out-corners its own roll limit
        assert_eq!(score_skidpad(&skid(1.2, 0.01, 0.55)).expect("score").light, Light::Red);
        let tall = result(
            "skidpad",
            &[("mu", 1.4), ("radius_m", 10.0), ("track_m", 1.6), ("cg_height_m", 1.6), ("front_load_fraction", 0.55)],
            &[("max_lat_accel_g", 0.9), ("understeer_gradient_rad_per_g", 0.01)],
        );
        assert_eq!(score_skidpad(&tall).expect("score").checks[0].light, Light::Red, "t/2h = 0.5 g is the bound");
    }

    #[test]
    fn a_front_heavy_truck_is_expected_to_understeer_and_the_check_says_unvalidated() {
        let ok = score_skidpad(&skid(0.8, 0.012, 0.58)).expect("score");
        assert_eq!(ok.checks[1].light, Light::Green);
        assert!(ok.checks[1].note.contains("UNVALIDATED"));
        assert_eq!(score_skidpad(&skid(0.8, -0.012, 0.58)).expect("score").checks[1].light, Light::Red);
        assert_eq!(score_skidpad(&skid(0.8, 0.012, 0.5)).expect("score").checks[1].light, Light::NotMeasured);
        assert!(ok.checks[0].note.contains("grip plateau"), "what limited it is carried");
    }
}
