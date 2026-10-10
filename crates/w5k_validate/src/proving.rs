//! The proving-ground result schema `w5k.proving.result.v1`: what a runner writes and the scorer reads.
//! Spec: `docs/validation/proving-ground.md` section 2. Maps are ordered so the output is deterministic.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const SCHEMA: &str = "w5k.proving.result.v1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvingResult {
    pub schema: String,
    pub test: String,
    pub vehicle: String,
    #[serde(default)]
    pub contract_pin: String,
    /// Every number the oracle needs, as the simulation used it (SI; the key suffix is the unit).
    pub inputs: BTreeMap<String, f64>,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    pub measured: BTreeMap<String, f64>,
    /// `None` for a normal finish; otherwise why the run ended early (rolled over, stuck, NaN).
    #[serde(default)]
    pub ended_early: Option<String>,
    #[serde(default)]
    pub replay: Option<String>,
}

/// The tests of phase 1 with the `inputs` keys each oracle needs and the `measured` keys it reads (a key ending in `_at_` is a prefix:
/// one key per speed, e.g. `rms_az_m_s2_at_10`).
#[derive(Debug)]
pub struct TestSpec {
    pub id: &'static str,
    pub inputs: &'static [&'static str],
    pub measured: &'static [&'static str],
}

pub const TESTS: &[TestSpec] = &[
    TestSpec {
        id: "accel_0_48kmh",
        inputs: &["mass_kg", "power_w", "mu", "driven_load_fraction"],
        measured: &["t_0_48_s"],
    },
    TestSpec { id: "braking_50kmh", inputs: &["speed_m_s", "mu"], measured: &["stop_distance_m", "peak_decel_g"] },
    TestSpec {
        id: "gradeability",
        inputs: &[
            "mass_kg",
            "mu",
            "driven_load_fraction",
            "wheel_torque_crawl_nm",
            "wheel_radius_m",
            "rolling_resistance_coeff",
        ],
        measured: &["max_grade_ratio", "bracket_lo", "bracket_hi"],
    },
    TestSpec {
        id: "skidpad",
        inputs: &["mu", "radius_m", "track_m", "cg_height_m", "front_load_fraction"],
        measured: &["max_lat_accel_g", "understeer_gradient_rad_per_g"],
    },
    TestSpec { id: "side_slope_rollover", inputs: &["track_m", "cg_height_m", "mu"], measured: &["slope_angle_rad"] },
    TestSpec {
        id: "ride_iso8608_c",
        inputs: &[
            "sprung_mass_kg",
            "spring_rate_n_m",
            "damping_n_s_m",
            "unsprung_mass_kg",
            "tyre_rate_n_m",
            "class_gd_m3",
            "seed",
        ],
        measured: &["rms_az_m_s2_at_5", "rms_az_m_s2_at_10", "rms_az_m_s2_at_15", "rms_az_m_s2_at_20"],
    },
    TestSpec {
        id: "ride_iso8608_d",
        inputs: &[
            "sprung_mass_kg",
            "spring_rate_n_m",
            "damping_n_s_m",
            "unsprung_mass_kg",
            "tyre_rate_n_m",
            "class_gd_m3",
            "seed",
        ],
        measured: &["rms_az_m_s2_at_5", "rms_az_m_s2_at_10", "rms_az_m_s2_at_15", "rms_az_m_s2_at_20"],
    },
    TestSpec {
        id: "ride_washboard",
        inputs: &[
            "sprung_mass_kg",
            "spring_rate_n_m",
            "damping_n_s_m",
            "unsprung_mass_kg",
            "tyre_rate_n_m",
            "lambda_m",
        ],
        measured: &["rms_az_m_s2_at_5", "rms_az_m_s2_at_10", "rms_az_m_s2_at_15", "rms_az_m_s2_at_20"],
    },
    TestSpec { id: "step_climb", inputs: &["wheel_radius_m", "mu"], measured: &["step_height_m"] },
];

impl ProvingResult {
    pub fn from_json(text: &str) -> Result<ProvingResult, String> {
        serde_json::from_str(text).map_err(|e| format!("proving result JSON: {e}"))
    }

    /// Errors name the missing key; a run that is not valid is never silently scored.
    pub fn check(&self) -> Result<&'static TestSpec, String> {
        if self.schema != SCHEMA {
            return Err(format!("unknown schema `{}` (expected `{SCHEMA}`)", self.schema));
        }
        let spec = TESTS.iter().find(|t| t.id == self.test).ok_or_else(|| format!("unknown test `{}`", self.test))?;
        for k in spec.inputs {
            self.inputs.get(*k).ok_or_else(|| format!("{}: missing input `{k}`", self.test))?;
        }
        // an early end carries its own explanation; the measured keys may legitimately be absent then
        if self.ended_early.is_none() {
            for k in spec.measured {
                self.measured.get(*k).ok_or_else(|| format!("{}: missing measured `{k}`", self.test))?;
            }
        }
        for (k, v) in self.inputs.iter().chain(self.measured.iter()) {
            if !v.is_finite() {
                return Err(format!("{}: `{k}` is not finite", self.test));
            }
        }
        Ok(spec)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BRAKING: &str = r#"{"schema":"w5k.proving.result.v1","test":"braking_50kmh","vehicle":"mule_4x4",
        "inputs":{"speed_m_s":13.89,"mu":0.8},"measured":{"stop_distance_m":14.9,"peak_decel_g":0.81},"ended_early":null}"#;

    #[test]
    fn a_runner_result_parses_and_round_trips() {
        let r = ProvingResult::from_json(BRAKING).expect("parse");
        assert!(r.check().is_ok());
        let again = ProvingResult::from_json(&serde_json::to_string(&r).expect("ser")).expect("reparse");
        assert_eq!(r, again);
    }

    #[test]
    fn a_missing_key_is_an_error_that_names_it() {
        let text = BRAKING.replace("\"mu\":0.8,", "");
        let e = ProvingResult::from_json(&text).expect("parse").check().expect_err("missing mu");
        assert!(e.contains("`mu`"), "{e}");
    }

    #[test]
    fn an_unknown_schema_or_test_is_rejected_and_an_early_end_may_omit_measurements() {
        let bad = BRAKING.replace("result.v1", "result.v9");
        assert!(ProvingResult::from_json(&bad).expect("parse").check().is_err());
        let unknown = BRAKING.replace("braking_50kmh", "teleport");
        assert!(ProvingResult::from_json(&unknown).expect("parse").check().is_err());
        let early = BRAKING
            .replace("\"ended_early\":null", "\"ended_early\":\"rolled over\"")
            .replace("\"measured\":{\"stop_distance_m\":14.9,\"peak_decel_g\":0.81}", "\"measured\":{}");
        assert!(ProvingResult::from_json(&early).expect("parse").check().is_ok());
    }

    #[test]
    fn every_phase_one_test_of_the_spec_is_in_the_table() {
        let doc = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/validation/proving-ground.md"),
        )
        .expect("spec");
        for t in TESTS {
            assert!(doc.contains(&format!("`{}`", t.id)), "{} missing from the spec", t.id);
        }
    }
}
