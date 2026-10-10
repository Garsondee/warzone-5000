//! `w5k tracks`: the command line of lane TRACKS (only that lane edits this file).

use w5k_terramech::reference::reference_soils;
use w5k_terramech::soil;

const PA_PER_KPA: f64 = 1e3; // const-ok: unit conversion at the display edge
const MM_PER_M: f64 = 1e3; // const-ok: unit conversion at the display edge
const USAGE: &str = "usage: w5k tracks bench plate --out DIR   (plate pressure-sinkage of the reference soils, CSV)";

/// Entry point for `w5k tracks <args>`.
pub fn run(args: &[String]) -> Result<(), String> {
    match (args.first().map(String::as_str), args.get(1).map(String::as_str)) {
        (Some("bench"), Some("plate")) => plate(&args[2..]),
        _ => Err(USAGE.to_string()),
    }
}

fn out_dir(args: &[String]) -> Result<std::path::PathBuf, String> {
    let i = args.iter().position(|a| a == "--out").ok_or_else(|| USAGE.to_string())?;
    let dir = std::path::PathBuf::from(args.get(i + 1).ok_or_else(|| USAGE.to_string())?);
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    Ok(dir)
}

/// Sinkage of a 0.5 m wide plate against the ground pressure on it, for each reference soil: `z = (p / (kc/b + kphi))^(1/n)`.
fn plate(args: &[String]) -> Result<(), String> {
    let dir = out_dir(args)?;
    let soils = reference_soils();
    let b_m = 0.5;
    let mut csv = String::from("pressure_kpa");
    for m in &soils {
        csv += &format!(",{}_mm", m.name);
    }
    csv.push('\n');
    for kpa in (0..=100).step_by(2) {
        csv += &format!("{kpa}");
        for m in &soils {
            let s = m.soil.ok_or("reference soil without soil data")?;
            csv += &format!(",{:.2}", soil::sinkage_m(&s, b_m, f64::from(kpa) * PA_PER_KPA) * MM_PER_M);
        }
        csv.push('\n');
    }
    let path = dir.join("plate_sinkage.csv");
    std::fs::write(&path, csv).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}
