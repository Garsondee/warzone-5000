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
use w5k_geo::skin::Skin;
use w5k_replay::ReplayFile;
use w5k_world::strip::DataStrip;

const USAGE: &str = "usage:
  w5k viewer render <replay.json|replay.w5kr> --out clip.mp4 [--rig rig.json] [--rig a.json,b.json] [--skin utility_4x4[,..]] [--strip standard] [--terrain terrain.json] [--camera rts|quarter|front|chase|orbit] [--plots inset|full|off] [--seconds N] [--start S] [--fps N]
  w5k viewer page   <replay.json|replay.w5kr> --out page.html [--rig rig.json]   (a self-contained page to open in a browser)
  w5k viewer fake-fleet <replay> --out fleet.w5kr [--n 3] [--offset S]   (test data: the first vehicle repeated n times, each S seconds behind the last)
  w5k viewer pack-skin <utility_4x4|scout_4x4|rig.json> --out tools/viewer/dist/skins/<id>.skin   (the compact skin file the test-drive page fetches)
  w5k viewer plot   <data.csv> --out chart.png [--title T] [--xlabel X] [--ylabel Y] [--width W] [--height H]
  w5k viewer dump-canned <truck|tank> --out <dir>   (writes replay.w5kr, replay.json and rig.json)";

/// Entry point for `w5k viewer <args>`.
pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("dump-canned") => dump_canned(&args[1..]),
        Some("render") => render(&args[1..], true),
        Some("page") => render(&args[1..], false),
        Some("plot") => plot(&args[1..]),
        Some("fake-fleet") => fake_fleet(&args[1..]),
        Some("pack-skin") => pack_skin(&args[1..]),
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

/// The meshes to draw over the physics rig's skeleton: one of GEOMETRY's skins by id (`utility_4x4`, `scout_4x4`, built the way
/// `w5k geometry export` builds them) or a serialised `RenderRig` file.
fn skin_rig(name: &str) -> Result<RenderRig, String> {
    if let Some(skin) = Skin::for_id(name) {
        return Ok(render_rig(skin.kind.id(), &skin.parts(1), &FlagParams::default_params()));
    }
    let s = std::fs::read_to_string(name).map_err(|e| {
        format!("--skin is a GEOMETRY skin id (utility_4x4, scout_4x4) or a rig file; cannot read {name}: {e}")
    })?;
    serde_json::from_str(&s).map_err(|e| format!("cannot parse {name}: {e}"))
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

/// The `i`-th item of a comma-separated option (the last one repeats for later vehicles): `--rig a.json,b.json`, `--skin x,y,z`.
fn pick(list: Option<&str>, i: usize) -> Option<&str> {
    let items: Vec<&str> = list?.split(',').collect();
    items.get(i).or(items.last()).copied()
}

/// The rig to draw: `--rig file.json` (a serialised `RenderRig`), or, for the canned stand-ins, the rig named in the header.
fn rig_for(replay: &ReplayFile, vehicle: usize, rig_arg: Option<&str>) -> Result<RenderRig, String> {
    if let Some(p) = rig_arg {
        let s = std::fs::read_to_string(p).map_err(|e| format!("cannot read {p}: {e}"))?;
        return serde_json::from_str(&s).map_err(|e| format!("cannot parse {p}: {e}"));
    }
    match replay.header.vehicles.get(vehicle).map(|v| v.rig_id.as_str()) {
        Some("box_truck") => Ok(box_truck().1),
        Some("box_tank") => Ok(box_tank().1),
        // GEOMETRY's utility truck (same node layout as the stand-in truck, so the canned truck replay drives it).
        Some(id) if Skin::for_id(id).is_some() => skin_rig(id),
        Some(other) => Err(format!("no built-in rig for {other}; pass --rig <rig.json>")),
        None => Err("the replay has no vehicles".to_string()),
    }
}

fn render(args: &[String], record: bool) -> Result<(), String> {
    let input = args.first().ok_or(USAGE)?;
    let out = opt(args, "--out").ok_or("--out is required")?;
    let replay = read_any(Path::new(input))?;
    let tmp = std::env::temp_dir().join(format!("w5k-viewer-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| format!("cannot create {}: {e}", tmp.display()))?;
    // One rig per vehicle in the header (`--rig` and `--skin` take comma-separated lists, the last item repeats).
    let mut rig_files = Vec::new();
    for i in 0..replay.header.vehicles.len() {
        let mut rig = rig_for(&replay, i, pick(opt(args, "--rig"), i))?;
        if let Some(skin) = pick(opt(args, "--skin"), i) {
            rig = w5k_replay::skin::retarget(&skin_rig(skin)?, &rig)?;
        }
        let file = tmp.join(format!("rig{i}.json"));
        std::fs::write(&file, serde_json::to_string(&rig).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        rig_files.push(file.display().to_string());
    }
    let bin = tmp.join("replay.w5kr");
    w5k_replay::write_bin(&bin, &replay)?;
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
    let mut build: Vec<String> =
        ["--rig", &rig_files.join(","), "--replay", &bin.display().to_string(), "--out", &page.display().to_string()]
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

/// Packs a detailed rig (GEOMETRY's truck, or a serialised `RenderRig`) into the compact skin file the live page loads.
fn pack_skin(args: &[String]) -> Result<(), String> {
    let name = args.first().ok_or(USAGE)?;
    let out = opt(args, "--out").ok_or("--out is required")?;
    let rig = skin_rig(name)?;
    let bytes = w5k_replay::skinpack::pack(&rig);
    if let Some(dir) = Path::new(out).parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    std::fs::write(out, &bytes).map_err(|e| format!("cannot write {out}: {e}"))?;
    println!("{out}: {} bytes, {} triangles, {} meshes", bytes.len(), rig.triangle_count(), rig.meshes.len());
    Ok(())
}

/// Test data for multi-vehicle replays: vehicle 0 repeated `n` times, vehicle `i` driving `i * offset` seconds behind it (it waits at
/// the start), named `<name>-A`, `-B`, ... Until a real scenario writes several vehicles.
fn fake_fleet(args: &[String]) -> Result<(), String> {
    let input = args.first().ok_or(USAGE)?;
    let out = opt(args, "--out").ok_or("--out is required")?;
    let n: usize = opt(args, "--n").map_or(Ok(3), str::parse).map_err(|_| "--n is a number")?;
    let offset_s: f64 = opt(args, "--offset").map_or(Ok(3.0), str::parse).map_err(|_| "--offset is seconds")?;
    let mut replay = read_any(Path::new(input))?;
    let base = replay.header.vehicles.first().ok_or("the replay has no vehicles")?.clone();
    replay.header.vehicles = (0..n)
        .map(|i| {
            let mut v = base.clone();
            v.name = format!("{}-{}", base.name, char::from(b'A' + i as u8));
            v
        })
        .collect();
    let lag = (offset_s / replay.header.frame_dt_s).round() as usize;
    let source = replay.frames.clone();
    for (k, f) in replay.frames.iter_mut().enumerate() {
        let first = f.vehicles[0].clone();
        f.vehicles = (0..n)
            .map(|i| {
                let mut v = if i == 0 { first.clone() } else { source[k.saturating_sub(i * lag)].vehicles[0].clone() };
                v.vehicle = i as u32;
                v
            })
            .collect();
    }
    w5k_replay::write_bin(Path::new(out), &replay)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `w5k drive --web tools/viewer/dist` serves the committed page; no Node runs on the player's PC, so the build output is in git.
    /// `node tools/viewer/build.mjs --live --check` (a developer's check) says whether it is current; this one says it is there and
    /// speaks the protocol.
    #[test]
    fn the_test_drive_page_is_built_into_dist_and_calls_every_endpoint_it_needs() {
        let page = viewer_dir().join("dist/index.html");
        let html = std::fs::read_to_string(&page)
            .unwrap_or_else(|e| panic!("{} is missing ({e}): run node tools/viewer/build.mjs --live", page.display()));
        for endpoint in ["/api/vehicles", "/api/world", "/api/rig/", "/api/select", "/api/input", "/api/stream"] {
            assert!(html.contains(endpoint), "the page never mentions {endpoint}");
        }
        assert!(
            html.len() < 1024 * 1024,
            "the page is {} bytes: the media lint refuses committed files over 1 MiB",
            html.len()
        ); // const-ok: the lint's limit
    }

    fn skin_files() -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(viewer_dir().join("dist/skins"))
            .map(|d| {
                d.filter_map(Result::ok)
                    .filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".skin")).map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    /// The bodies the page draws are generated files. If GEOMETRY changes a body and nobody packs it again, the page would ship the old one,
    /// so every skin of GEOMETRY's registry must have a committed file that is byte for byte what `pack-skin` writes now (and no file may be
    /// left for an id the registry no longer has).
    #[test]
    fn every_committed_skin_is_what_pack_skin_generates_now() {
        for id in w5k_geo::skin::IDS {
            let file = viewer_dir().join(format!("dist/skins/{id}.skin"));
            let fix = format!(
                "run `w5k viewer pack-skin {id} --out tools/viewer/dist/skins/{id}.skin` and `node tools/viewer/build.mjs --live`"
            );
            let committed =
                std::fs::read(&file).unwrap_or_else(|e| panic!("{} is missing ({e}); {fix}", file.display()));
            let fresh = w5k_replay::skinpack::pack(&skin_rig(id).expect("a registry skin builds"));
            assert!(
                committed == fresh,
                "{id}.skin is stale ({} bytes committed, {} generated now); {fix}",
                committed.len(),
                fresh.len()
            );
        }
        for name in skin_files() {
            assert!(
                w5k_geo::skin::IDS.contains(&name.as_str()),
                "dist/skins/{name}.skin is not a skin of GEOMETRY's registry: delete it"
            );
        }
    }

    /// The page asks only for the skins listed in its data block (so it never probes for a missing file); that list is written when the page
    /// is built, so it must be the skins that are committed.
    #[test]
    fn the_page_lists_exactly_the_skins_it_ships() {
        let page = viewer_dir().join("dist/index.html");
        let html = std::fs::read_to_string(&page).unwrap_or_else(|e| panic!("{} is missing ({e})", page.display()));
        let key = "\"skins\":[";
        let at = html.find(key).expect("the page's data block lists its skins") + key.len();
        let end = at + html[at..].find(']').expect("the skin list ends");
        let listed: Vec<&str> =
            html[at..end].split(',').map(|n| n.trim().trim_matches('"')).filter(|n| !n.is_empty()).collect();
        assert_eq!(
            listed,
            skin_files(),
            "dist/index.html lists other skins than dist/skins holds: run `node tools/viewer/build.mjs --live`"
        );
    }
}
