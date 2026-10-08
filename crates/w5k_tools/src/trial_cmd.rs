//! `w5k trial`: put designs on a course, one at a time, and write everything a viewer needs.
//!
//! ```text
//! w5k trial <content> --out DIR [--course content/courses/hill_valley.ron] [--terrain content/terrain.ron] [--only a,b] [--limit 180] [--mode sim|parade]
//! ```
//!
//! `sim` (the default) runs the physics of `w5k_sim::mover` on a spec baked from each design (`w5k_forge::mover`); `parade`
//! drives every vehicle at its sheet speed, with no physics, as a reference lap.
//!
//! Output (in DIR): `scene.json` (the course and every vehicle's mesh), `replay.json` (one recorded run per vehicle),
//! `results.json` / `results.md` (who finished, how fast, or why not) and `course.png` (the elevation profile). The page is
//! built from the first two by `tools/trial/build.py`.

use std::path::Path;
use std::time::Instant;

use serde_json::{json, Value};
use w5k_forge::export::{export_joints, export_mesh, ExportJoint};
use w5k_forge::preview::palette_colours;
use w5k_forge::raster::Rgb;
use w5k_forge::Forge;
use w5k_sim::trial::{parade, run as simulate, ParadeSpec};
use w5k_sim::{Course, CourseDef, MoverSpec, Outcome, Run, TerrainDef, HZ};

use crate::plot::{Axis, Plot};
use crate::trial_plots::{results_png, traces_png, Lane};

/// Standard base64 (RFC 4648, with padding).
pub fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

/// Everything about one vehicle's trial.
struct Entry {
    id: String,
    name: String,
    class: String,
    movement: String,
    mass_kg: f64,
    power_kw: f64,
    top_kmh: f64,
    ground_kpa: f64,
    size: [f64; 3],
    problems: Vec<String>,
    palette: [Rgb; 8],
    mesh: w5k_forge::export::ExportMesh,
    joints: Vec<ExportJoint>,
    spec: MoverSpec,
    run: Run,
    /// Seconds the same vehicle takes on the same course with every surface rigid (to show what the soil cost it); `None` when it
    /// does not finish even then.
    dry_s: Option<f64>,
}

fn entry(forge: &Forge, course: &Course, dry_course: &Course, id: &str, mode: &str) -> Result<Entry, String> {
    let design = forge.designs.get(id).ok_or_else(|| format!("no design '{id}'"))?;
    let (built, asm, sheet) = forge.build_design(design);
    let spec = forge.mover_spec(id, &asm, &sheet);
    let class = if sheet.locomotion.is_empty() { "none".to_string() } else { spec.class.name().to_string() };
    let run = match mode {
        "sim" => simulate(course, &spec),
        "parade" => {
            let spec = ParadeSpec { rated_ms: sheet.top_speed_kmh / 3.6, span_m: (0.7 * sheet.length_m).clamp(1.0, 20.0), accel_ms2: 4.0 };
            parade(course, id, &spec)
        }
        other => return Err(format!("unknown mode '{other}' (sim or parade)")),
    };
    let dry_s = if mode == "sim" && course.has_soft_ground() { simulate(dry_course, &spec).outcome.time_s() } else { None };
    Ok(Entry {
        id: id.to_string(),
        name: design.name.clone(),
        class,
        movement: sheet.movement.clone(),
        mass_kg: sheet.mass_kg,
        power_kw: sheet.power_kw,
        top_kmh: sheet.top_speed_kmh,
        ground_kpa: sheet.ground_pressure_kpa,
        size: [sheet.width_m, sheet.height_m, sheet.length_m],
        problems: sheet.problems.clone(),
        palette: palette_colours(&forge.lib, design.palette.as_deref()),
        mesh: export_mesh(&built.mesh),
        joints: export_joints(&asm),
        spec,
        run,
        dry_s,
    })
}

fn outcome_json(o: &Outcome) -> Value {
    match o {
        Outcome::Finished { ticks, splits } => json!({"kind": "finished", "ticks": ticks, "time_s": *ticks as f64 / HZ as f64, "splits": splits}),
        Outcome::Dnf { cause, s_mm, ticks, detail } => json!({"kind": "dnf", "cause": format!("{cause:?}"), "s_m": *s_mm as f64 / 1000.0, "ticks": ticks, "detail": detail}),
        Outcome::Dns { cause } => json!({"kind": "dns", "cause": cause}),
    }
}

fn outcome_text(o: &Outcome) -> String {
    match o {
        Outcome::Finished { ticks, .. } => format!("finished in {:.1} s", *ticks as f64 / HZ as f64),
        Outcome::Dnf { cause, s_mm, detail, .. } => format!("DNF at {:.0} m: {cause:?} ({detail})", *s_mm as f64 / 1000.0),
        Outcome::Dns { cause } => format!("DNS: {cause}"),
    }
}

fn plain(x: f64) -> String {
    format!("{x:.0}")
}

fn rgb(c: [f32; 3]) -> Rgb {
    c
}

/// The elevation profile of the course as a picture (vertical scale exaggerated, which is the point: the hill is small).
fn course_png(course: &Course, path: &Path) {
    let (heights, surf) = course.profile(0.5);
    let s0 = course.s_min().to_f64();
    let ds = 0.5;
    let (hmin, hmax) = heights.iter().fold((f64::MAX, f64::MIN), |(a, b), &h| (a.min(h), b.max(h)));
    let (ylo, yhi) = ((hmin - 2.5).floor(), (hmax + 3.0).ceil());
    let (xlo, xhi) = (s0 - 5.0, course.s_max().to_f64() + 5.0);
    // The plot panel is 1134 x 492 px: how much taller than true scale the picture draws the hill.
    let exaggeration = ((492.0 / (yhi - ylo)) / (1134.0 / (xhi - xlo))).round();
    let mut p = Plot::new(
        1500,
        640,
        &format!("{}: elevation profile", course.name),
        &format!("Height above the start pad against distance from the start line. The vertical scale is exaggerated about {exaggeration:.0} times: the hill is small."),
        Axis::lin(xlo, xhi, "distance from the start line (m)", plain),
        Axis::lin(ylo, yhi, "height (m)", plain),
    );
    let colour = |id: u8| -> Rgb {
        match course.surface_names()[id as usize].as_str() {
            "concrete" => rgb([0.42, 0.46, 0.55]),
            "soft_earth" => rgb([0.62, 0.38, 0.17]),
            _ => rgb([0.5, 0.5, 0.5]),
        }
    };
    for (i, &h) in heights.iter().enumerate() {
        let s = s0 + i as f64 * ds;
        p.seg(s, ylo, s, h, colour(surf[i]), 0.55);
    }
    for i in 1..heights.len() {
        let (a, b) = (s0 + (i - 1) as f64 * ds, s0 + i as f64 * ds);
        for off in [-0.06, 0.0, 0.06] {
            p.seg(a, heights[i - 1] + off, b, heights[i] + off, [0.95, 0.96, 1.0], 0.9);
        }
    }
    let names = ["start", "valley entry", "valley exit", "finish"];
    for (k, g) in course.checkpoints().iter().enumerate() {
        let s = g.to_f64();
        p.seg(s, ylo, s, yhi, [0.35, 0.85, 1.0], 0.5);
        let label = if course.checkpoints().len() == names.len() { names[k].to_string() } else { format!("gate {k}") };
        p.label(s + 1.5, yhi - 0.9, &label, [0.6, 0.9, 1.0]);
    }
    p.legend(&[("concrete".to_string(), colour(course.surface_id("concrete").unwrap_or(0))), ("soft earth".to_string(), colour(course.surface_id("soft_earth").unwrap_or(0)))]);
    p.notes(&["Heights are relative to the start pad.", "Every change of grade is eased over a 12 m vertical curve."]);
    if let Err(e) = p.img.save_png(path) {
        eprintln!("error: {e}");
    }
}

pub fn run(forge: &Forge, course_path: &Path, terrain_path: &Path, only: Option<&[String]>, limit_s: Option<f64>, mode: &str, out: &Path) -> bool {
    let t0 = Instant::now();
    let text = match std::fs::read_to_string(course_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {}: {e}", course_path.display());
            return false;
        }
    };
    let mut def: CourseDef = match ron::from_str(&text) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {}: {e}", course_path.display());
            return false;
        }
    };
    if let Some(l) = limit_s {
        def.time_limit_s = l;
    }
    let terrain: TerrainDef = match std::fs::read_to_string(terrain_path).map_err(|e| e.to_string()).and_then(|t| ron::from_str(&t).map_err(|e| e.to_string())) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {}: {e}", terrain_path.display());
            return false;
        }
    };
    let course = match Course::bake_with(&def, &terrain) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: course: {e}");
            return false;
        }
    };
    // The same course with every surface rigid, for the reference run.
    let dry_course = Course::bake(&def).expect("a course that bakes with a terrain table bakes without one");
    if std::fs::create_dir_all(out).is_err() {
        eprintln!("error: cannot create {}", out.display());
        return false;
    }

    let ids: Vec<String> = only.map(|o| o.to_vec()).unwrap_or_else(|| forge.designs.keys().cloned().collect());
    let threads = std::thread::available_parallelism().map(|x| x.get()).unwrap_or(4).min(8);
    let mut slots: Vec<Option<Result<Entry, String>>> = (0..ids.len()).map(|_| None).collect();
    std::thread::scope(|sc| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let (ids, course, dry_course) = (&ids, &course, &dry_course);
                sc.spawn(move || {
                    let mut mine = Vec::new();
                    let mut i = t;
                    while i < ids.len() {
                        mine.push((i, entry(forge, course, dry_course, &ids[i], mode)));
                        i += threads;
                    }
                    mine
                })
            })
            .collect();
        for h in handles {
            for (i, e) in h.join().expect("trial thread panicked") {
                slots[i] = Some(e);
            }
        }
    });
    let mut entries: Vec<Entry> = Vec::new();
    for (i, s) in slots.into_iter().enumerate() {
        match s.expect("every slot is filled") {
            Ok(e) => entries.push(e),
            Err(msg) => eprintln!("skipping {}: {msg}", ids[i]),
        }
    }
    if entries.is_empty() {
        eprintln!("error: nothing to run");
        return false;
    }

    // Scene: the course and every vehicle's mesh.
    let (heights, surf) = course.profile(1.0);
    let scene = json!({
        "course": {
            "name": course.name,
            "s0": course.s_min().to_f64(),
            "ds": 1.0,
            "heights": heights.iter().map(|h| (h * 1000.0).round() / 1000.0).collect::<Vec<_>>(),
            "surf": surf,
            "surface_names": course.surface_names(),
            "checkpoints": course.checkpoints().iter().map(|c| c.to_f64()).collect::<Vec<_>>(),
            "finish": course.finish().to_f64(),
            "time_limit_s": course.time_limit_ticks() as f64 / HZ as f64,
        },
        "vehicles": entries.iter().map(|e| json!({
            "id": e.id, "name": e.name, "class": e.class, "movement": e.movement,
            "mass_kg": e.mass_kg, "power_kw": e.power_kw, "top_kmh": e.top_kmh, "ground_kpa": e.ground_kpa,
            "width_m": e.size[0], "height_m": e.size[1], "length_m": e.size[2], "problems": e.problems,
            "spec": e.spec, "dry_s": e.dry_s, "joints": e.joints,
            "palette": e.palette.iter().map(|c| json!([c[0], c[1], c[2]])).collect::<Vec<_>>(),
            "mesh": {
                "lo": e.mesh.lo, "hi": e.mesh.hi, "ground_y": e.mesh.ground_y,
                "n_vertices": e.mesh.n_vertices, "n_indices": e.mesh.n_indices, "wide": e.mesh.wide_indices,
                "b64": b64(&e.mesh.blob),
            },
        })).collect::<Vec<_>>(),
    });
    let replay = json!({
        "hz": HZ,
        "runs": entries.iter().map(|e| json!({
            "id": e.id,
            "outcome": outcome_json(&e.run.outcome),
            "frames": e.run.frames.len(),
            "frames_b64": b64(&e.run.frame_bytes()),
            "hash": format!("{:016x}", e.run.final_hash),
            "checks": e.run.checkpoint_hashes.iter().map(|h| format!("{h:016x}")).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    });
    let results = json!({
        "course": course.name,
        "mode": mode,
        "results": entries.iter().map(|e| json!({"id": e.id, "name": e.name, "class": e.class, "outcome": outcome_json(&e.run.outcome), "hash": format!("{:016x}", e.run.final_hash)})).collect::<Vec<_>>(),
    });

    let write = |name: &str, v: &Value| -> bool {
        let path = out.join(name);
        match std::fs::write(&path, serde_json::to_vec(v).expect("json")) {
            Ok(()) => {
                println!("wrote {} ({:.0} KB)", path.display(), std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) as f64 / 1024.0);
                true
            }
            Err(e) => {
                eprintln!("error: {}: {e}", path.display());
                false
            }
        }
    };
    let ok = write("scene.json", &scene) & write("replay.json", &replay) & write("results.json", &results);

    // A readable table, fastest first, then the ones that did not finish.
    let mut order: Vec<&Entry> = entries.iter().collect();
    order.sort_by(|a, b| {
        let key = |e: &Entry| match &e.run.outcome {
            Outcome::Finished { ticks, .. } => (0, *ticks as i64),
            Outcome::Dnf { s_mm, .. } => (1, -(*s_mm as i64)),
            Outcome::Dns { .. } => (2, 0),
        };
        key(a).cmp(&key(b)).then_with(|| a.id.cmp(&b.id))
    });
    let mut md = format!("# {} ({mode})\n\n| Vehicle | Running gear | Mass | Top speed (sheet) | Result |\n|---|---|---|---|---|\n", course.name);
    for e in &order {
        let mass = if e.mass_kg >= 1000.0 { format!("{:.1} t", e.mass_kg / 1000.0) } else { format!("{:.0} kg", e.mass_kg) };
        md += &format!("| {} | {} | {} | {:.0} km/h | {} |\n", e.name, e.class, mass, e.top_kmh, outcome_text(&e.run.outcome));
    }
    let _ = std::fs::write(out.join("results.md"), md);
    course_png(&course, &out.join("course.png"));
    let lanes: Vec<Lane> = order.iter().map(|e| Lane { name: &e.name, class: &e.class, run: &e.run, dry_s: e.dry_s }).collect();
    traces_png(&lanes, &course, &out.join("traces.png"));
    results_png(&lanes, &course, &out.join("results.png"));
    println!("wrote course.png, traces.png and results.png in {}", out.display());
    for e in &order {
        println!("{:<22} {:<10} {}", e.id, e.class, outcome_text(&e.run.outcome));
    }
    println!("{} vehicles in {:.1}s", entries.len(), t0.elapsed().as_secs_f64());
    ok
}

#[cfg(test)]
mod tests {
    use super::b64;

    #[test]
    fn base64_matches_the_rfc_vectors() {
        assert_eq!(b64(b""), "");
        assert_eq!(b64(b"f"), "Zg==");
        assert_eq!(b64(b"fo"), "Zm8=");
        assert_eq!(b64(b"foo"), "Zm9v");
        assert_eq!(b64(b"foob"), "Zm9vYg==");
        assert_eq!(b64(b"fooba"), "Zm9vYmE=");
        assert_eq!(b64(b"foobar"), "Zm9vYmFy");
    }
}
