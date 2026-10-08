//! Track units, sized by the hull they bolt onto.
//!
//! The hull's socket says how long the track must be and how far it reaches up and down from the socket
//! (`ctx.length`, `ctx.top`, `ctx.bottom`). The sliders choose the track's **width** and its **skirt armour**:
//! wider tracks spread the vehicle's weight over more ground (lower ground pressure, so less sinking in soft
//! ground) at the cost of mass and a wider vehicle. Wheel count is a free choice of look.
//!
//! Part space: x = 0 is the track's centre line, the mount is on its inner face (-X), the socket height is y = 0.

use super::style::{self, p};
use super::{ctx, stat, Family, Host, Param, Role, Scale, Stat, Values};
use crate::geom::V3;
use crate::schema::{MaterialLibrary, Axis, Category, Function, Locomotion, PartDef, SizeClass, SocketDef, SocketKind, Slot};
use crate::Built;

pub struct Track;

/// Allowable average ground pressure used to rate a track's load (Pa).
const RATED_PRESSURE: f64 = 150_000.0;

impl Family for Track {
    fn id(&self) -> &'static str {
        "track"
    }
    fn name(&self) -> &'static str {
        "Track unit"
    }
    fn fits(&self) -> &'static [SocketKind] {
        &[SocketKind::Gear, SocketKind::Station]
    }
    fn host(&self) -> Option<Host> {
        Some(Host { hull: "hull_bastion", hull_params: &[], socket: "gear_*", kw_per_t: 18.0 })
    }
    fn params(&self) -> Vec<Param> {
        vec![
            Param {
                id: "width_m",
                name: "Track width",
                unit: "m",
                min: 0.25,
                max: 2.5,
                default: 0.6,
                scale: Scale::Linear,
                role: Role::Budgeted,
                help: "Wider tracks lower ground pressure (less sinking, more load) but weigh more.",
            },
            Param {
                id: "skirt_mm",
                name: "Skirt armour",
                unit: "mm",
                min: 0.0,
                max: 60.0,
                default: 0.0,
                scale: Scale::Linear,
                role: Role::Budgeted,
                help: "Side skirts: spaced armour in front of the wheels and the hull side.",
            },
            Param {
                id: "wheels",
                name: "Road wheels",
                unit: "",
                min: 3.0,
                max: 12.0,
                default: 6.0,
                scale: Scale::Linear,
                role: Role::Free,
                help: "Number of road wheels per side (looks).",
            },
        ]
    }

    fn generate(&self, v: &Values, _lib: &MaterialLibrary) -> PartDef {
        let w = v["width_m"];
        let lt = ctx(v, "ctx.length", 6.0);
        let top = ctx(v, "ctx.top", 0.3);
        let bot = -ctx(v, "ctx.bottom", 0.7);
        let ht = top - bot;
        let n = v["wheels"].round().clamp(3.0, 12.0) as usize;
        let prof = [
            (-0.40 * lt, bot),
            (0.40 * lt, bot),
            (0.49 * lt, bot + 0.35 * ht),
            (0.5 * lt, bot + 0.62 * ht),
            (0.45 * lt, top),
            (-0.45 * lt, top),
            (-0.5 * lt, bot + 0.62 * ht),
            (-0.49 * lt, bot + 0.3 * ht),
        ];
        let pts: Vec<V3> = [-w / 2.0, w / 2.0].iter().flat_map(|&x| prof.iter().map(move |&(z, y)| p(x, y, z))).collect();
        let mut shapes = vec![style::hull(pts, "running_gear", Slot::Dark, None, (0.06 * ht).min(0.06))];
        // Cleats on the top run.
        let cleats = ((0.88 * lt) / (0.12 + 0.25 * ht)).floor().max(4.0) as usize;
        for k in 0..cleats {
            let z = -0.43 * lt + 0.86 * lt * (k as f64 + 0.5) / cleats as f64;
            shapes.push(style::bx([w + 0.02, 0.03 + 0.03 * ht, 0.05 + 0.08 * ht], p(0.0, top + 0.005, z), "running_gear", Slot::Rubber, 0.0));
        }
        // Road wheels with glowing hub caps, sprocket and idler.
        let r = (0.4 * ht).min(0.8 * lt / (2.0 * n as f64));
        let x_out = w / 2.0;
        let wl = 0.1 + 0.06 * w;
        for k in 0..n {
            let z = -0.4 * lt + r + (0.8 * lt - 2.0 * r) * if n > 1 { k as f64 / (n - 1) as f64 } else { 0.5 };
            shapes.push(style::cyl(r, wl, Axis::X, 14, 1.0, p(x_out - 0.3 * wl, bot + r, z), "running_gear", Slot::Secondary, None, 0.12 * r));
            shapes.push(style::cyl(0.38 * r, 0.04 + 0.2 * wl, Axis::X, 8, 1.0, p(x_out + 0.25 * wl, bot + r, z), "running_gear", Slot::Metal, None, 0.0));
            if k == 0 || k + 1 == n {
                // Only the end wheels carry a glowing cap: a small accent, not a light show.
                shapes.push(style::cyl(0.12 * r, 0.02 + 0.2 * wl, Axis::X, 8, 1.0, p(x_out + 0.36 * wl, bot + r, z), "fittings", Slot::Glow, None, 0.0));
            }
        }
        let rs = 0.42 * ht;
        // Sprocket (rear) and idler (front) sit inside the ends of the run, so the unit is exactly as long as asked.
        shapes.push(style::cyl(rs, wl * 1.3, Axis::X, 10, 1.0, p(x_out - 0.3 * wl, bot + 0.6 * ht, 0.5 * lt - 1.02 * rs), "running_gear", Slot::Dark, None, 0.1 * rs));
        shapes.push(style::cyl(0.9 * rs, wl * 1.2, Axis::X, 12, 1.0, p(x_out - 0.3 * wl, bot + 0.6 * ht, -0.5 * lt + 0.94 * rs), "running_gear", Slot::Secondary, None, 0.1 * rs));
        // Fender over the run.
        let fy = top + 0.06 + 0.02 * ht;
        shapes.push(style::bx([w + 0.12, 0.04 + 0.02 * ht, 0.97 * lt], p(0.02, fy, 0.0), "fittings", Slot::Secondary, 0.012));
        // Optional skirts: real spaced armour, with a trim stripe.
        let skirt = v["skirt_mm"] / 1000.0;
        if skirt > 0.0005 {
            let thick = skirt.max(0.006);
            let (sy0, sy1) = (bot + 0.35 * ht, fy - 0.02);
            let x = w / 2.0 + 0.06 + thick / 2.0;
            shapes.push(style::bx([thick, sy1 - sy0, 0.9 * lt], p(x, (sy0 + sy1) / 2.0, 0.0), "hard_steel", Slot::Primary, (0.4 * thick).min(0.02)));
            shapes.push(style::bx([0.01, 0.08 * ht, 0.86 * lt], p(x + thick / 2.0 + 0.004, sy0 + 0.25 * (sy1 - sy0), 0.0), "fittings", Slot::Trim, 0.0));
        }
        let contact = w * 0.8 * lt;
        PartDef {
            id: format!("track_{w:.2}m_{lt:.1}m"),
            name: format!("Track {w:.2} m"),
            category: Category::Locomotion,
            size: SizeClass::Medium,
            tags: vec!["tracks".into()],
            palette: None,
            voxels: Some(64),
            vital: Some(false),
            shapes,
            sockets: vec![SocketDef {
                name: "mount".into(),
                kind: SocketKind::Mount,
                size: SizeClass::Medium,
                at: [-w / 2.0, 0.0, 0.0],
                normal: [-1.0, 0.0, 0.0],
                forward: [0.0, 0.0, -1.0],
                hints: Default::default(),
            }],
            function: Function {
                locomotion: Some(Locomotion::Tracks),
                load_kg: contact * RATED_PRESSURE / 9.81,
                contact_m2: contact,
                rolling: Some(0.06),
                traction: Some(0.9),
                max_kmh: Some(70.0),
                ..Default::default()
            },
        }
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        let lt = ctx(v, "ctx.length", 6.0);
        vec![
            stat("track mass", built.mass.mass_kg / 1000.0, "t"),
            stat("contact area", v["width_m"] * 0.8 * lt, "m2"),
            stat("rated load", v["width_m"] * 0.8 * lt * RATED_PRESSURE / 9.81 / 1000.0, "t"),
        ]
    }
}
