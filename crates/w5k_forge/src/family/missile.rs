//! Missile rack: a rotating launcher with a grid of tubes, as one parametric family.
//!
//! * A missile is a slender body (length about nine calibres) of low average density, so its mass grows with the
//!   **cube of the calibre**: m = 4950 d^3 (plus a seeker).
//! * The mass splits into warhead, rocket motor and structure. Rocket physics (Tsiolkovsky) gives the burnout speed
//!   dv = Isp g ln(m0 / mf): the more of the mass is warhead, the less propellant, the less range. **Punch against
//!   reach** is the central trade of the family.
//! * A shaped-charge warhead penetrates about six calibres of steel, so a bigger missile punches deeper (with
//!   diminishing returns beyond 150 mm); its energy is the chemical energy of the explosive.
//! * Missiles fly themselves to the target (a seeker adds mass); a rack fires its **whole salvo**, then reloads
//!   from the magazine, slower for heavier missiles.
//! * Launch recoil is small (the missile pushes itself), so a rack mounts on a light hull.

use super::style::{self, p};
use super::{stat, Family, Host, Param, Role, Scale, Stat, Values};
use crate::schema::{Axis, Category, Function, MaterialLibrary, Node, PartDef, SizeClass, SocketDef, SocketKind, Slot, WeaponFn};
use crate::Built;

pub struct Missile;

const G: f64 = 9.81;
const ISP: f64 = 230.0;

fn param(id: &'static str, name: &'static str, unit: &'static str, min: f64, max: f64, default: f64, scale: Scale, role: Role, help: &'static str) -> Param {
    Param { id, name, unit, min, max, default, scale, role, help }
}

/// What one missile of these sliders is and does.
#[derive(Clone, Copy, Debug)]
pub struct Round {
    pub d: f64,
    pub length: f64,
    pub mass_kg: f64,
    pub warhead_kg: f64,
    pub burnout_mps: f64,
    pub range_m: f64,
    pub penetration_mm: f64,
    pub energy_j: f64,
    pub guided: bool,
}

pub fn round(v: &Values) -> Round {
    let d = v["tube_mm"] / 1000.0;
    let seeker = v["seeker"];
    let mass = 4950.0 * d.powi(3) * (1.0 + 0.15 * seeker);
    let wf = v["warhead"];
    let fp = (1.0 - wf - 0.2 - 0.08 * seeker).clamp(0.05, 0.9);
    let dv = ISP * G * (1.0 / (1.0 - fp)).ln();
    let range = 2800.0 * (dv / 1000.0).powf(1.4) * (d / 0.15).powf(0.4);
    let mm = v["tube_mm"];
    let pen_base = if mm <= 150.0 { 6.0 * mm } else { 900.0 * (mm / 150.0).powf(0.6) };
    Round {
        d,
        length: 9.0 * d + 0.08,
        mass_kg: mass,
        warhead_kg: wf * mass,
        burnout_mps: dv,
        range_m: range.clamp(300.0, 40_000.0),
        penetration_mm: pen_base * (wf / 0.3).powf(0.3),
        energy_j: wf * mass * 5.0e6,
        guided: seeker >= 0.5,
    }
}

/// Seconds between salvos: the ripple of the tubes plus a reload from the magazine.
fn cycle_s(r: &Round, tubes: usize) -> f64 {
    6.0 + 18.0 * (r.mass_kg / 20.0).powf(0.3) + 0.4 * tubes as f64
}

struct Layout {
    cols: usize,
    pitch: f64,
    wall: f64,
    wp: f64,
    hp: f64,
    lp: f64,
}

fn layout(v: &Values, r: &Round) -> Layout {
    let tubes = v["tubes"].round().clamp(1.0, 24.0) as usize;
    let cols = ((tubes as f64 * 1.3).sqrt().ceil() as usize).max(1);
    let rows = tubes.div_ceil(cols);
    let pitch = 1.22 * r.d + 0.03;
    let wall = (v["armour_mm"] / 1000.0).max(0.003);
    Layout { cols, pitch, wall, wp: cols as f64 * pitch + 2.0 * wall, hp: rows as f64 * pitch + 2.0 * wall, lp: r.length + 0.25 + 2.0 * wall }
}

impl Family for Missile {
    fn id(&self) -> &'static str {
        "turret_missile"
    }
    fn name(&self) -> &'static str {
        "Missile rack"
    }
    fn params(&self) -> Vec<Param> {
        vec![
            param("tube_mm", "Missile calibre", "mm", 40.0, 600.0, 120.0, Scale::Log, Role::Budgeted, "Missile mass grows with the cube of the calibre; so do warhead and range."),
            param("tubes", "Tubes", "", 1.0, 24.0, 4.0, Scale::Linear, Role::Budgeted, "Missiles in a salvo: the whole rack fires together."),
            param("armour_mm", "Housing armour", "mm", 3.0, 80.0, 10.0, Scale::Linear, Role::Budgeted, "Plate around the tubes: a hit that gets through sets off the missiles inside."),
            param("seeker", "Seeker", "", 0.0, 1.0, 0.7, Scale::Linear, Role::Budgeted, "Guided missiles steer onto their target (and cost mass); unguided ones are lighter and fly farther."),
            param("warhead", "Warhead share", "", 0.1, 0.6, 0.3, Scale::Linear, Role::Free, "Warhead against rocket motor: a big warhead punches harder, a big motor reaches farther."),
            param("elevation", "Elevation", "deg", 0.0, 60.0, 25.0, Scale::Linear, Role::Free, "Launch angle of the rack (looks; steep racks fire over cover)."),
        ]
    }
    fn fits(&self) -> &'static [SocketKind] {
        &[SocketKind::TurretRing]
    }
    fn host(&self) -> Option<Host> {
        Some(Host { hull: "hull_lancer", hull_params: &[("length_m", 7.0), ("width_m", 2.8), ("height_m", 1.1)], socket: "turret", kw_per_t: 18.0 })
    }

    fn generate(&self, v: &Values, _lib: &MaterialLibrary) -> PartDef {
        let r = round(v);
        let tubes = v["tubes"].round().clamp(1.0, 24.0) as usize;
        let l = layout(v, &r);
        let elev = v["elevation"];
        let mut shapes: Vec<Node> = Vec::new();
        // The rotating base: a squat armoured block on a ring bearing.
        let (wb, lb) = (l.wp + 0.14, 0.62 * l.lp + 0.1);
        let hb = 0.3 * l.hp + 0.1;
        let y0 = 0.05 + 0.03 * wb;
        let ring_r = (0.4 * wb.min(lb)).max(0.12);
        shapes.push(style::cyl(ring_r, y0, Axis::Y, 16, 1.0, p(0.0, y0 / 2.0, 0.0), "machinery", Slot::Dark, None, 0.0));
        let sx = 0.16 * hb;
        let mut base = Vec::new();
        for s in [-1.0, 1.0] {
            for z in [-1.0, 1.0] {
                base.push(p(s * wb / 2.0, y0, z * lb / 2.0).arr());
                base.push(p(s * (wb / 2.0 - sx), y0 + hb, z * (lb / 2.0 - sx)).arr());
            }
        }
        shapes.push(Node::Hull { points: base, at: [0.0; 3], rot: [0.0; 3], mat: "steel".into(), slot: Slot::Secondary, shell: Some(l.wall.max(0.005)), chamfer: (0.3 * l.wall).min(0.04) });
        // The pod, tilted about its rear pivot so the muzzle end rises.
        let mut pod: Vec<Node> = Vec::new();
        let (hw, hh, hl) = (l.wp / 2.0, l.hp / 2.0, l.lp / 2.0);
        pod.push(Node::Box { size: [l.wp, l.hp, l.lp], taper: None, shift: None, at: [0.0; 3], rot: [0.0; 3], mat: "steel".into(), slot: Slot::Primary, shell: Some(l.wall), chamfer: (0.3 * l.wall).min(0.05) });
        let ysw = -hh + l.wall;
        for i in 0..tubes {
            let (c, rw) = (i % l.cols, i / l.cols);
            let x = (c as f64 - (l.cols as f64 - 1.0) / 2.0) * l.pitch;
            let y = ysw + (rw as f64 + 0.5) * l.pitch;
            // The missile inside (ordnance), flush toward the muzzle, and its tube mouth with a seeker halo.
            pod.push(style::cyl(0.5 * r.d, r.length, Axis::Z, 10, 1.0, p(x, y, -hl + l.wall + 0.03 + r.length / 2.0), "ordnance", Slot::Dark, None, 0.0));
            if r.guided {
                pod.push(style::cyl(0.66 * r.d, 0.012, Axis::Z, 12, 1.0, p(x, y, -hl - 0.002), "fittings", Slot::Glow, None, 0.0));
            }
            pod.push(style::cyl(0.56 * r.d, 0.02, Axis::Z, 12, 1.0, p(x, y, -hl - 0.006), "fittings", Slot::Dark, None, 0.0));
        }
        let top_q: style::Quad = [p(-hw, hh, -hl), p(hw, hh, -hl), p(hw, hh, hl), p(-hw, hh, hl)];
        let inside = p(0.0, 0.0, 0.0);
        let t = 0.01 + 0.006 * l.wp.min(2.0);
        pod.push(style::plate(&top_q, inside, (0.1, 0.9), (0.14, 0.26), t, 0.4 * t, "fittings", Slot::Trim));
        pod.push(style::plate(&top_q, inside, (0.0, 1.0), (0.6, 1.0), t, t, "fittings", Slot::Secondary));
        let side_q = |s: f64| -> style::Quad { [p(s * hw, -hh, -hl), p(s * hw, -hh, hl), p(s * hw, hh, hl), p(s * hw, hh, -hl)] };
        for s in [-1.0, 1.0] {
            let q = side_q(s);
            shapes_push_glow(&mut pod, &q, inside, 0.5 * t);
        }
        shapes.push(Node::Group { at: [0.0, y0 + hb + 0.01, 0.0], rot: [elev, 0.0, 0.0], scale: 1.0, children: vec![Node::Group { at: [0.0, hh, -0.25 * l.lp], rot: [0.0; 3], scale: 1.0, children: pod }] });
        PartDef {
            id: format!("rack_{:.0}mm_x{tubes}", v["tube_mm"]),
            name: format!("{tubes}x {:.0} mm missile rack", v["tube_mm"]),
            category: Category::Turret,
            size: match wb {
                w if w < 0.8 => SizeClass::Small,
                w if w < 3.0 => SizeClass::Medium,
                w if w < 6.0 => SizeClass::Large,
                _ => SizeClass::Huge,
            },
            tags: vec!["turret".into(), "missile".into()],
            palette: None,
            voxels: Some(96),
            vital: Some(true),
            shapes,
            sockets: vec![SocketDef { name: "mount".into(), kind: SocketKind::Mount, size: SizeClass::Medium, at: [0.0; 3], normal: [0.0, -1.0, 0.0], forward: [0.0, 0.0, -1.0], hints: Default::default() }],
            function: Function {
                draw_kw: 0.5 + 0.3 * tubes as f64,
                ring_m: 2.0 * ring_r,
                weapon: Some(WeaponFn {
                    kind: "missile".into(),
                    energy_j: r.energy_j,
                    shots_per_min: 60.0 * tubes as f64 / cycle_s(&r, tubes),
                    penetration_mm: r.penetration_mm,
                    range_m: r.range_m,
                    recoil_ns: 0.5 * r.mass_kg * 30.0,
                    burst_kw: 0.0,
                    guided: r.guided,
                    salvo: tubes as f64,
                }),
                ..Default::default()
            },
        }
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        let r = round(v);
        let tubes = v["tubes"].round().clamp(1.0, 24.0) as usize;
        vec![
            stat("rack mass", built.mass.mass_kg / 1000.0, "t"),
            stat("missile mass", r.mass_kg, "kg"),
            stat("warhead", r.warhead_kg, "kg"),
            stat("burnout speed", r.burnout_mps, "m/s"),
            stat("range", r.range_m / 1000.0, "km"),
            stat("penetration", r.penetration_mm, "mm"),
            stat("salvo energy", r.energy_j * tubes as f64 / 1e6, "MJ"),
            stat("salvo interval", cycle_s(&r, tubes), "s"),
            stat("ring diameter", 2.0 * (0.4 * (layout(v, &r).wp + 0.14).min(0.62 * layout(v, &r).lp + 0.1)).max(0.12), "m"),
        ]
    }
}

fn shapes_push_glow(out: &mut Vec<Node>, q: &style::Quad, inside: crate::geom::V3, t: f64) {
    out.push(style::plate(q, inside, (0.08, 0.92), (0.82, 0.9), t, 0.0, "fittings", Slot::Glow));
}

/// The ring a rack needs (for fit checks in tests and the roller).
pub fn ring_m(v: &Values) -> f64 {
    let r = round(v);
    let l = layout(v, &r);
    2.0 * (0.4 * (l.wp + 0.14).min(0.62 * l.lp + 0.1)).max(0.12)
}
