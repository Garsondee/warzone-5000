//! `Param`: a physical constant with its pedigree. Every physical constant in a RON file is a `Param`; none live bare in code
//! (CI's constants lint enforces the code half of the rule).
//!
//! RON: `Param(v: 2.1, lo: 1.8, hi: 2.4, prov: Estimate, src: "typical for a 4x4 truck, Gillespie ch. 5")`.

use serde::{Deserialize, Serialize};

/// Where a number comes from. The validation harness reports on these, and the dossier lane keeps them honest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Provenance {
    /// A published specification (a manual, a manufacturer sheet).
    Spec,
    /// A published measurement or test result.
    Measured,
    /// Engineering judgement; must carry a band.
    Estimate,
    /// Calibrated against the validation set; every calibration is logged.
    Tuned,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Param {
    /// The value, in the unit implied by the field name it is stored under.
    pub v: f64,
    /// Lower end of the plausible band (published scatter or engineering range).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lo: Option<f64>,
    /// Upper end of the plausible band.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hi: Option<f64>,
    pub prov: Provenance,
    /// Where it comes from: a document, page, URL, or the reasoning for an estimate.
    #[serde(default)]
    pub src: String,
}

impl Param {
    pub fn new(v: f64, prov: Provenance, src: &str) -> Param {
        Param { v, lo: None, hi: None, prov, src: src.to_string() }
    }

    pub fn spec(v: f64, src: &str) -> Param {
        Param::new(v, Provenance::Spec, src)
    }

    pub fn measured(v: f64, src: &str) -> Param {
        Param::new(v, Provenance::Measured, src)
    }

    pub fn estimate(v: f64, lo: f64, hi: f64, src: &str) -> Param {
        Param { v, lo: Some(lo), hi: Some(hi), prov: Provenance::Estimate, src: src.to_string() }
    }

    pub fn tuned(v: f64, src: &str) -> Param {
        Param::new(v, Provenance::Tuned, src)
    }

    /// The plausible band; a degenerate band `(v, v)` when none is given.
    pub fn band(&self) -> (f64, f64) {
        (self.lo.unwrap_or(self.v), self.hi.unwrap_or(self.v))
    }

    /// Check the invariants the validation harness relies on. `name` is only used in the message.
    pub fn check(&self, name: &str) -> Result<(), String> {
        let (lo, hi) = self.band();
        if !self.v.is_finite() || !lo.is_finite() || !hi.is_finite() {
            return Err(format!("{name}: non-finite value or band"));
        }
        if lo > self.v || self.v > hi {
            return Err(format!("{name}: value {} outside its band [{lo}, {hi}]", self.v));
        }
        if matches!(self.prov, Provenance::Spec | Provenance::Measured) && self.src.trim().is_empty() {
            return Err(format!("{name}: a {:?} value needs a source", self.prov));
        }
        if self.prov == Provenance::Estimate && (self.lo.is_none() || self.hi.is_none()) {
            return Err(format!("{name}: an Estimate needs a band (lo and hi)"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ron_round_trip_and_checks() {
        let p = Param::estimate(2.1, 1.8, 2.4, "typical 4x4 truck");
        let text = ron::to_string(&p).expect("serialise");
        let back: Param = ron::from_str(&text).expect("parse");
        assert_eq!(p, back);
        assert!(p.check("ride_frequency_hz").is_ok());
    }

    #[test]
    fn a_spec_without_a_source_and_an_estimate_without_a_band_are_rejected() {
        assert!(Param::spec(1.0, "").check("x").is_err());
        assert!(Param::new(1.0, Provenance::Estimate, "judgement").check("x").is_err());
        assert!(Param::estimate(5.0, 1.0, 2.0, "bad band").check("x").is_err());
        assert!(Param::spec(f64::NAN, "x").check("x").is_err());
    }

    #[test]
    fn unknown_fields_are_refused() {
        let r: Result<Param, _> = ron::from_str("Param(v: 1.0, prov: Spec, src: \"x\", typo: 3)");
        assert!(r.is_err());
    }
}
