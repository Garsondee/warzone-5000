//! `w5k world`: the command line of lane WORLD (only that lane edits this file).
//!
//! `w5k world stats <course.ron> --out DIR` writes `stats.json` and a one-page `stats.png` for VALIDATION: slope shares, road grade
//! distributions, roughness spectra against the ISO 8608 classes and the material numbers beside their published ranges.
//!
//! `w5k world mobility <course.ron> --out DIR [--vehicle spec.ron ...]` writes `mobility.json` and `mobility.png`: for each vehicle
//! a go / slow / no-go map of the course and the quickest route from the start of the main road to its end (the answer key for a
//! route-choosing AI). Without `--vehicle` it uses the three placeholder specs in `content/world/mobility/`.
//!
//! `w5k world preview <course.ron> --out DIR` generates the course and writes `topdown.png` (hill-shaded, materials coloured, props
//! as dots). `w5k world export <course.ron> --out DIR [--step N]` writes `terrain.json` for the viewers (format in
//! `docs/swarm/requests/world-viewer-terrain.md`); a replay's `WorldHeader.terrain` names that file.

use std::path::PathBuf;

use w5k_contract::world::{PropKind, PropShape, WorldQuery};
use w5k_math::{scalar, Vec3};
use w5k_world::course::{generate, CourseDef};
use w5k_world::grid::CELL_M;
use w5k_world::mobility::{classify, route, Measured, Mobility, MobilitySpec};
use w5k_world::plot::{Canvas, CHAR_W, LINE_H};
use w5k_world::stats::{
    check_materials, grade_stats, iso8608_class, profile_along, psd, segment_for, slope_stats, Psd, PublishedRange,
    ISO_8608_UPPER_M3, N0_CYC_PER_M,
};

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
const SHADE_BANDS: f64 = 20.0; // const-ok: number of flat-shading bands in the preview
const SHADE_AMBIENT: f64 = 0.3; // const-ok: hill-shade ambient
const MM_PER_M: f64 = 1000.0; // const-ok: unit conversion for the export's rounding
const CHANNEL_MAX: f64 = 255.0; // const-ok: 8-bit channel

/// Entry point for `w5k world <args>`.
pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("preview") => preview(&args[1..]),
        Some("export") => export(&args[1..]),
        Some("stats") => stats(&args[1..]),
        Some("mobility") => mobility(&args[1..]),
        _ => {
            Err("usage: w5k world preview|export|stats|mobility <course.ron> --out DIR [--step N] [--vehicle spec.ron]"
                .to_string())
        }
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
            // Shade in a limited number of bands: it reads as flat-shaded (the house style) and the small ground detail no longer makes the
            // picture incompressible.
            let shade = ((w.normal(x, z).dot(light) * SHADE_GAIN + SHADE_AMBIENT).clamp(0.0, 1.0) * SHADE_BANDS)
                .round()
                / SHADE_BANDS;
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
    enc.set_compression(png::Compression::High);
    enc.set_filter(png::Filter::Adaptive);
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

/// Spacing of the samples of a road profile, m: fine enough to resolve a 0.5 m washboard (Nyquist 5 cycles/m).
/// Spacing of the samples of a base road profile, m: the roughness band tops out at 0.3 cycles/m, so half a metre (Nyquist 1 cycle/m)
/// is plenty, and a short road then still fills a long enough segment to resolve its long wavelengths.
const ROAD_BASE_SAMPLE_M: f64 = 0.5; // const-ok: sampling interval of the roughness analysis
const ROAD_SAMPLE_M: f64 = 0.1; // const-ok: sampling interval of the roughness analysis
/// Spacing of the samples of a cross-country profile, m (the 1 m grid has nothing finer than 0.5 cycles/m anyway).
const GROUND_SAMPLE_M: f64 = 1.0; // const-ok: one sample per grid cell: the grid has nothing finer, and longer segments resolve the long wavelengths
/// The rows of the cross-country lines, as plan z, m.
const GROUND_LINES_Z: [f64; 5] = [-150.0, -75.0, 0.0, 75.0, 150.0]; // const-ok: where the roughness is sampled across the course
/// Frequency range of the spectrum plot, cycles per metre.
/// Frequency ticks of the spectrum plot, cycles per metre.
const PLOT_TICKS: [f64; 3] = [0.01, 0.1, 1.0]; // const-ok: decade ticks
/// Range over which the waviness exponent is fitted, cycles per metre: the **roughness band**. Longer wavelengths (hills, grades) are
/// the course's *relief* and are reported by the slope shares and grade histograms, as ISO 8608 itself treats them as gradient. The
/// upper edge keeps clear of the grid's own limit: the surface a vehicle meets is bilinear, whose power response is `sinc^4` (0.77 at
/// 0.2 cycles/m, 0.54 at 0.3, 0.33 at 0.4), so the spectrum the world really has bends down above about 0.2. It stops below 0.5 because a 1 m grid holds nothing finer, and
/// bilinear interpolation rolls off as `n^-4` above it: fitting past it measures the interpolation, not the ground.
const WAVINESS_BAND: (f64, f64) = (0.06, 0.2); // const-ok: wavelengths of 5 to 17 m: above the relief, below where bilinear interpolation attenuates
/// Percent per unit share.
const PERCENT: f64 = 100.0; // const-ok: unit conversion
/// Margin kept from the course edge by the cross-country lines, m.
/// Rows averaged per ground line, and their spacing, m.
const GROUND_ROWS: usize = 10; // const-ok: rows averaged per ground line
const GROUND_ROW_STEP_M: f64 = 6.0; // const-ok: spacing of the averaged rows
/// A node is plain ground if its grade limit is within this factor of the course's median one.
const PLAIN_FACTOR: f64 = 1.05; // const-ok: separates designed relief from ordinary ground
/// Shortest stretch of plain ground worth measuring, nodes.
const MIN_PLAIN_RUN: usize = 128; // const-ok: one smallest Welch segment
const EDGE_MARGIN_M: f64 = 10.0; // const-ok: keeps the lines off the map edge
const PLOT_N: (f64, f64) = (0.01, 5.0); // const-ok: the plot's x range (ISO 8608 covers 0.01 to 10)
/// Range of the spectrum plot, m^3.
const PLOT_G: (f64, f64) = (1e-10, 1e2); // const-ok: the plot's y range

/// A road's name and its centreline `(x, z, y)`.
type NamedRoad<'a> = (String, &'a Vec<(f64, f64, f64)>);

struct Line {
    name: String,
    kind: &'static str,
    psd: Option<Psd>,
    gd: Option<f64>,
}

fn analyse(
    name: &str,
    kind: &'static str,
    world: &dyn w5k_contract::world::WorldQuery,
    line: &[(f64, f64)],
    dx: f64,
) -> Line {
    let z = profile_along(world, line, dx);
    let p = psd(&z, dx, segment_for(z.len()));
    let gd = p.as_ref().and_then(Psd::gd_n0);
    Line { name: name.to_string(), kind, psd: p, gd }
}

fn stats(args: &[String]) -> Result<(), String> {
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
    let w = &course.world;
    let half = (w.n() - 1) as f64 * 0.5 * CELL_M;
    let slope = slope_stats(w, half, CELL_M);
    let mut roads: Vec<NamedRoad> = vec![("main road".to_string(), &course.road)];
    for (k, r) in course.extra_roads.iter().enumerate() {
        roads.push((format!("track {}", k + 1), r));
    }
    // The scored roughness lines come from the course *without* its deliberate washboard, whoops and mud-pit dips (they are features, not roughness:
    // a 12 m whoop or a 25 m pit is a spike in the spectrum by design), which are analysed and reported separately.
    let mut base_def = def.clone();
    base_def.road.whoops.clear();
    base_def.road.washboards.clear();
    base_def.road.mud_crossings.clear();
    base_def.extra_roads.iter_mut().for_each(|r| {
        r.whoops.clear();
        r.washboards.clear();
        r.mud_crossings.clear();
    });
    let has_features = base_def != def;
    let base = generate(&base_def)?;
    let bw = &base.world;
    let mut lines: Vec<Line> = Vec::new();
    let mut feature_lines: Vec<Line> = Vec::new();
    let mut grades = Vec::new();
    for (name, r) in &roads {
        let xz: Vec<(f64, f64)> = r.iter().map(|p| (p.0, p.1)).collect();
        grades.push((name.clone(), grade_stats(r)));
        let base_road: &Vec<(f64, f64, f64)> = if name == "main road" {
            &base.road
        } else {
            &base.extra_roads[name.trim_start_matches("track ").parse::<usize>().unwrap_or(1) - 1]
        };
        let bxz: Vec<(f64, f64)> = base_road.iter().map(|p| (p.0, p.1)).collect();
        lines.push(analyse(name, "road", bw, &bxz, ROAD_BASE_SAMPLE_M));
        if has_features {
            feature_lines.push(analyse(name, "road_with_features", w, &xz, ROAD_SAMPLE_M));
        }
    }
    // Ground lines: rows across the course, on *plain* ground only (the cliff, bank and deck zones are designed relief, not roughness),
    // each the average of a few neighbouring rows so the estimate is not one noisy segment.
    let n_nodes = bw.n();
    // Plain ground is where the grade limit is the course's usual one (the median over the course, which is mostly ordinary ground);
    // cliff faces, river banks and bridge decks carry larger limits.
    let plain = {
        let mut g = base.grade_limit.clone();
        g.sort_by(f64::total_cmp);
        g[g.len() / 2] * PLAIN_FACTOR
    };
    for z0 in GROUND_LINES_Z {
        let mut rows: Vec<Vec<f64>> = Vec::new();
        for r in 0..GROUND_ROWS {
            let z = z0 + r as f64 * GROUND_ROW_STEP_M;
            if z.abs() >= half - EDGE_MARGIN_M {
                continue;
            }
            let j = ((z + half) / CELL_M).round() as usize;
            // The longest run of plain nodes along the row.
            let (mut best, mut cur) = ((0usize, 0usize), 0usize);
            for i in 0..n_nodes {
                if base.grade_limit[j * n_nodes + i] <= plain {
                    cur += 1;
                    if cur > best.1 {
                        best = (i + 1 - cur, cur);
                    }
                } else {
                    cur = 0;
                }
            }
            if best.1 >= MIN_PLAIN_RUN {
                rows.push((best.0..best.0 + best.1).map(|i| bw.height_at_node(i, j)).collect());
            }
        }
        let Some(shortest) = rows.iter().map(Vec::len).min() else { continue };
        let seg = segment_for(shortest);
        let psds: Vec<Psd> = rows.iter().filter_map(|z| psd(&z[..shortest], GROUND_SAMPLE_M, seg)).collect();
        let p = Psd::average(&psds);
        let gd = p.as_ref().and_then(Psd::gd_n0);
        lines.push(Line { name: format!("ground z={z0:.0}"), kind: "ground", psd: p, gd });
    }
    let ranges: Vec<PublishedRange> = ron::from_str(include_str!("../../../../content/world/published_ranges.ron"))
        .map_err(|e| format!("published_ranges.ron: {e}"))?;
    let checks = check_materials(&course.world.materials().clone(), &ranges)?;

    let line_to_json = |l: &Line| {
        {
            serde_json::json!({
                "name": l.name, "kind": l.kind,
                "gd_n0_m3": l.gd, "iso8608_class": l.gd.map(|g| iso8608_class(g).to_string()),
                "waviness_exponent": l.psd.as_ref().and_then(|p| p.exponent(WAVINESS_BAND.0, WAVINESS_BAND.1)),
                "segment_m": l.psd.as_ref().map(|p| p.segment_m), "segments": l.psd.as_ref().map(|p| p.segments),
                "spectrum": l.psd.as_ref().map(|p| p.log_binned(24).into_iter().map(|(n, g)| [n, g]).collect::<Vec<_>>()),
            })
        }
    };
    let line_json: Vec<_> = lines.iter().map(line_to_json).collect();
    let feature_json: Vec<_> = feature_lines.iter().map(line_to_json).collect();
    let doc = serde_json::json!({
        "course": def.name, "seed": def.seed, "grid_nodes": w.n(), "n0_cycles_per_m": N0_CYC_PER_M,
        "slope": slope,
        "roads": grades.iter().map(|(n, g)| serde_json::json!({"name": n, "grade": g})).collect::<Vec<_>>(),
        "roughness": line_json,
        "roughness_fit_band_cycles_per_m": [WAVINESS_BAND.0, WAVINESS_BAND.1],
        "roughness_features": feature_json,
        "materials": checks,
        "iso8608": {"standard": "ISO 8608:2016, G(n0) at n0 = 0.1 cycles/m, class upper bounds A..G in m^3", "upper_bounds_m3": ISO_8608_UPPER_M3},
    });
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    std::fs::write(out.join("stats.json"), serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    // ---- the one-page picture ----
    let ink = [30, 30, 40]; // const-ok: display colour
    let grey = [190, 190, 195]; // const-ok: display colour
    let mut cv = Canvas::new(PAGE_W, PAGE_H, [250, 250, 250]); // const-ok: display colour
    cv.text(20, 14, &format!("W5K WORLD STATS  {}  SEED {}  GRID {}", def.name, def.seed, w.n()), 3, ink);
    // Slope shares.
    cv.text(20, 50, "GROUND STEEPER THAN", 2, ink);
    let sl = [
        (">5 DEG", slope.share_over_5_deg),
        (">10", slope.share_over_10_deg),
        (">20", slope.share_over_20_deg),
        (">30", slope.share_over_30_deg),
    ];
    for (k, (lab, v)) in sl.iter().enumerate() {
        let (x, base) = (30 + k as i64 * 110, 270);
        let hgt = (v * 180.0) as i64; // const-ok: bar height scale, px
        cv.rect(x, base - hgt, 80, hgt.max(1), [70, 120, 190]); // const-ok: display colour
        cv.text(x, base - hgt - 16, &format!("{:.1}%", v * PERCENT), 2, ink);
        cv.text(x, base + 6, lab, 2, ink);
    }
    cv.text(30, 296, &format!("MEAN {:.1} DEG  MAX {:.1} DEG", slope.mean_deg, slope.max_deg), 2, ink);
    // Road grade histogram of the main road.
    cv.text(520, 50, &format!("{} GRADE (SHARE OF LENGTH)", grades[0].0), 2, ink);
    let g0 = &grades[0].1;
    for (k, v) in g0.histogram_2pct.iter().enumerate() {
        let (x, base) = (530 + k as i64 * 55, 270);
        let hgt = (v * 180.0) as i64; // const-ok: bar height scale, px
        cv.rect(x, base - hgt, 45, hgt.max(1), [200, 120, 60]); // const-ok: display colour
        cv.text(x, base + 6, &format!("{}", k * 2), 2, ink);
    }
    cv.text(
        530,
        296,
        &format!(
            "MAX {:.1}%  P95 {:.1}%  MEAN {:.1}%  LEN {:.0} M",
            g0.max_grade * PERCENT,
            g0.p95_grade * PERCENT,
            g0.mean_grade * PERCENT,
            g0.length_m
        ),
        2,
        ink,
    );
    // Spectrum.
    let (px0, py0, pw, ph) = (60i64, 350i64, 640i64, 480i64);
    cv.text(20, 322, "ROUGHNESS PSD G(N) M^3 VS N CYCLES/M, ISO 8608 CLASS BOUNDS A/B ... G/H", 2, ink);
    let (ln_lo, ln_hi) = (scalar::ln(PLOT_N.0), scalar::ln(PLOT_N.1));
    let (lg_lo, lg_hi) = (scalar::ln(PLOT_G.0), scalar::ln(PLOT_G.1));
    let mapx = |n: f64| px0 + (((scalar::ln(n) - ln_lo) / (ln_hi - ln_lo)).clamp(0.0, 1.0) * pw as f64) as i64;
    let mapy = |g: f64| py0 + ph - (((scalar::ln(g) - lg_lo) / (lg_hi - lg_lo)).clamp(0.0, 1.0) * ph as f64) as i64;
    cv.rect(px0, py0, pw, ph, [255, 255, 255]); // const-ok: display colour
    for (b, (gb, lab)) in [32e-6, 128e-6, 512e-6, 2048e-6, 8192e-6, 32768e-6, 131072e-6]
        .iter()
        .zip(["A", "B", "C", "D", "E", "F", "G"])
        .enumerate()
    {
        let mut prev: Option<(i64, i64)> = None;
        for k in 0..=120 {
            // const-ok: segments of a class line
            let n = PLOT_N.0 * scalar::pow(PLOT_N.1 / PLOT_N.0, k as f64 / 120.0);
            let g = gb * (N0_CYC_PER_M / n) * (N0_CYC_PER_M / n);
            let pt = (mapx(n), mapy(g));
            if let Some(q) = prev {
                if pt.1 > py0 && q.1 > py0 && pt.1 < py0 + ph {
                    cv.line(q.0, q.1, pt.0, pt.1, grey);
                }
            }
            prev = Some(pt);
        }
        cv.text(
            px0 + pw + 6,
            mapy(gb * (N0_CYC_PER_M / PLOT_N.1) * (N0_CYC_PER_M / PLOT_N.1)) - 3,
            &format!("{lab}/{}", ["B", "C", "D", "E", "F", "G", "H"][b]),
            1,
            ink,
        );
    }
    for decade in PLOT_TICKS {
        cv.line(mapx(decade), py0 + ph, mapx(decade), py0 + ph + 5, ink);
        cv.text(mapx(decade) - 8, py0 + ph + 8, &format!("{decade}"), 2, ink);
    }
    let palette = [
        [30, 90, 190],
        [20, 150, 150],
        [120, 60, 160],
        [200, 90, 30],
        [170, 140, 20],
        [90, 90, 90],
        [190, 50, 90],
        [60, 140, 60],
    ]; // const-ok: display colours
    for (k, l) in lines.iter().enumerate() {
        let col = palette[k % palette.len()];
        if let Some(p) = &l.psd {
            let pts: Vec<(i64, i64)> =
                p.log_binned(24).iter().filter(|(_, g)| *g > 0.0).map(|(n, g)| (mapx(*n), mapy(*g))).collect();
            for wnd in pts.windows(2) {
                for off in 0..2 {
                    // const-ok: line thickness
                    cv.line(wnd[0].0, wnd[0].1 + off, wnd[1].0, wnd[1].1 + off, col);
                }
            }
        }
        let y = py0 + 4 + (k as i64) * (LINE_H as i64 * 2 + 4);
        cv.rect(px0 + pw + 60, y, 12, 8, col);
        let cls = l.gd.map_or("-".to_string(), |g| format!("{}", iso8608_class(g)));
        cv.text(px0 + pw + 78, y, &format!("{} {}", l.name, cls), 2, ink);
    }
    cv.text(px0 + pw + 60, py0 + ph - 30, "CLASS = ISO 8608 AT N0=0.1", 1, ink);
    // Materials against published ranges.
    let ty = 860i64;
    cv.text(20, ty, "MATERIAL NUMBERS BESIDE PUBLISHED RANGES (UNVERIFIED UNTIL VALIDATION SOURCES THEM)", 2, ink);
    for (k, c) in checks.iter().enumerate() {
        let (col, row) = (k / 17, (k % 17) as i64); // const-ok: rows per column
        let (x, y) = (20 + col as i64 * 490, ty + 22 + row * (LINE_H as i64 * 2 + 4));
        let flag = if c.in_range { "OK" } else { "OUT" };
        let text = format!(
            "{} {} {:.3e} IN {:.2e}..{:.2e} {}",
            c.material,
            c.quantity.replace('_', " "),
            c.value,
            c.lo,
            c.hi,
            flag
        );
        cv.text(x, y, &text, 1, if c.in_range { ink } else { [190, 30, 30] }); // const-ok: display colour
    }
    let _ = CHAR_W;

    let path = out.join("stats.png");
    let f = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), cv.w as u32, cv.h as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    wr.write_image_data(&cv.rgb).map_err(|e| e.to_string())?;
    println!(
        "wrote {} and stats.json: slopes >10 deg {:.1}%, main road max grade {:.1}%, {}",
        path.display(),
        slope.share_over_10_deg * PERCENT,
        g0.max_grade * PERCENT,
        lines
            .iter()
            .filter(|l| l.kind == "road")
            .map(|l| format!("{} class {}", l.name, l.gd.map_or('-', iso8608_class)))
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok(())
}

/// The placeholder vehicles, embedded so the command works from any directory.
const DEFAULT_VEHICLES: [&str; 3] = [
    include_str!("../../../../content/world/mobility/scout.ron"),
    include_str!("../../../../content/world/mobility/mule.ron"),
    include_str!("../../../../content/world/mobility/hauler.ron"),
];
/// VALIDATION's proving-ground export, embedded so the command works from any directory (`--capability FILE` overrides it).
const DEFAULT_CAPABILITY: &str = include_str!("../../../../docs/lanes/validation/capability/capability.json");

/// What the proving ground measured for the vehicle called `name` (its entry is `<name>_...`). A reading its scorer marked `red` is
/// left out (a model result that failed its physics check), and so is anything missing.
fn measured_for(json: &str, name: &str) -> Result<Measured, String> {
    let all: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("capability json: {e}"))?;
    let entry =
        all.as_array().into_iter().flatten().find(|v| v["vehicle"].as_str().is_some_and(|n| n.starts_with(name)));
    let read = |key: &str| {
        let r = entry?.get(key)?;
        (r["light"] != "red").then(|| r["value"].as_f64()).flatten()
    };
    Ok(Measured {
        max_grade_ratio: read("max_grade_ratio"),
        max_side_slope_rad: read("max_side_slope_rad"),
        max_step_m: read("max_step_m"),
    })
}

const GO_RGB: [f64; 3] = [70.0, 170.0, 80.0]; // const-ok: display colour
const SLOW_RGB: [f64; 3] = [225.0, 180.0, 50.0]; // const-ok: display colour
const NOGO_RGB: [f64; 3] = [190.0, 55.0, 50.0]; // const-ok: display colour
const ROUTE_RGB: [u8; 3] = [255, 255, 255]; // const-ok: display colour
const PANEL_PX: usize = 400; // const-ok: width of one vehicle's map in the picture
const PANEL_GAP: usize = 10; // const-ok: gap between maps, px
const MAP_SHADE_FLOOR: f64 = 0.55; // const-ok: how dark the hill shading may make the class colour

fn mobility(args: &[String]) -> Result<(), String> {
    let (mut file, mut out, mut specs) = (None, None, Vec::new());
    let mut capability = DEFAULT_CAPABILITY.to_string();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--out" => out = it.next().map(PathBuf::from),
            "--capability" => {
                let f = it.next().ok_or("--capability needs a file")?;
                capability = std::fs::read_to_string(f).map_err(|e| format!("{f}: {e}"))?;
            }
            "--vehicle" => {
                let f = it.next().ok_or("--vehicle needs a file")?;
                specs.push(MobilitySpec::from_ron(&std::fs::read_to_string(f).map_err(|e| format!("{f}: {e}"))?)?);
            }
            _ => file = Some(a.clone()),
        }
    }
    let (file, out) = (file.ok_or("missing <course.ron>")?, out.ok_or("missing --out DIR")?);
    if specs.is_empty() {
        for t in DEFAULT_VEHICLES {
            specs.push(MobilitySpec::from_ron(t)?);
        }
    }
    let def = CourseDef::from_ron(&std::fs::read_to_string(&file).map_err(|e| format!("{file}: {e}"))?)?;
    let course = generate(&def)?;
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let w = &course.world;
    let n = w.n();
    for spec in &mut specs {
        let measured = measured_for(&capability, &spec.name)?;
        *spec = spec.clone().with_measured(&measured, w.materials());
    }
    let (start, finish) = (course.road[0], course.road[course.road.len() - 1]);
    let step = n.div_ceil(PANEL_PX);
    let panel = n.div_ceil(step);
    let light = Vec3::new(LIGHT_DIR[0], LIGHT_DIR[1], LIGHT_DIR[2]).normalized_or_zero();
    let head = LINE_H * 3;
    let mut cv = Canvas::new(
        PANEL_GAP + specs.len() * (panel + PANEL_GAP),
        head * 2 + panel + PANEL_GAP,
        [235, 235, 235], // const-ok: display colour
    );
    let mut report = Vec::new();
    for (k, spec) in specs.iter().enumerate() {
        let map = classify(&course, spec);
        let found = route(&course, spec, &map, (start.0, start.1), (finish.0, finish.1));
        let (go, slow, nogo) = map.shares();
        let (x0, y0) = ((PANEL_GAP + k * (panel + PANEL_GAP)) as i64, (head * 2) as i64);
        for r in 0..panel {
            for c in 0..panel {
                let (i, j) = ((c * step).min(n - 1), (r * step).min(n - 1));
                let (x, z) = w.node_xz(i, j);
                let shade = ((w.normal(x, z).dot(light) * SHADE_GAIN + SHADE_AMBIENT).clamp(0.0, 1.0) * SHADE_BANDS)
                    .round()
                    / SHADE_BANDS;
                let shade = MAP_SHADE_FLOOR + (1.0 - MAP_SHADE_FLOOR) * shade;
                let base = match map.class[j * n + i] {
                    Mobility::Go => GO_RGB,
                    Mobility::Slow => SLOW_RGB,
                    Mobility::NoGo => NOGO_RGB,
                };
                cv.put(x0 + c as i64, y0 + r as i64, [0, 1, 2].map(|q| (base[q] * shade).min(CHANNEL_MAX) as u8));
            }
        }
        let to_px = |p: (f64, f64)| {
            let half = (n - 1) as f64 * 0.5 * CELL_M;
            (x0 + ((p.0 + half) / CELL_M / step as f64) as i64, y0 + ((p.1 + half) / CELL_M / step as f64) as i64)
        };
        if let Some(rt) = &found {
            for pair in rt.points.windows(2) {
                let (a, b) = (to_px(pair[0]), to_px(pair[1]));
                cv.line(a.0, a.1, b.0, b.1, ROUTE_RGB);
            }
        }
        for (p, rgb) in [((start.0, start.1), [20, 90, 30]), ((finish.0, finish.1), [30, 30, 30])] {
            let q = to_px(p);
            cv.rect(q.0 - 2, q.1 - 2, 5, 5, rgb); // const-ok: marker size, px
        }
        let ink = [20, 20, 20]; // const-ok: display colour
        cv.text(x0, 4, &format!("{} {:.0} kg {:.2} m", spec.name, spec.mass_kg, spec.width_m), 1, ink); // const-ok: label position
        let line = match &found {
            Some(r) => format!("route {:.0} m {:.0} s {}", r.length_m, r.time_s, r.uses.join("+")),
            None => "NO ROUTE".to_string(),
        };
        cv.text(
            x0,
            4 + LINE_H as i64,
            &format!("go {:.0}% slow {:.0}% no-go {:.0}%", go * PERCENT, slow * PERCENT, nogo * PERCENT),
            1,
            ink,
        ); // const-ok: label position
        cv.text(x0, 4 + 2 * LINE_H as i64, &line, 1, ink); // const-ok: label position
        report.push(serde_json::json!({
            "vehicle": spec.name,
            "note": spec.note,
            "mass_kg": spec.mass_kg,
            "width_m": spec.width_m,
            "share_go": go,
            "share_slow": slow,
            "share_nogo": nogo,
            "route": found.as_ref().map(|r| serde_json::json!({
                "length_m": r.length_m,
                "time_s": r.time_s,
                "max_grade_climbed": r.max_grade_climbed,
                "max_cross_slope": r.max_cross_slope,
                "uses": r.uses,
                "points_m": r.points,
            })),
        }));
        println!(
            "{}: go {:.1}% slow {:.1}% no-go {:.1}%; {}",
            spec.name,
            go * PERCENT,
            slow * PERCENT,
            nogo * PERCENT,
            found.map_or("no route from start to finish".to_string(), |r| format!(
                "route {:.0} m in {:.0} s via [{}]",
                r.length_m,
                r.time_s,
                r.uses.join(", ")
            ))
        );
    }
    let doc = serde_json::json!({
        "format": "w5k-mobility-1",
        "course": def.name,
        "start_m": [start.0, start.1],
        "finish_m": [finish.0, finish.1],
        "vehicles": report,
    });
    std::fs::write(out.join("mobility.json"), serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let f = std::fs::File::create(out.join("mobility.png")).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), cv.w as u32, cv.h as u32);
    enc.set_compression(png::Compression::High);
    enc.set_filter(png::Filter::Adaptive);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    wr.write_image_data(&cv.rgb).map_err(|e| e.to_string())?;
    println!("wrote {} and mobility.json", out.join("mobility.png").display());
    Ok(())
}

const PAGE_W: usize = 1280; // const-ok: page size, px
const PAGE_H: usize = 1180; // const-ok: page size, px

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

    #[test]
    fn a_red_reading_is_not_used_and_a_vehicle_is_found_by_its_name_prefix() {
        let json = r#"[{"vehicle":"mule_4x4","max_grade_ratio":{"value":0.6,"light":"not scored"},
            "max_side_slope_rad":{"value":0.7,"light":"green"},"max_step_m":{"value":0.3,"light":"red"}}]"#;
        let m = measured_for(json, "mule").expect("parse");
        assert_eq!((m.max_grade_ratio, m.max_side_slope_rad, m.max_step_m), (Some(0.6), Some(0.7), None));
        assert_eq!(measured_for(json, "hauler").expect("parse"), Measured::default());
        let shipped = measured_for(DEFAULT_CAPABILITY, "scout").expect("the shipped export parses");
        assert!(shipped.max_grade_ratio.is_some(), "the shipped export has the scout's grade");
    }

    #[test]
    fn mobility_writes_a_map_per_vehicle_and_every_vehicle_crosses_the_river_on_the_road_bridge() {
        let dir = std::env::temp_dir().join(format!("w5k-world-mobility-{}", std::process::id()));
        let course = format!("{}/../../content/world/courses/crossing.ron", env!("CARGO_MANIFEST_DIR"));
        mobility(&[course, "--out".into(), dir.to_str().expect("utf8").into()]).expect("mobility");
        let doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("mobility.json")).expect("read")).expect("json");
        let vehicles = doc["vehicles"].as_array().expect("vehicles");
        assert_eq!(vehicles.len(), 3);
        for v in vehicles {
            let shares = ["share_go", "share_slow", "share_nogo"].map(|k| v[k].as_f64().expect("share"));
            assert!((shares.iter().sum::<f64>() - 1.0).abs() < 1e-9, "shares sum to one");
            let uses = v["route"]["uses"].as_array().expect("every vehicle has a route");
            assert!(uses.iter().any(|u| u == "bridge:Road"), "{} uses the road bridge", v["vehicle"]);
        }
        assert!(std::fs::metadata(dir.join("mobility.png")).expect("png").len() > 5_000);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stats_writes_json_with_every_section_and_a_page_image() {
        let dir = std::env::temp_dir().join(format!("w5k-world-stats-{}", std::process::id()));
        let course = format!("{}/../../content/world/courses/slice.ron", env!("CARGO_MANIFEST_DIR"));
        stats(&[course, "--out".into(), dir.to_str().expect("utf8").into()]).expect("stats");
        let doc: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("stats.json")).expect("read")).expect("json");
        for key in ["slope", "roads", "roughness", "materials", "iso8608"] {
            assert!(doc.get(key).is_some(), "stats.json has `{key}`");
        }
        let s = &doc["slope"];
        assert!(
            s["share_over_5_deg"].as_f64().expect("5") >= s["share_over_10_deg"].as_f64().expect("10"),
            "shares fall with the angle"
        );
        assert!(s["share_over_10_deg"].as_f64().expect("10") >= s["share_over_30_deg"].as_f64().expect("30"));
        assert!(
            doc["roads"][0]["grade"]["max_grade"].as_f64().expect("max grade") <= 0.08 + 1e-4,
            "the main road keeps its stated grade"
        );
        let road = &doc["roughness"][0];
        assert_eq!(road["kind"], "road");
        assert!(road["iso8608_class"].as_str().is_some(), "the road has an ISO 8608 class");
        assert!(doc["materials"].as_array().expect("materials").iter().all(|m| m["status"] == "Unverified"));
        let png = std::fs::metadata(dir.join("stats.png")).expect("png").len();
        assert!(png > 10_000, "a real picture ({png} bytes)");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// ISO 8608 says a road profile's PSD falls as `n^-2`; VALIDATION scores the fitted exponent (green within 0.5, amber within 1.0).
    #[test]
    fn roughness_spectra_fall_as_n_to_the_minus_two_in_the_roughness_band() {
        for course in ["slice", "crossing"] {
            let dir = std::env::temp_dir().join(format!("w5k-world-iso-{course}-{}", std::process::id()));
            let file = format!("{}/../../content/world/courses/{course}.ron", env!("CARGO_MANIFEST_DIR"));
            stats(&[file, "--out".into(), dir.to_str().expect("utf8").into()]).expect("stats");
            let doc: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(dir.join("stats.json")).expect("read")).expect("json");
            assert!(
                (doc["roughness_fit_band_cycles_per_m"][1].as_f64().expect("band") - WAVINESS_BAND.1).abs() < 1e-12
            );
            let (mut sum, mut cnt) = (0.0, 0);
            for r in doc["roughness"].as_array().expect("roughness") {
                let w = r["waviness_exponent"].as_f64().expect("every scored line has an exponent");
                assert!((w + 2.0).abs() < 1.0, "{course} {}: exponent {w} is red", r["name"]);
                if r["kind"] == "ground" {
                    sum += w;
                    cnt += 1;
                }
            }
            assert!(cnt >= 4, "{course}: ground lines measured ({cnt})");
            let mean = sum / cnt as f64;
            assert!((mean + 2.0).abs() < 0.3, "{course}: mean ground exponent {mean}");
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}
