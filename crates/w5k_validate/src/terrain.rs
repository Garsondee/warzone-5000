//! Terrain scorer: reads WORLD's `w5k world stats` JSON and gives traffic lights.
//! Spec: `docs/validation/proving-ground.md` section 4. Is the ground a believable ground?

use serde::Deserialize;

use crate::verdict::Light;

/// ISO 8608 assumes a road PSD that falls as `n^-w` with waviness `w = 2`. ESTIMATE bands: green within 0.5, amber within 1.0.
const WAVINESS_REF: f64 = 2.0; // const-ok: ISO 8608 reference waviness (standard paywalled: UNVERIFIED)
const WAVINESS_GREEN: f64 = 0.5; // const-ok: ESTIMATE band, proving-ground.md section 4
const WAVINESS_AMBER: f64 = 1.0; // const-ok: ESTIMATE band, proving-ground.md section 4
/// A rubber tyre does not exceed this peak adhesion on any ground (ESTIMATE, engineering judgement).
const MU_PHYSICAL_MAX: f64 = 1.2; // const-ok: ESTIMATE, proving-ground.md section 4

const PERCENT: f64 = 100.0; // const-ok: unit conversion for display

#[derive(Debug, Deserialize)]
pub struct Stats {
    pub course: String,
    #[serde(default)]
    pub roads: Vec<Road>,
    #[serde(default)]
    pub roughness: Vec<Roughness>,
    #[serde(default)]
    pub materials: Vec<Material>,
    pub slope: Slope,
}

#[derive(Debug, Deserialize)]
pub struct Road {
    pub name: String,
    pub grade: Grade,
}

#[derive(Debug, Deserialize)]
pub struct Grade {
    pub max_grade: f64,
}

#[derive(Debug, Deserialize)]
pub struct Roughness {
    pub name: String,
    pub kind: String,
    pub iso8608_class: String,
    pub waviness_exponent: f64,
    pub segment_m: f64,
}

#[derive(Debug, Deserialize)]
pub struct Material {
    pub material: String,
    pub quantity: String,
    pub value: f64,
    pub lo: f64,
    pub hi: f64,
    pub in_range: bool,
    pub source: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct Slope {
    pub share_over_20_deg: f64,
    pub share_over_30_deg: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TerrainCheck {
    pub scope: String,
    pub name: String,
    pub light: Light,
    /// True when the reference the check compares against is itself unverified: the light is provisional.
    pub provisional: bool,
    pub note: String,
}

fn check(scope: &str, name: &str, light: Light, provisional: bool, note: String) -> TerrainCheck {
    TerrainCheck { scope: scope.into(), name: name.into(), light, provisional, note }
}

pub fn from_json(text: &str) -> Result<Stats, String> {
    serde_json::from_str(text).map_err(|e| format!("world stats JSON: {e}"))
}

/// Score `stats`. `reference_grade` is the steepest grade the calibration vehicle is rated for (the M998's published 60% in the dossier).
pub fn score(stats: &Stats, reference_grade: f64) -> Vec<TerrainCheck> {
    let mut out = Vec::new();
    for r in &stats.roughness {
        let off = (r.waviness_exponent + WAVINESS_REF).abs();
        let light = if !off.is_finite() || off > WAVINESS_AMBER {
            Light::Red
        } else if off > WAVINESS_GREEN {
            Light::Amber
        } else {
            Light::Green
        };
        let note = format!(
            "{} {}: PSD falls as n^{:.2} (ISO 8608 assumes n^-2); fitted class {}; estimated over {:.0} m segments, so a steep fit may be partly windowing",
            r.kind, r.name, r.waviness_exponent, r.iso8608_class, r.segment_m
        );
        out.push(check(&r.name, "roughness_waviness", light, true, note));
    }
    for road in &stats.roads {
        let g = road.grade.max_grade;
        let light = if !g.is_finite() || g > reference_grade {
            Light::Red
        } else if g > 0.5 * reference_grade {
            Light::Amber // const-ok: ESTIMATE, a road should not need crawl gear (proving-ground.md section 4)
        } else {
            Light::Green
        };
        out.push(check(
            &road.name,
            "road_max_grade",
            light,
            true,
            format!("max grade {g:.3} against the reference vehicle's rated {reference_grade:.2}"),
        ));
    }
    out.push(check(
        &stats.course,
        "slope_share",
        Light::NotMeasured,
        false,
        format!(
            "reported, not judged: {:.2}% of ground steeper than 20 degrees, {:.2}% steeper than 30",
            PERCENT * stats.slope.share_over_20_deg,
            PERCENT * stats.slope.share_over_30_deg
        ),
    ));
    for m in &stats.materials {
        let name = format!("{} {}", m.material, m.quantity);
        let provisional = m.status != "Verified";
        let light = if !m.value.is_finite() || !m.in_range { Light::Red } else { Light::Green };
        let mut note = format!("{} in [{}, {}] ({}); {}", m.value, m.lo, m.hi, m.status, m.source);
        if !m.in_range {
            note.push_str(": outside the range WORLD cites");
        }
        out.push(check(&m.material, &name, light, provisional, note));
        if m.quantity == "mu_peak" {
            let sane = m.value > 0.0 && m.value <= MU_PHYSICAL_MAX;
            out.push(check(
                &m.material,
                "mu_physical",
                if sane { Light::Green } else { Light::Red },
                false,
                format!("peak adhesion {} must lie in (0, {MU_PHYSICAL_MAX}]", m.value),
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(waviness: f64, grade: f64, mu: f64, in_range: bool) -> Stats {
        let text = format!(
            r#"{{"course":"t","slope":{{"share_over_20_deg":0.01,"share_over_30_deg":0.0}},
            "roads":[{{"name":"r","grade":{{"max_grade":{grade}}}}}],
            "roughness":[{{"name":"r","kind":"road","iso8608_class":"B","waviness_exponent":{waviness},"segment_m":200.0}}],
            "materials":[{{"material":"asphalt","quantity":"mu_peak","value":{mu},"lo":0.8,"hi":1.0,"in_range":{in_range},"source":"s","status":"Unverified"}}]}}"#
        );
        from_json(&text).expect("parse")
    }

    fn light(cs: &[TerrainCheck], name: &str) -> Light {
        cs.iter().find(|c| c.name == name).expect("check").light
    }

    #[test]
    fn a_road_spectrum_falling_as_n_minus_2_is_green_and_one_falling_as_n_minus_4_is_red() {
        assert_eq!(light(&score(&stats(-2.1, 0.05, 0.9, true), 0.6), "roughness_waviness"), Light::Green);
        assert_eq!(light(&score(&stats(-2.8, 0.05, 0.9, true), 0.6), "roughness_waviness"), Light::Amber);
        assert_eq!(light(&score(&stats(-4.0, 0.05, 0.9, true), 0.6), "roughness_waviness"), Light::Red);
    }

    #[test]
    fn a_road_grade_beyond_the_reference_vehicle_rating_is_red() {
        assert_eq!(light(&score(&stats(-2.0, 0.05, 0.9, true), 0.6), "road_max_grade"), Light::Green);
        assert_eq!(light(&score(&stats(-2.0, 0.4, 0.9, true), 0.6), "road_max_grade"), Light::Amber);
        assert_eq!(light(&score(&stats(-2.0, 0.7, 0.9, true), 0.6), "road_max_grade"), Light::Red);
    }

    #[test]
    fn friction_above_what_rubber_can_give_is_red_and_an_unverified_range_makes_the_light_provisional() {
        let cs = score(&stats(-2.0, 0.05, 1.5, false), 0.6);
        assert_eq!(light(&cs, "mu_physical"), Light::Red);
        let m = cs.iter().find(|c| c.name == "asphalt mu_peak").expect("check");
        assert_eq!((m.light, m.provisional), (Light::Red, true));
    }

    #[test]
    fn worlds_real_stats_files_parse_and_every_roughness_entry_is_scored() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/lanes/world/media");
        for course in ["crossing", "ridge", "river", "slice"] {
            let text = std::fs::read_to_string(dir.join(format!("stats-{course}/stats.json"))).expect("stats.json");
            let s = from_json(&text).expect("parse");
            let cs = score(&s, 0.6);
            assert_eq!(cs.iter().filter(|c| c.name == "roughness_waviness").count(), s.roughness.len());
        }
    }
}
