//! `w5k viewer`: the command line of lane VIEWER (only that lane edits this file).

use std::path::PathBuf;

use w5k_contract::testing::{tank_slew_and_pitch, truck_over_bumps};
use w5k_replay::ReplayFile;

/// Entry point for `w5k viewer <args>`.
pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("dump-canned") => dump_canned(&args[1..]),
        _ => {
            Err("usage: w5k viewer dump-canned <truck|tank> --out <dir>   (writes replay.json and rig.json)"
                .to_string())
        }
    }
}

fn dump_canned(args: &[String]) -> Result<(), String> {
    let which = args.first().ok_or("dump-canned needs truck or tank")?;
    let out = args
        .iter()
        .position(|a| a == "--out")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .ok_or("dump-canned needs --out <dir>")?;
    let (header, frames, rig) = match which.as_str() {
        "truck" => truck_over_bumps(),
        "tank" => tank_slew_and_pitch(),
        other => return Err(format!("unknown canned replay {other}")),
    };
    w5k_replay::write_json(&out.join("replay.json"), &ReplayFile { header, frames })?;
    let rig_json = serde_json::to_string(&rig).map_err(|e| format!("cannot serialise the rig: {e}"))?;
    std::fs::write(out.join("rig.json"), rig_json).map_err(|e| format!("cannot write rig.json: {e}"))?;
    Ok(())
}
