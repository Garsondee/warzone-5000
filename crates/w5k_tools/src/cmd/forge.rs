//! `w5k forge`: the command line of lane FORGE (only that lane edits this file).
//!
//! - `w5k forge compile <vehicle.ron> --out DIR [--lever name=factor ...]`: the rig JSON, the render rig JSON and the compile report.
//! - `w5k forge audit <vehicle.ron> [--lever name=factor ...] [--out FILE]`: the design audit (`w5k.audit.v1`, JSON) on stdout or in FILE.
//!
//! The extras sidecar is `<vehicle>.extras.ron` beside the def. Levers (`w5k_forge::levers`) perturb the design before the compile.

use std::path::{Path, PathBuf};

use w5k_contract::def::VehicleDef;
use w5k_forge::audit::audit;
use w5k_forge::compile::{compile, parse_def, parse_extras, Compiled};
use w5k_forge::extras::Extras;
use w5k_forge::levers::apply_both;
use w5k_forge::render::render_rig;

const USAGE: &str = "usage:\n  w5k forge compile <vehicle.ron> --out DIR [--lever name=factor ...]\n  w5k forge audit <vehicle.ron> [--lever name=factor ...] [--out FILE]";

struct Args {
    vehicle: PathBuf,
    out: Option<PathBuf>,
    levers: Vec<(String, f64)>,
}

fn parse(args: &[String]) -> Result<Args, String> {
    let (mut vehicle, mut out, mut levers) = (None, None, Vec::new());
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--out" => out = Some(PathBuf::from(it.next().ok_or(USAGE)?)),
            "--lever" => {
                let spec = it.next().ok_or(USAGE)?;
                let (name, f) =
                    spec.split_once('=').ok_or_else(|| format!("--lever wants name=factor, got {spec:?}"))?;
                levers.push((
                    name.to_string(),
                    f.parse::<f64>().map_err(|e| format!("lever {name}: factor {f:?}: {e}"))?,
                ));
            }
            other if vehicle.is_none() && !other.starts_with("--") => vehicle = Some(PathBuf::from(other)),
            other => return Err(format!("unexpected argument {other:?}\n{USAGE}")),
        }
    }
    Ok(Args { vehicle: vehicle.ok_or(USAGE)?, out, levers })
}

fn load(path: &Path, levers: &[(String, f64)]) -> Result<(VehicleDef, Extras), String> {
    let read = |p: &Path| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
    let mut def = parse_def(&read(path)?)?;
    let mut ex = parse_extras(&read(&path.with_extension("extras.ron"))?)?;
    for (name, f) in levers {
        (def, ex) = apply_both(&def, &ex, name, *f)?;
    }
    Ok((def, ex))
}

fn compiled(def: &VehicleDef, ex: &Extras) -> Result<Compiled, String> {
    compile(def, ex).map_err(|e| e.iter().map(|r| r.to_string()).collect::<Vec<_>>().join("\n"))
}

/// Entry point for `w5k forge <args>`.
pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("compile") => {
            let a = parse(&args[1..])?;
            let dir = a.out.clone().ok_or(USAGE)?;
            let (def, ex) = load(&a.vehicle, &a.levers)?;
            let c = compiled(&def, &ex)?;
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let json = |v: &dyn erased::Ser| v.to_json();
            std::fs::write(dir.join("rig.json"), json(&c.rig)).map_err(|e| e.to_string())?;
            std::fs::write(dir.join("render_rig.json"), json(&render_rig(&c.rig, c.hull_size_m)))
                .map_err(|e| e.to_string())?;
            std::fs::write(dir.join("report.txt"), c.report.join("\n")).map_err(|e| e.to_string())?;
            println!("{}\nwrote rig.json, render_rig.json and report.txt in {}", c.report.join("\n"), dir.display());
            Ok(())
        }
        Some("audit") => {
            let a = parse(&args[1..])?;
            let (def, ex) = load(&a.vehicle, &a.levers)?;
            let c = compiled(&def, &ex)?;
            let text = serde_json::to_string_pretty(&audit(&def, &ex, &c)).map_err(|e| e.to_string())?;
            match &a.out {
                Some(p) => {
                    std::fs::write(p, &text).map_err(|e| format!("{}: {e}", p.display()))?;
                    println!("wrote {}", p.display());
                }
                None => println!("{text}"),
            }
            Ok(())
        }
        _ => Err(USAGE.to_string()),
    }
}

mod erased {
    /// A tiny object-safe wrapper so the two JSON writes above share one call.
    pub trait Ser {
        fn to_json(&self) -> String;
    }
    impl<T: serde::Serialize> Ser for T {
        fn to_json(&self) -> String {
            serde_json::to_string_pretty(self).unwrap_or_default()
        }
    }
}
