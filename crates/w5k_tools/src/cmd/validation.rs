//! `w5k validation`: the command line of lane VALIDATION (only that lane edits this file).
//!
//! `w5k validation dashboard --out DIR [--replay FILE] [--def FILE] [--dossier FILE] [--top-speed-run]`
//! scores a replay (default: the stand-in first-light run) against a dossier (default: M998) and writes `DIR/index.html`.

use std::path::{Path, PathBuf};

use w5k_validate::dashboard::{render, Page};
use w5k_validate::dossier::Dossier;
use w5k_validate::measure::from_frames;
use w5k_validate::score::{score, RunKind};

fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(String::as_str)
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Entry point for `w5k validation <args>`.
pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("dashboard") => dashboard(&args[1..]),
        _ => Err(
            "usage: w5k validation dashboard --out DIR [--replay FILE] [--def FILE] [--dossier FILE] [--top-speed-run]"
                .into(),
        ),
    }
}

fn dashboard(args: &[String]) -> Result<(), String> {
    let out = flag(args, "--out").ok_or("--out DIR is required")?;
    let def_path = flag(args, "--def").map_or_else(|| root().join("content/vehicles/game/mule_4x4.ron"), PathBuf::from);
    let dossier_path = flag(args, "--dossier").map_or_else(|| root().join("content/dossier/m998.ron"), PathBuf::from);
    let def_text = std::fs::read_to_string(&def_path).map_err(|e| format!("{}: {e}", def_path.display()))?;
    let def = w5k_forge::compile::parse_def(&def_text)?;
    let dossier = Dossier::load(&dossier_path)?;
    let problems = dossier.problems();
    if !problems.is_empty() {
        return Err(format!("dossier is unsound: {}", problems.join("; ")));
    }
    let (frames, dt, what) = match flag(args, "--replay") {
        Some(p) => {
            let path = Path::new(p);
            let r = if p.ends_with(".json") { w5k_replay::read_json(path)? } else { w5k_replay::read_bin(path)? };
            (r.frames, r.header.frame_dt_s, format!("replay {p} (scenario {})", r.header.scenario))
        }
        None => {
            let r = w5k_sim::scenario::first_light(w5k_sim::scenario::FirstLightParts::stand_ins());
            (
                r.frames,
                r.header.frame_dt_s,
                "stand-in first-light run (kinematic box truck: expected to fail the physics checks)".into(),
            )
        }
    };
    let kind = if args.iter().any(|a| a == "--top-speed-run") { RunKind::TopSpeed } else { RunKind::Other };
    let rows = score(&dossier, &def, &from_frames(&frames, 0, dt), kind);
    let subject = format!("Scored: {what}; design sheet {} against dossier {}.", def.id, dossier.id);
    let html = render(&Page { title: "Validation dashboard v0", subject: &subject, dossier: &dossier, rows: &rows });
    std::fs::create_dir_all(out).map_err(|e| format!("{out}: {e}"))?;
    let target = Path::new(out).join("index.html");
    std::fs::write(&target, html).map_err(|e| format!("{}: {e}", target.display()))?;
    println!("wrote {}", target.display());
    Ok(())
}
