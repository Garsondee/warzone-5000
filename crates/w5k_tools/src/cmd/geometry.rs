//! `w5k geometry`: the command line of lane GEOMETRY (only that lane edits this file). Pictures of generated parts.

use w5k_contract::render::SlotKind;
use w5k_geo::flags::{bake, FlagParams};
use w5k_geo::mesh::Mesh;
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
        _ => Err(USAGE.to_string()),
    }
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

/// Picture colours by material slot kind (LOOK owns the real materials).
fn slot_colour(k: SlotKind) -> [f64; 3] {
    match k {
        SlotKind::Paint => [0.30, 0.34, 0.22],  // const-ok: picture colours, not physics
        SlotKind::Metal => [0.45, 0.46, 0.47],  // const-ok: picture colours, not physics
        SlotKind::Rubber => [0.10, 0.10, 0.10], // const-ok: picture colours, not physics
        SlotKind::Glass => [0.30, 0.45, 0.55],  // const-ok: picture colours, not physics
        SlotKind::Canvas => [0.50, 0.46, 0.34], // const-ok: picture colours, not physics
        SlotKind::Optics => [0.85, 0.80, 0.55], // const-ok: picture colours, not physics
        _ => [0.5, 0.5, 0.5],
    }
} // const-ok: picture colours, not physics

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
