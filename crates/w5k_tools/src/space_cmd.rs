//! `w5k space`: sample thousands of auto-fitted designs and chart the possibility space.
//!
//! The sample is *stratified*: every hull is combined with every kind of running gear (35 cells), each cell at many
//! random sizes with random weapons and masts, so the charts show the space itself, not the odds of a random roll.
//! Evaluation is the quick path (exact-volume mass, four-direction armour), within about 10 % of the full build.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

use w5k_forge::explore::{describe, Explorer, Sample, Spec, GEARS, HULLS};
use w5k_forge::raster::{Image, Rgb};
use w5k_forge::schema::{DesignDef, MaterialLibrary};

use crate::explore_cmd::{draw_tile, fmt, mass_str, tiles, LABEL_H};
use crate::plot::{hex, Axis, Plot};

pub fn gear_colour(g: &str) -> Rgb {
    hex(match g {
        "track" => "#ffb454",
        "wheel" => "#a3e635",
        "legs" => "#ff5c8a",
        "rail" => "#cbd5e1",
        "hover" => "#38bdf8",
        "antigrav" => "#c084fc",
        _ => "#34d399",
    })
}

pub fn gear_name(g: &str) -> &'static str {
    match g {
        "track" => "tracks",
        "wheel" => "wheels",
        "legs" => "legs",
        "rail" => "rail",
        "hover" => "air cushion",
        "antigrav" => "anti-grav",
        _ => "rotors",
    }
}

fn hull_colour(h: &str) -> Rgb {
    hex(match h {
        "hull_lancer" => "#38bdf8",
        "hull_bastion" => "#fb923c",
        "hull_dreadnought" => "#f43f5e",
        "hull_strider" => "#a3e635",
        _ => "#c084fc",
    })
}

fn hull_name(h: &str) -> &'static str {
    match h {
        "hull_lancer" => "Lancer",
        "hull_bastion" => "Bastion",
        "hull_dreadnought" => "Dreadnought",
        "hull_strider" => "Strider",
        _ => "Skiff",
    }
}

fn weapon_colour(w: &str) -> Rgb {
    hex(match w {
        "turret_gun" => "#ffb454",
        "turret_missile" => "#38bdf8",
        "turret_beam" => "#f472b6",
        _ => "#94a3b8",
    })
}

fn mass_label(kg: f64) -> String {
    if kg >= 1000.0 {
        format!("{} t", (kg / 1000.0).round())
    } else {
        format!("{} kg", kg.round())
    }
}

fn plain(x: f64) -> String {
    if x >= 1.0 {
        format!("{}", x.round())
    } else {
        format!("{x}")
    }
}

fn percent(x: f64) -> String {
    format!("{:.0}%", x * 100.0)
}

fn save(img: &Image, path: &Path) {
    match img.save_png(path) {
        Ok(()) => println!("wrote {}", path.display()),
        Err(e) => eprintln!("error: {e}"),
    }
}

fn gear_of(s: &Sample) -> &str {
    s.gear.as_str()
}

/// Median of a list (0 when empty).
fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

/// Pearson correlation of two equal-length lists.
fn pearson(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len()) as f64;
    if n < 3.0 {
        return 0.0;
    }
    let (ma, mb) = (a.iter().sum::<f64>() / n, b.iter().sum::<f64>() / n);
    let (mut sab, mut saa, mut sbb) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b) {
        sab += (x - ma) * (y - mb);
        saa += (x - ma).powi(2);
        sbb += (y - mb).powi(2);
    }
    sab / (saa * sbb).sqrt().max(1e-12)
}

/// Fraction of values below `x` (a percentile rank in [0, 1]).
fn rank(sorted: &[f64], x: f64) -> f64 {
    let below = sorted.partition_point(|v| *v < x);
    below as f64 / sorted.len().max(1) as f64
}

pub fn run(lib: &MaterialLibrary, out: &Path, seed: u64, n: usize, csv_only: bool) {
    let t0 = Instant::now();
    let ex = Explorer::new(lib.clone());
    std::fs::create_dir_all(out).ok();
    // Stratified sample: every hull x gear cell gets the same number of attempts.
    let per = (n / (HULLS.len() * GEARS.len())).max(4);
    let mut all: Vec<Sample> = Vec::new();
    for (hi, hull) in HULLS.iter().enumerate() {
        for (gi, gear) in GEARS.iter().enumerate() {
            let spec = Spec { hull: Some(hull.to_string()), gear: Some(gear.to_string()), ..Default::default() };
            let first = seed + ((hi * GEARS.len() + gi) as u64) * 1_000_000;
            all.extend(ex.sample(first, per, &spec));
        }
    }
    let mountable: Vec<&Sample> = all.iter().filter(|s| s.gear != "none" && s.error.is_none()).collect();
    let valid: Vec<&Sample> = mountable.iter().copied().filter(|s| s.valid && s.sheet.mass_kg > 0.0).collect();
    println!("sampled {} designs ({} per hull x gear cell): {} mountable, {} valid, {:.0}s", all.len(), per, mountable.len(), valid.len(), t0.elapsed().as_secs_f64());

    csv(&all, &out.join("space_sample.csv"));
    if csv_only {
        return;
    }
    chart_speed(&valid, &out.join("space_speed.png"));
    chart_fire_armour(&valid, &out.join("space_fire_armour.png"));
    chart_sight_range(&valid, &out.join("space_sight_range.png"));
    chart_pressure(&valid, &out.join("space_pressure.png"));
    chart_viability(&mountable, &out.join("space_viability.png"));
    chart_scale(&mountable, &out.join("space_scale.png"));
    chart_triangle(&valid, &out.join("space_triangle.png"));
    corners(lib, &valid, &out.join("space_corners.png"));
    budget(lib, &valid, &out.join("space_budget.png"));
    println!("done in {:.0}s", t0.elapsed().as_secs_f64());
}

fn csv(all: &[Sample], path: &Path) {
    let mut f = match std::fs::File::create(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {}: {e}", path.display());
            return;
        }
    };
    writeln!(f, "seed,hull,gear,weapon,valid,mass_kg,top_speed_kmh,power_kw,armour_front_mm,armour_side_mm,frontal_m2,firepower_kw,alpha_mj,best_pen_mm,weapon_range_km,sight_km,ground_pressure_kpa,length_m,width_m,height_m,lift_kw,armour_kept,problem").ok();
    for s in all {
        let h = &s.sheet;
        writeln!(
            f,
            "{},{},{},{},{},{:.0},{:.1},{:.0},{:.0},{:.0},{:.1},{:.1},{:.2},{:.0},{:.2},{:.2},{:.1},{:.1},{:.1},{:.1},{:.0},{:.2},\"{}\"",
            s.seed,
            s.hull,
            s.gear,
            s.weapon,
            s.valid,
            h.mass_kg,
            h.top_speed_kmh,
            h.power_kw,
            h.armour_front_mm,
            h.armour_side_mm,
            h.frontal_m2,
            h.firepower_kw,
            h.alpha_mj,
            h.best_pen_mm,
            h.range_km,
            h.sight_km,
            h.ground_pressure_kpa,
            h.length_m,
            h.width_m,
            h.height_m,
            h.lift_kw,
            s.diet,
            s.error.clone().unwrap_or_else(|| h.problems.first().cloned().unwrap_or_default()).replace('"', "'")
        )
        .ok();
    }
    println!("wrote {}", path.display());
}

fn gear_legend() -> Vec<(String, Rgb)> {
    GEARS.iter().map(|g| (gear_name(g).to_string(), gear_colour(g))).collect()
}

fn hull_legend() -> Vec<(String, Rgb)> {
    HULLS.iter().map(|h| (hull_name(h).to_string(), hull_colour(h))).collect()
}

/// 1. Top speed against mass, by kind of running gear.
fn chart_speed(valid: &[&Sample], path: &Path) {
    let mut p = Plot::new(
        1500,
        900,
        "Where each kind of running gear lives: top speed against mass",
        &format!("{} valid auto-fitted designs. Each dot is a whole vehicle; the label sits at the median of its group.", valid.len()),
        Axis::log(100.0, 3.0e6, "vehicle mass", mass_label),
        Axis::lin(0.0, 260.0, "top speed (km/h)", plain),
    );
    for s in valid {
        p.dot(s.sheet.mass_kg, s.sheet.top_speed_kmh, 2.6, gear_colour(gear_of(s)), 0.55);
    }
    // Group labels at the group medians, nudged apart where they would collide.
    let mut labels: Vec<(f64, f64, &str)> = Vec::new();
    for g in GEARS {
        let m: Vec<&&Sample> = valid.iter().filter(|s| s.gear == g).collect();
        if m.is_empty() {
            continue;
        }
        let (mx, my) = (median(m.iter().map(|s| s.sheet.mass_kg).collect()), median(m.iter().map(|s| s.sheet.top_speed_kmh).collect()));
        labels.push((p.x(mx * 1.1), p.y(my) - 6.0, gear_name(g)));
    }
    labels.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    for i in 1..labels.len() {
        if labels[i].1 - labels[i - 1].1 < 14.0 && (labels[i].0 - labels[i - 1].0).abs() < 110.0 {
            labels[i].1 = labels[i - 1].1 + 14.0;
        }
    }
    for (x, y, name) in labels {
        p.text_px(x, y, name, [1.0, 1.0, 1.0]);
    }
    p.legend(&gear_legend());
    p.notes(&["Legs are slow at any size: a walker pays about 0.6 W per N of weight just to move.", "Rail and anti-gravity reach the highest speeds, and only at large masses.", "Tracks stop at 70 km/h however much power they are given."]);
    save(&p.img, path);
}

/// 2. Firepower density against front armour, by hull.
fn chart_fire_armour(valid: &[&Sample], path: &Path) {
    let armed: Vec<&&Sample> = valid.iter().filter(|s| s.sheet.firepower_kw > 0.0 && s.sheet.armour_front_mm > 0.0).collect();
    let mut p = Plot::new(
        1500,
        900,
        "Armour against firepower per tonne: nothing reaches the top right",
        &format!("{} armed designs. x: median steel-equivalent armour seen from the front; y: weapon energy delivered per second per tonne of vehicle.", armed.len()),
        Axis::log(2.0, 1500.0, "front armour, steel-equivalent (mm)", plain),
        Axis::log(0.05, 30000.0, "firepower per tonne (kW/t)", |x| if x >= 1.0 { format!("{}", x.round()) } else { format!("{x}") }),
    );
    let pts: Vec<(f64, f64, &Sample)> = armed.iter().map(|s| (s.sheet.armour_front_mm, s.sheet.firepower_kw / (s.sheet.mass_kg / 1000.0), **s)).collect();
    for (x, y, s) in &pts {
        p.dot(*x, *y, 2.6, hull_colour(&s.hull), 0.55);
    }
    // The efficient frontier: no design has more of both.
    let mut front: Vec<(f64, f64)> = Vec::new();
    let mut sorted: Vec<(f64, f64)> = pts.iter().map(|(x, y, _)| (*x, *y)).collect();
    sorted.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    let mut best = 0.0;
    for (x, y) in sorted {
        if y > best {
            best = y;
            front.push((x, y));
        }
    }
    front.reverse();
    for w in front.windows(2) {
        p.seg(w[0].0, w[0].1, w[1].0, w[1].1, [1.0, 1.0, 1.0], 0.9);
    }
    p.legend(&hull_legend());
    p.notes(&["White line: the efficient frontier. No sampled design has more of both."]);
    save(&p.img, path);
}

/// 3. Sight against weapon range, by weapon.
fn chart_sight_range(valid: &[&Sample], path: &Path) {
    let armed: Vec<&&Sample> = valid.iter().filter(|s| s.sheet.range_km > 0.0).collect();
    let mut p = Plot::new(
        1500,
        900,
        "Sight against weapon range: what you can shoot, you must first see",
        &format!("{} armed designs. Above the diagonal a weapon outranges its own eyes and needs an allied spotter (the vision model: height buys horizon).", armed.len()),
        Axis::log(1.5, 8.0, "sight range, horizon included (km)", plain_km),
        Axis::log(0.25, 20.0, "weapon range (km)", plain_km),
    );
    p.seg(1.5, 1.5, 8.0, 8.0, [0.8, 0.8, 0.9], 0.8);
    for s in &armed {
        let kind = s.weapon.as_str();
        p.dot(s.sheet.sight_km, s.sheet.range_km, 2.6, weapon_colour(kind), 0.5);
    }
    p.label(1.6, 14.0, "needs a spotter", [1.0, 0.9, 0.6]);
    p.label(5.6, 0.32, "sees more than it can hit", [0.7, 0.8, 1.0]);
    p.legend(&[("gun".into(), weapon_colour("turret_gun")), ("missile".into(), weapon_colour("turret_missile")), ("beam".into(), weapon_colour("turret_beam"))]);
    p.notes(&["Sight is the smaller of the horizon (height buys it) and the optics: plain optics reach 4 km, so tall vehicles pile up at that stripe. Only a fitted sensor sees further.", "Guns hug the bottom; missiles and beams climb past the eyes and need an allied spotter."]);
    save(&p.img, path);
}

fn plain_km(x: f64) -> String {
    if x >= 1.0 {
        format!("{}", x.round())
    } else {
        format!("{x}")
    }
}

/// 4. Ground pressure against mass.
fn chart_pressure(valid: &[&Sample], path: &Path) {
    let ground: Vec<&&Sample> = valid.iter().filter(|s| s.sheet.ground_pressure_kpa > 0.0).collect();
    let mut p = Plot::new(
        1500,
        900,
        "Ground pressure against mass: who sinks",
        &format!("{} designs with ground contact. Soft ground gives way between about 50 and 100 kPa; tracks press about 90, an air cushion a few.", ground.len()),
        Axis::log(100.0, 3.0e6, "vehicle mass", mass_label),
        Axis::log(0.5, 3000.0, "ground pressure (kPa)", plain),
    );
    let (x0, x1) = (p.x(100.0), p.x(3.0e6));
    let (y0, y1) = (p.y(100.0), p.y(50.0));
    p.img.fill_rect(x0 as i64, y0 as i64, (x1 - x0) as i64, (y1 - y0) as i64, [0.5, 0.35, 0.1], 0.18);
    p.label(150.0, 70.0, "soft ground gives way", [1.0, 0.8, 0.4]);
    for s in &ground {
        p.dot(s.sheet.mass_kg, s.sheet.ground_pressure_kpa, 2.6, gear_colour(gear_of(s)), 0.55);
    }
    p.legend(&gear_legend());
    p.notes(&["Walkers size their feet to hold about 100 kPa however heavy they get: a 1000 t walker needs pads several metres across.", "An air cushion floats at a few kPa at any size, but only if the hull is light."]);
    save(&p.img, path);
}

/// 5. Which hull x gear mixes survive the physics.
fn chart_viability(mountable: &[&Sample], path: &Path) {
    let (cw, ch, l, t) = (160usize, 96usize, 140usize, 110usize);
    let (w, h) = (l + cw * GEARS.len() + 40, t + ch * HULLS.len() + 150);
    let mut img = Image::new(w, h, crate::plot::BG);
    img.text(24, 18, 2, "Which mixes survive the physics", [0.9, 0.91, 0.94]);
    img.text(24, 44, 1, "One hull with one kind of running gear, at many random sizes. Valid = the auto-fit found gear and an engine that work.", [0.5, 0.52, 0.58]);
    img.text(24, 58, 1, "Armour kept = the median share of the requested armour that survived the fit (fliers and floaters must shed weight to stay up).", [0.5, 0.52, 0.58]);
    for (gi, g) in GEARS.iter().enumerate() {
        img.text((l + gi * cw + 10) as i64, (t - 20) as i64, 1, gear_name(g), gear_colour(g));
    }
    for (hi, hl) in HULLS.iter().enumerate() {
        img.text(20, (t + hi * ch + ch / 2) as i64 - 4, 1, hull_name(hl), hull_colour(hl));
        for (gi, g) in GEARS.iter().enumerate() {
            let cell: Vec<&&Sample> = mountable.iter().filter(|s| s.hull == *hl && s.gear == *g).collect();
            let (x, y) = (l + gi * cw, t + hi * ch);
            if cell.is_empty() {
                img.fill_rect(x as i64 + 3, y as i64 + 3, cw as i64 - 6, ch as i64 - 6, [0.01, 0.01, 0.012], 1.0);
                img.text(x as i64 + 10, y as i64 + ch as i64 / 2 - 4, 1, "no mount", [0.35, 0.36, 0.4]);
                continue;
            }
            let ok = cell.iter().filter(|s| s.valid).count() as f64 / cell.len() as f64;
            // Dark violet to bright amber.
            let c = w5k_forge::raster::mix([0.03, 0.02, 0.08], [1.0, 0.72, 0.2], (ok as f32).powf(0.8));
            img.fill_rect(x as i64 + 3, y as i64 + 3, cw as i64 - 6, ch as i64 - 6, c, 1.0);
            let label = percent(ok);
            let tc = if ok > 0.55 { [0.05, 0.05, 0.08] } else { [0.95, 0.95, 0.98] };
            img.text(x as i64 + 12, y as i64 + 16, 3, &label, tc);
            let kept = median(cell.iter().filter(|s| s.valid).map(|s| s.diet).collect());
            img.text(x as i64 + 12, y as i64 + ch as i64 - 38, 1, "armour kept", tc);
            img.text(x as i64 + 12, y as i64 + ch as i64 - 26, 1, &format!("{:.0}% (median)", 100.0 * kept), tc);
            img.text(x as i64 + 12, y as i64 + ch as i64 - 14, 1, &format!("{} tried", cell.len()), tc);
        }
    }
    // Why they fail, in words.
    let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
    for s in mountable.iter().filter(|s| !s.valid) {
        let why = s.sheet.problems.first().cloned().unwrap_or_default();
        let key = if why.contains("overloaded") {
            "running gear cannot carry the weight"
        } else if why.contains("cannot hover") || why.contains("cannot float") || why.contains("cannot rise") {
            "cannot lift itself (not enough power)"
        } else if why.contains("engine") {
            "engine does not fit its bay"
        } else if why.contains("turret ring") || why.contains("mount") {
            "a part does not fit its mount"
        } else {
            "other"
        };
        *reasons.entry(key.to_string()).or_default() += 1;
    }
    let total: usize = reasons.values().sum();
    let mut y = t + ch * HULLS.len() + 24;
    img.text(20, y as i64, 1, "Why designs fail:", [0.8, 0.82, 0.86]);
    let mut rs: Vec<_> = reasons.into_iter().collect();
    rs.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for (k, n) in rs {
        y += 18;
        img.text(40, y as i64, 1, &format!("{:>5.1}%  {k}", 100.0 * n as f64 / total.max(1) as f64), [0.6, 0.62, 0.68]);
    }
    save(&img, path);
}

/// 6. By scale, per kind of running gear: the share of designs that are valid, and the armour they get to keep.
fn chart_scale(mountable: &[&Sample], path: &Path) {
    let edges: Vec<f64> = (0..=17).map(|k| 100.0 * 10f64.powf(k as f64 * 0.5)).collect();
    for (name, title, sub, kept) in [
        ("space_scale_valid.png", "Viability by scale: share of designs the physics allows", "Share of auto-fitted designs that are valid, by vehicle mass (half-decade bins; bins with fewer than 8 designs are not drawn).", false),
        ("space_scale_armour.png", "Armour the fitter lets you keep, by scale", "Median share of the requested armour that survived the fitter (valid designs only). Air cushions and rotors must shed plating at every size; ground gear and anti-gravity keep it all.", true),
    ] {
        let mut p = Plot::new(1500, 900, title, sub, Axis::log(100.0, 3.0e6, "vehicle mass", mass_label), Axis::lin(0.0, 1.05, if kept { "armour kept" } else { "valid" }, percent));
        for g in GEARS {
            let c = gear_colour(g);
            let mut prev: Option<(f64, f64)> = None;
            for w in edges.windows(2) {
                let cell: Vec<&&Sample> = mountable.iter().filter(|s| s.gear == g && s.sheet.mass_kg >= w[0] && s.sheet.mass_kg < w[1]).collect();
                let ok: Vec<&&&Sample> = cell.iter().filter(|s| s.valid).collect();
                if cell.len() < 8 || (kept && ok.len() < 4) {
                    prev = None;
                    continue;
                }
                let y = if kept { median(ok.iter().map(|s| s.diet).collect()) } else { ok.len() as f64 / cell.len() as f64 };
                let xm = (w[0] * w[1]).sqrt();
                p.dot(xm, y, 4.0, c, 0.95);
                if let Some((px, py)) = prev {
                    p.seg(px, py, xm, y, c, 0.8);
                }
                prev = Some((xm, y));
            }
        }
        p.legend(&gear_legend());
        save(&p.img, &path.with_file_name(name));
    }
}

/// 7. The trade triangle: at equal mass, firepower, armour and speed pull against each other.
fn chart_triangle(valid: &[&Sample], path: &Path) {
    // The mass band with the most armed, moving designs.
    let armed: Vec<&&Sample> = valid.iter().filter(|s| s.sheet.firepower_kw > 0.0 && s.sheet.top_speed_kmh > 1.0 && s.sheet.armour_front_mm > 0.0).collect();
    let mut best = (0usize, 0.0f64);
    for k in 0..40 {
        let m0 = 300.0 * 10f64.powf(k as f64 * 0.1);
        let c = armed.iter().filter(|s| s.sheet.mass_kg > m0 / 1.3 && s.sheet.mass_kg < m0 * 1.3).count();
        if c > best.0 {
            best = (c, m0);
        }
    }
    let m0 = best.1;
    let band: Vec<&&&Sample> = armed.iter().filter(|s| s.sheet.mass_kg > m0 / 1.3 && s.sheet.mass_kg < m0 * 1.3).collect();
    let (mut fp, mut ar, mut sp): (Vec<f64>, Vec<f64>, Vec<f64>) = (vec![], vec![], vec![]);
    for s in &band {
        fp.push(s.sheet.firepower_kw);
        ar.push(s.sheet.armour_front_mm);
        sp.push(s.sheet.top_speed_kmh);
    }
    for v in [&mut fp, &mut ar, &mut sp] {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    }
    let (w, h) = (1500usize, 1000usize);
    let mut img = Image::new(w, h, crate::plot::BG);
    // How the three traits move together across the band (Spearman: Pearson of the percentile ranks).
    let rf: Vec<f64> = band.iter().map(|s| rank(&fp, s.sheet.firepower_kw)).collect();
    let ra: Vec<f64> = band.iter().map(|s| rank(&ar, s.sheet.armour_front_mm)).collect();
    let rs: Vec<f64> = band.iter().map(|s| rank(&sp, s.sheet.top_speed_kmh)).collect();
    let (c_fa, c_as, c_fs) = (pearson(&rf, &ra), pearson(&ra, &rs), pearson(&rf, &rs));
    let top3 = band.iter().enumerate().filter(|(i, _)| rf[*i] > 0.67 && ra[*i] > 0.67 && rs[*i] > 0.67).count();
    println!("triangle: {} designs; rank correlations firepower-armour {c_fa:+.2}, armour-speed {c_as:+.2}, firepower-speed {c_fs:+.2}; top third on all three: {top3} ({:.1}%, independent traits would give 3.7%)", band.len(), 100.0 * top3 as f64 / band.len().max(1) as f64);
    img.text(24, 18, 2, "The trade triangle: three traits at equal mass", [0.9, 0.91, 0.94]);
    img.text(24, 44, 1, &format!("{} armed designs between {} and {}. Each is placed by the shares of its three percentile ranks; bigger and brighter = strong on all three.", band.len(), mass_str(m0 / 1.3), mass_str(m0 * 1.3)), [0.5, 0.52, 0.58]);
    img.text(24, 58, 1, &format!("Rank correlations: firepower-armour {c_fa:+.2}, armour-speed {c_as:+.2}, firepower-speed {c_fs:+.2}. Top third on all three: {top3} designs ({:.1}%; independent traits give 3.7%).", 100.0 * top3 as f64 / band.len().max(1) as f64), [0.5, 0.52, 0.58]);
    let (cx, top, side) = (620.0, 150.0, 760.0);
    let hgt = side * 3f64.sqrt() / 2.0;
    let (a, b, c) = ((cx, top), (cx - side / 2.0, top + hgt), (cx + side / 2.0, top + hgt));
    let tri = [(a, b), (b, c), (c, a)];
    for (p, q) in tri {
        img.line(p.0, p.1, q.0, q.1, [0.45, 0.47, 0.54], 1.0);
    }
    // Faint iso-lines.
    for k in 1..5 {
        let f = k as f64 / 5.0;
        for (p, q, r) in [(a, b, c), (b, c, a), (c, a, b)] {
            let (x0, y0) = (p.0 + (q.0 - p.0) * f, p.1 + (q.1 - p.1) * f);
            let (x1, y1) = (p.0 + (r.0 - p.0) * f, p.1 + (r.1 - p.1) * f);
            img.line(x0, y0, x1, y1, [0.06, 0.065, 0.08], 1.0);
        }
    }
    img.text(a.0 as i64 - 40, a.1 as i64 - 26, 2, "FIREPOWER", [1.0, 0.8, 0.4]);
    img.text(b.0 as i64 - 20, b.1 as i64 + 14, 2, "ARMOUR", [0.5, 0.8, 1.0]);
    img.text(c.0 as i64 - 90, c.1 as i64 + 14, 2, "SPEED", [0.6, 1.0, 0.7]);
    let mut order: Vec<usize> = (0..band.len()).collect();
    let quality: Vec<f64> = band
        .iter()
        .map(|s| {
            let (rf, ra, rs) = (rank(&fp, s.sheet.firepower_kw), rank(&ar, s.sheet.armour_front_mm), rank(&sp, s.sheet.top_speed_kmh));
            (rf.max(0.02) * ra.max(0.02) * rs.max(0.02)).cbrt()
        })
        .collect();
    order.sort_by(|i, j| quality[*i].partial_cmp(&quality[*j]).unwrap());
    for i in order {
        let s = band[i];
        let (rf, ra, rs) = (rank(&fp, s.sheet.firepower_kw) + 0.03, rank(&ar, s.sheet.armour_front_mm) + 0.03, rank(&sp, s.sheet.top_speed_kmh) + 0.03);
        let sum = rf + ra + rs;
        let (wa, wb, wc) = (rf / sum, ra / sum, rs / sum);
        let (x, y) = (wa * a.0 + wb * b.0 + wc * c.0, wa * a.1 + wb * b.1 + wc * c.1);
        let r = 2.5 + 7.0 * quality[i].powf(2.0);
        let col = hull_colour(&s.hull);
        for dy in -(r as i64 + 1)..=(r as i64 + 1) {
            for dx in -(r as i64 + 1)..=(r as i64 + 1) {
                let d = ((dx * dx + dy * dy) as f64).sqrt();
                if d <= r {
                    img.blend(x as i64 + dx, y as i64 + dy, col, (0.25 + 0.65 * quality[i]) as f32);
                }
            }
        }
    }
    // Legend and the reading guide.
    let mut y = 180i64;
    for (name, c) in hull_legend() {
        for dy in -5i64..=5 {
            for dx in -5i64..=5 {
                if ((dx * dx + dy * dy) as f64).sqrt() <= 5.0 {
                    img.blend(1130 + dx, y + dy, c, 1.0);
                }
            }
        }
        img.text(1146, y - 4, 1, &name, [0.8, 0.82, 0.86]);
        y += 22;
    }
    let strongest = c_fa.abs().max(c_as.abs()).max(c_fs.abs());
    let verdict: Vec<String> = if strongest < 0.3 {
        vec![
            "What it shows:".into(),
            "At equal mass the three traits are nearly".into(),
            "  independent in this model: the rank".into(),
            "  correlations are all weak. Speed is cheap".into(),
            "  (the engine is roughly 2-15 % of the mass".into(),
            "  and each gear tops out at its rating), so".into(),
            "  only firepower and armour compete, a little.".into(),
            "".into(),
            "So the trade-offs the game wants must come".into(),
            "  from price (command points) and from the".into(),
            "  physics gates (gear limits, lift), not".into(),
            "  from mass alone.".into(),
        ]
    } else {
        vec!["What it shows:".into(), "At equal mass the traits pull against".into(), "  each other: the corners hold specialists".into(), "  and the middle holds few strong designs.".into()]
    };
    let mut guide: Vec<String> = vec!["How to read it:".into(), "A dot near a corner is a specialist.".into(), "A dot in the middle has similar shares".into(), "  of all three; big and bright means all".into(), "  three shares are high.".into(), "".into()];
    guide.extend(verdict);
    let notes: Vec<&str> = guide.iter().map(|s| s.as_str()).collect();
    let mut y = 360i64;
    for line in notes {
        img.text(1100, y, 1, line, [0.6, 0.62, 0.68]);
        y += 16;
    }
    save(&img, path);
}

/// Pick the best sample by a score (valid, armed or not as the score demands).
fn best_by<'a>(valid: &[&'a Sample], score: impl Fn(&Sample) -> f64) -> Option<&'a Sample> {
    valid.iter().copied().filter(|s| score(s).is_finite()).max_by(|a, b| score(a).partial_cmp(&score(b)).unwrap())
}

/// 8. The corners of the space: the extreme designs.
fn corners(lib: &MaterialLibrary, valid: &[&Sample], path: &Path) {
    let moving = |s: &Sample| s.sheet.top_speed_kmh > 1.0;
    let picks: Vec<(&str, Option<&Sample>)> = vec![
        ("fastest", best_by(valid, |s| s.sheet.top_speed_kmh)),
        ("most armoured (front)", best_by(valid, |s| if moving(s) { s.sheet.armour_front_mm } else { f64::NAN })),
        ("most firepower", best_by(valid, |s| s.sheet.firepower_kw)),
        ("firepower per tonne", best_by(valid, |s| if s.sheet.mass_kg > 500.0 { s.sheet.firepower_kw / s.sheet.mass_kg } else { f64::NAN })),
        ("longest range", best_by(valid, |s| s.sheet.range_km)),
        ("sees farthest", best_by(valid, |s| s.sheet.sight_km)),
        ("heaviest", best_by(valid, |s| s.sheet.mass_kg)),
        ("lightest armed", best_by(valid, |s| if s.sheet.firepower_kw > 0.0 { -s.sheet.mass_kg } else { f64::NAN })),
        ("gentlest on the ground", best_by(valid, |s| if s.sheet.ground_pressure_kpa > 0.0 && s.sheet.mass_kg > 2000.0 { -s.sheet.ground_pressure_kpa } else { f64::NAN })),
        ("highest alpha strike", best_by(valid, |s| s.sheet.alpha_mj)),
        ("biggest", best_by(valid, |s| s.sheet.length_m.max(s.sheet.width_m))),
        ("tallest", best_by(valid, |s| s.sheet.height_m)),
    ];
    let mut designs: Vec<DesignDef> = Vec::new();
    let mut labels: Vec<(String, String)> = Vec::new();
    for (name, s) in picks {
        if let Some(s) = s {
            let mut d = s.spec.clone();
            d.name = name.to_string();
            labels.push((name.to_string(), describe(&s.spec)));
            println!("corner {:<24} {:<34} {:>9} {:>6.0} km/h arm {:>4.0} fire {:>9} kW range {:>5.1} km see {:>4.1} km", name, describe(&s.spec), mass_str(s.sheet.mass_kg), s.sheet.top_speed_kmh, s.sheet.armour_front_mm, fmt(s.sheet.firepower_kw), s.sheet.range_km, s.sheet.sight_km);
            designs.push(d);
        }
    }
    let size = 330usize;
    let cols = 4usize;
    let mut tl = tiles(lib, &designs);
    for (t, (name, what)) in tl.iter_mut().zip(&labels) {
        let numbers = std::mem::take(&mut t.line1);
        t.title = name.clone();
        t.line1 = what.clone();
        t.line2 = numbers;
    }
    let rows = tl.len().div_ceil(cols);
    let hdr = 36;
    let mut img = Image::new(cols * size, hdr + rows * (size + LABEL_H), crate::plot::BG);
    img.text(10, 10, 2, "The corners of the possibility space: the extremes among the sampled designs", [0.85, 0.86, 0.88]);
    for (i, t) in tl.iter().enumerate() {
        draw_tile(&mut img, lib, t, (i % cols) * size, hdr + (i / cols) * (size + LABEL_H), size, 35.0, 20.0);
    }
    save(&img, path);
}

/// 9. What one budget buys: the same mass, very different vehicles, side by side at true scale.
fn budget(lib: &MaterialLibrary, valid: &[&Sample], path: &Path) {
    // The mass band (around 30 t, widening if sparse) with enough armed, moving designs.
    let target = 30_000.0;
    let mut pool: Vec<&Sample> = Vec::new();
    let mut band = 1.12;
    for tol in [1.12, 1.2, 1.35, 1.6] {
        band = tol;
        pool = valid.iter().copied().filter(|s| s.sheet.firepower_kw > 0.0 && s.sheet.top_speed_kmh > 1.0 && s.sheet.mass_kg > target / tol && s.sheet.mass_kg < target * tol).collect();
        if pool.len() >= 24 {
            break;
        }
    }
    if pool.len() < 6 {
        println!("budget: only {} designs near {}", pool.len(), mass_str(target));
        return;
    }
    let mut fp: Vec<f64> = pool.iter().map(|s| s.sheet.firepower_kw).collect();
    let mut ar: Vec<f64> = pool.iter().map(|s| s.sheet.armour_front_mm).collect();
    let mut sp: Vec<f64> = pool.iter().map(|s| s.sheet.top_speed_kmh).collect();
    for v in [&mut fp, &mut ar, &mut sp] {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    }
    let balanced = |s: &Sample| (rank(&fp, s.sheet.firepower_kw) * rank(&ar, s.sheet.armour_front_mm) * rank(&sp, s.sheet.top_speed_kmh)).cbrt();
    let picks: Vec<(&str, Option<&Sample>)> = vec![
        ("most armour", best_by(&pool, |s| s.sheet.armour_front_mm)),
        ("most firepower", best_by(&pool, |s| s.sheet.firepower_kw)),
        ("fastest", best_by(&pool, |s| s.sheet.top_speed_kmh)),
        ("sees farthest", best_by(&pool, |s| s.sheet.sight_km)),
        ("longest range", best_by(&pool, |s| s.sheet.range_km)),
        ("best all-rounder", best_by(&pool, balanced)),
    ];
    let mut designs: Vec<DesignDef> = Vec::new();
    let mut labels: Vec<(String, String)> = Vec::new();
    for (name, s) in picks {
        let Some(s) = s else { continue };
        println!("budget {:<18} {:<34} {:>9} {:>6.0} km/h arm {:>4.0} fire {:>8} kW range {:>5.1} km see {:>4.1} km", name, describe(&s.spec), mass_str(s.sheet.mass_kg), s.sheet.top_speed_kmh, s.sheet.armour_front_mm, fmt(s.sheet.firepower_kw), s.sheet.range_km, s.sheet.sight_km);
        let mut d = s.spec.clone();
        d.name = name.to_string();
        labels.push((name.to_string(), describe(&s.spec)));
        designs.push(d);
    }
    let size = 400usize;
    let cols = 3usize;
    let mut tl = tiles(lib, &designs);
    for (t, (name, what)) in tl.iter_mut().zip(&labels) {
        let numbers = std::mem::take(&mut t.line1);
        t.title = name.clone();
        t.line1 = what.clone();
        t.line2 = numbers;
    }
    let rows = tl.len().div_ceil(cols);
    let hdr = 36;
    let mut img = Image::new(cols * size, hdr + rows * (size + LABEL_H), crate::plot::BG);
    img.text(10, 10, 2, &format!("One mass budget ({} to {}), six ways to spend it", mass_str(target / band), mass_str(target * band)), [0.85, 0.86, 0.88]);
    for (i, t) in tl.iter().enumerate() {
        draw_tile(&mut img, lib, t, (i % cols) * size, hdr + (i / cols) * (size + LABEL_H), size, 35.0, 20.0);
    }
    save(&img, path);
}
