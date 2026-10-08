//! `w5k`: command-line tools for warzone-5000.
//!
//! ```text
//! w5k render <content-dir> --out <dir> [--only id,id,...]   contact sheets, GLB and stats for parts and vehicles
//! w5k check  <content-dir>                                 build everything and report problems
//! w5k family <content-dir> --out <dir>                     sweep and slider-coupling sheets for parametric families
//! ```

mod family_cmd;

use std::path::{Path, PathBuf};
use std::time::Instant;

use w5k_forge::geom::V3;
use w5k_forge::gltf::{write_glb, GlbSocket};
use w5k_forge::preview::{self, Frame};
use w5k_forge::raster::Image;
use w5k_forge::{Built, Forge, StatsFile};

fn usage() -> ! {
    eprintln!("usage:\n  w5k render <content-dir> --out <dir> [--only id,id,...]\n  w5k check <content-dir>\n  w5k family <content-dir> --out <dir>\n  w5k lineup <content-dir> --out <dir> [--only design,design,...]");
    std::process::exit(2)
}

struct Args {
    content: PathBuf,
    out: Option<PathBuf>,
    only: Option<Vec<String>>,
    palette: Option<String>,
}

fn parse(rest: &[String]) -> Args {
    let mut content = None;
    let mut out = None;
    let mut only = None;
    let mut palette = None;
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--out" => out = Some(PathBuf::from(it.next().unwrap_or_else(|| usage()))),
            "--only" => only = Some(it.next().unwrap_or_else(|| usage()).split(',').map(|s| s.trim().to_string()).collect()),
            "--palette" => palette = Some(it.next().unwrap_or_else(|| usage()).clone()),
            s if s.starts_with("--") => usage(),
            s => content = Some(PathBuf::from(s)),
        }
    }
    Args { content: content.unwrap_or_else(|| usage()), out, only, palette }
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
        "lineup" => lineup(&forge, a.out.as_deref().unwrap_or_else(|| usage()), a.only.as_deref(), a.palette.as_deref()),
        "factions" => factions(&forge, a.out.as_deref().unwrap_or_else(|| usage()), a.only.as_deref()),
        _ => usage(),
    };
    if !ok {
        std::process::exit(1);
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
