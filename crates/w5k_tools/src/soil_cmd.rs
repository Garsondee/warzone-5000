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

// ---------------------------------------------------------------------------------------------------------------------
// The whole possibility space on the course.

use w5k_forge::explore::{Explorer, Spec, GEARS, HULLS};
use w5k_forge::raster::Image;

/// How a sampled design ended, for the chart.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum End {
    /// Finished, losing under 5 % of its time to the soil.
    Clean,
    /// Finished, but the soil cost it 5 % or more.
    Slowed,
    Bogged,
    Stalled,
    TimedOut,
    /// Never started: the design is invalid, or rail-bound.
    DidNotStart,
}

const ENDS: [(End, &str, &str); 6] = [
    (End::Clean, "finished, soil costs under 5 %", "#7ee787"),
    (End::Slowed, "finished, slowed by the soil", "#e3c35a"),
    (End::Bogged, "bogged in the soft earth", "#ff6b88"),
    (End::Stalled, "stalled (not enough engine or grip)", "#ff9f43"),
    (End::TimedOut, "timed out", "#8e97aa"),
    (End::DidNotStart, "did not start (invalid, rail-bound)", "#3a4256"),
];

fn end_of(soft: &Outcome, dry: &Outcome) -> End {
    match soft {
        Outcome::Dns { .. } => End::DidNotStart,
        Outcome::Dnf { cause, .. } => match cause {
            w5k_sim::Cause::Bogged => End::Bogged,
            w5k_sim::Cause::Stalled => End::Stalled,
            w5k_sim::Cause::TimedOut => End::TimedOut,
        },
        Outcome::Finished { ticks, .. } => match dry.time_s() {
            Some(d) if (*ticks as f64 / HZ as f64) < 1.05 * d => End::Clean,
            Some(_) => End::Slowed,
            None => End::Clean,
        },
    }
}

/// Sample `n` auto-fitted designs (every hull x every kind of gear gets the same share) and run each on the course, with the soft earth and
/// with every surface rigid. Writes `soil_space.png` (how each kind of running gear fares) and `soil_space.csv`.
pub fn run_space(forge: &Forge, course_path: &Path, terrain_path: &Path, n: usize, seed: u64, out: &Path) -> bool {
    let load = |p: &Path| std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()));
    let parsed = (|| -> Result<(CourseDef, TerrainDef), String> {
        let c: CourseDef = ron::from_str(&load(course_path)?).map_err(|e| format!("{}: {e}", course_path.display()))?;
        let t: TerrainDef = ron::from_str(&load(terrain_path)?).map_err(|e| format!("{}: {e}", terrain_path.display()))?;
        Ok((c, t))
    })();
    let (cdef, terrain) = match parsed {
        Ok(x) => x,
        Err(e) => {
            eprintln!("error: {e}");
            return false;
        }
    };
    let (soft, dry) = match (Course::bake_with(&cdef, &terrain), Course::bake(&cdef)) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("error: course: {e}");
            return false;
        }
    };
    let ex = Explorer::new(forge.lib.clone());
    let per = (n / (HULLS.len() * GEARS.len())).max(4);
    let mut samples = Vec::new();
    for (hi, hull) in HULLS.iter().enumerate() {
        for (gi, gear) in GEARS.iter().enumerate() {
            let spec = Spec { hull: Some(hull.to_string()), gear: Some(gear.to_string()), ..Default::default() };
            samples.extend(ex.sample(seed + ((hi * GEARS.len() + gi) * per) as u64, per, &spec));
        }
    }
    // Run each design on both courses, in parallel (every run is independent).
    let threads = std::thread::available_parallelism().map(|x| x.get()).unwrap_or(4).min(8);
    let mut rows: Vec<Option<(String, f64, f64, End)>> = (0..samples.len()).map(|_| None).collect();
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let (samples, soft, dry, lib) = (&samples, &soft, &dry, &forge.lib);
                s.spawn(move || {
                    let mut mine = Vec::new();
                    let mut i = t;
                    while i < samples.len() {
                        let smp = &samples[i];
                        let row = if smp.valid {
                            let mut f = Forge::new(lib.clone());
                            match f.instantiate(&smp.spec) {
                                Ok(inst) => {
                                    let (asm, sheet) = f.quick_design(&inst);
                                    let spec = f.mover_spec(&format!("seed{}", smp.seed), &asm, &sheet);
                                    let (a, b) = (run(soft, &spec), run(dry, &spec));
                                    Some((smp.gear.clone(), sheet.ground_pressure_kpa, b.outcome.time_s().unwrap_or(0.0), end_of(&a.outcome, &b.outcome)))
                                }
                                Err(_) => Some((smp.gear.clone(), 0.0, 0.0, End::DidNotStart)),
                            }
                        } else {
                            Some((smp.gear.clone(), 0.0, 0.0, End::DidNotStart))
                        };
                        mine.push((i, row));
                        i += threads;
                    }
                    mine
                })
            })
            .collect();
        for h in handles {
            for (i, r) in h.join().expect("sample thread panicked") {
                rows[i] = r;
            }
        }
    });
    let rows: Vec<(String, f64, f64, End)> = rows.into_iter().flatten().collect();

    // Count by kind of running gear.
    let gear_label = |g: &str| match g {
        "track" => "tracks",
        "wheel" => "wheels",
        "legs" => "legs",
        "rail" => "rail",
        "hover" => "air cushion",
        "antigrav" => "anti-gravity",
        _ => "rotor",
    };
    println!("{} designs sampled and run on the course, with and without the soft earth", rows.len());
    let mut table: Vec<(String, [usize; 6])> = Vec::new();
    for g in GEARS {
        let mut c = [0usize; 6];
        for (gear, _, _, e) in rows.iter().filter(|r| r.0 == g) {
            let _ = gear;
            c[ENDS.iter().position(|(k, _, _)| k == e).unwrap()] += 1;
        }
        println!("  {:<13} {}", gear_label(g), ENDS.iter().zip(c).map(|((_, name, _), n)| format!("{n} {}", name.split(',').next().unwrap())).collect::<Vec<_>>().join(" | "));
        table.push((gear_label(g).to_string(), c));
    }

    // The chart: one stacked bar per kind of running gear.
    let (w, row_h, left) = (1500usize, 44i64, 210i64);
    let h = (150 + row_h * table.len() as i64 + 150) as usize;
    let mut img = Image::new(w, h, crate::plot::BG);
    img.text(24, 18, 2, "What the soft earth does to the possibility space", [0.9, 0.91, 0.94]);
    img.text(24, 44, 1, &format!("{} auto-fitted designs, every hull with every kind of running gear, each run on the hill-and-valley course. The bars are shares of each kind.", rows.len()), [0.5, 0.52, 0.58]);
    let bar_w = w as i64 - left - 60;
    let top = 100i64;
    for (i, (name, c)) in table.iter().enumerate() {
        let y = top + row_h * i as i64;
        let total: usize = c.iter().sum::<usize>().max(1);
        img.text(24, y + 14, 1, &format!("{name} ({total})"), [0.8, 0.82, 0.86]);
        let mut x = left as f64;
        for (k, (_, _, colour)) in ENDS.iter().enumerate() {
            let wk = bar_w as f64 * c[k] as f64 / total as f64;
            if wk >= 1.0 {
                img.fill_rect(x as i64, y + 4, wk as i64, row_h - 12, hex(colour), 0.95);
                if wk > 34.0 {
                    let ink = if ENDS[k].0 == End::DidNotStart { [0.85, 0.87, 0.92] } else { [0.02, 0.02, 0.03] };
                    img.text(x as i64 + 6, y + 14, 1, &format!("{:.0}%", 100.0 * c[k] as f64 / total as f64), ink);
                }
            }
            x += wk;
        }
    }
    let ly = top + row_h * table.len() as i64 + 24;
    for (k, (_, name, colour)) in ENDS.iter().enumerate() {
        let (cx, cy) = (left + (k as i64 % 3) * 420, ly + (k as i64 / 3) * 24);
        img.fill_rect(cx, cy, 12, 12, hex(colour), 1.0);
        img.text(cx + 20, cy + 2, 1, name, [0.8, 0.82, 0.86]);
    }
    // The footnote reads the numbers rather than asserting them.
    let share = |name: &str, f: &dyn Fn(&[usize; 6]) -> usize| {
        let c = table.iter().find(|t| t.0 == name).map(|t| t.1).unwrap_or([0; 6]);
        100.0 * f(&c) as f64 / c.iter().sum::<usize>().max(1) as f64
    };
    let cross = |c: &[usize; 6]| c[0] + c[1];
    img.text(24, ly + 70, 1, &format!(
        "Tracks: {:.0}% cross the valley, {:.0}% of them paying 5% or more in time. Wheels: {:.0}% bog. Legs: {:.0}% cross, {:.0}% run out of clock (they are slow).",
        share("tracks", &cross),
        100.0 * table.iter().find(|t| t.0 == "tracks").map(|t| t.1[1] as f64 / cross(&t.1).max(1) as f64).unwrap_or(0.0),
        share("wheels", &|c| c[2]),
        share("legs", &cross),
        share("legs", &|c| c[4])
    ), [0.5, 0.52, 0.58]);
    img.text(24, ly + 86, 1, "What floats or flies does not notice the soil. The auto-fitter sizes tracks to the narrowest that carries the weight, so a design that crosses well has been tuned for the ground.", [0.5, 0.52, 0.58]);
    if std::fs::create_dir_all(out).is_err() {
        eprintln!("error: cannot create {}", out.display());
        return false;
    }
    let path = out.join("soil_space.png");
    if let Err(e) = img.save_png(&path) {
        eprintln!("error: {e}");
        return false;
    }
    let mut csv = String::from("gear,ground_pressure_kpa,dry_time_s,end\n");
    for (g, kpa, t, e) in &rows {
        csv += &format!("{g},{kpa:.1},{t:.2},{e:?}\n");
    }
    let _ = std::fs::write(out.join("soil_space.csv"), csv);
    println!("wrote {}", path.display());
    true
}
