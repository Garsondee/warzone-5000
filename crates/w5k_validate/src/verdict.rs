//! Verdicts: published value versus simulated value, with ADR-0007's tolerance classes.
//!
//! PROVISIONAL(v0): one nominal run, no Monte Carlo envelope yet. The published *band* stands in for the envelope: a simulated value
//! inside the band, or within tolerance of it, is green. The envelope (build step 4) replaces this and can only make the test stricter.

use crate::dossier::{Evidence, Quantity};

/// ADR-0007 tolerance classes (fractions of the published value).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Static,
    Power,
    TopSpeed,
    Accel,
}

impl Class {
    pub fn tolerance(self) -> f64 {
        match self {
            Class::Static => 0.03,   // const-ok: ADR-0007 static tolerance
            Class::Power => 0.05,    // const-ok: ADR-0007 power tolerance
            Class::TopSpeed => 0.07, // const-ok: ADR-0007 top-speed tolerance
            Class::Accel => 0.15,    // const-ok: ADR-0007 acceleration and braking tolerance
        }
    }
}

/// Mobility limits are ratings, so the model must meet them without wildly exceeding them: published <= sim <= 1.25 x published.
pub const MOBILITY_UPPER_FACTOR: f64 = 1.25; // const-ok: ADR-0007 bracket

const AMBER_UNDER: f64 = 0.9; // const-ok: amber band of the bracket, PROVISIONAL(v0) pending a card
const AMBER_OVER: f64 = 1.5; // const-ok: amber band of the bracket, PROVISIONAL(v0) pending a card

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Light {
    Green,
    Amber,
    Red,
    /// No scenario measures this yet. Shown honestly; it is not a pass.
    NotMeasured,
}

impl Light {
    pub fn word(self) -> &'static str {
        match self {
            Light::Green => "green",
            Light::Amber => "amber",
            Light::Red => "red",
            Light::NotMeasured => "not measured",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: String,
    pub unit: String,
    pub published: f64,
    pub band: (f64, f64),
    pub simulated: Option<f64>,
    pub light: Light,
    /// True when the published figure is only Secondary (UNVERIFIED): the verdict is provisional.
    pub provisional: bool,
    pub note: String,
}

fn base(q: &Quantity, sim: Option<f64>, light: Light, note: &str) -> Row {
    Row {
        id: q.id.clone(),
        unit: q.unit.clone(),
        published: q.param.v,
        band: q.param.band(),
        simulated: sim,
        light,
        provisional: q.evidence != Evidence::Primary,
        note: note.to_string(),
    }
}

pub fn not_measured(q: &Quantity, why: &str) -> Row {
    base(q, None, Light::NotMeasured, why)
}

/// Green within the tolerance of the published band, amber within twice it, red otherwise. A non-finite simulated value is red.
pub fn judge(q: &Quantity, sim: f64, class: Class) -> Row {
    let (lo, hi) = q.param.band();
    let dev = if !sim.is_finite() {
        f64::INFINITY
    } else if sim < lo {
        lo - sim
    } else if sim > hi {
        sim - hi
    } else {
        0.0
    };
    let rel = dev / q.param.v.abs().max(f64::MIN_POSITIVE);
    let tol = class.tolerance();
    let light = if rel <= tol {
        Light::Green
    } else if rel <= 2.0 * tol {
        // const-ok: amber is within twice the tolerance (ADR-0007)
        Light::Amber
    } else {
        Light::Red
    };
    base(q, Some(sim), light, "")
}

/// Bracketed verdict for a mobility rating. Amber: within 10% under the rating or up to 1.5 x over it. Red beyond that.
pub fn judge_bracket(q: &Quantity, sim: f64) -> Row {
    let p = q.param.v;
    let light = if !sim.is_finite() {
        Light::Red
    } else if sim >= p && sim <= MOBILITY_UPPER_FACTOR * p {
        Light::Green
    } else if sim >= AMBER_UNDER * p && sim <= AMBER_OVER * p {
        Light::Amber
    } else {
        Light::Red
    };
    base(q, Some(sim), light, "bracket: published <= simulated <= 1.25 x published")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dossier::Category;
    use w5k_contract::Param;

    fn q(v: f64, lo: f64, hi: f64) -> Quantity {
        Quantity {
            id: "static.x_m".into(),
            category: Category::Static,
            unit: "m".into(),
            param: Param::estimate(v, lo, hi, "t"),
            evidence: Evidence::Primary,
            notes: String::new(),
        }
    }

    #[test]
    fn tolerance_classes_match_adr_0007() {
        let t = [Class::Static, Class::Power, Class::TopSpeed, Class::Accel].map(Class::tolerance);
        for (got, want) in t.iter().zip([0.03, 0.05, 0.07, 0.15]) {
            assert!((got - want).abs() < 1e-12);
        }
    }

    #[test]
    fn a_value_inside_the_envelope_and_within_tolerance_is_green() {
        assert_eq!(judge(&q(10.0, 9.0, 11.0), 10.5, Class::Static).light, Light::Green);
        assert_eq!(judge(&q(10.0, 10.0, 10.0), 10.25, Class::Static).light, Light::Green);
    }

    #[test]
    fn a_value_between_one_and_two_tolerances_is_amber_and_beyond_is_red() {
        let p = q(10.0, 10.0, 10.0);
        assert_eq!(judge(&p, 10.5, Class::Static).light, Light::Amber);
        assert_eq!(judge(&p, 11.0, Class::Static).light, Light::Red);
        assert_eq!(judge(&p, f64::NAN, Class::Static).light, Light::Red);
    }

    #[test]
    fn mobility_limit_bracketing_is_published_le_sim_le_1_25_published() {
        let p = q(0.6, 0.6, 0.6);
        assert_eq!(judge_bracket(&p, 0.6).light, Light::Green);
        assert_eq!(judge_bracket(&p, 0.75).light, Light::Green);
        assert_eq!(judge_bracket(&p, 0.8).light, Light::Amber);
        assert_eq!(judge_bracket(&p, 0.3).light, Light::Red);
    }
}
