//! `w5k viewer`: the command line of lane VIEWER (only that lane edits this file).
//!
//! The page, the recorder and the plotter are Node scripts in `tools/viewer` (headless Chromium does the drawing); these commands
//! prepare their inputs (a binary replay and a rig) and call them. Packages are installed by `npm ci` in `tools/viewer`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use w5k_contract::def::VehicleDef;
use w5k_contract::render::RenderRig;
use w5k_contract::testing::{box_tank, box_truck, tank_slew_and_pitch, truck_over_bumps};
use w5k_contract::world::WorldQuery;
use w5k_geo::export::render_rig;
use w5k_geo::flags::FlagParams;
use w5k_geo::skin::Skin;
use w5k_math::scalar;
use w5k_replay::ReplayFile;
use w5k_validate::impact::{BENCHES, EPS};
use w5k_validate::proving::ProvingResult;
use w5k_world::strip::DataStrip;

const USAGE: &str = "usage:
  w5k viewer render <replay.json|replay.w5kr> --out clip.mp4 [--rig rig.json] [--rig a.json,b.json] [--skin utility_4x4[,..]] [--strip standard] [--terrain terrain.json] [--camera rts|quarter|front|chase|orbit] [--plots inset|full|off] [--seconds N] [--start S] [--fps N]
  w5k viewer page   <replay.json|replay.w5kr> --out page.html [--rig rig.json]   (a self-contained page to open in a browser)
  w5k viewer fake-fleet <replay> --out fleet.w5kr [--n 3] [--offset S]   (test data: the first vehicle repeated n times, each S seconds behind the last)
  w5k viewer pack-skin <utility_4x4|scout_4x4|rig.json> --out tools/viewer/dist/skins/<id>.skin   (the compact skin file the test-drive page fetches)
  w5k viewer plot   <data.csv> --out chart.png [--title T] [--xlabel X] [--ylabel Y] [--width W] [--height H]
  w5k viewer tornado <impact.json> --out tornado.png [--theme light|dark] [--cols N] [--rows N]   (the impact.json of `w5k validation impact`: a panel per benchmark, a bar per vehicle for each lever; docs/lanes/viewer/charts.md)
  w5k viewer ladder  <ladder.json> --out ladder.png [--theme light|dark]   (sinkage up a ladder of load or track width, beside the soil theory)
  w5k viewer design <scout_4x4|mule_4x4|hauler_4x4|base.ron> --out DIR [--lever wheelbase=1.1,mass=0.9] [--skin preview|final|off] [--score off]   (the Workshop's step: levers in; the compiled design, its skin and the proving scoreboard out)
  w5k viewer dump-canned <truck|tank> --out <dir>   (writes replay.w5kr, replay.json and rig.json)";

/// Entry point for `w5k viewer <args>`.
pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("dump-canned") => dump_canned(&args[1..]),
        Some("render") => render(&args[1..], true),
        Some("page") => render(&args[1..], false),
        Some("plot") => plot(&args[1..]),
        Some(kind @ ("tornado" | "ladder")) => chart(kind, &args[1..]),
        Some("fake-fleet") => fake_fleet(&args[1..]),
        Some("pack-skin") => pack_skin(&args[1..]),
        Some("design") => design(&args[1..]),
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
/// `w5k geometry export` builds them), a packed `.skin` file (`pack-skin`, `design`) or a serialised `RenderRig` file.
fn skin_rig(name: &str) -> Result<RenderRig, String> {
    if let Some(skin) = Skin::for_id(name) {
        return Ok(render_rig(skin.kind.id(), &skin.parts(1), &FlagParams::default_params()));
    }
    if name.ends_with(".skin") {
        let bytes = std::fs::read(name).map_err(|e| format!("cannot read {name}: {e}"))?;
        return w5k_replay::skinpack::unpack(&bytes);
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

/// The evidence charts (`tornado`, `ladder`): `chart.mjs` validates the JSON, draws it in headless Chromium and writes a PNG.
fn chart(kind: &str, args: &[String]) -> Result<(), String> {
    args.first().ok_or(USAGE)?;
    let mut all = vec![kind.to_string()];
    all.extend_from_slice(args);
    node("chart.mjs", &all)
}

/// `name=factor[,name=factor...]`: FORGE's levers, each a positive factor on the base design (1 is the base). FORGE checks the names.
fn parse_levers(spec: &str) -> Result<Vec<(String, f64)>, String> {
    spec.split(',')
        .filter(|item| !item.is_empty())
        .map(|item| {
            let (name, f) = item.split_once('=').ok_or_else(|| format!("lever `{item}` is not name=factor"))?;
            let factor: f64 = f.parse().map_err(|_| format!("lever {name}: `{f}` is not a number"))?;
            if !(factor.is_finite() && factor > 0.0) {
                return Err(format!("lever {name}: the factor {factor} must be finite and positive"));
            }
            Ok((name.to_string(), factor))
        })
        .collect()
}

/// The scoreboard rows, by impact-matrix id: name, unit, scale from the result's unit to it, and whether a bigger number is better. (The
/// names carry their units in `impact::BENCHES` as the runner measures them; the board shows grades in percent and angles in degrees.)
const BOARD: [(&str, &str, &str, f64, bool); 6] = [
    ("B1", "0-48 km/h time", "s", 1.0, false),
    ("B4", "braking distance from 50 km/h", "m", 1.0, false),
    ("B6", "maximum gradient", "%", 100.0, true), // const-ok: a grade ratio shown as a percentage
    ("B7", "side-slope limit", "deg", 180.0 / scalar::PI, true), // const-ok: radians shown as degrees
    ("B11", "step height cleared", "m", 1.0, true),
    ("B12", "cornering limit", "g", 1.0, true),
];

/// The change of a scoreboard number as a percentage of the base (`None` for a base of zero) and whether it is for the better; a change
/// under the impact runner's no-change threshold is neither.
fn board_change(base: f64, value: f64, bigger_is_better: bool) -> (Option<f64>, Option<bool>) {
    let delta = (base != 0.0).then(|| 100.0 * (value - base) / base.abs()); // const-ok: percent
    let better = delta.filter(|d| d.abs() >= 100.0 * EPS).map(|d| (d > 0.0) == bigger_is_better); // const-ok: percent
    (delta, better)
}

/// The proving battery on one vehicle file (its `.extras.ron` beside it), by running `w5k scenario proving` as the impact runner does.
fn run_battery(vehicle: &Path, out: &Path) -> Result<BTreeMap<String, ProvingResult>, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let status = Command::new(exe)
        .args(["scenario", "proving", "--test", "all", "--vehicle"])
        .arg(vehicle)
        .arg("--out")
        .arg(out)
        .stdout(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("cannot run the proving ground: {e}"))?;
    if !status.success() {
        return Err(format!("the proving ground failed on {}", vehicle.display()));
    }
    let dir = std::fs::read_dir(out)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.is_dir());
    let mut results = BTreeMap::new();
    for b in BENCHES {
        if let Some(text) = dir.as_ref().and_then(|d| std::fs::read_to_string(d.join(format!("{}.json", b.test))).ok())
        {
            results.insert(b.id.to_string(), ProvingResult::from_json(&text)?);
        }
    }
    Ok(results)
}

/// A design: the base with levers applied, compiled by FORGE, and the skin that wears it.
struct Design {
    def: VehicleDef,
    extras: w5k_forge::extras::Extras,
    rig: RenderRig,
    report: Vec<String>,
    skin: Option<(Vec<u8>, usize, Vec<f64>)>,
}

/// Apply the levers in order (an error names the lever and says why, never a silent clamp), compile, and bake the skin of the result
/// (`quality`: `preview` is GEOMETRY's quick bake, `final` the full one, `off` none). A design FORGE refuses yields no skin.
fn build_design(base: &Path, levers: &[(String, f64)], quality: &str) -> Result<Design, String> {
    let read = |p: &Path| std::fs::read_to_string(p).map_err(|e| format!("cannot read {}: {e}", p.display()));
    let (mut def, mut extras) = (
        w5k_forge::compile::parse_def(&read(base)?)?,
        w5k_forge::compile::parse_extras(&read(&base.with_extension("extras.ron"))?)?,
    );
    for (name, factor) in levers {
        (def, extras) =
            w5k_forge::levers::apply_both(&def, &extras, name, *factor).map_err(|e| format!("lever {name}: {e}"))?;
    }
    let family = def.clone(); // GEOMETRY picks the hull family by the base id
    def.name = format!("{} (your design)", def.name);
    def.id = format!("{}_design", def.id);
    let compiled = w5k_forge::compile::compile(&def, &extras).map_err(|r| {
        format!("FORGE refused the design: {}", r.iter().map(ToString::to_string).collect::<Vec<_>>().join("; "))
    })?;
    let rig = w5k_forge::render::render_rig(&compiled.rig, compiled.hull_size_m);
    let skin = match quality {
        "off" => None,
        "preview" | "final" => {
            let skin = Skin::from_def(&family)?;
            let flags = if quality == "final" { FlagParams::default_params() } else { FlagParams::preview() };
            let baked = render_rig(skin.kind.id(), &skin.parts(1), &flags);
            Some((w5k_replay::skinpack::pack(&baked), baked.triangle_count(), skin.axles_z.clone()))
        }
        other => return Err(format!("--skin is preview, final or off, not {other}")),
    };
    Ok(Design { def, extras, rig, report: compiled.report, skin })
}

/// `w5k viewer design BASE --out DIR [--lever a=1.1,b=0.9] [--skin preview|final|off] [--score off]`: writes `<id>.ron`, `<id>.extras.ron`,
/// `<id>.rig.json`, `<id>.skin` and `design.json` (what each lever became, the compile report, the scoreboard against the base, timings).
#[allow(clippy::disallowed_methods)] // `Instant::now` only times this CLI run for design.json; nothing simulated reads it
fn design(args: &[String]) -> Result<(), String> {
    let base = args.first().ok_or(USAGE)?;
    let out = PathBuf::from(opt(args, "--out").ok_or("--out is required")?);
    let base_file = if base.ends_with(".ron") {
        PathBuf::from(base)
    } else {
        Path::new("content/vehicles/game").join(format!("{base}.ron"))
    };
    let levers = parse_levers(opt(args, "--lever").unwrap_or(""))?;
    let (quality, scored) = (opt(args, "--skin").unwrap_or("preview"), opt(args, "--score") != Some("off"));
    std::fs::create_dir_all(&out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;
    let t0 = Instant::now();
    let d = build_design(&base_file, &levers, quality)?;
    let (id, t_build) = (d.def.id.clone(), t0.elapsed().as_secs_f64());
    let write = |name: String, bytes: &[u8]| {
        std::fs::write(out.join(&name), bytes).map_err(|e| format!("cannot write {name}: {e}"))
    };
    let ron = |r: Result<String, ron::Error>| r.map_err(|e| e.to_string());
    write(format!("{id}.ron"), ron(ron::to_string(&d.def))?.as_bytes())?;
    write(format!("{id}.extras.ron"), ron(ron::to_string(&d.extras))?.as_bytes())?;
    write(format!("{id}.rig.json"), serde_json::to_string(&d.rig).map_err(|e| e.to_string())?.as_bytes())?;
    if let Some((bytes, _, _)) = &d.skin {
        write(format!("{id}.skin"), bytes)?;
    }
    let t1 = Instant::now();
    let board = if scored {
        let (base_results, design_results) = (
            run_battery(&base_file, &out.join("score/base"))?,
            run_battery(&out.join(format!("{id}.ron")), &out.join("score/design"))?,
        );
        BOARD
            .iter()
            .filter_map(|&(bench, name, unit, scale, bigger)| {
                let b = BENCHES.iter().find(|b| b.id == bench)?;
                let read = |m: &BTreeMap<String, ProvingResult>| m.get(bench).and_then(|r| r.measured.get(b.key)).map(|v| v * scale);
                let (base_v, value) = (read(&base_results)?, read(&design_results)?);
                let (delta, better) = board_change(base_v, value, bigger);
                let note = design_results.get(bench).and_then(|r| r.ended_early.clone());
                Some(serde_json::json!({"bench": bench, "name": name, "unit": unit, "base": base_v, "value": value, "delta_pct": delta, "better": better, "note": note}))
            })
            .collect()
    } else {
        Vec::new()
    };
    let doc = serde_json::json!({
        "schema": "w5k-design-1", "base": base, "id": id,
        "levers": levers.iter().map(|(n, f)| serde_json::json!({"name": n, "factor": f})).collect::<Vec<_>>(),
        "report": d.report, "scoreboard": board,
        "skin": d.skin.as_ref().map(|(b, tris, axles)| serde_json::json!({"file": format!("{id}.skin"), "bytes": b.len(), "triangles": tris, "quality": quality, "axles_z_m": axles})),
        "timings_s": {"levers_compile_skin": t_build, "score": t1.elapsed().as_secs_f64(), "total": t0.elapsed().as_secs_f64()},
    });
    write("design.json".into(), serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?.as_bytes())?;
    println!(
        "{id}: {} levers, skin {}, {} scoreboard rows, {:.2} s in all -> {}",
        levers.len(),
        quality,
        doc["scoreboard"].as_array().map_or(0, Vec::len),
        t0.elapsed().as_secs_f64(),
        out.join("design.json").display()
    );
    for row in doc["scoreboard"].as_array().into_iter().flatten() {
        println!(
            "  {} {}: {:.3} -> {:.3} {} ({})",
            row["bench"].as_str().unwrap_or("?"),
            row["name"].as_str().unwrap_or(""),
            row["base"].as_f64().unwrap_or(f64::NAN),
            row["value"].as_f64().unwrap_or(f64::NAN),
            row["unit"].as_str().unwrap_or(""),
            row["delta_pct"].as_f64().map_or("n/a".into(), |d| format!("{d:+.1}%"))
        );
    }
    Ok(())
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

    fn sample(name: &str) -> serde_json::Value {
        let path = viewer_dir().join("samples").join(name);
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{} is missing ({e})", path.display()));
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{name} is not JSON: {e}"))
    }

    /// The tornado reads the `impact.json` of `w5k validation impact` as it is written; the sample is a copy of one real run (its
    /// numbers date from that run, the shape is what this guards): every entry carries what the chart reads, and the runner's counts
    /// are the counts of its entries.
    #[test]
    fn the_impact_sample_is_a_report_whose_counts_agree_with_its_entries() {
        let report = sample("impact.json");
        let entries = report["entries"].as_array().expect("entries");
        assert!(!entries.is_empty());
        let word = |e: &serde_json::Value, k: &str| {
            e[k].as_str().unwrap_or_else(|| panic!("entry {e} has no `{k}`")).to_string()
        };
        for e in entries {
            let (base, perturbed, delta) =
                (e["base"].as_f64().unwrap(), e["perturbed"].as_f64().unwrap(), e["delta"].as_f64().unwrap());
            if base != 0.0 {
                assert!(
                    (delta - (perturbed - base) / base.abs()).abs() < 1e-9,
                    "delta of {e} is not (perturbed - base) / |base|"
                );
            }
            assert!(["Minus", "Zero", "Plus"].contains(&word(e, "observed").as_str()), "unknown sign in {e}");
            assert!(
                ["Right", "Wrong", "Unlisted", "Quiet"].contains(&word(e, "verdict").as_str()),
                "unknown verdict in {e}"
            );
            word(e, "vehicle");
            word(e, "lever");
            word(e, "bench");
        }
        let count = |v: &str| entries.iter().filter(|e| e["verdict"] == v).count() as u64;
        assert_eq!(report["right"].as_u64(), Some(count("Right")));
        assert_eq!(report["scored"].as_u64(), Some(count("Right") + count("Wrong")));
        assert!(report["dead_levers"].is_array() && report["orphan_benchmarks"].is_array());
    }

    /// The same shape from the producer itself: if VALIDATION renames a field or a word the chart matches on, this fails here instead of
    /// the tornado silently drawing nothing.
    #[test]
    fn the_impact_runners_report_has_the_fields_and_words_the_tornado_reads() {
        use w5k_validate::impact;
        let levers = impact::levers();
        let lever = levers.first().expect("the runner has levers").id.to_string();
        let mut obs =
            impact::Observations { pairs: Default::default(), regimes: Default::default(), labels: Default::default() };
        obs.pairs.insert(("mule_4x4".into(), lever.clone(), "B1".into()), (10.0, 11.0));
        obs.pairs.insert(("mule_4x4".into(), lever, "B4".into()), (10.0, 10.0));
        let report = serde_json::to_value(impact::evaluate(&[], &levers, &obs)).expect("the report serialises");
        let entries = report["entries"].as_array().expect("entries");
        assert_eq!(entries.len(), 2);
        for key in ["vehicle", "lever", "bench", "observed", "verdict"] {
            assert!(entries.iter().all(|e| e[key].is_string()), "entries lost their string `{key}`");
        }
        for key in ["base", "perturbed", "delta"] {
            assert!(entries.iter().all(|e| e[key].is_number()), "entries lost their number `{key}`");
        }
        let moved = entries.iter().find(|e| e["bench"] == "B1").expect("B1");
        assert_eq!((moved["observed"].as_str(), moved["verdict"].as_str()), (Some("Plus"), Some("Unlisted")));
        let still = entries.iter().find(|e| e["bench"] == "B4").expect("B4");
        assert_eq!((still["observed"].as_str(), still["verdict"].as_str()), (Some("Zero"), Some("Quiet")));
        assert!(report["right"].is_u64() && report["scored"].is_u64());
        assert!(report["dead_levers"].is_array() && report["orphan_benchmarks"].is_array());
    }

    #[test]
    fn the_ladder_sample_gives_every_rung_a_load_a_simulated_and_a_predicted_sinkage() {
        let ladder = sample("ladder-stub.json");
        assert_eq!(ladder["schema"], "w5k-ladder-1");
        for v in ladder["vehicles"].as_array().expect("vehicles") {
            let rungs = v["rungs"].as_array().expect("rungs");
            assert!(rungs.len() >= 2, "{} needs a ladder of at least two rungs", v["id"]);
            for r in rungs {
                assert!(
                    r["x"].is_number() && r["sim"].is_number() && r["theory"].is_number(),
                    "{} has a rung without x, sim and theory",
                    v["id"]
                );
            }
        }
    }

    fn base(id: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/vehicles/game").join(format!("{id}.ron"))
    }

    #[test]
    fn levers_parse_into_names_and_positive_factors_and_bad_ones_are_refused() {
        assert_eq!(
            parse_levers("wheelbase=1.1,mass=0.9").unwrap(),
            vec![("wheelbase".to_string(), 1.1), ("mass".to_string(), 0.9)]
        );
        assert!(parse_levers("").unwrap().is_empty());
        for bad in ["wheelbase", "mass=x", "mass=0", "mass=-1", "mass=inf", "mass=NaN"] {
            assert!(parse_levers(bad).is_err(), "{bad} should be refused");
        }
    }

    /// FORGE moves the axles about the front one, so the skin that wears the design must be cut for exactly that wheelbase.
    #[test]
    fn a_wheelbase_lever_stretches_the_skins_axle_spacing_by_the_same_factor() {
        let spacing = |levers: &[(String, f64)]| {
            let axles = build_design(&base("scout_4x4"), levers, "preview").unwrap().skin.unwrap().2;
            axles.last().unwrap() - axles.first().unwrap()
        };
        let factor = 1.25;
        let ratio = spacing(&[("wheelbase".to_string(), factor)]) / spacing(&[]);
        assert!((ratio - factor).abs() < 1e-9, "the skin's axle spacing grew by {ratio}, not {factor}");
    }

    #[test]
    fn a_lever_the_physics_cannot_honour_is_refused_with_its_reason_and_an_unknown_one_lists_the_names() {
        let refused =
            build_design(&base("scout_4x4"), &[("ride_frequency".to_string(), 30.0)], "off").err().expect("refused");
        assert!(
            refused.contains("ride rate") && refused.contains("tyre stiffness"),
            "the reason should say what broke: {refused}"
        );
        let unknown =
            build_design(&base("scout_4x4"), &[("flux_capacitor".to_string(), 1.1)], "off").err().expect("refused");
        assert!(unknown.contains("flux_capacitor") && unknown.contains("engine_peak_power"), "{unknown}");
    }

    #[test]
    fn a_scoreboard_change_is_a_percentage_of_the_base_and_says_which_way_is_better() {
        assert_eq!(board_change(10.0, 9.0, false), (Some(-10.0), Some(true)));
        assert_eq!(board_change(10.0, 9.0, true), (Some(-10.0), Some(false)));
        assert_eq!(
            board_change(10.0, 10.01, true).1,
            None,
            "a change under the impact runner's threshold is neither better nor worse"
        );
        assert_eq!(board_change(0.0, 1.0, true), (None, None));
    }

    /// The board names benchmarks by the impact matrix's ids: if VALIDATION renames or drops one, a row must not vanish silently.
    #[test]
    fn every_scoreboard_row_is_a_benchmark_the_impact_runner_measures() {
        for (id, ..) in BOARD {
            assert!(BENCHES.iter().any(|b| b.id == id), "{id} is not in w5k_validate::impact::BENCHES");
        }
    }

    #[test]
    fn a_packed_skin_file_is_accepted_where_a_skin_id_is() {
        let file = viewer_dir().join("dist/skins/scout_4x4.skin");
        let rig = skin_rig(file.to_str().unwrap()).unwrap();
        assert!(rig.triangle_count() > 0 && rig.id == skin_rig("scout_4x4").unwrap().id);
    }
}
