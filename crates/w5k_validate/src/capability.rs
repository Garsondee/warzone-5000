//! The capability export: per vehicle, what the proving ground measured, as JSON the mobility map (WORLD) and the AI can read.
//! Shape: `docs/swarm/requests/validation-world-capability-table.md`. Every value carries the test that produced it and, where a
//! scorer exists, its traffic light, so a consumer can decide how far to trust it. Nothing here is a hand-written guess.

use std::collections::BTreeMap;

use serde::Serialize;
use w5k_math::scalar::G;

use crate::oracle;
use crate::proving::ProvingResult;
use crate::verdict::Light;

/// A proving-ground scorer: result in, scored test out.
type Scorer = fn(&ProvingResult) -> Result<oracle::TestScore, String>;

pub const SCHEMA: &str = "w5k.capability.v1";

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Entry {
    pub value: f64,
    pub unit: &'static str,
    /// The proving-ground test that measured it.
    pub test: String,
    /// `green`, `amber`, `red` (a scorer judged it against its oracle), or `not scored` (no scorer yet).
    pub light: &'static str,
    /// What limited it, where the runner says (side-slope mode, skidpad `limited_by`).
    pub note: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Capability {
    pub schema: &'static str,
    pub vehicle: String,
    pub contract_pin: String,
    /// The surface of the proving ground the numbers were measured on (hard dry ground).
    pub surface: String,
    /// Steepest grade (rise/run) the vehicle starts on and holds.
    pub max_grade_ratio: Option<Entry>,
    /// Side slope before it rolls or slides, rad.
    pub max_side_slope_rad: Option<Entry>,
    /// Braking distance from `braking_speed_m_s`, m.
    pub braking_distance_m: Option<Entry>,
    pub braking_speed_m_s: Option<f64>,
    pub time_0_48kmh_s: Option<Entry>,
    /// Lateral acceleration at the limit of the skidpad, m/s^2.
    pub max_lateral_accel_m_s2: Option<Entry>,
    /// Highest vertical step cleared, m.
    pub max_step_m: Option<Entry>,
    /// Tests that did not run or ended early: the table has no value for them (never a guess).
    pub missing: Vec<String>,
    /// Tests that do not apply to this kind of vehicle (a tracked vehicle has no steered wheel for a skidpad, and a belt is a different
    /// obstacle problem from a wheel's step): not "missing", and never a value.
    pub not_applicable: Vec<String>,
}

fn light_word(l: Light) -> &'static str {
    match l {
        Light::Green => "green",
        Light::Amber => "amber",
        Light::Red => "red",
        Light::NotMeasured => "not scored",
    }
}

/// Tests that do not apply to a tracked vehicle.
const NOT_FOR_TRACKED: &[&str] = &["skidpad", "step_climb"];

/// Build the table from the results of one vehicle (any subset of the tests, keyed by test id). `tracked` is true for a vehicle with
/// tracks (`rig.tracks` non-empty): skidpad and step climb then read "does not apply", not "missing".
pub fn from_results(vehicle: &str, results: &BTreeMap<String, ProvingResult>, tracked: bool) -> Capability {
    let mut c = Capability {
        schema: SCHEMA,
        vehicle: vehicle.to_string(),
        surface: "dry asphalt (the proving ground's flat plane)".into(),
        ..Capability::default()
    };
    let mut missing = Vec::new();
    let mut not_applicable = Vec::new();
    let mut entry = |test: &str,
                     key: &str,
                     factor: f64,
                     unit: &'static str,
                     score: Option<Scorer>,
                     note_key: Option<&str>|
     -> Option<Entry> {
        if tracked && NOT_FOR_TRACKED.contains(&test) {
            not_applicable.push(test.to_string());
            return None;
        }
        let Some(r) = results.get(test).filter(|r| r.ended_early.is_none() && r.measured.contains_key(key)) else {
            missing.push(test.to_string());
            return None;
        };
        let scored = score.map(|f| f(r));
        let oracle_note = match &scored {
            Some(Ok(s)) => s.checks.iter().find(|c| c.name == key).map(|c| c.note.clone()).unwrap_or_default(),
            _ => String::new(),
        };
        let light = match scored {
            // the light of the check that judges this very value (the skidpad also has an understeer-sign check)
            Some(Ok(s)) => light_word(s.checks.iter().find(|c| c.name == key).map_or(s.light, |c| c.light)),
            Some(Err(_)) => "red",
            None => "not scored",
        };
        // what the runner says limited it, then the oracle's own explanation when it has one
        let label = note_key.and_then(|k| r.labels.get(k)).cloned().unwrap_or_default();
        let note = match (label.is_empty(), oracle_note.is_empty() || oracle_note.contains(&label)) {
            (true, _) => oracle_note,
            (false, true) => label,
            (false, false) => format!("{label}; {oracle_note}"),
        };
        Some(Entry { value: r.measured[key] * factor, unit, test: test.to_string(), light, note })
    };
    c.max_grade_ratio = entry("gradeability", "max_grade_ratio", 1.0, "ratio", Some(oracle::score_gradeability), None);
    c.max_side_slope_rad =
        entry("side_slope_rollover", "slope_angle_rad", 1.0, "rad", Some(oracle::score_side_slope), Some("mode"));
    c.braking_distance_m = entry("braking_50kmh", "stop_distance_m", 1.0, "m", Some(oracle::score_braking), None);
    c.time_0_48kmh_s = entry("accel_0_48kmh", "t_0_48_s", 1.0, "s", Some(oracle::score_accel), None);
    c.max_lateral_accel_m_s2 =
        entry("skidpad", "max_lat_accel_g", G, "m/s^2", Some(oracle::score_skidpad), Some("limited_by"));
    c.max_step_m = entry("step_climb", "step_height_m", 1.0, "m", Some(oracle::score_step), None);
    c.braking_speed_m_s = results.get("braking_50kmh").and_then(|r| r.inputs.get("speed_m_s")).copied();
    c.contract_pin = results.values().next().map(|r| r.contract_pin.clone()).unwrap_or_default();
    c.missing = missing;
    c.not_applicable = not_applicable;
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(test: &str, inputs: &[(&str, f64)], measured: &[(&str, f64)], early: Option<&str>) -> ProvingResult {
        ProvingResult {
            schema: crate::proving::SCHEMA.into(),
            test: test.into(),
            vehicle: "t".into(),
            contract_pin: "contract-v0.3".into(),
            inputs: inputs.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            labels: [("mode".to_string(), "slide".to_string())].into(),
            measured: measured.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
            ended_early: early.map(String::from),
            replay: None,
        }
    }

    fn set(rs: Vec<ProvingResult>) -> BTreeMap<String, ProvingResult> {
        rs.into_iter().map(|r| (r.test.clone(), r)).collect()
    }

    #[test]
    fn the_table_carries_values_with_their_test_light_and_unit_conversions() {
        let t = set(vec![
            result(
                "skidpad",
                &[
                    ("mu", 0.9),
                    ("radius_m", 10.0),
                    ("track_m", 1.8),
                    ("cg_height_m", 0.9),
                    ("front_load_fraction", 0.55),
                ],
                &[("max_lat_accel_g", 0.8), ("understeer_gradient_rad_per_g", 0.01)],
                None,
            ),
            result(
                "braking_50kmh",
                &[("speed_m_s", 13.89), ("mu", 0.8)],
                &[("stop_distance_m", 13.0), ("peak_decel_g", 0.7)],
                None,
            ),
            result(
                "side_slope_rollover",
                &[("track_m", 1.8), ("cg_height_m", 0.9), ("mu", 0.8)],
                &[("slope_angle_rad", 0.6)],
                None,
            ),
        ]);
        let c = from_results("t", &t, false);
        let lat = c.max_lateral_accel_m_s2.expect("skidpad");
        assert!((lat.value - 0.8 * G).abs() < 1e-12 && lat.unit == "m/s^2");
        assert_eq!(lat.light, "green", "0.8 g is below min(mu, t/2h) = 0.9 g and above 70% of it");
        assert_eq!(c.braking_distance_m.expect("braking").light, "green");
        let side = c.max_side_slope_rad.expect("side");
        assert_eq!((side.light, side.note.as_str()), ("green", "slide"));
        assert_eq!(c.braking_speed_m_s, Some(13.89));
        assert_eq!(c.contract_pin, "contract-v0.3");
    }

    #[test]
    fn a_test_that_did_not_run_or_ended_early_leaves_no_value_and_is_listed() {
        let t = set(vec![result("step_climb", &[("wheel_radius_m", 0.47), ("mu", 0.8)], &[], Some("stuck"))]);
        let c = from_results("t", &t, false);
        assert!(c.max_step_m.is_none());
        assert!(c.missing.contains(&"step_climb".to_string()) && c.missing.contains(&"gradeability".to_string()));
    }

    #[test]
    fn a_value_the_oracle_rejects_is_exported_with_a_red_light_not_hidden() {
        // a stop shorter than friction allows is red, and the consumer sees it
        let t = set(vec![result(
            "braking_50kmh",
            &[("speed_m_s", 13.89), ("mu", 0.8)],
            &[("stop_distance_m", 5.0), ("peak_decel_g", 1.5)],
            None,
        )]);
        assert_eq!(from_results("t", &t, false).braking_distance_m.expect("braking").light, "red");
    }

    #[test]
    fn skidpad_and_step_do_not_apply_to_a_tracked_vehicle_and_are_not_missing() {
        let t = set(vec![result(
            "braking_50kmh",
            &[("speed_m_s", 13.89), ("mu", 0.8)],
            &[("stop_distance_m", 13.0), ("peak_decel_g", 0.7)],
            None,
        )]);
        let c = from_results("carrier", &t, true);
        assert_eq!(c.not_applicable, vec!["skidpad".to_string(), "step_climb".to_string()]);
        assert!(!c.missing.contains(&"skidpad".to_string()) && !c.missing.contains(&"step_climb".to_string()));
        assert!(c.max_lateral_accel_m_s2.is_none() && c.max_step_m.is_none());
        let wheeled = from_results("truck", &t, false);
        assert!(
            wheeled.not_applicable.is_empty() && wheeled.missing.contains(&"skidpad".to_string()),
            "a wheeled vehicle without the result is missing"
        );
    }

    #[test]
    fn acceleration_gradeability_and_skidpad_carry_an_oracle_light_not_not_scored() {
        let t = set(vec![
            result(
                "accel_0_48kmh",
                &[("mass_kg", 2620.0), ("power_w", 100000.0), ("mu", 0.9), ("driven_load_fraction", 1.0)],
                &[("t_0_48_s", 6.0)],
                None,
            ),
            result(
                "gradeability",
                &[
                    ("mass_kg", 2620.0),
                    ("mu", 0.9),
                    ("driven_load_fraction", 1.0),
                    ("wheel_torque_crawl_nm", 20000.0),
                    ("wheel_radius_m", 0.41),
                    ("rolling_resistance_coeff", 0.018),
                ],
                &[("max_grade_ratio", 0.85), ("bracket_lo", 0.85), ("bracket_hi", 0.86)],
                None,
            ),
        ]);
        let c = from_results("t", &t, false);
        assert_eq!(c.time_0_48kmh_s.expect("accel").light, "green");
        assert_eq!(c.max_grade_ratio.expect("grade").light, "green");
    }
}
