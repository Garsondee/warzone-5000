//! `w5k scenario ...`: run a scenario from the command line.

use std::path::PathBuf;

use w5k_replay::ReplayFile;

use crate::scenario::{first_light, FirstLightParts};

const USAGE: &str = "usage: w5k scenario first-light --out DIR   (writes replay.json and summary.json; runs on stand-ins until the lanes land real parts)";

/// `w5k scenario <name> [--out DIR]`.
pub fn scenario(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("first-light") => {}
        _ => return Err(USAGE.into()),
    }
    let mut out: Option<PathBuf> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                out = Some(PathBuf::from(args.get(i).ok_or(USAGE)?));
            }
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
        i += 1;
    }
    let result = first_light(FirstLightParts::stand_ins());
    let s = &result.summary;
    println!(
        "first-light: {} frames, {:.1} m driven, top speed {:.1} m/s, final speed {:.2} m/s, pitch up to {:.1} deg, suspension travel up to {:.3} m, finite: {}, state hash {:016x}",
        s.frames, s.distance_m, s.max_speed_m_s, s.final_speed_m_s, s.max_abs_pitch_deg, s.max_abs_travel_m, s.all_finite, result.final_hash
    );
    if let Some(dir) = out {
        let replay = ReplayFile { header: result.header, frames: result.frames };
        w5k_replay::write_json(&dir.join("replay.json"), &replay)?;
        let summary = serde_json::to_string_pretty(&result.summary).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("summary.json"), summary).map_err(|e| format!("cannot write summary.json: {e}"))?;
        println!("wrote {}/replay.json and summary.json", dir.display());
    }
    if !result.summary.all_finite {
        return Err("the run produced a non-finite value".into());
    }
    Ok(())
}
