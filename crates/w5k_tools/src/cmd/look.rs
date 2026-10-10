//! `w5k look`: the command line of lane LOOK (only that lane edits this file).
//!
//! `w5k look bake <scheme.ron>... --out DIR` turns authoring RON (sRGB hex, `Param`s) into the JSON the shaders, the viewer's page build and the
//! Python reference read: linear albedo, coverage, scale. The pattern itself lives in `assets/reference/w5k_look.py` and the shaders.

use serde::{Deserialize, Serialize};
use std::path::Path;
use w5k_contract::{Param, Provenance};
use w5k_math::scalar;

/// Entry point for `w5k look <args>`.
pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("bake") => bake(&args[1..]),
        _ => Err("usage: w5k look bake <scheme.ron>... --out DIR".to_string()),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scheme {
    pub id: String,
    pub pattern: Pattern,
    pub scale_m: Param,
    pub seed_salt: u32,
    pub colours: Vec<Colour>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Pattern {
    Blob { octaves: u32, lacunarity: f64, gain: f64 },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Colour {
    pub name: String,
    /// The authoring edge: `#RRGGBB`. Everything downstream is linear.
    pub srgb: String,
    pub coverage: Param,
    pub prov: Provenance,
    /// Uncertainty of the colour in CIELAB delta E; required (a colour with no band is not an ESTIMATE).
    pub band_de: f64,
    pub src: String,
}

#[derive(Serialize)]
struct BakedColour {
    name: String,
    linear: [f64; 3],
    coverage: f64,
    prov: Provenance,
    band_de: f64,
    src: String,
}

#[derive(Serialize)]
struct Baked {
    id: String,
    pattern: BakedPattern,
    scale_m: f64,
    seed_salt: u32,
    colours: Vec<BakedColour>,
}

#[derive(Serialize)]
struct BakedPattern {
    octaves: u32,
    lacunarity: f64,
    gain: f64,
}

fn srgb_to_linear(c8: u8) -> f64 {
    let v = f64::from(c8) / 255.0; // const-ok: 8-bit channel range
    const KNEE: f64 = 0.04045; // const-ok: sRGB EOTF (IEC 61966-2-1)
    const LOW_DIV: f64 = 12.92; // const-ok: sRGB EOTF
    const OFFSET: f64 = 0.055; // const-ok: sRGB EOTF
    const SCALE: f64 = 1.055; // const-ok: sRGB EOTF
    const EXPONENT: f64 = 2.4; // const-ok: sRGB EOTF
    if v <= KNEE {
        v / LOW_DIV
    } else {
        scalar::pow((v + OFFSET) / SCALE, EXPONENT)
    }
}

fn parse_hex(h: &str) -> Result<[f64; 3], String> {
    let b = h.strip_prefix('#').filter(|s| s.len() == 6).ok_or_else(|| format!("colour {h:?} is not #RRGGBB"))?; // const-ok: hex length
    let ch =
        |i: usize| u8::from_str_radix(&b[i..i + 2], 16).map(srgb_to_linear).map_err(|e| format!("colour {h:?}: {e}"));
    Ok([ch(0)?, ch(2)?, ch(4)?])
}

/// Check a scheme and produce its linear form.
fn bake_scheme(s: &Scheme) -> Result<Baked, String> {
    let Pattern::Blob { octaves, lacunarity, gain } = s.pattern;
    if (octaves, lacunarity, gain) != (3, 2.0, 0.5) {
        // const-ok: the GLSL and Godot chunks are written for exactly this fractal
        return Err(format!("{}: the shaders support Blob(octaves: 3, lacunarity: 2.0, gain: 0.5) only", s.id));
    }
    if s.colours.len() < 2 || s.colours.iter().any(|c| c.src.trim().is_empty() || c.band_de <= 0.0) {
        return Err(format!("{}: at least two colours, each with a source and a delta-E band", s.id));
    }
    let total: f64 = s.colours.iter().map(|c| c.coverage.v).sum();
    if (total - 1.0).abs() > 1e-9 {
        // const-ok: coverage tolerance
        return Err(format!("{}: coverage sums to {total}, not 1", s.id));
    }
    let colours = s
        .colours
        .iter()
        .map(|c| {
            Ok(BakedColour {
                name: c.name.clone(),
                linear: parse_hex(&c.srgb)?,
                coverage: c.coverage.v,
                prov: c.prov,
                band_de: c.band_de,
                src: c.src.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Baked {
        id: s.id.clone(),
        pattern: BakedPattern { octaves, lacunarity, gain },
        scale_m: s.scale_m.v,
        seed_salt: s.seed_salt,
        colours,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WeatheringFile {
    params: std::collections::BTreeMap<String, Param>,
    /// `#RRGGBB` at the authoring edge, written out as `<name>_linear`.
    colours: std::collections::BTreeMap<String, String>,
}

/// `weathering.ron` to flat JSON: each `Param` becomes its value, each colour becomes `<name>_linear`. Every Param must sit inside its own band.
pub fn bake_weathering_text(text: &str) -> Result<String, String> {
    let file: WeatheringFile = ron::from_str(text).map_err(|e| e.to_string())?;
    let mut out = serde_json::Map::new();
    for (k, p) in file.params {
        if p.lo.is_some_and(|lo| p.v < lo) || p.hi.is_some_and(|hi| p.v > hi) || p.src.trim().is_empty() {
            return Err(format!("{k}: outside its band or without a source"));
        }
        out.insert(k, p.v.into());
    }
    for (k, h) in file.colours {
        out.insert(format!("{k}_linear"), parse_hex(&h)?.to_vec().into());
    }
    let get = |k: &str| out.get(k).and_then(serde_json::Value::as_f64).unwrap_or(f64::NAN);
    if get("wear_bias") <= get("wear_noise_amp") / 2.0 {
        return Err("wear_bias must exceed wear_noise_amp / 2 (a face with edge 0 must never chip)".to_string());
    }
    serde_json::to_string_pretty(&out).map_err(|e| e.to_string())
}

fn bake_weathering(path: &Path) -> Result<String, String> {
    bake_weathering_text(&std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?)
}

/// Parse one scheme file (also used by the tests).
pub fn read_scheme(path: &Path) -> Result<Scheme, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    ron::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn bake(args: &[String]) -> Result<(), String> {
    let out_at = args.iter().position(|a| a == "--out").ok_or("--out DIR is required")?;
    let out = Path::new(args.get(out_at + 1).ok_or("--out needs a directory")?);
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    for f in args.iter().take(out_at) {
        let path = Path::new(f);
        let (name, json) = if path.file_stem().is_some_and(|n| n == "weathering") {
            ("weathering".to_string(), bake_weathering(path)?)
        } else {
            let baked = bake_scheme(&read_scheme(path)?)?;
            (baked.id.clone(), serde_json::to_string_pretty(&baked).map_err(|e| e.to_string())?)
        };
        std::fs::write(out.join(format!("{name}.json")), json + "\n").map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_camo_scheme_in_assets_parses_and_bakes() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/materials/camo");
        let mut n = 0;
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "ron") {
                bake_scheme(&read_scheme(&p).unwrap()).unwrap();
                n += 1;
            }
        }
        assert!(n >= 3, "three schemes by M1");
    }

    #[test]
    fn committed_baked_json_is_what_the_bake_produces() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/materials/camo");
        for e in std::fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "ron") {
                let baked = bake_scheme(&read_scheme(&p).unwrap()).unwrap();
                let fresh = serde_json::to_string_pretty(&baked).unwrap() + "\n";
                let committed = std::fs::read_to_string(dir.join("baked").join(format!("{}.json", baked.id))).unwrap();
                assert_eq!(
                    fresh, committed,
                    "run `w5k look bake assets/materials/camo/*.ron --out assets/materials/camo/baked`"
                );
            }
        }
    }

    #[test]
    fn weathering_ron_bakes_and_matches_the_committed_json() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/materials");
        let fresh = bake_weathering(&dir.join("weathering.ron")).unwrap() + "\n";
        assert_eq!(
            fresh,
            std::fs::read_to_string(dir.join("baked/weathering.json")).unwrap(),
            "re-run `w5k look bake`"
        );
    }

    #[test]
    fn srgb_white_is_linear_one_and_mid_grey_is_about_a_fifth() {
        assert!((parse_hex("#FFFFFF").unwrap()[0] - 1.0).abs() < 1e-12);
        assert!((parse_hex("#808080").unwrap()[1] - 0.2158605).abs() < 1e-6);
    }
}
