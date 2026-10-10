//! `w5k viewer`: the command line of lane VIEWER (only that lane edits this file).
//!
//! The page, the recorder and the plotter are Node scripts in `tools/viewer` (headless Chromium does the drawing); these commands
//! prepare their inputs (a binary replay and a rig) and call them. Packages are installed by `npm ci` in `tools/viewer`.

use std::path::{Path, PathBuf};
use std::process::Command;

use w5k_contract::render::RenderRig;
use w5k_contract::testing::{box_tank, box_truck, tank_slew_and_pitch, truck_over_bumps};
use w5k_contract::world::WorldQuery;
use w5k_geo::export::render_rig;
use w5k_geo::flags::FlagParams;
use w5k_geo::truck::{utility_4x4, UtilityDims};
use w5k_replay::ReplayFile;
use w5k_world::strip::DataStrip;

const USAGE: &str = "usage:
  w5k viewer render <replay.json|replay.w5kr> --out clip.mp4 [--rig rig.json] [--skin utility_4x4] [--strip standard] [--terrain terrain.json] [--camera rts|quarter|front|chase|orbit]  (default rts) [--plots inset|full|off] [--seconds N] [--start S] [--fps N]
  w5k viewer page   <replay.json|replay.w5kr> --out page.html [--rig rig.json]   (a self-contained page to open in a browser)
  w5k viewer plot   <data.csv> --out chart.png [--title T] [--xlabel X] [--ylabel Y] [--width W] [--height H]
  w5k viewer dump-canned <truck|tank> --out <dir>   (writes replay.w5kr, replay.json and rig.json)";

/// Entry point for `w5k viewer <args>`.
pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("dump-canned") => dump_canned(&args[1..]),
        Some("render") => render(&args[1..], true),
        Some("page") => render(&args[1..], false),
        Some("plot") => plot(&args[1..]),
        _ => Err(USAGE.to_string()),
    }
}

fn viewer_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/viewer")
}

fn opt<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter().position(|a| a == key).and_then(|i| args.get(i + 1)).map(String::as_str)
}

fn node(script: &str, args: &[String]) -> Result<(), String> {
    let status = Command::new("node")
        .arg(viewer_dir().join(script))
        .args(args)
        .status()
        .map_err(|e| format!("cannot run node ({e}); is it installed, and did `npm ci` run in tools/viewer?"))?;
    status.success().then_some(()).ok_or_else(|| format!("{script} failed ({status})"))
}

fn read_any(path: &Path) -> Result<ReplayFile, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    if bytes.starts_with(b"W5KR") {
        w5k_replay::binary::decode(&bytes)
    } else {
        w5k_replay::from_json_str(&String::from_utf8_lossy(&bytes))
    }
}

/// The meshes to draw over the physics rig's skeleton: a built-in rig id or a serialised `RenderRig` file.
fn skin_rig(name: &str) -> Result<RenderRig, String> {
    match name {
        "utility_4x4" => {
            Ok(render_rig("utility_4x4", &utility_4x4(&UtilityDims::placeholder(), 1), &FlagParams::default_params()))
        }
        file => {
            let s = std::fs::read_to_string(file).map_err(|e| {
                format!("--skin is a built-in rig id (utility_4x4) or a rig file; cannot read {file}: {e}")
            })?;
            serde_json::from_str(&s).map_err(|e| format!("cannot parse {file}: {e}"))
        }
    }
}

/// A heightfield of the ground the replay drove over, sampled from the world model on a regular grid (x, z, in metres) and written as
/// JSON for the page in WORLD's `w5k-terrain-1` format (rows of constant z, no props). Wide enough for the whole drive.
fn terrain_json(world: &dyn WorldQuery, replay: &ReplayFile) -> String {
    const STEP_M: f64 = 0.25; // const-ok: viewer mesh resolution
    const MARGIN_M: f64 = 20.0; // const-ok: ground shown beyond the driven path
    const HALF_WIDTH_M: f64 = 10.0; // const-ok: ground shown either side of the path
    let (mut z_lo, mut z_hi, mut x_lo, mut x_hi) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for v in replay.frames.iter().flat_map(|f| f.vehicles.iter()) {
        z_lo = z_lo.min(v.pos_m.z);
        z_hi = z_hi.max(v.pos_m.z);
        x_lo = x_lo.min(v.pos_m.x);
        x_hi = x_hi.max(v.pos_m.x);
    }
    let (x0, z0) = ((x_lo - HALF_WIDTH_M).floor(), (z_lo - MARGIN_M).floor());
    let (nx, nz) = (((x_hi + HALF_WIDTH_M - x0) / STEP_M) as usize + 1, ((z_hi + MARGIN_M - z0) / STEP_M) as usize + 1);
    let (mut h, mut mat) = (Vec::with_capacity(nx * nz), Vec::with_capacity(nx * nz));
    for j in 0..nz {
        for i in 0..nx {
            let (x, z) = (x0 + i as f64 * STEP_M, z0 + j as f64 * STEP_M);
            h.push((world.height_m(x, z) * 1e4).round() / 1e4); // const-ok: 0.1 mm
            mat.push(world.material_id_at(x, z).0);
        }
    }
    let names: Vec<String> = (0..=mat.iter().copied().max().unwrap_or(0))
        .map(|k| world.materials().get(w5k_contract::world::MaterialId(k)).name.clone())
        .collect();
    serde_json::json!({
        "format": "w5k-terrain-1", "course": "strip", "nx": nx, "nz": nz, "cell_m": STEP_M, "origin_m": { "x": x0, "z": z0 },
        "heights_m": h, "material_ids": mat, "materials": names, "road_m": [], "props": []
    })
    .to_string()
}

/// The rig to draw: `--rig file.json` (a serialised `RenderRig`), or, for the canned stand-ins, the rig named in the header.
fn rig_for(replay: &ReplayFile, rig_arg: Option<&str>) -> Result<RenderRig, String> {
    if let Some(p) = rig_arg {
        let s = std::fs::read_to_string(p).map_err(|e| format!("cannot read {p}: {e}"))?;
        return serde_json::from_str(&s).map_err(|e| format!("cannot parse {p}: {e}"));
    }
    match replay.header.vehicles.first().map(|v| v.rig_id.as_str()) {
        Some("box_truck") => Ok(box_truck().1),
        Some("box_tank") => Ok(box_tank().1),
        // GEOMETRY's utility truck (same node layout as the stand-in truck, so the canned truck replay drives it).
        Some("utility_4x4") => {
            Ok(render_rig("utility_4x4", &utility_4x4(&UtilityDims::placeholder(), 1), &FlagParams::default_params()))
        }
        Some(other) => Err(format!("no built-in rig for {other}; pass --rig <rig.json>")),
        None => Err("the replay has no vehicles".to_string()),
    }
}

fn render(args: &[String], record: bool) -> Result<(), String> {
    let input = args.first().ok_or(USAGE)?;
    let out = opt(args, "--out").ok_or("--out is required")?;
    let replay = read_any(Path::new(input))?;
    let mut rig = rig_for(&replay, opt(args, "--rig"))?;
    if let Some(skin) = opt(args, "--skin") {
        rig = w5k_replay::skin::retarget(&skin_rig(skin)?, &rig)?;
    }
    let tmp = std::env::temp_dir().join(format!("w5k-viewer-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| format!("cannot create {}: {e}", tmp.display()))?;
    let (bin, rig_json) = (tmp.join("replay.w5kr"), tmp.join("rig.json"));
    w5k_replay::write_bin(&bin, &replay)?;
    std::fs::write(&rig_json, serde_json::to_string(&rig).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let mut extra = Vec::new();
    match opt(args, "--strip") {
        None => {
            // A terrain file named in the header (relative to the replay) or given with --terrain: WORLD's export.
            let named = replay
                .header
                .world
                .terrain
                .as_ref()
                .map(|t| Path::new(input).parent().unwrap_or(Path::new(".")).join(t));
            if let Some(file) = opt(args, "--terrain").map(PathBuf::from).or(named.filter(|p| p.exists())) {
                extra.extend(["--terrain".to_string(), file.display().to_string()]);
            }
        }
        Some("standard") => {
            let terrain = tmp.join("terrain.json");
            std::fs::write(&terrain, terrain_json(&DataStrip::standard(), &replay)).map_err(|e| e.to_string())?;
            extra.extend(["--terrain".to_string(), terrain.display().to_string()]);
        }
        Some(other) => return Err(format!("unknown strip {other} (known: standard)")),
    }
    let page = if record { tmp.join("page.html") } else { PathBuf::from(out) };
    let mut build: Vec<String> = [
        "--rig",
        &rig_json.display().to_string(),
        "--replay",
        &bin.display().to_string(),
        "--out",
        &page.display().to_string(),
    ]
    .map(String::from)
    .to_vec();
    build.extend(extra);
    node("build.mjs", &build)?;
    if record {
        let mut a = vec![page.display().to_string(), "--out".to_string(), out.to_string()];
        for k in ["--camera", "--plots", "--seconds", "--start", "--fps", "--width", "--height"] {
            if let Some(v) = opt(args, k) {
                a.extend([k.to_string(), v.to_string()]);
            }
        }
        node("capture.mjs", &a)?;
    }
    let _ = std::fs::remove_dir_all(&tmp);
    Ok(())
}

fn plot(args: &[String]) -> Result<(), String> {
    args.first().ok_or(USAGE)?;
    node("plot.mjs", args)
}

fn dump_canned(args: &[String]) -> Result<(), String> {
    let which = args.first().ok_or("dump-canned needs truck or tank")?;
    let out = opt(args, "--out").map(PathBuf::from).ok_or("dump-canned needs --out <dir>")?;
    let (header, frames, rig) = match which.as_str() {
        "truck" => truck_over_bumps(),
        "tank" => tank_slew_and_pitch(),
        other => return Err(format!("unknown canned replay {other}")),
    };
    let replay = ReplayFile { header, frames };
    w5k_replay::write_bin(&out.join("replay.w5kr"), &replay)?;
    w5k_replay::write_json(&out.join("replay.json"), &replay)?;
    let rig_json = serde_json::to_string(&rig).map_err(|e| format!("cannot serialise the rig: {e}"))?;
    std::fs::write(out.join("rig.json"), rig_json).map_err(|e| format!("cannot write rig.json: {e}"))?;
    Ok(())
}
