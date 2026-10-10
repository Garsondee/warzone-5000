//! `w5k geometry`: the command line of lane GEOMETRY (only that lane edits this file). Pictures of generated parts.

use w5k_contract::render::NodeRole;
use w5k_geo::budget::wheeled_triangles;
use w5k_geo::export::{glb, render_rig, slot_colour};
use w5k_geo::flags::{bake, FlagParams};
use w5k_geo::mesh::Mesh;
use w5k_geo::mount::{ring_mount, RingMountDims};
use w5k_geo::part::Part;
use w5k_geo::raster::{render, Camera, Item, Mode};
use w5k_geo::skin::Skin;
use w5k_geo::truck::{utility_4x4, utility_assembly, utility_hull, utility_truck, UtilityDims};
use w5k_geo::weapon::{gun_module, GunDims};
use w5k_geo::wheel::{segments_for, wheel, WheelDims};
use w5k_math::Vec3;

const USAGE: &str =
    "usage: w5k geometry sheet <wheel|truck|scout|hauler|hull|truck6|truck-ring|truck-mg|truck-ac25>[,more subjects, stacked] --out DIR [--mode look|shaded|edge|cavity] [--detail 0|1|2] [--view front34,rear34,side,front,rear,top,low34,close,gun] [--size WxH]";

/// Entry point for `w5k geometry <args>`.
pub fn run(args: &[String]) -> Result<(), String> {
    let get = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    match (args.first().map(String::as_str), args.get(1)) {
        (Some("sheet"), Some(what)) => {
            let out = get("--out").ok_or(USAGE)?;
            let mode = match get("--mode").as_deref() {
                None | Some("look") => Mode::Look,
                Some("shaded") => Mode::Shaded,
                Some("edge") => Mode::Edge,
                Some("cavity") => Mode::Cavity,
                Some(m) => return Err(format!("unknown mode {m}; {USAGE}")),
            };
            let detail: u8 = get("--detail").map_or(Ok(1), |d| d.parse()).map_err(|_| USAGE.to_string())?;
            let subjects: Vec<&str> = what.split(',').collect();
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            let mode_name = get("--mode").unwrap_or_else(|| "look".into());
            let view = get("--view");
            let views: Vec<&str> =
                view.as_deref().map_or(vec!["front34", "side", "front", "top"], |v| v.split(',').collect());
            let (w, h) = match get("--size").as_deref().and_then(|s| s.split_once('x')) {
                Some((a, b)) => (a.parse().map_err(|_| USAGE.to_string())?, b.parse().map_err(|_| USAGE.to_string())?),
                None if views.len() == 1 => (1400, 900), // const-ok: picture size in pixels
                None => (600, 400),                      // const-ok: picture size in pixels
            };
            let what = what.replace(',', "+");
            let path = match &view {
                Some(v) if views.len() == 1 => format!("{out}/{what}-{v}-{mode_name}.png"),
                _ => format!("{out}/{what}-{mode_name}.png"),
            };
            let cols = if views.len() == 1 { 1 } else { 2 };
            let (pw, ph) = (cols * w, subjects.len() * views.len().div_ceil(cols) * h);
            let mut pixels = Vec::new();
            let all = subjects.iter().map(|name| subject(name, detail)).collect::<Result<Vec<_>, _>>()?;
            // one scale for every tile, the largest vehicle's, so that sizes can be compared
            let min_ext = all.iter().map(|p| extent(p)).fold(0.0, f64::max);
            for parts in &all {
                pixels.extend(sheet(parts, mode, &views, w, h, min_ext)?);
            }
            write_png(&path, pw as u32, ph as u32, &pixels)?;
            println!("wrote {path}");
            Ok(())
        }
        (Some("export"), Some(what)) if skin_id(what).is_some() => {
            let out = get("--out").ok_or(USAGE)?;
            let detail: u8 = get("--detail").map_or(Ok(1), |d| d.parse()).map_err(|_| USAGE.to_string())?;
            let skin = skin_id(what).and_then(Skin::for_id).ok_or(USAGE)?;
            let (id, parts) = (skin.kind.id(), skin.parts(detail));
            let rig = render_rig(id, &parts, &FlagParams::default_params());
            rig.validate().map_err(|e| e.join("; "))?;
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            std::fs::write(
                format!("{out}/{id}.renderrig.json"),
                serde_json::to_string(&rig).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            std::fs::write(format!("{out}/{id}.glb"), glb(&rig)).map_err(|e| e.to_string())?;
            let table = dimension_table(&skin.dims, &parts, rig.triangle_count());
            std::fs::write(format!("{out}/{id}.dimensions.md"), &table).map_err(|e| e.to_string())?;
            print!("{table}");
            Ok(())
        }
        _ => Err(USAGE.to_string()),
    }
}

/// Generated against stated, in percent. PLACEHOLDER until the dossier lands (`PROVISIONAL(C-002)`): "stated" is the number the part list was scaled to.
fn dimension_table(d: &UtilityDims, parts: &[Part], triangles: usize) -> String {
    let (lo, hi) = parts
        .iter()
        .filter(|p| p.role == NodeRole::Hull && !p.fitting)
        .map(|p| p.in_hull_frame().bounds())
        .fold((Vec3::splat(f64::MAX), Vec3::splat(f64::MIN)), |(a, b), (l, u)| (a.min(l), b.max(u)));
    let hubs: Vec<Vec3> =
        parts.iter().filter(|p| p.role == NodeRole::Wheel && p.name.starts_with("rim")).map(|p| p.pose.pos).collect();
    let (xmax, zmin, zmax) =
        hubs.iter().fold((0.0_f64, f64::MAX, f64::MIN), |(x, a, b), h| (x.max(h.x), a.min(h.z), b.max(h.z)));
    let tyre = parts
        .iter()
        .find(|p| p.name == "tread.0.r")
        .map_or(0.0, |p| p.mesh.v.iter().fold(0.0_f64, |r, v| r.max(w5k_math::scalar::hypot(v.y, v.z))));
    let rows = [
        ("length (hull box, no fittings)", hi.z - lo.z, d.length_m),
        ("width (hull box, no fittings)", hi.x - lo.x, d.width_m),
        ("height (hull box, no fittings)", hi.y - lo.y, d.height_m),
        ("wheelbase (hub to hub)", zmax - zmin, d.wheelbase_m),
        ("track (hub to hub)", 2.0 * xmax, d.track_m),
        ("wheel diameter (over the tread)", 2.0 * tyre, 2.0 * d.wheel.outer_radius_m),
        ("ground clearance (box bottom to ground)", d.ride_height_m() + lo.y, d.ground_clearance_m),
    ];
    let mut s = String::from("PLACEHOLDER dimensions, PROVISIONAL(C-002): generated against the numbers the part list was scaled to; the dossier comparison (A10) waits for VALIDATION.\n\n| dimension | generated (m) | stated (m) | error |\n|---|---|---|---|\n");
    for (name, got, want) in rows {
        // const-ok: percent
        s += &format!("| {name} | {got:.3} | {want:.3} | {:+.2}% |\n", (got / want - 1.0) * 100.0);
    }
    s + &format!("\nTriangles in the rig: {triangles} (budget {} for a wheeled vehicle).\n", wheeled_triangles())
}

/// The skin id of a subject name (`truck` is the utility truck), if it is a skin.
fn skin_id(subject: &str) -> Option<&'static str> {
    match subject {
        "truck" => Some("utility_4x4"),
        "scout" => Some("scout_4x4"),
        "hauler" => Some("hauler_4x4"),
        _ => None,
    }
}

/// Parts as (mesh, base colour, counts for the camera framing: a tall antenna should not move the view).
fn subject(what: &str, detail: u8) -> Result<Vec<(Mesh, [f64; 3], bool)>, String> {
    match what {
        "wheel" => {
            let w = wheel(&WheelDims::placeholder(), segments_for(detail));
            // const-ok: picture colours, not physics
            Ok(vec![
                (w.tyre, [0.10, 0.10, 0.10], true), // const-ok: picture colours and camera framing
                (w.lugs, [0.13, 0.13, 0.13], true), // const-ok: picture colours and camera framing
                (w.rim, [0.55, 0.57, 0.52], true),  // const-ok: picture colours and camera framing
                (w.nuts, [0.7, 0.7, 0.7], true),    // const-ok: picture colours and camera framing
            ])
        }
        "scout" | "hauler" => {
            let skin = skin_id(what).and_then(Skin::for_id).ok_or("no such skin")?;
            Ok(skin
                .parts(detail)
                .iter()
                .map(|p| (p.in_hull_frame(), slot_colour(p.slot), p.name != "antenna"))
                .collect())
        }
        "truck" | "hull" | "truck6" | "truck-ring" | "truck-mg" | "truck-ac25" => {
            let d = UtilityDims::placeholder();
            let z = d.wheelbase_m / 2.0;
            let parts = match what {
                "hull" => utility_hull(&d, &[-z, z], detail).parts,
                // front steer axle and a rear tandem 1.2 m apart
                "truck6" => utility_truck(&d, &[-z, z - 1.2, z], &[true, false, false], detail), // const-ok: tandem spacing for the picture
                "truck-ring" => mounted(&d, None, detail)?,
                "truck-mg" => mounted(&d, Some("machine_gun_12_7"), detail)?,
                "truck-ac25" => mounted(&d, Some("autocannon_25"), detail)?,
                _ => utility_4x4(&d, detail),
            };
            Ok(parts.iter().map(|p| (p.in_hull_frame(), slot_colour(p.slot), p.name != "antenna")).collect())
        }
        _ => Err(format!("unknown subject {what}; {USAGE}")),
    }
}

/// The 4x4 with the ring mount on its roof socket and, if one is named, that gun on the mount's trunnion.
fn mounted(d: &UtilityDims, gun: Option<&str>, detail: u8) -> Result<Vec<Part>, String> {
    let z = d.wheelbase_m / 2.0;
    let mut asm = utility_assembly(d, &[-z, z], &[true, false], detail);
    asm.attach("roof", &ring_mount(&RingMountDims::standard(), detail), 0.0, "ring").map_err(|e| e.to_string())?;
    if let Some(name) = gun {
        let cradle =
            asm.socket("trunnion.ring").and_then(|s| s.hint("cradle_w_m")).ok_or("the mount publishes no cradle")?;
        let dims = GunDims::preset(name).ok_or(format!("no gun preset {name}"))?;
        asm.attach("trunnion.ring", &gun_module(&dims, cradle, detail), 0.0, "gun").map_err(|e| e.to_string())?;
    }
    Ok(asm.parts)
}

/// Named camera directions (from the vehicle centre towards the eye, in the vehicle frame: -Z is forward) and projection.
// const-ok: picture camera directions, not physics
const VIEWS: [(&str, [f64; 3], bool); 9] = [
    ("front34", [0.9, 0.6, -1.0], true), // const-ok: camera direction
    ("rear34", [0.9, 0.5, 1.0], true),   // const-ok: camera direction
    ("side", [1.0, 0.0, 0.0], false),
    ("front", [0.0, 0.0, -1.0], false),
    ("rear", [0.0, 0.0, 1.0], false),
    ("top", [0.0, 1.0, 0.001], false),    // const-ok: camera direction
    ("low34", [-0.9, -0.15, -1.0], true), // const-ok: camera direction
    ("close", [1.0, 0.35, -0.55], true),  // const-ok: camera direction
    ("gun", [0.8, 0.45, -1.0], true),     // const-ok: camera direction
];

/// The diagonal of the box around the parts that count for the camera framing.
fn extent(parts: &[(Mesh, [f64; 3], bool)]) -> f64 {
    let (lo, hi) = parts
        .iter()
        .filter(|p| p.2)
        .map(|p| p.0.bounds())
        .fold((Vec3::splat(f64::MAX), Vec3::splat(f64::MIN)), |(a, b), (l, u)| (a.min(l), b.max(u)));
    (hi - lo).length()
}

/// One picture per name in `views`: a single view fills the image, four views make a 2 x 2 sheet.
fn sheet(
    parts: &[(Mesh, [f64; 3], bool)],
    mode: Mode,
    views: &[&str],
    w: usize,
    h: usize,
    min_ext: f64,
) -> Result<Vec<u8>, String> {
    let flags = bake(&parts.iter().map(|p| &p.0).collect::<Vec<_>>(), &FlagParams::default_params());
    let items: Vec<Item> = parts
        .iter()
        .zip(&flags)
        .map(|(p, (m, f))| Item { mesh: m, colour: p.1, edge: &f.edge, cavity: &f.cavity })
        .collect();
    let (lo, hi) = parts
        .iter()
        .filter(|p| p.2)
        .map(|p| p.0.bounds())
        .fold((Vec3::splat(f64::MAX), Vec3::splat(f64::MIN)), |(a, b), (l, u)| (a.min(l), b.max(u)));
    let (c, ext) = ((lo + hi) * 0.5, (hi - lo).length().max(min_ext));
    let cols = if views.len() == 1 { 1 } else { 2 };
    let mut out = vec![0u8; cols * w * views.len().div_ceil(cols) * h * 3];
    for (i, name) in views.iter().enumerate() {
        let &(_, dir, persp) = VIEWS.iter().find(|v| v.0 == *name).ok_or(format!("unknown view {name}"))?;
        let single = views.len() == 1;
        let close = *name == "close" || *name == "gun";
        // the close-up looks at the greenhouse: a little above and ahead of the vehicle centre, from nearer; `gun` at the roof mount
        let focus = match *name {
            "close" => c + Vec3::new(0.0, ext * 0.09, -ext * 0.06), // const-ok: camera framing
            "gun" => c + Vec3::new(0.0, ext * 0.12, ext * 0.03),    // const-ok: camera framing
            _ => c,
        };
        let cam = Camera {
            eye: focus + Vec3::new(dir[0], dir[1], dir[2]).normalized_or_zero() * ext * if close { 0.6 } else { 1.7 }, // const-ok: camera framing
            target: focus,
            // const-ok: camera field of view in radians
            fov_rad: persp.then_some(if close {
                0.45 // const-ok: camera field of view
            } else if single {
                0.42 // const-ok: camera field of view
            } else {
                0.5 // const-ok: camera field of view
            }),
            ortho_half_h_m: ext * if single { 0.30 } else { 0.40 }, // const-ok: camera framing
        };
        // const-ok: picture background
        let pane = render(&items, &cam, w, h, mode, [0.93, 0.93, 0.92]);
        let (ox, oy) = ((i % cols) * w, (i / cols) * h);
        for y in 0..h {
            let dst = ((oy + y) * cols * w + ox) * 3;
            out[dst..dst + w * 3].copy_from_slice(&pane[y * w * 3..(y + 1) * w * 3]);
        }
    }
    Ok(out)
}

fn write_png(path: &str, w: u32, h: u32, rgb: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().and_then(|mut wr| wr.write_image_data(rgb)).map_err(|e| e.to_string())
}
