//! `w5k soil`: ladders that show how weight and footprint decide the valley.
//!
//! ```text
//! w5k soil <content> --design lancer_mk1 --out DIR [--course file] [--terrain file]
//! ```
//!
//! Takes one family-level design (`content/vehicles/<id>.ron`) and varies **one thing at a time**, leaving the running gear and the
//! engine as they are: its **armour** (so its weight, and with the same tracks its ground pressure, rises), and its **track
//! width** (so the same weight is spread over more or less ground). Every variant runs the whole trial, and the chart plots its
//! time against its ground pressure: where the valley starts to cost time, and where the vehicle stops finishing.

use std::path::Path;

use w5k_forge::raster::Rgb;
use w5k_forge::schema::DesignDef;
use w5k_forge::Forge;
use w5k_sim::trial::run;
use w5k_sim::{Course, CourseDef, MoverSpec, Outcome, TerrainDef, HZ};

use crate::plot::{hex, Axis, Plot};

/// One variant of the base design and how it fared.
struct Rung {
    label: String,
    mass_t: f64,
    kpa: f64,
    track_w: f64,
    sink_m: f64,
    outcome: Outcome,
    note: String,
}

/// The design with its **hull plating** scaled (the turret keeps its armour: a thicker turret ring no longer fits its socket).
fn scaled_armour(base: &DesignDef, k: f64) -> DesignDef {
    let mut d = base.clone();
    for (key, v) in d.hull_params.iter_mut() {
        if key.ends_with("_mm") {
            *v *= k;
        }
    }
    d
}

fn with_gear_width(base: &DesignDef, width: f64) -> DesignDef {
    let mut d = base.clone();
    for a in &mut d.attach {
        if matches!(a.family.as_deref(), Some("track" | "wheel")) {
            a.params.insert("width_m".into(), width);
        }
    }
    d
}

fn gear_width(base: &DesignDef) -> Option<f64> {
    base.attach.iter().find(|a| matches!(a.family.as_deref(), Some("track" | "wheel"))).and_then(|a| a.params.get("width_m").copied())
}

fn evaluate(forge_lib: &w5k_forge::schema::MaterialLibrary, design: &DesignDef, course: &Course, label: String) -> Result<Rung, String> {
    let mut forge = Forge::new(forge_lib.clone());
    let inst = forge.instantiate(design)?;
    let (asm, sheet) = forge.quick_design(&inst);
    let spec: MoverSpec = forge.mover_spec(&design.id, &asm, &sheet);
    let r = run(course, &spec);
    let sink_m = r.frames.iter().map(|f| f.sink_mm as f64).fold(0.0, f64::max) / 1000.0;
    let note = match &r.outcome {
        Outcome::Dns { cause } => cause.clone(),
        Outcome::Dnf { detail, .. } => detail.clone(),
        Outcome::Finished { .. } => String::new(),
    };
    Ok(Rung { label, mass_t: sheet.mass_kg / 1000.0, kpa: sheet.ground_pressure_kpa, track_w: spec.contact_width_m, sink_m, outcome: r.outcome, note })
}

fn series(name: &str, rungs: &[Rung], c: Rgb, p: &mut Plot, ytop: f64) {
    let pts: Vec<&Rung> = rungs.iter().filter(|r| r.kpa > 0.0).collect();
    for w in pts.windows(2) {
        let y = |r: &Rung| r.outcome.time_s().unwrap_or(ytop * 0.96);
        if r_finished(w[0]) && r_finished(w[1]) {
            p.seg(w[0].kpa, y(w[0]), w[1].kpa, y(w[1]), c, 0.8);
            p.seg(w[0].kpa, y(w[0]) + 0.3, w[1].kpa, y(w[1]) + 0.3, c, 0.8);
        }
    }
    for r in &pts {
        match &r.outcome {
            Outcome::Finished { ticks, .. } => p.dot(r.kpa, *ticks as f64 / HZ as f64, 4.5, c, 1.0),
            Outcome::Dnf { .. } => {
                p.dot(r.kpa, ytop * 0.96, 5.5, [0.95, 0.2, 0.3], 1.0);
                p.dot(r.kpa, ytop * 0.96, 3.0, c, 1.0);
            }
            Outcome::Dns { .. } => {}
        }
    }
    let _ = name;
}

fn r_finished(r: &Rung) -> bool {
    matches!(r.outcome, Outcome::Finished { .. })
}

fn print_table(title: &str, rungs: &[Rung]) {
    println!("{title}");
    for r in rungs {
        let res = match &r.outcome {
            Outcome::Finished { ticks, .. } => format!("{:6.1} s", *ticks as f64 / HZ as f64),
            Outcome::Dnf { cause, s_mm, .. } => format!("DNF {:3.0} m {cause:?}", *s_mm as f64 / 1000.0),
            Outcome::Dns { .. } => "DNS".into(),
        };
        println!("  {:<14} {:6.1} t  {:6.1} kPa  track {:.2} m  sunk {:.2} m  {res}", r.label, r.mass_t, r.kpa, r.track_w, r.sink_m);
    }
}

pub fn run_ladders(forge: &Forge, content: &Path, design_id: &str, course_path: &Path, terrain_path: &Path, out: &Path) -> bool {
    let load = |p: &Path| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
    let parsed = (|| -> Result<(DesignDef, CourseDef, TerrainDef), String> {
        let d: DesignDef = ron::from_str(&load(&content.join("vehicles").join(format!("{design_id}.ron")))?).map_err(|e| format!("design {design_id}: {e}"))?;
        let c: CourseDef = ron::from_str(&load(course_path)?).map_err(|e| format!("{}: {e}", course_path.display()))?;
        let t: TerrainDef = ron::from_str(&load(terrain_path)?).map_err(|e| format!("{}: {e}", terrain_path.display()))?;
        Ok((d, c, t))
    })();
    let (base, cdef, terrain) = match parsed {
        Ok(x) => x,
        Err(e) => {
            eprintln!("error: {e}");
            return false;
        }
    };
    let course = match Course::bake_with(&cdef, &terrain) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: course: {e}");
            return false;
        }
    };
    let Some(w0) = gear_width(&base) else {
        eprintln!("error: {design_id} has no track or wheel family with a width_m to vary");
        return false;
    };
    let lib = &forge.lib;
    let mut by_armour = Vec::new();
    for k in [0.2, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 5.0] {
        match evaluate(lib, &scaled_armour(&base, k), &course, format!("hull plate x{k:.1}")) {
            Ok(r) => by_armour.push(r),
            Err(e) => eprintln!("armour x{k}: {e}"),
        }
    }
    let mut by_width = Vec::new();
    for k in [0.4, 0.55, 0.7, 0.85, 1.0, 1.25, 1.5, 2.0, 2.5, 3.0] {
        match evaluate(lib, &with_gear_width(&base, w0 * k), &course, format!("width {:.2} m", w0 * k)) {
            Ok(r) => by_width.push(r),
            Err(e) => eprintln!("width x{k}: {e}"),
        }
    }
    print_table(&format!("{design_id}: hull-plating ladder (tracks {w0:.2} m wide, engine unchanged)"), &by_armour);
    print_table(&format!("{design_id}: track-width ladder (armour unchanged)"), &by_width);

    let pmax = by_armour.iter().chain(&by_width).map(|r| r.kpa).fold(60.0f64, f64::max);
    let xhi = ((pmax * 1.1) / 20.0).ceil() * 20.0;
    let slowest = by_armour.iter().chain(&by_width).filter_map(|r| r.outcome.time_s()).fold(30.0f64, f64::max);
    let ytop = ((slowest * 1.25) / 10.0).ceil() * 10.0;
    let mut p = Plot::new(
        1500,
        760,
        &format!("{}: weight and footprint decide the valley", base.name),
        "Time over the course against ground pressure, for variants of one design with the same engine. Red rings did not finish; the number is where they stopped.",
        Axis::lin(0.0, xhi, "ground pressure (kPa)", |v| format!("{v:.0}")),
        Axis::lin(0.0, ytop, "time to finish (s)", |v| format!("{v:.0}")),
    );
    let (c_armour, c_width) = (hex("#ffb454"), hex("#38d6ff"));
    p.seg(100.0, 0.0, 100.0, ytop, [0.4, 0.2, 0.25], 0.6);
    p.label(101.0, ytop * 0.5, "the sheet warns", [0.8, 0.45, 0.5]);
    p.label(101.0, ytop * 0.5 - 4.0 * ytop / 70.0, "from here", [0.8, 0.45, 0.5]);
    series("armour", &by_armour, c_armour, &mut p, ytop);
    series("width", &by_width, c_width, &mut p, ytop);
    // Label a few rungs of each ladder (all of them overlap), and give each bogged one the distance it stopped at.
    for (rungs, c, keep, above) in [(&by_armour, c_armour, vec![0usize, 2, 3, 4], true), (&by_width, c_width, vec![0, 2, 4, 7, 9], false)] {
        let rungs: Vec<&Rung> = rungs.iter().filter(|r| r.kpa > 0.0).collect();
        for (i, r) in rungs.iter().enumerate() {
            let (px, py) = match &r.outcome {
                Outcome::Dnf { s_mm, .. } => {
                    let (x, y) = (p.x(r.kpa), p.y(ytop * 0.96));
                    p.text_px(x - 16.0, y + 12.0, &format!("{:.0} m", *s_mm as f64 / 1000.0), c);
                    continue;
                }
                _ => (p.x(r.kpa) + 8.0, p.y(r.outcome.time_s().unwrap_or(0.0)) + if above { -16.0 } else { 10.0 }),
            };
            if keep.contains(&i) {
                p.text_px(px, py, &r.label, c);
            }
        }
    }
    p.legend(&[("heavier hull plating".to_string(), c_armour), ("wider tracks".to_string(), c_width)]);
    p.notes(&[
        "Sinkage is the ground pressure over the soil's stiffness; the soil costs the vehicle compaction resistance, and drags the hull once it has sunk to the ground clearance.",
        "Wider tracks lower the pressure but weigh more, so the quickest are in the middle.",
        "Heavier plating raises the pressure and also takes power-to-weight away from the same engine: the climb is where it bogs first.",
    ]);
    if std::fs::create_dir_all(out).is_err() {
        eprintln!("error: cannot create {}", out.display());
        return false;
    }
    let path = out.join(format!("soil_ladders_{design_id}.png"));
    if let Err(e) = p.img.save_png(&path) {
        eprintln!("error: {e}");
        return false;
    }
    let mut csv = String::from("ladder,label,mass_t,pressure_kpa,track_width_m,sink_m,outcome,time_s,note\n");
    for (name, rungs) in [("armour", &by_armour), ("width", &by_width)] {
        for r in rungs {
            let (o, t) = match &r.outcome {
                Outcome::Finished { ticks, .. } => ("finished".to_string(), format!("{:.2}", *ticks as f64 / HZ as f64)),
                Outcome::Dnf { cause, .. } => (format!("{cause:?}"), String::new()),
                Outcome::Dns { .. } => ("dns".to_string(), String::new()),
            };
            csv += &format!("{name},{},{:.3},{:.1},{:.3},{:.3},{o},{t},\"{}\"\n", r.label, r.mass_t, r.kpa, r.track_w, r.sink_m, r.note.replace('"', "'"));
        }
    }
    let _ = std::fs::write(out.join(format!("soil_ladders_{design_id}.csv")), csv);
    println!("wrote {}", path.display());
    true
}
