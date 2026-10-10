//! `w5k tracks`: the command line of lane TRACKS (only that lane edits this file).

use w5k_contract::testing::{box_tank, standard_materials};
use w5k_contract::{Material, MaterialId};
use w5k_terramech::gear::GearConfig;
use w5k_terramech::ladder::{ladder_row, LadderTank};
use w5k_terramech::plan::PlanVehicle;
use w5k_terramech::reference::reference_soils;
use w5k_terramech::{soil, Tuning};

const PA_PER_KPA: f64 = 1e3; // const-ok: unit conversion at the display edge
const MM_PER_M: f64 = 1e3; // const-ok: unit conversion at the display edge
const USAGE: &str = "usage: w5k tracks bench <plate|thrust|pivot|ladder> --out DIR [--mass-t T] [--soil NAME]   (CSV of plate sinkage, drawbar pull against slip, or pivot-turn response)";
const GRAVITY_M_S2: f64 = 9.80665; // const-ok: standard gravity, SPEC; the same value as content/physics/tracks/tuning.ron
const DT_S: f64 = 0.001; // const-ok: bench time step, 1 kHz
const SLIP_STEP: f64 = 0.025; // const-ok: slip axis step of the thrust bench
const DEFAULT_MASS_T: f64 = 15.0; // const-ok: bench default mass, light enough that all three reference soils carry it
const LADDER_RUNGS: u32 = 30; // const-ok: rungs of the ladder
const LADDER_STEP_T: f64 = 5.0; // const-ok: tonnes per rung
const LADDER_MAX_SINKAGE_M: f64 = 1.0; // const-ok: search bound for the equilibrium sinkage
const KG_PER_TONNE: f64 = 1e3; // const-ok: unit conversion at the input edge
const N_PER_KN: f64 = 1e3; // const-ok: unit conversion at the display edge

/// Entry point for `w5k tracks <args>`.
pub fn run(args: &[String]) -> Result<(), String> {
    match (args.first().map(String::as_str), args.get(1).map(String::as_str)) {
        (Some("bench"), Some("plate")) => plate(&args[2..]),
        (Some("bench"), Some("thrust")) => thrust(&args[2..]),
        (Some("bench"), Some("pivot")) => pivot(&args[2..]),
        (Some("bench"), Some("ladder")) => ladder(&args[2..]),
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

fn write(dir: &std::path::Path, name: &str, csv: String) -> Result<(), String> {
    let path = dir.join(name);
    std::fs::write(&path, csv).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}

/// The reference tank (`box_tank`'s tracks) on `ground`, carrying `mass_t` tonnes; `None` if the ground cannot hold it.
fn tank(ground: &Material, mass_t: f64) -> Option<PlanVehicle> {
    let (rig, _) = box_tank();
    let gauge_m = (rig.stations[0].rest_pos_m.x - rig.stations[rig.stations.len() - 1].rest_pos_m.x).abs();
    let cfg = GearConfig::from_rig(&rig, 0);
    PlanVehicle::new(cfg, Tuning::shipped(), gauge_m, mass_t * KG_PER_TONNE * GRAVITY_M_S2, ground, 0.5)
    // const-ok: sinkage search bound, m
}

fn mass_t(args: &[String]) -> f64 {
    args.iter()
        .position(|a| a == "--mass-t")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MASS_T)
    // const-ok: default bench mass, t
}

/// Net drawbar pull (shear thrust minus compaction resistance) of the reference tank against track slip, on each reference soil, at 1 m/s.
fn thrust(args: &[String]) -> Result<(), String> {
    let dir = out_dir(args)?;
    let soils = reference_soils();
    let mut csv = String::from("slip");
    for m in &soils {
        csv += &format!(",{}_kn", m.name);
    }
    csv.push('\n');
    for step in 0..=20 {
        let slip = f64::from(step) * SLIP_STEP;
        csv += &format!("{slip:.3}");
        for m in &soils {
            let mut v = tank(m, mass_t(args))
                .ok_or_else(|| format!("{} cannot carry {} t: lower --mass-t", m.name, mass_t(args)))?;
            v.motion.vx_m_s = 1.0;
            let mut pull = 0.0;
            for _ in 0..6000 {
                pull = v.step((1.0 / (1.0 - slip), 1.0 / (1.0 - slip)), DT_S).fx_n;
            }
            csv += &format!(",{:.3}", pull / N_PER_KN);
        }
        csv.push('\n');
    }
    write(&dir, "thrust_slip.csv", csv)
}

/// Yaw rate against time of the reference tank pivoting (belts at plus and minus 1 m/s) on asphalt, dirt and mud.
fn pivot(args: &[String]) -> Result<(), String> {
    let dir = out_dir(args)?;
    let table = standard_materials();
    let grounds: Vec<Material> = (0..3).map(|i| table.get(MaterialId(i)).clone()).collect();
    let mut vehicles = grounds
        .iter()
        .map(|g| tank(g, mass_t(args)).ok_or_else(|| format!("{} cannot carry the tank", g.name)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut csv = String::from("t_s");
    for g in &grounds {
        csv += &format!(",{}_yaw_rad_s", g.name);
    }
    csv.push('\n');
    for step in 0..=15_000 {
        if step % 100 == 0 {
            csv += &format!("{:.2}", f64::from(step) * DT_S);
            for v in &vehicles {
                csv += &format!(",{:.4}", v.motion.yaw_rate_rad_s);
            }
            csv.push('\n');
        }
        for v in &mut vehicles {
            v.step_dynamic((-1.0, 1.0), (0.0, 0.0), DT_S);
        }
    }
    write(&dir, "pivot_response.csv", csv)
}

/// The ladder: the reference tank from 5 t upward on one reference soil (`--soil`, default `snow`): sinkage against the soil theory, the
/// clearance, the net drawbar pull and the pull below which the tank counts as bogged.
fn ladder(args: &[String]) -> Result<(), String> {
    let dir = out_dir(args)?;
    let name = args.iter().position(|a| a == "--soil").and_then(|i| args.get(i + 1)).map_or("snow", String::as_str);
    let ground = reference_soils()
        .into_iter()
        .find(|m| m.name == name)
        .ok_or_else(|| format!("no reference soil called {name}"))?;
    let (rig, _) = box_tank();
    let (tuning, cfg) = (Tuning::shipped(), GearConfig::from_rig(&rig, 0));
    let belly = LadderTank::shipped().belly(&tuning);
    let gauge_m = (rig.stations[0].rest_pos_m.x - rig.stations[rig.stations.len() - 1].rest_pos_m.x).abs();
    let mut csv = String::from("mass_t,theory_mm,sinkage_mm,clearance_mm,net_pull_kn,bog_limit_kn\n");
    for step in 1..=LADDER_RUNGS {
        let mass_kg = f64::from(step) * LADDER_STEP_T * KG_PER_TONNE;
        let r = ladder_row(&cfg, belly, gauge_m, tuning, &ground, mass_kg, LADDER_MAX_SINKAGE_M);
        let limit = tuning.bog_pull_fraction * mass_kg * tuning.gravity_m_s2;
        csv += &format!(
            "{:.0},{:.1},{:.1},{:.1},{:.2},{:.2}\n",
            mass_kg / KG_PER_TONNE,
            r.predicted_sinkage_m * MM_PER_M,
            r.sinkage_m * MM_PER_M,
            belly.clearance_m * MM_PER_M,
            r.net_pull_n / N_PER_KN,
            limit / N_PER_KN
        );
    }
    write(&dir, &format!("ladder_{name}.csv"), csv)
}
