//! `w5k viewer`: the command line of lane VIEWER (only that lane edits this file).
//!
//! The page, the recorder and the plotter are Node scripts in `tools/viewer` (headless Chromium does the drawing); these commands
//! prepare their inputs (a binary replay and a rig) and call them. Packages are installed by `npm ci` in `tools/viewer`.

use std::path::{Path, PathBuf};
use std::process::Command;

use w5k_contract::render::RenderRig;
use w5k_contract::testing::{box_tank, box_truck, tank_slew_and_pitch, truck_over_bumps};
use w5k_replay::ReplayFile;

const USAGE: &str = "usage:
  w5k viewer render <replay.json|replay.w5kr> --out clip.mp4 [--rig rig.json] [--camera chase|orbit] [--seconds N] [--start S] [--fps N]
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

/// The rig to draw: `--rig file.json` (a serialised `RenderRig`), or, for the canned stand-ins, the rig named in the header.
fn rig_for(replay: &ReplayFile, rig_arg: Option<&str>) -> Result<RenderRig, String> {
    if let Some(p) = rig_arg {
        let s = std::fs::read_to_string(p).map_err(|e| format!("cannot read {p}: {e}"))?;
        return serde_json::from_str(&s).map_err(|e| format!("cannot parse {p}: {e}"));
    }
    match replay.header.vehicles.first().map(|v| v.rig_id.as_str()) {
        Some("box_truck") => Ok(box_truck().1),
        Some("box_tank") => Ok(box_tank().1),
        Some(other) => Err(format!("no built-in rig for {other}; pass --rig <rig.json>")),
        None => Err("the replay has no vehicles".to_string()),
    }
}

fn render(args: &[String], record: bool) -> Result<(), String> {
    let input = args.first().ok_or(USAGE)?;
    let out = opt(args, "--out").ok_or("--out is required")?;
    let replay = read_any(Path::new(input))?;
    let rig = rig_for(&replay, opt(args, "--rig"))?;
    let tmp = std::env::temp_dir().join(format!("w5k-viewer-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| format!("cannot create {}: {e}", tmp.display()))?;
    let (bin, rig_json) = (tmp.join("replay.w5kr"), tmp.join("rig.json"));
    w5k_replay::write_bin(&bin, &replay)?;
    std::fs::write(&rig_json, serde_json::to_string(&rig).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let page = if record { tmp.join("page.html") } else { PathBuf::from(out) };
    node(
        "build.mjs",
        &[
            "--rig".into(),
            rig_json.display().to_string(),
            "--replay".into(),
            bin.display().to_string(),
            "--out".into(),
            page.display().to_string(),
        ],
    )?;
    if record {
        let mut a = vec![page.display().to_string(), "--out".to_string(), out.to_string()];
        for k in ["--camera", "--seconds", "--start", "--fps", "--width", "--height"] {
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
