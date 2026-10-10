//! `w5k world`: the command line of lane WORLD (only that lane edits this file).
//!
//! `w5k world preview <course.ron> --out DIR` generates the course and writes `topdown.png` (hill-shaded, materials coloured, props
//! as dots).

use std::path::PathBuf;

use w5k_contract::world::{PropKind, WorldQuery};
use w5k_math::Vec3;
use w5k_world::course::{generate, CourseDef};

// Display-only colours and lighting for the preview (never feed the simulation).
const ASPHALT_RGB: [f64; 3] = [150.0, 150.0, 156.0]; // const-ok: display colour
const MUD_RGB: [f64; 3] = [105.0, 80.0, 52.0]; // const-ok: display colour
const GROUND_RGB: [f64; 3] = [95.0, 135.0, 78.0]; // const-ok: display colour
const HEIGHT_TINT_PER_M: f64 = 5.0; // const-ok: display colour ramp
const LIGHT_DIR: [f64; 3] = [-0.5, 0.8, -0.4]; // const-ok: display light direction
const SHADE_GAIN: f64 = 0.7; // const-ok: hill-shade contrast
const SHADE_AMBIENT: f64 = 0.3; // const-ok: hill-shade ambient
const CHANNEL_MAX: f64 = 255.0; // const-ok: 8-bit channel

/// Entry point for `w5k world <args>`.
pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("preview") => preview(&args[1..]),
        _ => Err("usage: w5k world preview <course.ron> --out DIR".to_string()),
    }
}

fn preview(args: &[String]) -> Result<(), String> {
    let (mut file, mut out) = (None, None);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--out" {
            out = it.next().map(PathBuf::from);
        } else {
            file = Some(a.clone());
        }
    }
    let (file, out) = (file.ok_or("missing <course.ron>")?, out.ok_or("missing --out DIR")?);
    let def = CourseDef::from_ron(&std::fs::read_to_string(&file).map_err(|e| format!("{file}: {e}"))?)?;
    let course = generate(&def)?;
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;

    let w = &course.world;
    let (lo, hi) = w.bounds();
    let px = 2usize; // pixels per metre (cells are 1 m)
    let side = ((hi.x - lo.x) as usize + 1) * px;
    let light = Vec3::new(LIGHT_DIR[0], LIGHT_DIR[1], LIGHT_DIR[2]).normalized_or_zero();
    let mut img = vec![0u8; side * side * 3];
    for r in 0..side {
        for c in 0..side {
            let (x, z) = (lo.x + c as f64 / px as f64, lo.z + r as f64 / px as f64);
            let shade = (w.normal(x, z).dot(light) * SHADE_GAIN + SHADE_AMBIENT).clamp(0.0, 1.0);
            let name = &w.material_at(x, z).name;
            let base = match name.as_str() {
                "asphalt" => ASPHALT_RGB,
                "mud" => MUD_RGB,
                _ => [GROUND_RGB[0] + (w.height_m(x, z) - lo.y) * HEIGHT_TINT_PER_M, GROUND_RGB[1], GROUND_RGB[2]],
            };
            for k in 0..3 {
                img[(r * side + c) * 3 + k] = (base[k] * shade).min(CHANNEL_MAX) as u8;
            }
        }
    }
    let mut dot = |x: f64, z: f64, rad: i64, rgb: [u8; 3]| {
        let (cx, cz) = (((x - lo.x) * px as f64) as i64, ((z - lo.z) * px as f64) as i64);
        for dz in -rad..=rad {
            for dx in -rad..=rad {
                let (c, r) = (cx + dx, cz + dz);
                if dx * dx + dz * dz <= rad * rad && c >= 0 && r >= 0 && (c as usize) < side && (r as usize) < side {
                    img[(r as usize * side + c as usize) * 3..][..3].copy_from_slice(&rgb);
                }
            }
        }
    };
    for p in w.props() {
        match p.kind {
            PropKind::Tree => dot(p.transform.pos.x, p.transform.pos.z, 2, [20, 70, 30]),
            PropKind::Barricade | PropKind::Wall => dot(p.transform.pos.x, p.transform.pos.z, 3, [200, 60, 40]),
            _ => dot(p.transform.pos.x, p.transform.pos.z, 3, [90, 90, 120]),
        }
    }
    let (s, f) = (course.road[0], course.road[course.road.len() - 1]);
    dot(s.0, s.1, 6, [40, 200, 60]);
    dot(f.0, f.1, 6, [230, 200, 40]);

    let path = out.join("topdown.png");
    let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), side as u32, side as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    wr.write_image_data(&img).map_err(|e| e.to_string())?;
    println!(
        "wrote {} ({} x {} px, {} props, road {} points)",
        path.display(),
        side,
        side,
        w.props().len(),
        course.road.len()
    );
    Ok(())
}
