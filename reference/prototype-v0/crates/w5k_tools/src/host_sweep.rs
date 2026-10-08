//! Sweeps of a family mounted on its host hull: `<family>_sweep.png`.
//!
//! The first two budgeted sliders are swept (rows and columns) while everything else stays at its default, and each
//! variant is a whole vehicle: the host hull, an engine and the family on every matching socket. So each tile shows
//! the look *and* the vehicle sheet (mass, speed, ground pressure, lift power, problems).

use std::collections::BTreeMap;
use std::path::Path;

use w5k_forge::family::{self, Family, Host, Param, Role, Values};
use w5k_forge::mesh::Mesh;
use w5k_forge::preview::{self, Frame};
use w5k_forge::raster::{Image, Rgb};
use w5k_forge::schema::{Attach, DesignDef, MaterialLibrary, SocketKind};
use w5k_forge::Forge;

const TEXT: Rgb = [0.62, 0.64, 0.67];
const DIM: Rgb = [0.3, 0.31, 0.33];
const WARN: Rgb = [0.9, 0.2, 0.14];
const BG: Rgb = [0.018, 0.019, 0.022];
pub const TILE: usize = 300;

fn fmt(x: f64) -> String {
    if x >= 100.0 {
        format!("{x:.0}")
    } else if x >= 10.0 {
        format!("{x:.1}")
    } else {
        format!("{x:.2}")
    }
}

/// A design that puts `fam` (with `values`) on its host hull, plus a turbine to power it.
pub fn host_design(host: &Host, fam: &dyn Family, values: &Values, power_kw: f64) -> DesignDef {
    let params: BTreeMap<String, f64> = values.iter().filter(|(k, _)| !k.starts_with("ctx.")).map(|(k, v)| (k.clone(), *v)).collect();
    let mut attach = vec![Attach {
        socket: host.socket.into(),
        part: String::new(),
        family: Some(fam.id().into()),
        params,
        mirror: false,
        spin: 0.0,
        children: vec![],
    }];
    // Families that are not running gear (weapons, sensors) ride on ordinary tracks.
    let gear = [SocketKind::Station, SocketKind::Hip, SocketKind::Keel, SocketKind::Belly, SocketKind::Gear];
    if !fam.fits().iter().any(|k| gear.contains(k)) {
        attach.push(Attach { socket: "gear_*".into(), part: String::new(), family: Some("track".into()), params: BTreeMap::new(), mirror: false, spin: 0.0, children: vec![] });
    }
    if power_kw > 0.0 {
        attach.push(Attach {
            socket: "engine".into(),
            part: String::new(),
            family: Some("engine".into()),
            params: [("power_kw".to_string(), power_kw), ("tech".to_string(), 1.0)].into_iter().collect(),
            mirror: false,
            spin: 0.0,
            children: vec![],
        });
    }
    DesignDef {
        id: format!("host_{}", fam.id()),
        name: format!("{} on {}", fam.name(), host.hull),
        hull: host.hull.into(),
        hull_params: host.hull_params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        palette: Some("vanguard".into()),
        attach,
        lift_m: 0.0,
    }
}

struct Cell {
    mesh: Mesh,
    label: String,
    stats: String,
    problem: Option<String>,
}

fn evaluate(lib: &MaterialLibrary, host: &Host, fam: &dyn Family, values: &Values, label: String) -> Cell {
    let mut forge = Forge::new(lib.clone());
    // Size the engine to the vehicle: a first guess, then installed power = kw_per_t x the mass that came out.
    let mut power = 400.0;
    let mut out = None;
    for pass in 0..2 {
        let design = host_design(host, fam, values, power);
        match forge.instantiate(&design) {
            Ok(d) => {
                let (built, _, s) = forge.build_design(&d);
                if pass == 0 {
                    power = (host.kw_per_t * s.mass_kg / 1000.0).clamp(30.0, 60_000.0);
                    out = Some((built, s));
                } else {
                    out = Some((built, s));
                }
            }
            Err(e) => return Cell { mesh: Mesh::default(), label, stats: String::new(), problem: Some(e) },
        }
    }
    let (built, s) = out.unwrap();
    let mut stats = format!("{} t  {:.0} km/h", fmt(s.mass_kg / 1000.0), s.top_speed_kmh);
    if s.ground_pressure_kpa > 0.0 {
        stats += &format!("  {:.0} kPa", s.ground_pressure_kpa);
    }
    if s.lift_kw > 0.0 {
        stats += &format!("  lift {} kW", fmt(s.lift_kw));
    }
    Cell { mesh: built.mesh, label, stats, problem: s.problems.first().cloned() }
}

fn bounds(cells: &[Cell]) -> Frame {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for c in cells {
        for p in &c.mesh.positions {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
    }
    let (lo, hi) = (lo.map(|x| x as f64), hi.map(|x| x as f64));
    let centre = w5k_forge::geom::v3((lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0);
    let r = 0.5 * ((hi[0] - lo[0]).powi(2) + (hi[1] - lo[1]).powi(2) + (hi[2] - lo[2]).powi(2)).sqrt() * 1.04;
    Frame { centre, radius: r, ground: lo[1], grid: preview::nice_step(r / 2.0) }
}

/// Which sliders to sweep: the first two budgeted ones (falling back to the first two).
fn axes(fam: &dyn Family) -> (Param, Param) {
    let params = fam.params();
    let mut picks: Vec<&Param> = params.iter().filter(|p| p.role == Role::Budgeted).collect();
    for p in &params {
        if picks.len() >= 2 {
            break;
        }
        if !picks.iter().any(|q| q.id == p.id) {
            picks.push(p);
        }
    }
    (picks[0].clone(), picks[1.min(picks.len() - 1)].clone())
}

pub fn run(lib: &MaterialLibrary, out: &Path, only: Option<&[String]>) {
    for fam in family::all() {
        let Some(host) = fam.host() else { continue };
        if let Some(o) = only {
            if !o.iter().any(|x| x == fam.id()) {
                continue;
            }
        }
        sweep(lib, fam.as_ref(), &host, out);
    }
}

fn sweep(lib: &MaterialLibrary, fam: &dyn Family, host: &Host, out: &Path) {
    let (pa, pb) = axes(fam);
    let rows = [0.12, 0.5, 0.88];
    let cols = [0.0, 0.25, 0.5, 0.75, 1.0];
    let jobs: Vec<(usize, usize, Values, String)> = rows
        .iter()
        .enumerate()
        .flat_map(|(r, &tr)| {
            cols.iter().enumerate().map(move |(c, &tc)| (r, c, tr, tc))
        })
        .map(|(r, c, tr, tc)| {
            let (va, vb) = (pa.value_at(tr), pb.value_at(tc));
            let v = fam.with(&[(pa.id, va), (pb.id, vb)]);
            (r, c, v, format!("{} {}  {} {}", pa.name, fmt(va), pb.name, fmt(vb)))
        })
        .collect();
    let mut cells: Vec<Option<Cell>> = (0..jobs.len()).map(|_| None).collect();
    std::thread::scope(|s| {
        let handles: Vec<_> = jobs
            .chunks(4)
            .map(|chunk| {
                s.spawn(move || {
                    chunk
                        .iter()
                        .map(|(r, c, v, label)| (*r * cols.len() + *c, evaluate(lib, host, fam, v, label.clone())))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for h in handles {
            for (i, cell) in h.join().expect("sweep thread panicked") {
                cells[i] = Some(cell);
            }
        }
    });
    let cells: Vec<Cell> = cells.into_iter().map(|c| c.unwrap()).collect();
    let mut frames: Vec<Frame> = (0..rows.len()).map(|r| bounds(&cells[r * cols.len()..(r + 1) * cols.len()])).collect();
    if fam.id() == "rail" {
        // Bogies are a thin strip under a long hull: look at the nose end, low down and close in.
        for fr in frames.iter_mut() {
            fr.centre.y = fr.ground + 0.2 * fr.radius;
            fr.centre.z -= 0.64 * fr.radius;
            fr.radius *= 0.45;
        }
    }
    let colours = preview::palette_colours(lib, Some("vanguard"));
    // Running gear under a hull (rail bogies) is best seen from low and to the side.
    let (az, el) = if fam.id() == "rail" { (62.0, 7.0) } else { (35.0, 24.0) };
    let (hdr, lbl) = (36usize, 44usize);
    let mut img = Image::new(TILE * cols.len(), hdr + (TILE + lbl) * rows.len(), BG);
    img.text(10, 10, 2, &format!("{}: {} (down) x {} (across), on {}", fam.name(), pa.name, pb.name, host.hull), [0.85, 0.86, 0.88]);
    for (i, cell) in cells.iter().enumerate() {
        let (r, c) = (i / cols.len(), i % cols.len());
        let (x, y) = (c * TILE, hdr + r * (TILE + lbl));
        if !cell.mesh.positions.is_empty() {
            img.blit(&preview::render_view(&cell.mesh, &colours, &frames[r], az, el, TILE), x, y);
        }
        img.text(x as i64 + 5, (y + TILE + 6) as i64, 1, &cell.label, TEXT);
        img.text(x as i64 + 5, (y + TILE + 19) as i64, 1, &cell.stats, DIM);
        if let Some(p) = &cell.problem {
            let short: String = p.chars().take(TILE / 8 - 1).collect();
            img.text(x as i64 + 5, (y + TILE + 31) as i64, 1, &short, WARN);
        }
    }
    let path = out.join(format!("{}_sweep.png", fam.id()));
    match img.save_png(&path) {
        Ok(()) => println!("sweep {} -> {}", fam.id(), path.display()),
        Err(e) => eprintln!("error: {e}"),
    }
}
