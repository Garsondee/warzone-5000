//! Preview images: an eight-view contact sheet with a stats panel and armour plots.
//!
//! Views are shaded from the same per-vertex attributes the game shader will use (colour slot, chamfer edge
//! flag, ambient occlusion), so what the previews show is what the engine will get. Lights are fixed relative
//! to the camera so every view of a sheet is lit the same way and views can be compared.

use crate::armour::ArmourTable;
use crate::geom::{v3, V3};
use crate::mesh::Mesh;
use crate::raster::{hex_linear, mix, raster_tri, scale, Camera, Image, Rgb, Target};
use crate::schema::{MaterialLibrary, Slot};
use crate::StatsFile;

/// Fallback colours per slot (sRGB) when a palette leaves one out.
const DEFAULT_HEX: [&str; 8] = ["#6b7350", "#4a5236", "#b89a45", "#2b2d2f", "#8a9096", "#1e1e20", "#58d8ff", "#2f4752"];

pub const SUPERSAMPLE: usize = 2;
pub const TILE: usize = 300;
const FOV_DEG: f64 = 30.0;
const VIEWS: [(f64, &str); 8] = [
    (0.0, "front"),
    (45.0, "front right"),
    (90.0, "right"),
    (135.0, "rear right"),
    (180.0, "rear"),
    (225.0, "rear left"),
    (270.0, "left"),
    (315.0, "front left"),
];

const TEXT: Rgb = [0.62, 0.64, 0.67];
const DIM: Rgb = [0.25, 0.26, 0.28];
const WARN: Rgb = [0.9, 0.18, 0.12];

/// Linear colours for each slot from a named palette (falling back to "default", then built-in colours).
pub fn palette_colours(lib: &MaterialLibrary, name: Option<&str>) -> [Rgb; 8] {
    let pal = name.and_then(|n| lib.palettes.get(n)).or_else(|| lib.palettes.get("default"));
    let mut out = [[0.5; 3]; 8];
    for (i, s) in Slot::ALL.iter().enumerate() {
        let hex = pal.and_then(|p| p.colours.get(s)).map(|s| s.as_str()).unwrap_or(DEFAULT_HEX[i]);
        out[i] = hex_linear(hex).or_else(|| hex_linear(DEFAULT_HEX[i])).unwrap();
    }
    out
}

/// Framing shared by all views of one object, so they are drawn at the same scale.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub centre: V3,
    pub radius: f64,
    /// Height of the ground plane (the lowest point of the mesh).
    pub ground: f64,
    /// Ground grid spacing (m).
    pub grid: f64,
}

impl Frame {
    pub fn of(mesh: &Mesh) -> Frame {
        let mut lo = v3(f64::MAX, f64::MAX, f64::MAX);
        let mut hi = v3(f64::MIN, f64::MIN, f64::MIN);
        for p in &mesh.positions {
            let q = v3(p[0] as f64, p[1] as f64, p[2] as f64);
            lo = lo.min(q);
            hi = hi.max(q);
        }
        if mesh.positions.is_empty() {
            return Frame { centre: V3::ZERO, radius: 1.0, ground: 0.0, grid: 1.0 };
        }
        let radius = ((hi - lo).len() * 0.5).max(1e-3);
        Frame { centre: (lo + hi) * 0.5, radius: radius * 1.04, ground: lo.y, grid: nice_step(radius / 2.0) }
    }
}

/// Largest value of the form {1, 2, 5} x 10^k not above `x`.
pub fn nice_step(x: f64) -> f64 {
    let mut best = 0.01;
    for e in -2..=4 {
        for m in [1.0, 2.0, 5.0] {
            let s = m * 10f64.powi(e);
            if s <= x {
                best = s;
            }
        }
    }
    best
}

fn pos(m: &Mesh, i: usize) -> V3 {
    let p = m.positions[i];
    v3(p[0] as f64, p[1] as f64, p[2] as f64)
}

fn nrm(m: &Mesh, i: usize) -> V3 {
    let n = m.normals[i];
    v3(n[0] as f64, n[1] as f64, n[2] as f64)
}

struct Lights {
    key: V3,
    fill: V3,
    rim: V3,
    view: V3,
}

/// Flat shading of one face: (direct light, ambient light, emission). Ambient occlusion is applied per pixel.
fn shade_face(base: Rgb, slot: Slot, edge: f32, n: V3, l: &Lights) -> (Rgb, Rgb, Rgb) {
    // Worn, lighter chamfer edges: the cue that makes faceted shapes read as machined metal.
    let base = if edge > 0.5 { mix(base, [base[0] * 1.7 + 0.025, base[1] * 1.7 + 0.025, base[2] * 1.7 + 0.025], 0.7) } else { base };
    let dk = n.dot(l.key).max(0.0) as f32;
    let df = n.dot(l.fill).max(0.0) as f32;
    let dr = (n.dot(l.rim).max(0.0) as f32).powi(2);
    let (spec, shin) = match slot {
        Slot::Metal => (0.16, 24),
        Slot::Glass => (0.9, 90),
        Slot::Rubber => (0.015, 8),
        _ => (0.05, 14),
    };
    let half = (l.key + l.view).norm();
    let sp = spec * (n.dot(half).max(0.0) as f32).powi(shin) * dk.min(1.0).sqrt();
    let key = [1.30, 1.24, 1.15];
    let fill = [0.24, 0.27, 0.34];
    let mut direct = [0.0; 3];
    for k in 0..3 {
        direct[k] = base[k] * (key[k] * dk + fill[k] * df) + 0.16 * dr + sp;
    }
    let sky = mix([0.17, 0.15, 0.13], [0.40, 0.45, 0.53], (0.5 + 0.5 * n.y) as f32);
    let ambient = [base[0] * sky[0], base[1] * sky[1], base[2] * sky[2]];
    if slot == Slot::Glow {
        return (scale(direct, 0.3), scale(ambient, 0.3), scale(base, 2.2));
    }
    (direct, ambient, [0.0; 3])
}

fn draw_ground(img: &mut Image, cam: &Camera, fr: &Frame) {
    let ext = fr.radius * 1.6;
    let n = (ext / fr.grid).floor() as i64;
    let (cx, cz) = ((fr.centre.x / fr.grid).round() * fr.grid, (fr.centre.z / fr.grid).round() * fr.grid);
    const SEGS: usize = 48;
    for k in -n..=n {
        let off = k as f64 * fr.grid;
        for along_x in [true, false] {
            let mut prev: Option<[f64; 3]> = None;
            for s in 0..=SEGS {
                let t = -ext + 2.0 * ext * s as f64 / SEGS as f64;
                let p = if along_x { v3(cx + t, fr.ground, cz + off) } else { v3(cx + off, fr.ground, cz + t) };
                let r = ((p.x - fr.centre.x).powi(2) + (p.z - fr.centre.z).powi(2)).sqrt() / ext;
                let q = cam.project(p);
                if let (Some(a), Some(b)) = (prev, q) {
                    let fade = (1.0 - r).max(0.0) as f32;
                    img.line(a[0], a[1], b[0], b[1], [0.17, 0.18, 0.2], 0.6 * fade * fade);
                }
                prev = q;
            }
        }
    }
}

/// Render one view of a mesh, `size` pixels square.
pub fn render_view(mesh: &Mesh, colours: &[Rgb; 8], fr: &Frame, az: f64, el: f64, size: usize) -> Image {
    let s = size * SUPERSAMPLE;
    let cam = Camera::orbit(fr.centre, fr.radius, az, el, FOV_DEG, s, s);
    let mut img = Image::new(s, s, [0.0; 3]);
    img.gradient([0.105, 0.115, 0.13], [0.035, 0.038, 0.042]);
    let lights = Lights {
        key: (cam.up * 0.85 - cam.right * 0.55 - cam.fwd * 0.45).norm(),
        fill: (cam.right * 0.8 + cam.up * 0.05 - cam.fwd * 0.35).norm(),
        rim: (cam.up * 0.35 + cam.fwd * 0.95).norm(),
        view: -cam.fwd,
    };

    if el > 0.0 {
        draw_ground(&mut img, &cam, fr);
        // Contact shadow cast by a high sun from the key light's side.
        let kh = v3(lights.key.x, 0.0, lights.key.z).norm();
        let sun = (kh * 0.6 + v3(0.0, 1.0, 0.0)).norm();
        let mut shadow = vec![false; s * s];
        for t in mesh.indices.chunks_exact(3) {
            let q: Option<Vec<[f64; 3]>> = t
                .iter()
                .map(|&i| {
                    let p = pos(mesh, i as usize);
                    cam.project(p - sun * ((p.y - fr.ground) / sun.y))
                })
                .collect();
            if let Some(q) = q {
                raster_tri(s, s, [[q[0][0], q[0][1]], [q[1][0], q[1][1]], [q[2][0], q[2][1]]], |x, y, _| shadow[y * s + x] = true);
            }
        }
        for (i, sh) in shadow.iter().enumerate() {
            if *sh {
                img.px[i] = scale(img.px[i], 0.45);
            }
        }
    }

    let mut tgt = Target::new(s, s);
    for (ti, t) in mesh.indices.chunks_exact(3).enumerate() {
        let ix = [t[0] as usize, t[1] as usize, t[2] as usize];
        let p = ix.map(|i| pos(mesh, i));
        let n = nrm(mesh, ix[0]);
        if n.dot(cam.eye - p[0]) <= 0.0 {
            continue; // back face
        }
        let (Some(a), Some(b), Some(c)) = (cam.project(p[0]), cam.project(p[1]), cam.project(p[2])) else { continue };
        let slot = Slot::ALL[mesh.slots[ix[0]] as usize];
        let (direct, ambient, emit) = shade_face(colours[slot.index() as usize], slot, mesh.edge[ix[0]], n, &lights);
        let ao = ix.map(|i| mesh.ao[i]);
        tgt.triangle([a, b, c], ti as u32 + 1, |w| {
            let o = w[0] * ao[0] + w[1] * ao[1] + w[2] * ao[2];
            let d = scale(direct, 0.55 + 0.45 * o);
            let am = scale(ambient, 0.1 + 0.9 * o);
            [d[0] + am[0] + emit[0], d[1] + am[1] + emit[1], d[2] + am[2] + emit[2]]
        });
    }
    for i in 0..s * s {
        if tgt.id[i] != 0 {
            img.px[i] = tgt.colour[i];
        }
    }
    // Ink outlines: darken covered pixels that border the background or a much farther surface.
    let jump = (fr.radius * 0.06) as f32;
    let mut dark = vec![false; s * s];
    for y in 1..s - 1 {
        for x in 1..s - 1 {
            let i = y * s + x;
            if tgt.id[i] == 0 {
                continue;
            }
            let d = tgt.depth[i];
            for (dx, dy) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1), (-1, -1), (1, 1), (-1, 1), (1, -1)] {
                let j = (y as i64 + dy) as usize * s + (x as i64 + dx) as usize;
                if tgt.id[j] == 0 || tgt.depth[j] - d > jump {
                    dark[i] = true;
                    break;
                }
            }
        }
    }
    for (i, d) in dark.iter().enumerate() {
        if *d {
            img.px[i] = scale(img.px[i], 0.3);
        }
    }
    img.downsample(SUPERSAMPLE)
}

fn mass_text(kg: f64) -> String {
    if kg < 1000.0 {
        format!("{kg:.1} kg")
    } else {
        format!("{:.2} t", kg / 1000.0)
    }
}

fn kw_text(kw: f64) -> String {
    if kw < 10.0 {
        format!("{kw:.1}")
    } else {
        format!("{kw:.0}")
    }
}

/// Human-readable stats for the panel under the views.
pub fn stats_lines(st: &StatsFile, grid_m: f64) -> Vec<(String, Rgb)> {
    let m = &st.mass;
    let d = [0, 1, 2].map(|i| m.bounds_max[i] - m.bounds_min[i]);
    let a = &st.armour;
    let mw = |az: f64, el: f64| {
        let s = a.at(az, el);
        format!("{:.0}/{:.0}", s.mean_mm, s.weak_mm)
    };
    let mut out = vec![
        (format!("mass {}   {} tris   voxel {:.0} mm", mass_text(m.mass_kg), st.triangles, st.voxel_cell_m * 1000.0), TEXT),
        (format!("size {:.2} w x {:.2} h x {:.2} l m   grid {} m", d[0], d[1], d[2], grid_m), TEXT),
        (format!("material {:.3} m3   internal {:.3} m3", m.material_m3, m.internal_m3), TEXT),
        (
            format!("centre of mass {:+.2} {:+.2} {:+.2}", m.centre_of_mass[0], m.centre_of_mass[1], m.centre_of_mass[2]),
            TEXT,
        ),
        (format!("armour mm mean/weak: front {}  side {}", mw(0.0, 0.0), mw(90.0, 0.0)), TEXT),
        (format!("  rear {}  top {}  low {}", mw(180.0, 0.0), mw(0.0, 90.0), mw(0.0, -30.0)), TEXT),
        (format!("area m2: front {:.2}  side {:.2}  top {:.2}", a.at(0.0, 0.0).area_m2, a.at(90.0, 0.0).area_m2, a.at(0.0, 90.0).area_m2), TEXT),
    ];
    if let Some(v) = &st.vehicle {
        out.push((format!("power {} kW   draw {} kW   load {}", kw_text(v.power_kw), kw_text(v.draw_kw), mass_text(v.load_kg)), TEXT));
        out.push((format!("top speed {:.0} km/h ({})   {:.1} kW/t", v.top_speed_kmh, v.speed_limited_by, v.power_to_weight_kw_t), TEXT));
        let hover = if v.hover_kw > 0.0 { format!("   hover {} kW", kw_text(v.hover_kw)) } else { String::new() };
        out.push((format!("{:?}{hover}", v.locomotion), TEXT));
        for p in &v.problems {
            out.push((format!("! {p}"), WARN));
        }
    }
    out
}

fn circle(img: &mut Image, cx: f64, cy: f64, r: f64, c: Rgb, a: f32) {
    let n = 72;
    for k in 0..n {
        let (a0, a1) = (k as f64 / n as f64 * std::f64::consts::TAU, (k + 1) as f64 / n as f64 * std::f64::consts::TAU);
        img.line(cx + r * a0.cos(), cy + r * a0.sin(), cx + r * a1.cos(), cy + r * a1.sin(), c, a);
    }
}

/// Polar plot of armour around the vehicle at one elevation: front at the top, right side to the right.
/// Mean thickness is drawn in amber, weak spots (10th percentile) in red.
pub fn polar_plot(img: &mut Image, cx: f64, cy: f64, r: f64, table: &ArmourTable, el: f64, label: &str) {
    let Some(row) = table.elevations.iter().position(|e| *e == el) else { return };
    let row = &table.rows[row];
    let maxv = row.iter().map(|d| d.mean_mm).fold(0.0, f64::max).max(10.0);
    let step = nice_step(maxv / 2.0);
    let rings = (maxv / step).ceil().max(1.0) as usize;
    let maxr = rings as f64 * step;
    for k in 1..=rings {
        circle(img, cx, cy, r * k as f64 / rings as f64, DIM, 0.8);
        img.text((cx + 3.0) as i64, (cy - r * k as f64 / rings as f64 - 9.0) as i64, 1, &format!("{:.0}", k as f64 * step), DIM);
    }
    for k in 0..8 {
        let a = k as f64 * std::f64::consts::FRAC_PI_4;
        img.line(cx, cy, cx + r * a.sin(), cy - r * a.cos(), DIM, 0.5);
    }
    let n = row.len();
    let pt = |i: usize, v: f64| {
        let a = (i % n) as f64 / n as f64 * std::f64::consts::TAU;
        let rr = r * v / maxr;
        (cx + rr * a.sin(), cy - rr * a.cos())
    };
    for (vals, c) in [(row.iter().map(|d| d.weak_mm).collect::<Vec<_>>(), WARN), (row.iter().map(|d| d.mean_mm).collect(), [0.95, 0.62, 0.15])] {
        for i in 0..n {
            let (x0, y0) = pt(i, vals[i]);
            let (x1, y1) = pt(i + 1, vals[(i + 1) % n]);
            img.line(x0, y0, x1, y1, c, 1.0);
        }
    }
    img.text((cx - 4.0) as i64, (cy - r - 14.0) as i64, 1, "F", TEXT);
    img.text((cx - r) as i64, (cy + r + 8.0) as i64, 1, label, TEXT);
}

/// The full sheet: eight views over a stats panel with two armour plots.
pub fn contact_sheet(mesh: &Mesh, colours: &[Rgb; 8], stats: &StatsFile) -> Image {
    let fr = Frame::of(mesh);
    let lines = stats_lines(stats, fr.grid);
    let w = TILE * 4;
    let panel_h = (64 + lines.len() * 22).max(300);
    let mut img = Image::new(w, TILE * 2 + panel_h, [0.018, 0.019, 0.022]);
    let tiles: Vec<Image> = std::thread::scope(|s| {
        let fr = &fr;
        let hs: Vec<_> = VIEWS.iter().map(|&(az, _)| s.spawn(move || render_view(mesh, colours, fr, az, 25.0, TILE))).collect();
        hs.into_iter().map(|h| h.join().expect("render thread panicked")).collect()
    });
    for (k, t) in tiles.iter().enumerate() {
        let (x, y) = ((k % 4) * TILE, (k / 4) * TILE);
        img.blit(t, x, y);
        img.text(x as i64 + 8, y as i64 + 8, 1, VIEWS[k].1, DIM);
    }
    let y0 = (TILE * 2) as i64;
    img.text(16, y0 + 14, 3, &stats.name, [0.85, 0.86, 0.88]);
    img.text(16 + 8 * 3 * (stats.name.len() as i64 + 1), y0 + 22, 1, &stats.id, DIM);
    for (i, (l, c)) in lines.iter().enumerate() {
        img.text(16, y0 + 54 + i as i64 * 22, 2, l, *c);
    }
    let pr = 100.0;
    let py = y0 as f64 + 36.0 + pr;
    polar_plot(&mut img, w as f64 - 380.0, py, pr, &stats.armour, 0.0, "armour mm, level");
    polar_plot(&mut img, w as f64 - 130.0, py, pr, &stats.armour, 25.0, "from 25 deg above");
    img
}

/// A grid of labelled thumbnails (one hero view per item).
pub fn overview(items: &[(String, Image)], cols: usize) -> Image {
    let cols = cols.max(1);
    let t = items.first().map(|i| i.1.w).unwrap_or(1);
    let rows = items.len().div_ceil(cols);
    let mut img = Image::new(t * cols, (t + 20) * rows.max(1), [0.018, 0.019, 0.022]);
    for (k, (name, im)) in items.iter().enumerate() {
        let (x, y) = ((k % cols) * t, (k / cols) * (t + 20));
        img.blit(im, x, y);
        img.text(x as i64 + 6, (y + t + 6) as i64, 1, name, TEXT);
    }
    img
}
