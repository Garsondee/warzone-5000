//! `w5k family <content-dir> --out <dir>`: sheets that show parametric families at work.
//!
//! * `<family>_sweep.png`: the family across its main sliders (calibre down, armour across).
//! * `<family>_coupling.png`: a sequence of slider moves with "hold mass" on, showing the other sliders give way.

use std::path::Path;

use w5k_forge::family::{self, turret, Family, Param, Values};
use w5k_forge::preview::{self, Frame};
use w5k_forge::raster::{Image, Rgb};
use w5k_forge::schema::MaterialLibrary;
use w5k_forge::{Built, Forge};

const TEXT: Rgb = [0.62, 0.64, 0.67];
const DIM: Rgb = [0.3, 0.31, 0.33];
const AMBER: Rgb = [0.95, 0.62, 0.15];
const LOCKED: Rgb = [0.45, 0.47, 0.5];
const BG: Rgb = [0.018, 0.019, 0.022];

fn build(lib: &MaterialLibrary, fam: &dyn Family, v: &Values) -> Built {
    let def = fam.generate(v);
    let id = def.id.clone();
    let forge = Forge::from_parts(lib.clone(), vec![def]).unwrap_or_else(|e| panic!("{e}"));
    forge.build_part(&id).expect("built")
}

fn stat(stats: &[family::Stat], name: &str) -> f64 {
    stats.iter().find(|s| s.name == name).map(|s| s.value).unwrap_or(f64::NAN)
}

fn fmt(x: f64) -> String {
    if x >= 100.0 {
        format!("{x:.0}")
    } else if x >= 10.0 {
        format!("{x:.1}")
    } else {
        format!("{x:.2}")
    }
}

/// Draw the family's sliders as bars: amber when free to move, grey when locked.
fn slider_panel(img: &mut Image, x: i64, y: i64, w: i64, params: &[Param], v: &Values, locked: &[&str], moved: Option<&str>) -> i64 {
    let mut yy = y;
    for p in params {
        let pos = p.position_of(v[p.id]);
        let is_locked = locked.contains(&p.id);
        let c = if is_locked { LOCKED } else { AMBER };
        let label = format!("{}{}", p.name, if is_locked { " [locked]" } else if Some(p.id) == moved { " <" } else { "" });
        img.text(x, yy, 1, &label, if Some(p.id) == moved { [0.9, 0.9, 0.9] } else { TEXT });
        let val = format!("{} {}", fmt(v[p.id]), p.unit);
        img.text(x + w - 8 * val.len() as i64, yy, 1, &val, TEXT);
        img.fill_rect(x, yy + 11, w, 5, DIM, 1.0);
        img.fill_rect(x, yy + 11, (w as f64 * pos) as i64, 5, c, 1.0);
        yy += 22;
    }
    yy
}

/// Frame a view on the turret body (the barrel may run out of the picture).
fn body_frame(v: &Values, margin: f64) -> Frame {
    let (centre, r, ground) = turret::body_frame(v);
    let r = r * margin;
    Frame { centre, radius: r, ground, grid: preview::nice_step(r / 2.0) }
}

fn sweep(lib: &MaterialLibrary, fam: &dyn Family, out: &Path) {
    // Rows: calibre; columns: light, medium and heavy armour. Loader and ammunition follow the calibre, as a
    // player would set them (rotary guns at the small end, mechanical loaders for the big guns).
    let calibres: [f64; 7] = [7.62, 20.0, 40.0, 75.0, 120.0, 200.0, 406.0];
    let armours = [(10.0, 0.1, "light"), (60.0, 0.45, "medium"), (180.0, 0.9, "heavy")];
    let tile = 330usize;
    let label_h = 46usize;
    let header = 40usize;
    let mut img = Image::new(tile * armours.len(), header + (tile + label_h) * calibres.len(), BG);
    img.text(12, 12, 2, &format!("{}: calibre (rows) x armour and slope (columns)", fam.name()), [0.85, 0.86, 0.88]);
    for (r, &cal) in calibres.iter().enumerate() {
        for (c, &(arm, slope, name)) in armours.iter().enumerate() {
            let loader = if cal < 25.0 || cal >= 150.0 { 1.0 } else { 0.0 };
            let ammo = (3000.0 * (7.62 / cal).powf(1.6)).clamp(8.0, 3000.0).round();
            let v = fam.with(&[("calibre_mm", cal), ("armour_mm", arm), ("slope", slope), ("loader", loader), ("ammo", ammo)]);
            let built = build(lib, fam, &v);
            let stats = fam.performance(&v, &built);
            let colours = preview::palette_colours(lib, Some("default"));
            let fr = body_frame(&v, 1.25);
            let view = preview::render_view(&built.mesh, &colours, &fr, 35.0, 25.0, tile);
            let (x, y) = (c * tile, header + r * (tile + label_h));
            img.blit(&view, x, y);
            img.text(x as i64 + 6, y as i64 + 6, 1, &format!("{cal} mm, {name} armour ({arm:.0} mm)"), TEXT);
            img.text(x as i64 + 6, y as i64 + 18, 1, &format!("grid {} m", fr.grid), DIM);
            let l1 = format!(
                "{} t  pen {} mm  reload {} s",
                fmt(stat(&stats, "turret mass")),
                fmt(stat(&stats, "penetration at 1 km")),
                fmt(stat(&stats, "reload"))
            );
            let l2 = format!(
                "front {} mm  traverse {} s  crew {}",
                fmt(stat(&stats, "front armour (effective)")),
                fmt(stat(&stats, "traverse 180 deg")),
                stat(&stats, "crew")
            );
            img.text(x as i64 + 6, (y + tile + 8) as i64, 1, &l1, TEXT);
            img.text(x as i64 + 6, (y + tile + 24) as i64, 1, &l2, TEXT);
            println!("sweep {cal:>6} mm {name:<6} {:>9.0} kg  {l1} | {l2}", built.mass.mass_kg);
        }
    }
    save(&img, &out.join(format!("{}_sweep.png", fam.id())));
}

fn coupling(lib: &MaterialLibrary, fam: &dyn Family, out: &Path) {
    let params = fam.params();
    // (slider moved, new value, locked sliders, caption)
    let steps: [(&str, f64, &[&str], &str); 4] = [
        ("", 0.0, &[], "start: 75 mm, hold mass on"),
        ("calibre_mm", 120.0, &[], "calibre up: everything else gives way"),
        ("calibre_mm", 120.0, &["armour_mm"], "same move, armour locked"),
        ("calibre_mm", 200.0, &["armour_mm", "ammo"], "200 mm, armour and ammo locked"),
    ];
    let tile = 300usize;
    let panel = 170usize;
    let mut img = Image::new(tile * steps.len(), 40 + tile + panel + 40, BG);
    img.text(12, 12, 2, &format!("{}: sliders coupled by mass (hold mass)", fam.name()), [0.85, 0.86, 0.88]);
    let start = fam.defaults();
    // One frame for every step, so growth and shrinkage show.
    let fr = body_frame(&start, 1.6);
    for (k, (id, target, locked, caption)) in steps.iter().enumerate() {
        let v = if id.is_empty() { start.clone() } else { family::set_slider(fam, lib, &start, id, *target, locked) };
        let built = build(lib, fam, &v);
        let stats = fam.performance(&v, &built);
        let colours = preview::palette_colours(lib, Some("default"));
        let view = preview::render_view(&built.mesh, &colours, &fr, 35.0, 25.0, tile);
        let x = k * tile;
        img.blit(&view, x, 40);
        img.text(x as i64 + 6, 46, 1, caption, TEXT);
        img.text(x as i64 + 6, 58, 1, &format!("grid {} m", fr.grid), DIM);
        let mut y = slider_panel(&mut img, x as i64 + 10, (40 + tile + 8) as i64, tile as i64 - 20, &params, &v, locked, if id.is_empty() { None } else { Some(id) });
        let l = format!("mass {} t   reload {} s", fmt(built.mass.mass_kg / 1000.0), fmt(stat(&stats, "reload")));
        let l2 = format!("pen {} mm   front {} mm", fmt(stat(&stats, "penetration at 1 km")), fmt(stat(&stats, "front armour (effective)")));
        y += 4;
        img.text(x as i64 + 10, y, 1, &l, [0.85, 0.86, 0.88]);
        img.text(x as i64 + 10, y + 14, 1, &l2, [0.85, 0.86, 0.88]);
        println!("coupling {k}: {caption:<40} estimate {:>8.0} kg, measured {:>8.0} kg | {l}", family::mass_estimate(fam, &v, lib), built.mass.mass_kg);
    }
    save(&img, &out.join(format!("{}_coupling.png", fam.id())));
}

fn save(img: &Image, path: &Path) {
    if let Err(e) = img.save_png(path) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

pub fn run(forge: &Forge, out: &Path) -> bool {
    std::fs::create_dir_all(out).unwrap_or_else(|e| {
        eprintln!("error: {}: {e}", out.display());
        std::process::exit(1)
    });
    for fam in family::all() {
        sweep(&forge.lib, fam.as_ref(), out);
        coupling(&forge.lib, fam.as_ref(), out);
    }
    true
}
