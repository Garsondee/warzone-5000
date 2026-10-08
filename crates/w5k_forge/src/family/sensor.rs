//! Sensor mast: optics, radar or both on a mast, as one parametric family.
//!
//! * **Eye height buys horizon.** The sheet limits sight to the horizon distance sqrt(2 R h) + sqrt(2 R 3 m): a
//!   tall mast sees over the hills, but a mast is a column that must stay stiff under its own load, so its mass
//!   grows steeply with height (the walls thicken with the height too).
//! * **Optics** see detail but their range grows slowly with the aperture, R = 4 km (D / 0.5 m)^0.6, and stop
//!   at haze.
//! * **Radar** follows the radar equation: R^4 is proportional to power x antenna gain^2 / ..., and gain grows with
//!   the aperture, so R = 10 km D^0.9 (P / 50 kW)^0.25. It sees farther and through weather, but it draws
//!   power, and its emissions are heard by everyone.
//! * Both want the mast: a head must fit the socket (`ctx.mast_max`).

use super::style::{self, p};
use super::{stat, Family, Host, Param, Role, Scale, Stat, Values};
use crate::geom::V3;
use crate::schema::{Axis, Category, Function, MaterialLibrary, Node, PartDef, SensorFn, SizeClass, SocketDef, SocketKind, Slot};
use crate::Built;

pub struct Sensor;

fn param(id: &'static str, name: &'static str, unit: &'static str, min: f64, max: f64, default: f64, scale: Scale, role: Role, help: &'static str) -> Param {
    Param { id, name, unit, min, max, default, scale, role, help }
}

#[derive(Clone, Copy, Debug)]
pub struct Reach {
    pub optical_m: f64,
    pub radar_m: f64,
    pub range_m: f64,
    pub draw_kw: f64,
    pub kind: &'static str,
}

pub fn reach(v: &Values) -> Reach {
    let (d, radar) = (v["aperture_m"], v["radar"]);
    let draw = 0.3 + 60.0 * radar * d * d;
    let optical = 4000.0 * (d / 0.5).powf(0.6);
    let rad = 10_000.0 * d.powf(0.9) * (((5.0 + 60.0 * d * d) * radar.max(0.02)) / 50.0).powf(0.25);
    let range = ((1.0 - radar).sqrt() * optical).max(radar.sqrt() * rad);
    Reach { optical_m: optical, radar_m: rad, range_m: range, draw_kw: draw, kind: if radar < 0.25 { "optical" } else if radar >= 0.75 { "radar" } else { "combined" } }
}

impl Family for Sensor {
    fn id(&self) -> &'static str {
        "sensor"
    }
    fn name(&self) -> &'static str {
        "Sensor mast"
    }
    fn params(&self) -> Vec<Param> {
        vec![
            param("height_m", "Mast height", "m", 0.2, 30.0, 2.0, Scale::Log, Role::Budgeted, "Eye height: the horizon grows with its square root; the mast gets heavy fast."),
            param("aperture_m", "Aperture", "m", 0.05, 6.0, 0.5, Scale::Log, Role::Budgeted, "Lens or antenna size: bigger sees farther and weighs more."),
            param("radar", "Radar share", "", 0.0, 1.0, 0.5, Scale::Linear, Role::Free, "Optics (0) see detail but not far; radar (1) sees far and through weather but draws power."),
        ]
    }
    fn fits(&self) -> &'static [SocketKind] {
        &[SocketKind::Mast]
    }
    fn host(&self) -> Option<Host> {
        Some(Host { hull: "hull_lancer", hull_params: &[("length_m", 7.0), ("width_m", 3.2), ("height_m", 1.1)], socket: "mast_1", kw_per_t: 18.0 })
    }

    fn generate(&self, v: &Values, _lib: &MaterialLibrary) -> PartDef {
        let (h, d, radar) = (v["height_m"], v["aperture_m"], v["radar"]);
        let r = reach(v);
        let mut shapes: Vec<Node> = Vec::new();
        // Mast: a tapering tube, its walls thickening with the height.
        let rm = 0.035 + 0.012 * h + 0.05 * d;
        let wall = 0.004 + 0.0012 * h;
        shapes.push(style::cyl(rm, h, Axis::Y, 12, 0.7, p(0.0, h / 2.0, 0.0), "steel", Slot::Secondary, Some(wall), 0.0));
        shapes.push(style::cyl(1.6 * rm, 0.1 + 0.03 * h, Axis::Y, 12, 1.0, p(0.0, 0.05 + 0.015 * h, 0.0), "steel", Slot::Dark, None, 0.0));
        shapes.push(style::cyl(1.2 * rm * 0.7, 0.04, Axis::Y, 12, 1.0, p(0.0, 0.8 * h, 0.0), "fittings", Slot::Trim, None, 0.0));
        let top = p(0.0, h, 0.0);
        shapes.push(style::cyl(1.3 * rm * 0.7 + 0.02, 0.12 * d + 0.05, Axis::Y, 12, 1.0, top + p(0.0, 0.03, 0.0), "machinery", Slot::Dark, None, 0.0));
        // Optical head: a ball with a lens barrel and glowing lens, to the front.
        let so = d * (1.0 - radar).sqrt() + 0.08;
        let radar_w = radar.sqrt();
        let optical = |at: V3, s: f64, shapes: &mut Vec<Node>| {
            shapes.push(style::sphere(0.5 * s, at, "electronics", Slot::Secondary));
            shapes.push(style::cyl(0.28 * s, 0.7 * s, Axis::Z, 12, 1.0, at + p(0.0, 0.0, -0.5 * s), "electronics", Slot::Dark, None, 0.0));
            shapes.push(style::cyl(0.22 * s, 0.03 * s + 0.01, Axis::Z, 12, 1.0, at + p(0.0, 0.0, -0.86 * s), "glass", Slot::Glow, None, 0.0));
            shapes.push(style::bx([0.18 * s, 0.1 * s, 0.4 * s], at + p(0.0, 0.34 * s, -0.2 * s), "electronics", Slot::Trim, 0.0));
        };
        // Radar head: a flat array on a yoke (swept back like the fins), glowing face.
        let sr = d * radar_w + 0.1;
        let array = |at: V3, s: f64, shapes: &mut Vec<Node>| {
            shapes.push(style::bx([1.1 * s, 0.7 * s, 0.1 * s], at, "electronics", Slot::Primary, 0.01 * s));
            shapes.push(style::bx([0.98 * s, 0.58 * s, 0.02 * s], at + p(0.0, 0.0, -0.055 * s), "fittings", Slot::Glow, 0.0));
            shapes.push(style::bx([0.12 * s, 0.5 * s, 0.25 * s], at + p(0.0, -0.1 * s, 0.14 * s), "electronics", Slot::Dark, 0.0));
        };
        let head_y = top + p(0.0, 0.12 * d + 0.06, 0.0);
        if radar < 0.25 {
            optical(head_y + p(0.0, 0.5 * so, 0.0), so, &mut shapes);
        } else if radar >= 0.75 {
            array(head_y + p(0.0, 0.4 * sr, 0.0), sr, &mut shapes);
        } else {
            array(head_y + p(0.0, 0.4 * sr, 0.0), sr, &mut shapes);
            optical(head_y + p(0.0, 0.8 * sr + 0.5 * so, -0.3 * sr), 0.8 * so, &mut shapes);
        }
        let head_w = if radar < 0.25 { so } else { 1.1 * sr };
        PartDef {
            id: format!("sensor_{h:.1}m_{d:.2}m_{radar:.2}"),
            name: format!("{} mast {h:.1} m", r.kind),
            category: Category::Sensor,
            size: SizeClass::Small,
            tags: vec!["sensor".into()],
            palette: None,
            voxels: Some(96),
            vital: Some(false),
            shapes,
            sockets: vec![SocketDef { name: "mount".into(), kind: SocketKind::Mount, size: SizeClass::Small, at: [0.0; 3], normal: [0.0, -1.0, 0.0], forward: [0.0, 0.0, -1.0], hints: Default::default() }],
            function: Function {
                draw_kw: r.draw_kw,
                mast_m: (1.6 * rm).max(head_w * 0.5),
                sensor: Some(SensorFn { kind: r.kind.into(), range_m: r.range_m, height_m: h }),
                ..Default::default()
            },
        }
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        let r = reach(v);
        vec![
            stat("sensor mass", built.mass.mass_kg, "kg"),
            stat("optical range", r.optical_m / 1000.0, "km"),
            stat("radar range", r.radar_m * v["radar"].sqrt() / 1000.0, "km"),
            stat("detection range", r.range_m / 1000.0, "km"),
            stat("horizon from the mast", super::super::sheet::horizon_m(v["height_m"]) / 1000.0, "km"),
            stat("power draw", r.draw_kw, "kW"),
        ]
    }
}
