//! Commands that mix and match components: `roll`, `atlas`, `rates`, `fit` and `ladder`.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use w5k_forge::explore::{describe, Explorer, Sample, Spec, GEARS, HULLS, PALETTES};
use w5k_forge::mesh::Mesh;
use w5k_forge::preview::{self, Frame};
use w5k_forge::raster::{Image, Rgb};
use w5k_forge::schema::{DesignDef, MaterialLibrary};

const TEXT: Rgb = [0.78, 0.80, 0.84];
const DIM: Rgb = [0.5, 0.52, 0.56];
const WARN: Rgb = [0.95, 0.35, 0.25];
const OK: Rgb = [0.4, 0.85, 0.55];
const BG: Rgb = [0.018, 0.019, 0.022];

pub fn fmt(x: f64) -> String {
    if x >= 100.0 {
        format!("{x:.0}")
    } else if x >= 10.0 {
        format!("{x:.1}")
    } else {
        format!("{x:.2}")
    }
}

/// Mass in the unit that reads best.
pub fn mass_str(kg: f64) -> String {
    if kg >= 10_000.0 {
        format!("{} t", fmt(kg / 1000.0))
    } else if kg >= 1000.0 {
        format!("{:.2} t", kg / 1000.0)
    } else {
        format!("{kg:.0} kg")
    }
}

pub struct Tile {
    pub mesh: Mesh,
    pub palette: Option<String>,
    pub title: String,
    pub line1: String,
    pub line2: String,
    pub problem: Option<String>,
}

/// One tile: a render of the design framed on itself, with a title, two stat lines and the first problem.
pub fn tile(lib: &MaterialLibrary, d: &DesignDef) -> Tile {
    let ex = Explorer::new(lib.clone());
    match ex.build(d) {
        Ok((built, s, _)) => Tile {
            mesh: built.mesh,
            palette: d.palette.clone(),
            title: if d.name.is_empty() { describe(d) } else { d.name.clone() },
            line1: format!("{}  {:.0} km/h  {} m", mass_str(s.mass_kg), s.top_speed_kmh, fmt(s.length_m.max(s.width_m))),
            line2: format!("arm {:.0}/{:.0} fire {}kW see {:.1}km", s.armour_front_mm, s.armour_side_mm, fmt(s.firepower_kw), s.sight_km),
            problem: s.problems.first().cloned(),
        },
        Err(e) => Tile { mesh: Mesh::default(), palette: None, title: describe(d), line1: String::new(), line2: String::new(), problem: Some(e) },
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_tile(img: &mut Image, lib: &MaterialLibrary, t: &Tile, x: usize, y: usize, size: usize, az: f64, el: f64) {
    if !t.mesh.positions.is_empty() {
        let fr = Frame::of(&t.mesh);
        let colours = preview::palette_colours(lib, t.palette.as_deref());
        img.blit(&preview::render_view(&t.mesh, &colours, &fr, az, el, size), x, y);
    }
    let cw = (size / 8).saturating_sub(1);
    let clip = |s: &str| -> String { s.chars().take(cw).collect() };
    img.text(x as i64 + 5, (y + size + 5) as i64, 1, &clip(&t.title), TEXT);
    img.text(x as i64 + 5, (y + size + 17) as i64, 1, &clip(&t.line1), DIM);
    img.text(x as i64 + 5, (y + size + 28) as i64, 1, &clip(&t.line2), DIM);
    match &t.problem {
        Some(p) => {
            img.text(x as i64 + 5, (y + size + 40) as i64, 1, &clip(p), WARN);
        }
        None => {
            img.text(x as i64 + 5, (y + size + 40) as i64, 1, "valid", OK);
        }
    }
}

pub const LABEL_H: usize = 54;

/// Build tiles for many designs in parallel (full builds are the slow part).
pub fn tiles(lib: &MaterialLibrary, designs: &[DesignDef]) -> Vec<Tile> {
    let threads = std::thread::available_parallelism().map(|x| x.get()).unwrap_or(4).min(16);
    let mut out: Vec<Option<Tile>> = (0..designs.len()).map(|_| None).collect();
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                s.spawn(move || {
                    let mut mine = Vec::new();
                    let mut i = t;
                    while i < designs.len() {
                        mine.push((i, tile(lib, &designs[i])));
                        i += threads;
                    }
                    mine
                })
            })
            .collect();
        for h in handles {
            for (i, tl) in h.join().expect("tile thread panicked") {
                out[i] = Some(tl);
            }
        }
    });
    out.into_iter().flatten().collect()
}

fn grid(lib: &MaterialLibrary, tl: &[Tile], cols: usize, size: usize, title: &str, az: f64, el: f64) -> Image {
    let rows = tl.len().div_ceil(cols);
    let hdr = 36;
    let mut img = Image::new(cols * size, hdr + rows * (size + LABEL_H), BG);
    img.text(10, 10, 2, title, [0.85, 0.86, 0.88]);
    for (i, t) in tl.iter().enumerate() {
        draw_tile(&mut img, lib, t, (i % cols) * size, hdr + (i / cols) * (size + LABEL_H), size, az, el);
    }
    img
}

fn save(img: &Image, path: &Path) {
    match img.save_png(path) {
        Ok(()) => println!("wrote {}", path.display()),
        Err(e) => eprintln!("error: {e}"),
    }
}

/// `w5k roll --seed S --count N --out dir`: a gallery of valid random designs.
pub fn roll(lib: &MaterialLibrary, out: &Path, seed: u64, count: usize, cols: usize, spec: &Spec) {
    let t0 = Instant::now();
    let ex = Explorer::new(lib.clone());
    // Roll more than asked, keep the valid ones.
    let all = ex.sample(seed, count * 3, spec);
    let valid: Vec<&Sample> = all.iter().filter(|s| s.valid).take(count).collect();
    println!("rolled {} designs, {} valid, in {:.1}s", all.len(), all.iter().filter(|s| s.valid).count(), t0.elapsed().as_secs_f64());
    let designs: Vec<DesignDef> = valid
        .iter()
        .map(|s| {
            let mut d = s.spec.clone();
            d.name = describe(&d);
            d
        })
        .collect();
    let tl = tiles(lib, &designs);
    let img = grid(lib, &tl, cols, 300, &format!("Random designs (seed {seed}): hull / running gear / weapon, every one auto-fitted"), 35.0, 22.0);
    save(&img, &out.join(format!("roll_{seed}.png")));
}

/// `w5k rates --count N`: how often each hull x gear combination fits into a valid vehicle.
pub fn rates(lib: &MaterialLibrary, seed: u64, per: usize) {
    let ex = Explorer::new(lib.clone());
    let t0 = Instant::now();
    println!("{:<18}{}", "valid / tried", GEARS.iter().map(|g| format!("{g:>10}")).collect::<String>());
    let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
    for hull in HULLS {
        let mut line = format!("{hull:<18}");
        for gear in GEARS {
            let spec = Spec { hull: Some(hull.into()), gear: Some(gear.into()), ..Default::default() };
            let s = ex.sample(seed, per, &spec);
            let socketless = s.iter().filter(|x| x.gear == "none").count();
            let ok = s.iter().filter(|x| x.valid).count();
            for x in s.iter().filter(|x| !x.valid && x.gear != "none") {
                let why = x.error.clone().or_else(|| x.sheet.problems.first().cloned()).unwrap_or_default();
                let key: String = why.split(|c: char| c.is_ascii_digit()).next().unwrap_or("").chars().take(48).collect();
                *reasons.entry(key).or_default() += 1;
            }
            if socketless == s.len() {
                line += &format!("{:>10}", "-");
            } else {
                line += &format!("{:>10}", format!("{ok}/{}", s.len()));
            }
        }
        println!("{line}");
    }
    println!("elapsed {:.1}s\nwhy designs fail:", t0.elapsed().as_secs_f64());
    let mut v: Vec<_> = reasons.into_iter().collect();
    v.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for (k, n) in v.iter().take(14) {
        println!("  {n:>5}  {k}");
    }
}


/// `w5k why --hull H --gear G --count N`: print the problems of the designs that fail (for debugging the fitter).
pub fn why(lib: &MaterialLibrary, seed: u64, count: usize, spec: &Spec) {
    let ex = Explorer::new(lib.clone());
    for s in ex.sample(seed, count, spec) {
        let sh = &s.sheet;
        println!(
            "seed {:>4} {:<30} {:>9} {:>6.0} km/h  power {:>7.0}/{:<7.0} kW lift {:>7.0}  load {:>8.0}  {}",
            s.seed,
            describe(&s.spec),
            mass_str(sh.mass_kg),
            sh.top_speed_kmh,
            sh.power_kw,
            sh.draw_kw,
            sh.lift_kw,
            sh.load_kg,
            if s.valid { "ok".to_string() } else { s.error.clone().unwrap_or_else(|| sh.problems.join("; ")) }
        );
    }
}

/// `w5k atlas`: every hull with every kind of running gear, each one auto-fitted (or the reason it cannot be).
pub fn atlas(lib: &MaterialLibrary, out: &Path, seed: u64) {
    let t0 = Instant::now();
    let ex = Explorer::new(lib.clone());
    let size = 280usize;
    let mut designs: Vec<Option<DesignDef>> = Vec::new();
    let mut why: Vec<String> = Vec::new();
    for (r, hull) in HULLS.iter().enumerate() {
        for gear in GEARS {
            let sz = match gear {
                "rotor" => 0.3,
                "hover" => 0.4,
                _ => 0.5,
            };
            let spec = Spec {
                hull: Some(hull.to_string()),
                gear: Some(gear.to_string()),
                size: Some(sz),
                weapon: Some("turret_gun".into()),
                turrets: Some(if *hull == "hull_dreadnought" { 2 } else { 1 }),
                palette: Some(PALETTES[r % 4].to_string()),
            };
            let tries = ex.sample(seed, 16, &spec);
            let socketless = tries.iter().all(|t| t.gear == "none" || t.error.is_some());
            match tries.into_iter().find(|t| t.valid) {
                Some(t) => {
                    let mut d = t.spec;
                    d.name = describe(&d);
                    designs.push(Some(d));
                    why.push(String::new());
                }
                None => {
                    designs.push(None);
                    why.push(if socketless { "no mount for this gear".into() } else { "no valid design in 16 tries".into() });
                }
            }
        }
    }
    let present: Vec<DesignDef> = designs.iter().flatten().cloned().collect();
    let mut tl = tiles(lib, &present).into_iter();
    let cols = GEARS.len();
    let rows = HULLS.len();
    let hdr = 36;
    let mut img = Image::new(cols * size, hdr + rows * (size + LABEL_H), BG);
    img.text(10, 10, 2, "Possibility atlas: every hull with every kind of running gear, auto-fitted (gear and engine sized by the physics)", [0.85, 0.86, 0.88]);
    for (i, d) in designs.iter().enumerate() {
        let (x, y) = ((i % cols) * size, hdr + (i / cols) * (size + LABEL_H));
        match d {
            Some(_) => {
                let t = tl.next().unwrap();
                draw_tile(&mut img, lib, &t, x, y, size, 35.0, 22.0);
            }
            None => {
                img.fill_rect(x as i64 + 6, y as i64 + 6, size as i64 - 12, size as i64 - 12, [0.006, 0.006, 0.008], 1.0);
                img.text(x as i64 + 16, (y + size / 2) as i64 - 10, 1, &format!("{} / {}", HULLS[i / cols].trim_start_matches("hull_"), GEARS[i % cols]), DIM);
                img.text(x as i64 + 16, (y + size / 2) as i64 + 6, 1, &why[i], WARN);
            }
        }
    }
    save(&img, &out.join("atlas.png"));
    println!("atlas: {} of {} combinations built, {:.1}s", present.len(), designs.len(), t0.elapsed().as_secs_f64());
}

/// `w5k ladder`: one archetype at several sizes, side by side at true scale, with the fitted running gear.
#[allow(clippy::too_many_arguments)]
pub fn ladder(lib: &MaterialLibrary, out: &Path, seed: u64, hull: &str, gear: &str, steps: usize, weapon: &str) {
    let ex = Explorer::new(lib.clone());
    let mut designs = Vec::new();
    for k in 0..steps {
        let sz = 0.08 + 0.84 * k as f64 / (steps.max(2) - 1) as f64;
        let spec = Spec {
            hull: Some(hull.into()),
            gear: Some(gear.into()),
            size: Some(sz),
            weapon: Some(weapon.into()),
            turrets: Some(1),
            palette: Some("vanguard".into()),
        };
        if let Some(t) = ex.sample(seed, 24, &spec).into_iter().find(|t| t.valid) {
            let mut d = t.spec;
            d.name = fmt(t.sheet.length_m.max(t.sheet.width_m)) + " m";
            println!("size {:>4.2}: {:>9}  {:>6.1} km/h  {}", sz, mass_str(t.sheet.mass_kg), t.sheet.top_speed_kmh, gear_params(&d));
            designs.push(d);
        } else {
            println!("size {sz:.2}: no valid design");
        }
    }
    let forge_lib = lib.clone();
    let built: Vec<_> = designs.iter().filter_map(|d| ex.build(d).ok().map(|(b, _, _)| (b, d.name.clone()))).collect();
    let colours = preview::palette_colours(&forge_lib, Some("vanguard"));
    let units: Vec<(&Mesh, &str)> = built.iter().map(|(b, n)| (&b.mesh, n.as_str())).collect();
    let img = preview::lineup(&units, &colours, 1700, 640);
    save(&img, &out.join(format!("ladder_{}_{}.png", hull.trim_start_matches("hull_"), gear)));
}

fn gear_params(d: &DesignDef) -> String {
    d.attach
        .iter()
        .find(|a| a.family.as_deref().map(|f| GEARS.contains(&f)).unwrap_or(false))
        .map(|a| a.params.iter().filter(|(k, _)| !k.starts_with("ctx.")).map(|(k, v)| format!("{k} {}", fmt(*v))).collect::<Vec<_>>().join(", "))
        .unwrap_or_default()
}

/// `w5k fit --spec file.ron --out file.ron`: auto-fit a design file and write the fitted design.
pub fn fit_file(lib: &MaterialLibrary, spec: &Path, out: &Path) -> bool {
    let text = match std::fs::read_to_string(spec) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {}: {e}", spec.display());
            return false;
        }
    };
    let d: DesignDef = match ron::from_str(&text) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {}: {e}", spec.display());
            return false;
        }
    };
    let ex = Explorer::new(lib.clone());
    let f = match ex.fit(&d) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {e}");
            return false;
        }
    };
    println!("fitted {} in {} passes: {}  {:.0} km/h  {}", d.id, f.iterations, mass_str(f.sheet.mass_kg), f.sheet.top_speed_kmh, if f.valid() { "valid".to_string() } else { f.sheet.problems.join("; ") });
    let head = format!("#![enable(implicit_some)]\n// Fitted by `w5k fit` from {}: running gear and engine sized to the vehicle.\n", spec.file_name().and_then(|s| s.to_str()).unwrap_or("a spec"));
    let mut tidy = f.spec.clone();
    let round = |x: f64| -> f64 {
        if x == 0.0 || !x.is_finite() {
            return x;
        }
        let scale = 10f64.powi(3 - x.abs().log10().floor() as i32);
        (x * scale).round() / scale
    };
    tidy.hull_params.values_mut().for_each(|v| *v = round(*v));
    for a in tidy.attach.iter_mut() {
        a.params.values_mut().for_each(|v| *v = round(*v));
    }
    let body = ron::ser::to_string_pretty(&tidy, ron::ser::PrettyConfig::default().compact_arrays(true)).expect("design serialises");
    if let Some(p) = out.parent() {
        std::fs::create_dir_all(p).ok();
    }
    match std::fs::write(out, head + &body) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("error: {}: {e}", out.display());
            false
        }
    }
}

/// `w5k gallery --only a,b,c --out file.png`: labelled renders of designs from the content directory.
pub fn gallery(forge: &w5k_forge::Forge, ids: &[String], out: &Path, cols: usize, size: usize, title: &str) {
    let threads = std::thread::available_parallelism().map(|x| x.get()).unwrap_or(4).min(16);
    let mut slots: Vec<Option<Tile>> = (0..ids.len()).map(|_| None).collect();
    std::thread::scope(|s| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                s.spawn(move || {
                    let mut mine = Vec::new();
                    let mut i = t;
                    while i < ids.len() {
                        let tl = match forge.designs.get(&ids[i]) {
                            Some(d) => {
                                let (built, _, sh) = forge.build_design(d);
                                Tile {
                                    mesh: built.mesh,
                                    palette: d.palette.clone(),
                                    title: d.name.clone(),
                                    line1: format!("{}  {:.0} km/h  {} m", mass_str(sh.mass_kg), sh.top_speed_kmh, fmt(sh.length_m.max(sh.width_m))),
                                    line2: format!("arm {:.0}/{:.0} fire {}kW see {:.1}km", sh.armour_front_mm, sh.armour_side_mm, fmt(sh.firepower_kw), sh.sight_km),
                                    problem: sh.problems.first().cloned(),
                                }
                            }
                            None => Tile { mesh: Mesh::default(), palette: None, title: ids[i].clone(), line1: String::new(), line2: String::new(), problem: Some("no such design".into()) },
                        };
                        mine.push((i, tl));
                        i += threads;
                    }
                    mine
                })
            })
            .collect();
        for h in handles {
            for (i, tl) in h.join().expect("gallery thread panicked") {
                slots[i] = Some(tl);
            }
        }
    });
    let tl: Vec<Tile> = slots.into_iter().flatten().collect();
    save(&grid(&forge.lib, &tl, cols, size, title, 35.0, 20.0), out);
}
