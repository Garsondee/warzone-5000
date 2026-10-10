//! `w5k geometry`: the command line of lane GEOMETRY (only that lane edits this file). Pictures of generated parts.

use w5k_contract::render::NodeRole;
use w5k_geo::export::{glb, render_rig, slot_colour};
use w5k_geo::flags::{bake, FlagParams};
use w5k_geo::mesh::Mesh;
use w5k_geo::part::Part;
use w5k_geo::raster::{render, Camera, Item, Mode};
use w5k_geo::truck::{utility_4x4, UtilityDims};
use w5k_geo::wheel::{segments_for, wheel, WheelDims};
use w5k_math::Vec3;

const USAGE: &str =
    "usage: w5k geometry sheet <wheel|truck> --out DIR [--mode look|shaded|edge|cavity] [--detail 0|1|2]";

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
            let parts = subject(what, detail)?;
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            let path = format!("{out}/{what}-{}.png", get("--mode").unwrap_or_else(|| "look".into()));
            write_png(&path, 1200, 800, &sheet(&parts, mode, 600, 400))?;
            println!("wrote {path}");
            Ok(())
        }
        (Some("export"), Some(what)) if what == "truck" => {
            let out = get("--out").ok_or(USAGE)?;
            let detail: u8 = get("--detail").map_or(Ok(1), |d| d.parse()).map_err(|_| USAGE.to_string())?;
            let (dims, parts) = (UtilityDims::placeholder(), utility_4x4(&UtilityDims::placeholder(), detail));
            let rig = render_rig("utility_4x4", &parts, &FlagParams::default_params());
            rig.validate().map_err(|e| e.join("; "))?;
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            std::fs::write(
                format!("{out}/utility_4x4.renderrig.json"),
                serde_json::to_string(&rig).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            std::fs::write(format!("{out}/utility_4x4.glb"), glb(&rig)).map_err(|e| e.to_string())?;
            let table = dimension_table(&dims, &parts, rig.triangle_count());
            std::fs::write(format!("{out}/utility_4x4.dimensions.md"), &table).map_err(|e| e.to_string())?;
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
    s + &format!("\nTriangles in the rig: {triangles} (budget 40,000 for a wheeled vehicle).\n")
}

/// Parts as (mesh, base colour).
fn subject(what: &str, detail: u8) -> Result<Vec<(Mesh, [f64; 3])>, String> {
    match what {
        "wheel" => {
            let w = wheel(&WheelDims::placeholder(), segments_for(detail));
            // const-ok: picture colours, not physics
            Ok(vec![
                (w.tyre, [0.10, 0.10, 0.10]), // const-ok: picture colours and camera framing
                (w.lugs, [0.13, 0.13, 0.13]), // const-ok: picture colours and camera framing
                (w.rim, [0.55, 0.57, 0.52]),  // const-ok: picture colours and camera framing
                (w.nuts, [0.7, 0.7, 0.7]),    // const-ok: picture colours and camera framing
            ])
        }
        "truck" => {
            let parts = utility_4x4(&UtilityDims::placeholder(), detail);
            Ok(parts.iter().map(|p| (p.in_hull_frame(), slot_colour(p.slot))).collect())
        }
        _ => Err(format!("unknown subject {what}; {USAGE}")),
    }
}

/// Four views in a 2 x 2 sheet: three-quarter, side, front, top.
fn sheet(parts: &[(Mesh, [f64; 3])], mode: Mode, w: usize, h: usize) -> Vec<u8> {
    let flags = bake(&parts.iter().map(|p| &p.0).collect::<Vec<_>>(), &FlagParams::default_params());
    let items: Vec<Item> = parts
        .iter()
        .zip(&flags)
        .map(|(p, (m, f))| Item { mesh: m, colour: p.1, edge: &f.edge, cavity: &f.cavity })
        .collect();
    let (lo, hi) = parts
        .iter()
        .map(|p| p.0.bounds())
        .fold((Vec3::splat(f64::MAX), Vec3::splat(f64::MIN)), |(a, b), (l, u)| (a.min(l), b.max(u)));
    let (c, ext) = ((lo + hi) * 0.5, (hi - lo).length());
    let views = [
        // const-ok: picture colours and camera framing, not physics
        (Vec3::new(0.9, 0.6, -1.0), Some(0.5)),
        (Vec3::new(1.0, 0.0, 0.0), None),
        (Vec3::new(0.0, 0.0, 1.0), None),
        // const-ok: picture colours and camera framing, not physics
        (Vec3::new(0.0, 1.0, 0.001), None),
    ];
    let panes: Vec<Vec<u8>> = views
        .iter()
        .map(|&(dir, fov)| {
            let cam = Camera {
                eye: c + dir.normalized_or_zero() * ext * 1.7, // const-ok: picture colours and camera framing
                target: c,
                fov_rad: fov,
                ortho_half_h_m: ext * 0.40, // const-ok: picture colours and camera framing
            };
            // const-ok: picture colours and camera framing, not physics
            render(&items, &cam, w, h, mode, [0.93, 0.93, 0.92])
        })
        .collect();
    let mut out = vec![0u8; 2 * w * 2 * h * 3];
    for (i, p) in panes.iter().enumerate() {
        let (ox, oy) = ((i % 2) * w, (i / 2) * h);
        for y in 0..h {
            let dst = ((oy + y) * 2 * w + ox) * 3;
            out[dst..dst + w * 3].copy_from_slice(&p[y * w * 3..(y + 1) * w * 3]);
        }
    }
    out
}

fn write_png(path: &str, w: u32, h: u32, rgb: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().and_then(|mut wr| wr.write_image_data(rgb)).map_err(|e| e.to_string())
}
