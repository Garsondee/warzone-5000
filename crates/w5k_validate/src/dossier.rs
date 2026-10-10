//! Dossiers: the real vehicle's published figures, each with its band, provenance and source.
//! Schema and rules: `docs/lanes/validation/design-note.md` section 1 and `docs/validation/METHOD.md`.

use std::path::Path;

use serde::{Deserialize, Serialize};
use w5k_contract::{Param, Provenance};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Category {
    Static,
    Powertrain,
    Dynamics,
    Mobility,
    Turret,
}

impl Category {
    /// The id prefix a quantity of this category must carry.
    pub fn prefix(self) -> &'static str {
        match self {
            Category::Static => "static",
            Category::Powertrain => "powertrain",
            Category::Dynamics => "dynamics",
            Category::Mobility => "mobility",
            Category::Turret => "turret",
        }
    }
}

/// How well the figure is evidenced. This is separate from `Provenance`: a figure can be a published `Spec`
/// that we only saw second-hand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Evidence {
    /// A manual, data sheet or test report that the lane read.
    Primary,
    /// Seen only in a search summary or a third-party page; the source starts with `UNVERIFIED`.
    Secondary,
    /// An open vehicle model (e.g. Project Chrono). Plausibility only; never scored, never imported.
    Cross,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Calibration,
    HeldOut,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quantity {
    pub id: String,
    pub category: Category,
    pub unit: String,
    pub param: Param,
    pub evidence: Evidence,
    #[serde(default)]
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dossier {
    pub id: String,
    pub name: String,
    pub role: Role,
    pub era: String,
    /// The fictional vehicle this dossier stands for in the game (the game never shows real names).
    #[serde(default)]
    pub game_id: Option<String>,
    pub quantities: Vec<Quantity>,
}

impl Dossier {
    pub fn from_ron(text: &str) -> Result<Dossier, String> {
        ron::from_str(text).map_err(|e| format!("dossier parse error: {e}"))
    }

    pub fn load(path: &Path) -> Result<Dossier, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Dossier::from_ron(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Quantities that count towards the score (everything except open-model cross-checks).
    pub fn scored(&self) -> impl Iterator<Item = &Quantity> {
        self.quantities.iter().filter(|q| q.evidence != Evidence::Cross)
    }

    /// Every rule of the method, as a list of messages (empty means the dossier is sound).
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut seen: Vec<&str> = Vec::new();
        for q in &self.quantities {
            let name = format!("{}/{}", self.id, q.id);
            if let Err(e) = q.param.check(&name) {
                out.push(e);
            }
            if !q.id.starts_with(&format!("{}.", q.category.prefix())) {
                out.push(format!("{name}: id must start with `{}.`", q.category.prefix()));
            }
            if seen.contains(&q.id.as_str()) {
                out.push(format!("{name}: figure listed twice (give one value and widen its band)"));
            }
            seen.push(&q.id);
            let unverified = q.param.src.trim_start().starts_with("UNVERIFIED");
            match q.evidence {
                Evidence::Secondary if !unverified => {
                    out.push(format!("{name}: a secondary figure must say UNVERIFIED in its source"))
                }
                Evidence::Primary if unverified => out.push(format!("{name}: a primary figure cannot be UNVERIFIED")),
                _ => {}
            }
            if q.evidence != Evidence::Primary && q.param.prov == Provenance::Tuned {
                out.push(format!("{name}: a dossier figure is never Tuned"));
            }
            if q.param.src.trim().is_empty() {
                out.push(format!("{name}: every figure needs a source or a reasoning note"));
            }
        }
        out
    }

    /// Counts for the provenance meter: (primary, secondary, estimate-only, cross).
    pub fn evidence_counts(&self) -> (usize, usize, usize, usize) {
        let n = |e| self.quantities.iter().filter(|q| q.evidence == e).count();
        (n(Evidence::Primary), n(Evidence::Secondary), 0, n(Evidence::Cross))
    }
}

/// Load every `*.ron` in `dir`, in file-name order.
pub fn load_dir(dir: &Path) -> Result<Vec<Dossier>, String> {
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "ron"))
        .collect();
    paths.sort();
    paths.iter().map(|p| Dossier::load(p)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dossier_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/dossier")
    }

    fn q(id: &str, param: Param, evidence: Evidence) -> Quantity {
        Quantity { id: id.into(), category: Category::Static, unit: "m".into(), param, evidence, notes: String::new() }
    }

    fn one(quantities: Vec<Quantity>) -> Dossier {
        Dossier {
            id: "t".into(),
            name: "t".into(),
            role: Role::Calibration,
            era: "t".into(),
            game_id: None,
            quantities,
        }
    }

    #[test]
    fn dossier_ron_round_trips_and_every_quantity_passes_param_check() {
        let all = load_dir(&dossier_dir()).expect("dossiers load");
        assert!(!all.is_empty());
        for d in &all {
            let text = ron::to_string(d).expect("serialise");
            assert_eq!(&Dossier::from_ron(&text).expect("parse"), d);
            assert!(d.problems().is_empty(), "{:?}", d.problems());
        }
    }

    #[test]
    fn every_dossier_quantity_has_a_source_or_is_an_estimate_with_a_band() {
        let bad = one(vec![q("static.a_m", Param::new(1.0, Provenance::Spec, ""), Evidence::Primary)]);
        assert!(!bad.problems().is_empty());
        let no_band = one(vec![q("static.a_m", Param::new(1.0, Provenance::Estimate, "judgement"), Evidence::Primary)]);
        assert!(!no_band.problems().is_empty());
        let ok = one(vec![q("static.a_m", Param::estimate(1.0, 0.9, 1.1, "judgement"), Evidence::Primary)]);
        assert!(ok.problems().is_empty());
        for d in load_dir(&dossier_dir()).expect("load") {
            for qty in &d.quantities {
                assert!(!qty.param.src.trim().is_empty(), "{} lacks a source", qty.id);
            }
        }
    }

    #[test]
    fn no_figure_is_duplicated_with_a_different_value() {
        let dup = one(vec![
            q("static.a_m", Param::spec(1.0, "x"), Evidence::Primary),
            q("static.a_m", Param::spec(2.0, "y"), Evidence::Primary),
        ]);
        assert!(dup.problems().iter().any(|p| p.contains("twice")));
    }

    #[test]
    fn a_secondary_figure_must_say_unverified_and_a_primary_one_must_not() {
        let a = one(vec![q("static.a_m", Param::spec(1.0, "a web page"), Evidence::Secondary)]);
        assert!(!a.problems().is_empty());
        let b = one(vec![q("static.a_m", Param::spec(1.0, "UNVERIFIED: a web page"), Evidence::Primary)]);
        assert!(!b.problems().is_empty());
    }

    #[test]
    fn the_m998_dossier_has_at_least_fifteen_scored_quantities() {
        let all = load_dir(&dossier_dir()).expect("load");
        let m998 = all.iter().find(|d| d.id == "m998").expect("m998");
        assert!(m998.scored().count() >= 15, "{}", m998.scored().count());
    }
}
