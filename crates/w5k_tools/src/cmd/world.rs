//! `w5k world`: the command line of lane WORLD (only that lane edits this file).
//!
//! `w5k world preview <course.ron> --out DIR` generates the course and writes `topdown.png` (hill-shaded, materials coloured, props
//! as dots). `w5k world export <course.ron> --out DIR [--step N]` writes `terrain.json` for the viewers (format in
//! `docs/swarm/requests/world-viewer-terrain.md`); a replay's `WorldHeader.terrain` names that file.

use std::path::PathBuf;

use w5k_contract::world::{PropKind, PropShape, WorldQuery};
use w5k_math::Vec3;
use w5k_world::course::{generate, CourseDef};
use w5k_world::grid::CELL_M;

// Display-only colours and lighting for the preview (never feed the simulation).
const ASPHALT_RGB: [f64; 3] = [150.0, 150.0, 156.0]; // const-ok: display colour
const GRAVEL_RGB: [f64; 3] = [140.0, 132.0, 118.0]; // const-ok: display colour
const WATER_RGB: [f64; 3] = [40.0, 90.0, 150.0]; // const-ok: display colour
const WATER_DEEP_RGB: [f64; 3] = [15.0, 40.0, 100.0]; // const-ok: display colour
const WATER_DEPTH_FULL_M: f64 = 2.5; // const-ok: depth at which the water colour is darkest
const PLANKS_RGB: [f64; 3] = [120.0, 90.0, 55.0]; // const-ok: display colour
const TRACK_RGB: [u8; 3] = [200, 170, 110]; // const-ok: display colour of a track's centreline
const SAND_RGB: [f64; 3] = [215.0, 190.0, 125.0]; // const-ok: display colour
const MUD_RGB: [f64; 3] = [105.0, 80.0, 52.0]; // const-ok: display colour
const GROUND_RGB: [f64; 3] = [95.0, 135.0, 78.0]; // const-ok: display colour
const HEIGHT_TINT_PER_M: f64 = 5.0; // const-ok: display colour ramp
const LIGHT_DIR: [f64; 3] = [-0.5, 0.8, -0.4]; // const-ok: display light direction
const SHADE_GAIN: f64 = 0.7; // const-ok: hill-shade contrast
const SHADE_AMBIENT: f64 = 0.3; // const-ok: hill-shade ambient
const MM_PER_M: f64 = 1000.0; // const-ok: unit conversion for the export's rounding
const CHANNEL_MAX: f64 = 255.0; // const-ok: 8-bit channel

/// Entry point for `w5k world <args>`.
pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("preview") => preview(&args[1..]),
        Some("export") => export(&args[1..]),
        _ => Err("usage: w5k world preview|export <course.ron> --out DIR [--step N]".to_string()),
    }
}

/// A heightfield mesh for the viewers: every `step`-th node of the grid (heights rounded to the millimetre, the viewer never sees more
/// than it can draw), the material id per node and names, and the props. Plan x runs along `x_m`, z along `z_m`; `+Y` is up.
fn export(args: &[String]) -> Result<(), String> {
    let (mut file, mut out, mut step) = (None, None, 2usize);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--out" => out = it.next().map(PathBuf::from),
            "--step" => step = it.next().and_then(|v| v.parse().ok()).ok_or("--step needs a whole number")?,
            _ => file = Some(a.clone()),
        }
    }
    let (file, out) = (file.ok_or("missing <course.ron>")?, out.ok_or("missing --out DIR")?);
    let def = CourseDef::from_ron(&std::fs::read_to_string(&file).map_err(|e| format!("{file}: {e}"))?)?;
    let course = generate(&def)?;
    let w = &course.world;
    let n = w.n();
    if step == 0 || (n - 1) % step != 0 {
        return Err(format!("--step must divide {} (the grid has {n} nodes per side)", n - 1));
    }
    let idx: Vec<usize> = (0..n).step_by(step).collect();
    let mm = |v: f64| (v * MM_PER_M).round() / MM_PER_M;
    let (x0, z0) = w.node_xz(0, 0);
    let mut heights = Vec::with_capacity(idx.len() * idx.len());
    let mut mats = Vec::with_capacity(idx.len() * idx.len());
    let mut water = Vec::with_capacity(idx.len() * idx.len());
    for &j in &idx {
        for &i in &idx {
            heights.push(mm(w.height_at_node(i, j)));
            mats.push(w.splat_at_node(i, j));
            // Water surface height where the node is under water, else null (the viewer draws a plane clipped to these nodes).
            water.push(w.water_at_node(i, j).filter(|&s| s > w.height_at_node(i, j)).map(mm));
        }
    }
    let props: Vec<_> = w
        .props()
        .iter()
        .map(|p| {
            let shape = match p.shape {
                PropShape::Sphere { radius_m } => serde_json::json!({"type": "sphere", "radius_m": radius_m}),
                PropShape::Cylinder { radius_m, height_m } => serde_json::json!({"type": "cylinder", "radius_m": radius_m, "height_m": height_m}),
                PropShape::Box { half_m } => serde_json::json!({"type": "box", "half_m": [half_m.x, half_m.y, half_m.z]}),
            };
            let (q, t) = (p.transform.rot, p.transform.pos);
            serde_json::json!({"id": p.id.0, "kind": format!("{:?}", p.kind), "shape": shape, "pos_m": [mm(t.x), mm(t.y), mm(t.z)], "rot_wxyz": [q.w, q.x, q.y, q.z]})
        })
        .collect();
    let doc = serde_json::json!({
        "format": "w5k-terrain-1",
        "course": def.name,
        "seed": def.seed,
        "nx": idx.len(), "nz": idx.len(),
        "cell_m": step as f64 * CELL_M,
        "origin_m": {"x": x0, "z": z0},
        "heights_m": heights,
        "material_ids": mats,
        "materials": w.materials().materials.iter().map(|m| m.name.clone()).collect::<Vec<_>>(),
        "water_m": water,
        "bridges": course.bridges.iter().map(|b| serde_json::json!({"kind": format!("{:?}", b.kind), "a_m": [mm(b.a.0), mm(b.a.1)], "b_m": [mm(b.b.0), mm(b.b.1)], "deck_len_m": b.deck_len_m, "width_m": b.width_m, "deck_y_m": mm(b.deck_y_m), "load_limit_kg": b.load_limit_kg})).collect::<Vec<_>>(),
        "extra_roads_m": course.extra_roads.iter().map(|r| r.iter().step_by(step).map(|p| [mm(p.0), mm(p.2), mm(p.1)]).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "road_m": course.road.iter().step_by(step).map(|r| [mm(r.0), mm(r.2), mm(r.1)]).collect::<Vec<_>>(),
        "props": props,
    });
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let path = out.join("terrain.json");
    std::fs::write(&path, serde_json::to_string(&doc).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let bytes = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
    println!(
        "wrote {} ({} x {} nodes at {} m, {} props, {} KB)",
        path.display(),
        idx.len(),
        idx.len(),
        step as f64 * CELL_M,
        w.props().len(),
        bytes / 1000
    );
    Ok(())
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
            let water = w.water_surface_m(x, z).map(|s| ((s - w.height_m(x, z)) / WATER_DEPTH_FULL_M).clamp(0.0, 1.0));
            let base = if let Some(depth) = water {
                [
                    WATER_RGB[0] + (WATER_DEEP_RGB[0] - WATER_RGB[0]) * depth,
                    WATER_RGB[1] + (WATER_DEEP_RGB[1] - WATER_RGB[1]) * depth,
                    WATER_RGB[2] + (WATER_DEEP_RGB[2] - WATER_RGB[2]) * depth,
                ]
            } else {
                match name.as_str() {
                    "asphalt" => ASPHALT_RGB,
                    "mud" => MUD_RGB,
                    "gravel" => GRAVEL_RGB,
                    "planks" => PLANKS_RGB,
                    "sand" => SAND_RGB,
                    _ => [GROUND_RGB[0] + (w.height_m(x, z) - lo.y) * HEIGHT_TINT_PER_M, GROUND_RGB[1], GROUND_RGB[2]],
                }
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
    for road in &course.extra_roads {
        for p in road.iter().step_by(2) {
            dot(p.0, p.1, 1, TRACK_RGB);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_writes_a_self_describing_terrain_file_whose_heights_match_the_world() {
        let dir = std::env::temp_dir().join(format!("w5k-world-export-{}", std::process::id()));
        let args: Vec<String> =
            ["content/world/courses/slice.ron", "--out", dir.to_str().expect("utf8"), "--step", "4"]
                .map(String::from)
                .to_vec();
        // Tests run from the crate directory; the course lives at the workspace root.
        let args: Vec<String> = std::iter::once(format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), args[0]))
            .chain(args[1..].iter().cloned())
            .collect();
        export(&args).expect("export");
        let doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("terrain.json")).expect("read")).expect("json");
        assert_eq!(doc["format"], "w5k-terrain-1");
        let (nx, nz) = (doc["nx"].as_u64().expect("nx") as usize, doc["nz"].as_u64().expect("nz") as usize);
        assert_eq!(doc["heights_m"].as_array().expect("heights").len(), nx * nz);
        assert_eq!(doc["material_ids"].as_array().expect("materials").len(), nx * nz);
        let def = CourseDef::from_ron(include_str!("../../../../content/world/courses/slice.ron")).expect("course");
        let w = generate(&def).expect("generate").world;
        // Node (i, j) of the export is every 4th grid node, to the millimetre.
        let (i, j) = (37usize, 91usize);
        let h = doc["heights_m"][j * nx + i].as_f64().expect("h");
        assert!((h - w.height_at_node(i * 4, j * 4)).abs() < 1e-3);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
