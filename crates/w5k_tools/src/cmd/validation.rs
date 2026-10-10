//! `w5k validation`: the command line of lane VALIDATION (only that lane edits this file).
//!
//! `w5k validation dashboard --out DIR [--replay FILE] [--def FILE] [--dossier FILE] [--top-speed-run]`
//! scores a replay (default: the stand-in first-light run) against a dossier (default: M998) and writes `DIR/index.html`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use w5k_validate::dashboard::{render, Page};
use w5k_validate::dossier::Dossier;
use w5k_validate::impact;
use w5k_validate::measure::from_frames;
use w5k_validate::proving::ProvingResult;
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
        Some("terrain") => terrain(&args[1..]),
        Some("impact") => impact(&args[1..]),
        Some("capability") => capability(&args[1..]),
        _ => Err(
            "usage: w5k validation capability --out DIR [--vehicles a.ron,...] | impact --out DIR [--vehicles a.ron,b.ron] | terrain --stats FILE [--reference-grade X] | dashboard --out DIR [--replay FILE] [--def FILE] [--dossier FILE] [--top-speed-run]"
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

/// `w5k validation terrain --stats stats.json [--reference-grade 0.6]`: traffic lights for WORLD's stats, one line each.
fn terrain(args: &[String]) -> Result<(), String> {
    let path = flag(args, "--stats").ok_or("--stats FILE is required")?;
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let stats = w5k_validate::terrain::from_json(&text)?;
    let grade = match flag(args, "--reference-grade") {
        Some(g) => g.parse::<f64>().map_err(|e| format!("--reference-grade: {e}"))?,
        None => {
            let d = Dossier::load(&root().join("content/dossier/m998.ron"))?;
            d.quantities
                .iter()
                .find(|q| q.id == "mobility.max_grade_ratio")
                .map(|q| q.param.v)
                .ok_or("m998 dossier has no grade")?
        }
    };
    for c in w5k_validate::terrain::score(&stats, grade) {
        println!(
            "{:<12} {:<28} {}{}  {}",
            c.light.word(),
            c.name,
            c.scope,
            if c.provisional { " [provisional]" } else { "" },
            c.note
        );
    }
    Ok(())
}

const IMPACT_VEHICLES: &str =
    "content/vehicles/game/scout_4x4.ron,content/vehicles/game/mule_4x4.ron,content/vehicles/game/hauler_4x4.ron";
/// The lever is perturbed by this factor (+10%).
const IMPACT_FACTOR: f64 = 1.1; // const-ok: the impact matrix perturbation (IMPACT-MATRIX.md)

/// Run the proving ground on one vehicle file (its `.extras.ron` sidecar next to it) and return `test -> measured`.
fn proving_results(vehicle: &Path, out: &Path) -> Result<BTreeMap<String, ProvingResult>, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let status = std::process::Command::new(exe)
        .args(["scenario", "proving", "--test", "all", "--vehicle"])
        .arg(vehicle)
        .arg("--out")
        .arg(out)
        .stdout(std::process::Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("the proving runner failed on {}", vehicle.display()));
    }
    let mut results = BTreeMap::new();
    for b in impact::BENCHES {
        let dir = std::fs::read_dir(out).map_err(|e| e.to_string())?.filter_map(|e| e.ok()).find(|e| e.path().is_dir());
        let Some(dir) = dir else { continue };
        let file = dir.path().join(format!("{}.json", b.test));
        if let Ok(text) = std::fs::read_to_string(&file) {
            results.insert(b.test.to_string(), ProvingResult::from_json(&text)?);
        }
    }
    Ok(results)
}

/// `w5k validation impact --out DIR [--vehicles a.ron,b.ron]`: perturb each lever by +10% on each vehicle, rerun the proving ground,
/// compare signs with `docs/validation/IMPACT-MATRIX.md`; writes `impact.json` and `impact.md`.
fn impact(args: &[String]) -> Result<(), String> {
    let out = PathBuf::from(flag(args, "--out").ok_or("--out DIR is required")?);
    let list = flag(args, "--vehicles").unwrap_or(IMPACT_VEHICLES);
    let table = impact::parse_table(
        &std::fs::read_to_string(root().join("docs/validation/IMPACT-MATRIX.md")).map_err(|e| e.to_string())?,
    );
    let levers = impact::levers();
    let mut obs = impact::Observations { pairs: BTreeMap::new(), regimes: BTreeMap::new(), labels: BTreeMap::new() };
    for path in list.split(',').filter(|v| !v.is_empty()) {
        let path = root().join(path);
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let def = w5k_forge::compile::parse_def(&text)?;
        let extras = std::fs::read_to_string(path.with_extension("extras.ron"))
            .map_err(|e| format!("extras of {}: {e}", def.id))?;
        let variant =
            |name: &str, d: &w5k_contract::def::VehicleDef| -> Result<BTreeMap<String, ProvingResult>, String> {
                let dir = out.join("runs").join(&def.id).join(name);
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let file = dir.join(format!("{}.ron", def.id));
                std::fs::write(&file, ron::to_string(d).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
                std::fs::write(file.with_extension("extras.ron"), &extras).map_err(|e| e.to_string())?;
                proving_results(&file, &dir.join("results"))
            };
        let base = variant("baseline", &def)?;
        if let Some(b) = base.get("braking_50kmh") {
            if let (Some(a), Some(mu)) = (b.measured.get("peak_decel_g"), b.inputs.get("mu")) {
                obs.regimes.insert(def.id.clone(), impact::braking_regime(*a, *mu));
            }
        }
        for (bench, test, key) in [("B7", "side_slope_rollover", "mode"), ("B12", "skidpad", "limited_by")] {
            if let Some(l) = base.get(test).and_then(|r| r.labels.get(key)) {
                obs.labels.insert((def.id.clone(), bench.to_string()), l.clone());
            }
        }
        for lever in &levers {
            let mut d = def.clone();
            (lever.apply)(&mut d, IMPACT_FACTOR);
            let pert = variant(lever.id, &d)?;
            for bench in impact::BENCHES {
                let value = |m: &BTreeMap<String, ProvingResult>| {
                    m.get(bench.test)
                        .filter(|r| r.ended_early.is_none())
                        .and_then(|r| r.measured.get(bench.key))
                        .copied()
                };
                if let (Some(a), Some(b)) = (value(&base), value(&pert)) {
                    obs.pairs.insert((def.id.clone(), lever.id.to_string(), bench.id.to_string()), (a, b));
                }
            }
        }
    }
    let report = impact::evaluate(&table, &levers, &obs);
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    std::fs::write(out.join("impact.json"), serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let md = impact::render_markdown(&report);
    std::fs::write(out.join("impact.md"), &md).map_err(|e| e.to_string())?;
    println!("{md}");
    Ok(())
}

/// `w5k validation capability --out DIR [--vehicles a.ron,...]`: run the proving ground on each vehicle and write
/// `DIR/<vehicle>.json` (one `w5k.capability.v1` table) and `DIR/capability.json` (all vehicles).
fn capability(args: &[String]) -> Result<(), String> {
    let out = PathBuf::from(flag(args, "--out").ok_or("--out DIR is required")?);
    let list = flag(args, "--vehicles").unwrap_or(IMPACT_VEHICLES);
    let mut all = Vec::new();
    for path in list.split(',').filter(|v| !v.is_empty()) {
        let path = root().join(path);
        let def = w5k_forge::compile::parse_def(
            &std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?,
        )?;
        let results = proving_results(&path, &out.join("runs").join(&def.id))?;
        let table = w5k_validate::capability::from_results(&def.id, &results);
        std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        let text = serde_json::to_string_pretty(&table).map_err(|e| e.to_string())?;
        std::fs::write(out.join(format!("{}.json", def.id)), format!("{text}\n")).map_err(|e| e.to_string())?;
        println!("{}: {} values, missing {:?}", def.id, 6 - table.missing.len(), table.missing);
        all.push(table);
    }
    let text = serde_json::to_string_pretty(&all).map_err(|e| e.to_string())?;
    std::fs::write(out.join("capability.json"), format!("{text}\n")).map_err(|e| e.to_string())?;
    Ok(())
}
