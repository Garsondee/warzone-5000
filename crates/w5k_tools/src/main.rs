//! `w5k`: command-line tools for warzone-5000.
//!
//! ```text
//! w5k render <content-dir> --out <dir> [--only id,id,...]   contact sheets, GLB and stats for parts and vehicles
//! w5k check  <content-dir>                                 build everything and report problems
//! w5k family <content-dir> --out <dir>                     sweep and slider-coupling sheets for parametric families
//! ```

mod explore_cmd;
mod family_cmd;
mod host_sweep;
mod plot;
mod soil_cmd;
mod space_cmd;
mod trial_cmd;
mod trial_plots;

use std::path::{Path, PathBuf};
use std::time::Instant;

use w5k_forge::geom::V3;
use w5k_forge::gltf::{write_glb, GlbSocket};
use w5k_forge::preview::{self, Frame};
use w5k_forge::raster::Image;
use w5k_forge::{Built, Forge, StatsFile};

fn usage() -> ! {
    eprintln!("usage:\n  w5k render <content-dir> --out <dir> [--only id,id,...]\n  w5k check <content-dir>\n  w5k family <content-dir> --out <dir>\n  w5k sweeps <content-dir> --out <dir> [--only family,family]\n  w5k view <content-dir> --design id --out file.png [--az 40 --el 20 --focus x,y,z --radius r --size WxH]\n  w5k lineup <content-dir> --out <dir> [--only design,design,...]\n  w5k trial <content-dir> --out <dir> [--course file.ron --terrain file.ron --only id,id --limit seconds --mode sim|parade]\n  w5k soil <content-dir> --design id --out <dir> [--course file.ron --terrain file.ron]");
    std::process::exit(2)
}

struct Args {
    content: PathBuf,
    out: Option<PathBuf>,
    only: Option<Vec<String>>,
    palette: Option<String>,
    /// Any other `--key value` pair (for example `--az 40` for `w5k view`).
    opts: std::collections::BTreeMap<String, String>,
}

impl Args {
    fn num(&self, key: &str, default: f64) -> f64 {
        self.opts.get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
    }
}

fn parse(rest: &[String]) -> Args {
    let mut content = None;
    let mut out = None;
    let mut only = None;
    let mut palette = None;
    let mut opts = std::collections::BTreeMap::new();
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--out" => out = Some(PathBuf::from(it.next().unwrap_or_else(|| usage()))),
            "--only" => only = Some(it.next().unwrap_or_else(|| usage()).split(',').map(|s| s.trim().to_string()).collect()),
            "--palette" => palette = Some(it.next().unwrap_or_else(|| usage()).clone()),
            s if s.starts_with("--") => {
                let v = it.next().unwrap_or_else(|| usage()).clone();
                opts.insert(s.trim_start_matches("--").to_string(), v);
            }
            s => content = Some(PathBuf::from(s)),
        }
    }
    Args { content: content.unwrap_or_else(|| usage()), out, only, palette, opts }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = args.first() else { usage() };
    let a = parse(&args[1..]);
    let forge = Forge::load(&a.content).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1)
    });
    let ok = match cmd.as_str() {
        "render" => render(&forge, a.out.as_deref().unwrap_or_else(|| usage()), a.only.as_deref()),
        "check" => check(&forge),
        "family" => family_cmd::run(&forge, a.out.as_deref().unwrap_or_else(|| usage())),
        "sweeps" => {
            let out = a.out.as_deref().unwrap_or_else(|| usage());
            std::fs::create_dir_all(out).ok();
            host_sweep::run(&forge.lib, out, a.only.as_deref());
            true
        }
        "view" => view(&forge, &a),
        "rates" => {
            explore_cmd::rates(&forge.lib, a.num("seed", 1.0) as u64, a.num("count", 12.0) as usize);
            true
        }
        "space" => {
            space_cmd::run(&forge.lib, a.out.as_deref().unwrap_or_else(|| usage()), a.num("seed", 1.0) as u64, a.num("count", 2000.0) as usize, a.opts.contains_key("csv-only"));
            true
        }
        "atlas" => {
            explore_cmd::atlas(&forge.lib, a.out.as_deref().unwrap_or_else(|| usage()), a.num("seed", 1.0) as u64);
            true
        }
        "ladder" => {
            let hull = a.opts.get("hull").map(|h| if h.starts_with("hull_") { h.clone() } else { format!("hull_{h}") }).unwrap_or_else(|| "hull_strider".into());
            let gear = a.opts.get("gear").cloned().unwrap_or_else(|| "legs".into());
            let weapon = a.opts.get("weapon").map(|w| if w.starts_with("turret_") { w.clone() } else { format!("turret_{w}") }).unwrap_or_else(|| "turret_gun".into());
            explore_cmd::ladder(&forge.lib, a.out.as_deref().unwrap_or_else(|| usage()), a.num("seed", 1.0) as u64, &hull, &gear, a.num("steps", 6.0) as usize, &weapon);
            true
        }
        "fit" => {
            let spec = a.opts.get("spec").map(PathBuf::from).unwrap_or_else(|| usage());
            explore_cmd::fit_file(&forge.lib, &spec, a.out.as_deref().unwrap_or_else(|| usage()))
        }
        "why" => {
            explore_cmd::why(&forge.lib, a.num("seed", 1.0) as u64, a.num("count", 12.0) as usize, &explore_cmd_spec(&a));
            true
        }
        "roll" => {
            let spec = explore_cmd_spec(&a);
            explore_cmd::roll(&forge.lib, a.out.as_deref().unwrap_or_else(|| usage()), a.num("seed", 1.0) as u64, a.num("count", 24.0) as usize, a.num("cols", 6.0) as usize, &spec);
            true
        }
        "gallery" => {
            let ids: Vec<String> = a.only.clone().unwrap_or_else(|| forge.designs.keys().cloned().collect());
            let title = a.opts.get("title").cloned().unwrap_or_else(|| "Designs".into());
            explore_cmd::gallery(&forge, &ids, a.out.as_deref().unwrap_or_else(|| usage()), a.num("cols", 4.0) as usize, a.num("size", 360.0) as usize, &title);
            true
        }
        "trial" => {
            let course = a.opts.get("course").map(PathBuf::from).unwrap_or_else(|| a.content.join("courses/hill_valley.ron"));
            let mode = a.opts.get("mode").map(|m| m.as_str()).unwrap_or("sim");
            let terrain = a.opts.get("terrain").map(PathBuf::from).unwrap_or_else(|| a.content.join("terrain.ron"));
            trial_cmd::run(&forge, &course, &terrain, a.only.as_deref(), a.opts.get("limit").and_then(|v| v.parse().ok()), mode, a.out.as_deref().unwrap_or_else(|| usage()))
        }
        "soil" => {
            let course = a.opts.get("course").map(PathBuf::from).unwrap_or_else(|| a.content.join("courses/hill_valley.ron"));
            let terrain = a.opts.get("terrain").map(PathBuf::from).unwrap_or_else(|| a.content.join("terrain.ron"));
            let design = a.opts.get("design").map(|d| d.as_str()).unwrap_or("lancer_mk1");
            soil_cmd::run_ladders(&forge, &a.content, design, &course, &terrain, a.out.as_deref().unwrap_or_else(|| usage()))
        }
        "lineup" => lineup(&forge, a.out.as_deref().unwrap_or_else(|| usage()), a.only.as_deref(), a.palette.as_deref()),
        "factions" => factions(&forge, a.out.as_deref().unwrap_or_else(|| usage()), a.only.as_deref()),
        _ => usage(),
    };
    if !ok {
        std::process::exit(1);
    }
}

/// One large view of a design: `w5k view <content> --design id --out file.png [--az 40 --el 20 --focus x,y,z
/// --radius r --size 1100x800 --palette name]`.
fn explore_cmd_spec(a: &Args) -> w5k_forge::explore::Spec {
    w5k_forge::explore::Spec {
        hull: a.opts.get("hull").map(|h| if h.starts_with("hull_") { h.clone() } else { format!("hull_{h}") }),
        gear: a.opts.get("gear").cloned(),
        weapon: a.opts.get("weapon").map(|w| if w.starts_with("turret_") { w.clone() } else { format!("turret_{w}") }),
        size: a.opts.get("size").and_then(|x| x.parse().ok()),
        palette: a.opts.get("palette").cloned(),
        turrets: a.opts.get("turrets").and_then(|x| x.parse().ok()),
    }
}

fn view(forge: &Forge, a: &Args) -> bool {
    let Some(id) = a.opts.get("design") else { usage() };
    let Some(d) = forge.designs.get(id) else {
        eprintln!("error: no design '{id}'");
        return false;
    };
    let (built, _, _) = forge.build_design(d);
    let pal = a.palette.clone().or_else(|| d.palette.clone());
    let colours = preview::palette_colours(&forge.lib, pal.as_deref());
    let mut fr = Frame::of(&built.mesh);
    if let Some(f) = a.opts.get("focus") {
        let v: Vec<f64> = f.split(',').filter_map(|x| x.parse().ok()).collect();
        if v.len() == 3 {
            fr.centre = w5k_forge::geom::v3(v[0], v[1], v[2]);
        }
    }
    if let Some(r) = a.opts.get("radius").and_then(|r| r.parse::<f64>().ok()) {
        fr.radius = r;
        fr.grid = preview::nice_step(r / 2.0);
    }
    let zoom = a.num("zoom", 1.0);
    if zoom != 1.0 {
        fr.radius *= zoom;
        fr.grid = preview::nice_step(fr.radius / 2.0);
    }
    let (w, h) = a
        .opts
        .get("size")
        .and_then(|s| s.split_once('x'))
        .and_then(|(w, h)| Some((w.parse::<usize>().ok()?, h.parse::<usize>().ok()?)))
        .unwrap_or((1100, 800));
    let img = preview::render_view_wh(&built.mesh, &colours, &fr, a.num("az", 35.0), a.num("el", 22.0), w, h);
    let path = match &a.out {
        Some(p) if p.extension().is_some() => p.clone(),
        Some(p) => {
            std::fs::create_dir_all(p).ok();
            p.join(format!("{id}_view.png"))
        }
        None => usage(),
    };
    match img.save_png(&path) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("error: {e}");
            false
        }
    }
}

fn wanted(only: Option<&[String]>, id: &str) -> bool {
    only.map(|o| o.iter().any(|x| x == id)).unwrap_or(true)
}

fn write(path: &Path, bytes: &[u8]) {
    if let Err(e) = std::fs::write(path, bytes) {
        eprintln!("error: {}: {e}", path.display());
        std::process::exit(1);
    }
}

fn emit(dir: &Path, stats: &StatsFile, built: &Built, colours: &[[f32; 3]; 8], sockets: &[GlbSocket], thumbs: &mut Vec<(String, Image)>) {
    let sheet = preview::contact_sheet(&built.mesh, colours, stats);
    if let Err(e) = sheet.save_png(&dir.join(format!("{}.png", stats.id))) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
    write(&dir.join(format!("{}.glb", stats.id)), &write_glb(&stats.id, &built.mesh, colours, sockets));
    let ron = ron::ser::to_string_pretty(stats, ron::ser::PrettyConfig::default()).expect("stats serialise");
    write(&dir.join(format!("{}.stats.ron", stats.id)), ron.as_bytes());
    let fr = Frame::of(&built.mesh);
    thumbs.push((stats.id.clone(), preview::render_view(&built.mesh, colours, &fr, 35.0, 28.0, 240)));
}

fn render(forge: &Forge, out: &Path, only: Option<&[String]>) -> bool {
    let (parts_dir, veh_dir) = (out.join("parts"), out.join("vehicles"));
    for d in [&parts_dir, &veh_dir] {
        std::fs::create_dir_all(d).unwrap_or_else(|e| {
            eprintln!("error: {}: {e}", d.display());
            std::process::exit(1)
        });
    }
    let mut ok = true;
    let mut thumbs = Vec::new();
    for (id, part) in &forge.parts {
        if !wanted(only, id) || part.generated {
            continue;
        }
        let t = Instant::now();
        let built = forge.build_part(id).expect("part exists");
        let stats = forge.stats_file(id, &part.def.name, &built, None);
        let colours = preview::palette_colours(&forge.lib, part.def.palette.as_deref());
        let sockets: Vec<GlbSocket> = part
            .def
            .sockets
            .iter()
            .map(|s| GlbSocket { name: s.name.clone(), at: V3::from_arr(s.at), normal: V3::from_arr(s.normal), forward: V3::from_arr(s.forward) })
            .collect();
        emit(&parts_dir, &stats, &built, &colours, &sockets, &mut thumbs);
        println!(
            "part    {id:<24} {:>9.1} kg {:>6} tris  voxel {:>3.0} mm  {:>5.2}s",
            stats.mass.mass_kg,
            stats.triangles,
            stats.voxel_cell_m * 1000.0,
            t.elapsed().as_secs_f64()
        );
    }
    for (id, design) in &forge.designs {
        if !wanted(only, id) {
            continue;
        }
        let t = Instant::now();
        let (built, asm, sheet) = forge.build_design(design);
        ok &= sheet.problems.is_empty();
        let stats = forge.stats_file(id, &design.name, &built, Some(sheet));
        let colours = preview::palette_colours(&forge.lib, design.palette.as_deref());
        let sockets: Vec<GlbSocket> = asm
            .sockets
            .iter()
            .map(|s| GlbSocket { name: format!("{}_{}_{}", s.part_index, asm.parts[s.part_index].0, s.def.name), at: s.at, normal: s.normal, forward: s.forward })
            .collect();
        emit(&veh_dir, &stats, &built, &colours, &sockets, &mut thumbs);
        let v = stats.vehicle.as_ref().unwrap();
        println!(
            "vehicle {id:<24} {:>9.1} kg {:>6} tris  {:>5.1} km/h  {:>5.2}s{}",
            stats.mass.mass_kg,
            stats.triangles,
            v.top_speed_kmh,
            t.elapsed().as_secs_f64(),
            if v.problems.is_empty() { String::new() } else { format!("  PROBLEMS: {}", v.problems.join("; ")) }
        );
    }
    if !thumbs.is_empty() {
        let ov = preview::overview(&thumbs, 5);
        if let Err(e) = ov.save_png(&out.join("overview.png")) {
            eprintln!("error: {e}");
            return false;
        }
    }
    ok
}

/// One image of several vehicles at true relative scale, all in one army's palette (`--palette`, or the first
/// design's).
fn lineup(forge: &Forge, out: &Path, only: Option<&[String]>, palette: Option<&str>) -> bool {
    let ids: Vec<String> = match only {
        Some(o) => o.to_vec(),
        None => forge.designs.keys().cloned().collect(),
    };
    let mut built = Vec::new();
    for id in &ids {
        let Some(d) = forge.designs.get(id) else {
            eprintln!("error: no design '{id}'");
            return false;
        };
        let (b, _, _) = forge.build_design(d);
        built.push((b, d.name.clone(), d.palette.clone()));
    }
    let pal = palette.map(|s| s.to_string()).or_else(|| built.first().and_then(|b| b.2.clone()));
    let colours = preview::palette_colours(&forge.lib, pal.as_deref());
    let units: Vec<(&w5k_forge::mesh::Mesh, &str)> = built.iter().map(|(b, n, _)| (&b.mesh, n.as_str())).collect();
    std::fs::create_dir_all(out).ok();
    let img = preview::lineup(&units, &colours, 1600, 700);
    match img.save_png(&out.join(format!("lineup_{}.png", pal.unwrap_or_else(|| "default".into())))) {
        Ok(()) => true,
        Err(e) => {
            eprintln!("error: {e}");
            false
        }
    }
}

/// The first given design in each faction palette, side by side.
fn factions(forge: &Forge, out: &Path, only: Option<&[String]>) -> bool {
    let id = only.and_then(|o| o.first().cloned()).unwrap_or_else(|| forge.designs.keys().next().cloned().unwrap_or_default());
    let Some(d) = forge.designs.get(&id) else {
        eprintln!("error: no design '{id}'");
        return false;
    };
    let (b, _, _) = forge.build_design(d);
    let fr = Frame::of(&b.mesh);
    let names = ["vanguard", "crimson", "verdant", "ultraviolet"];
    let tile = 400;
    let mut img = Image::new(tile * names.len(), tile + 24, [0.018, 0.019, 0.022]);
    for (k, name) in names.iter().enumerate() {
        let colours = preview::palette_colours(&forge.lib, Some(name));
        let view = preview::render_view(&b.mesh, &colours, &fr, 35.0, 28.0, tile);
        img.blit(&view, k * tile, 0);
        img.text((k * tile) as i64 + 8, tile as i64 + 6, 1, name, [0.8, 0.82, 0.86]);
    }
    std::fs::create_dir_all(out).ok();
    img.save_png(&out.join(format!("factions_{id}.png"))).is_ok()
}

fn check(forge: &Forge) -> bool {
    let mut ok = true;
    for (id, part) in &forge.parts {
        if part.generated {
            continue;
        }
        let b = forge.build_part(id).expect("part exists");
        println!("part    {id:<24} {:>9.1} kg {:>6} tris", b.mass.mass_kg, b.mesh.triangle_count());
    }
    for (id, d) in &forge.designs {
        let (_, _, sheet) = forge.build_design(d);
        let status = if sheet.problems.is_empty() { "ok".to_string() } else { sheet.problems.join("; ") };
        ok &= sheet.problems.is_empty();
        println!("vehicle {id:<24} {:>9.1} kg {:>5.1} km/h  {status}", sheet.mass_kg, sheet.top_speed_kmh);
    }
    ok
}
