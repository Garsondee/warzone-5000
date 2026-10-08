//! Beam turret: an energy weapon that fires from capacitors, as one parametric family.
//!
//! * **Energy per shot** is the beam power times its dwell, E = P t. It is stored in capacitors (25 kJ/kg) and
//!   drawn from the vehicle over the firing cycle, so the *average* draw is E / (efficiency x cycle). A beam turret
//!   has no ammunition, but on a small engine it cannot keep up (the sheet scales its fire by spare power).
//! * **Focus.** Diffraction and turbulence blur the spot as the beam travels: r(R) = 15 mm + 0.06 mrad x R / (D/0.3 m).
//!   A big aperture D keeps the beam tight, which is range.
//! * **Burn-through.** The depth the beam melts in time t is E x coupling / (pi r^2 x 1e10 J/m^3), where 1e10 J/m^3
//!   is the energy to melt a cubic metre of steel. Because it falls with r^2 it rewards a tight beam enormously.
//! * Heat: the waste (60 % of the input) soaks into metal heat sinks (150 kJ/kg) between shots.

use super::style::{self, p};
use super::{stat, Family, Host, Param, Role, Scale, Stat, Values};
use crate::schema::{Axis, Category, Function, MaterialLibrary, Node, PartDef, SizeClass, SocketDef, SocketKind, Slot, WeaponFn};
use crate::Built;

pub struct Beam;

const EFFICIENCY: f64 = 0.4;
const COUPLING: f64 = 0.3;
const MELT_J_M3: f64 = 1.0e10;
const CAP_J_KG: f64 = 25_000.0;
const CAP_DENSITY: f64 = 1500.0;

#[allow(clippy::too_many_arguments)]
fn param(id: &'static str, name: &'static str, unit: &'static str, min: f64, max: f64, default: f64, scale: Scale, role: Role, help: &'static str) -> Param {
    Param { id, name, unit, min, max, default, scale, role, help }
}

#[derive(Clone, Copy, Debug)]
pub struct Shot {
    pub energy_j: f64,
    pub energy_in_j: f64,
    pub cycle_s: f64,
    pub range_m: f64,
    pub spot_1km_m: f64,
    pub penetration_mm: f64,
    pub cap_kg: f64,
    pub sink_kg: f64,
}

pub fn shot(v: &Values) -> Shot {
    let d = v["aperture_m"];
    let e = v["power_mw"] * 1e6 * v["dwell_s"];
    let e_in = e / EFFICIENCY;
    let spot = |range: f64| 0.015 + range * 0.06e-3 / (d / 0.3);
    let r1 = spot(1000.0);
    Shot {
        energy_j: e,
        energy_in_j: e_in,
        cycle_s: v["dwell_s"] + 1.0 + 1.5 * (e / 5.0e6).powf(0.4),
        range_m: ((0.14 - 0.015) * (d / 0.3) / 0.06e-3).clamp(200.0, 20_000.0),
        spot_1km_m: r1,
        penetration_mm: 1000.0 * e * COUPLING / (std::f64::consts::PI * r1 * r1 * MELT_J_M3),
        cap_kg: e_in / CAP_J_KG,
        sink_kg: (1.0 - EFFICIENCY) * e_in / 150_000.0,
    }
}

/// Side of the turret body, from the volume of capacitors and sinks it must hold (they fill about 45 % of it).
fn body(v: &Values, s: &Shot) -> (f64, f64, f64) {
    let vol = (s.cap_kg / CAP_DENSITY + s.sink_kg / 2700.0) / 0.45;
    let w = (vol / 1.04).cbrt().max(0.5 * v["aperture_m"] + 0.2).max(0.3);
    (w, 0.8 * w, 1.3 * w)
}

impl Family for Beam {
    fn id(&self) -> &'static str {
        "turret_beam"
    }
    fn name(&self) -> &'static str {
        "Beam turret"
    }
    fn params(&self) -> Vec<Param> {
        vec![
            param("power_mw", "Beam power", "MW", 0.05, 400.0, 8.0, Scale::Log, Role::Budgeted, "Power in the beam: energy per shot grows with it, and so does the capacitor bank."),
            param("aperture_m", "Aperture", "m", 0.05, 2.0, 0.4, Scale::Log, Role::Budgeted, "Emitter mirror: a wide one keeps the beam tight, which is range and burn-through."),
            param("armour_mm", "Housing armour", "mm", 3.0, 120.0, 15.0, Scale::Linear, Role::Budgeted, "Plate around the capacitors: a hit that gets through disables the weapon."),
            param("dwell_s", "Dwell", "s", 0.1, 2.0, 0.5, Scale::Linear, Role::Free, "Longer dwell deposits more energy per shot, with a longer cooldown."),
        ]
    }
    fn fits(&self) -> &'static [SocketKind] {
        &[SocketKind::TurretRing]
    }
    fn host(&self) -> Option<Host> {
        Some(Host { hull: "hull_lancer", hull_params: &[("length_m", 7.0), ("width_m", 2.8), ("height_m", 1.1)], socket: "turret", kw_per_t: 30.0 })
    }

    fn generate(&self, v: &Values, _lib: &MaterialLibrary) -> PartDef {
        let s = shot(v);
        let d = v["aperture_m"];
        let t = (v["armour_mm"] / 1000.0).max(0.003);
        let (w, h, l) = body(v, &s);
        let y0 = 0.05 + 0.03 * w;
        let ring_r = (0.4 * w.min(l)).max(0.12);
        let mut shapes: Vec<Node> = Vec::new();
        shapes.push(style::cyl(ring_r, y0, Axis::Y, 16, 1.0, p(0.0, y0 / 2.0, 0.0), "machinery", Slot::Dark, None, 0.0));
        // Housing: a faceted block, narrower at the top, like the gun turrets.
        let (tw, tl) = (0.78 * w, 0.84 * l);
        let mut pts = Vec::new();
        for sx in [-1.0, 1.0] {
            for sz in [-1.0, 1.0] {
                pts.push(p(sx * w / 2.0, y0, sz * l / 2.0).arr());
                pts.push(p(sx * tw / 2.0, y0 + h, sz * tl / 2.0).arr());
            }
        }
        shapes.push(Node::Hull { points: pts, at: [0.0; 3], rot: [0.0; 3], mat: "steel".into(), slot: Slot::Primary, shell: Some(t), chamfer: (0.4 * t + 0.01 * w).min(0.08 * w) });
        // Inside: the capacitor bank and the heat sinks (volumes of the right mass).
        let inner_w = tw - 2.0 * t - 0.04;
        let cap_v = s.cap_kg / CAP_DENSITY;
        let cap_h = (0.6 * (h - 2.0 * t)).max(0.02);
        let cap_l = (cap_v / (inner_w * cap_h)).min(0.8 * (tl - 2.0 * t)).max(0.02);
        shapes.push(style::bx([inner_w, cap_h, cap_l], p(0.0, y0 + t + 0.5 * cap_h + 0.01, 0.3 * l - 0.5 * cap_l), "electronics", Slot::Dark, 0.0));
        let sink_v = s.sink_kg / 2700.0;
        let sink_l = (sink_v / (inner_w * 0.25 * h)).clamp(0.01, 0.3 * l);
        shapes.push(style::bx([inner_w, 0.25 * h, sink_l], p(0.0, y0 + t + 0.4 * h, 0.25 * l), "aluminium", Slot::Dark, 0.0));
        // Livery: Secondary side cheeks, a Trim stripe, a glowing capacitor strip, roof vents.
        let top_y = y0 + h;
        let inside = p(0.0, y0 + 0.5 * h, 0.0);
        let q = |sx: f64| -> style::Quad { [p(sx * w / 2.0, y0, -l / 2.0), p(sx * w / 2.0, y0, l / 2.0), p(sx * tw / 2.0, top_y, tl / 2.0), p(sx * tw / 2.0, top_y, -tl / 2.0)] };
        let dt = 0.008 + 0.005 * w;
        for sx in [-1.0, 1.0] {
            shapes.push(style::plate(&q(sx), inside, (0.04, 0.96), (0.0, 0.4), dt, dt, "fittings", Slot::Secondary));
            shapes.push(style::plate(&q(sx), inside, (0.08, 0.7), (0.62, 0.7), 0.7 * dt, 0.0, "fittings", Slot::Glow));
        }
        let roof: style::Quad = [p(-tw / 2.0, top_y, -tl / 2.0), p(tw / 2.0, top_y, -tl / 2.0), p(tw / 2.0, top_y, tl / 2.0), p(-tw / 2.0, top_y, tl / 2.0)];
        shapes.push(style::plate(&roof, inside, (0.1, 0.9), (0.08, 0.16), dt, 0.4 * dt, "fittings", Slot::Trim));
        style::vents(&roof, inside, (0.2, 0.8), (0.62, 0.92), 3, dt, &mut shapes);
        // The emitter: a beam director, cooling rings, a flared mirror cowl and a glowing lens.
        let yb = y0 + 0.5 * h;
        let (rd, len) = (0.18 * d + 0.02, 1.5 * d + 0.3);
        let z_front = -l / 2.0;
        shapes.push(style::cyl(rd, len, Axis::Z, 12, 1.0, p(0.0, yb, z_front - len / 2.0 + 0.1), "gun_steel", Slot::Primary, None, 0.0));
        for k in 0..3 {
            shapes.push(style::cyl(rd * 1.35, 0.05 * d + 0.015, Axis::Z, 12, 1.0, p(0.0, yb, z_front - 0.22 * len - k as f64 * 0.22 * len), "gun_steel", Slot::Secondary, None, 0.0));
        }
        let cowl_l = 0.45 * d + 0.1;
        let z_cowl = z_front - len - 0.5 * cowl_l + 0.1;
        shapes.push(style::cyl(0.5 * d, cowl_l, Axis::Z, 16, 0.5, p(0.0, yb, z_cowl), "gun_steel", Slot::Secondary, Some(0.01 + 0.01 * d), 0.0));
        shapes.push(style::cyl(0.5 * d + 0.02, 0.03 + 0.03 * d, Axis::Z, 16, 1.0, p(0.0, yb, z_cowl - 0.5 * cowl_l), "gun_steel", Slot::Trim, None, 0.0));
        shapes.push(style::cyl(0.38 * d, 0.02 + 0.02 * d, Axis::Z, 16, 1.0, p(0.0, yb, z_cowl - 0.5 * cowl_l - 0.02 - 0.015 * d), "fittings", Slot::Glow, None, 0.0));
        // Mounting yoke blocks either side of the director.
        for sx in [-1.0, 1.0] {
            shapes.push(style::bx([0.12 * w, 0.4 * h, 0.2 * l], p(sx * 0.55 * (rd + 0.3 * w), yb, z_front + 0.04), "steel", Slot::Dark, 0.01));
        }
        PartDef {
            id: format!("beam_{:.1}mw_{:.2}m", v["power_mw"], d),
            name: format!("{:.1} MW beam turret", v["power_mw"]),
            category: Category::Turret,
            size: match w {
                w if w < 0.8 => SizeClass::Small,
                w if w < 3.0 => SizeClass::Medium,
                w if w < 6.0 => SizeClass::Large,
                _ => SizeClass::Huge,
            },
            tags: vec!["turret".into(), "beam".into()],
            palette: None,
            voxels: Some(96),
            vital: Some(true),
            shapes,
            sockets: vec![SocketDef { name: "mount".into(), kind: SocketKind::Mount, size: SizeClass::Medium, at: [0.0; 3], normal: [0.0, -1.0, 0.0], forward: [0.0, 0.0, -1.0], hints: Default::default() }],
            function: Function {
                draw_kw: 0.5 + 2.0 * w * w,
                ring_m: 2.0 * ring_r,
                weapon: Some(WeaponFn {
                    kind: "beam".into(),
                    energy_j: s.energy_j,
                    shots_per_min: 60.0 / s.cycle_s,
                    penetration_mm: s.penetration_mm,
                    range_m: s.range_m,
                    recoil_ns: 0.0,
                    burst_kw: s.energy_in_j / s.cycle_s / 1000.0,
                    guided: true,
                    salvo: 1.0,
                }),
                ..Default::default()
            },
        }
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        let s = shot(v);
        vec![
            stat("turret mass", built.mass.mass_kg / 1000.0, "t"),
            stat("energy per shot", s.energy_j / 1e6, "MJ"),
            stat("capacitors", s.cap_kg, "kg"),
            stat("spot radius at 1 km", s.spot_1km_m * 1000.0, "mm"),
            stat("burn-through at 1 km", s.penetration_mm, "mm"),
            stat("range", s.range_m / 1000.0, "km"),
            stat("cycle", s.cycle_s, "s"),
            stat("average draw", s.energy_in_j / s.cycle_s / 1000.0, "kW"),
        ]
    }
}
