//! Power plants. One family, three technologies with very different power per kilogram:
//! diesel (cheap, heavy), turbine (light for its power, thirsty later) and fusion cell (very light, rare tech).
//! Mass follows from power: m = P / (specific power); volume from a filled-block density.

use super::style::{self, p};
use super::{stat, Family, Param, Role, Scale, Stat, Values};
use crate::schema::{MaterialLibrary, Axis, Category, Function, PartDef, SizeClass, SocketDef, SocketKind, Slot};
use crate::Built;

pub struct Engine;

/// (name, kW per kg) for each technology.
pub const TECH: [(&str, f64); 3] = [("diesel", 0.22), ("turbine", 0.75), ("fusion cell", 2.5)];
const DENSITY: f64 = 1600.0;

pub fn tech_of(v: &Values) -> usize {
    v["tech"].round().clamp(0.0, 2.0) as usize
}

impl Family for Engine {
    fn id(&self) -> &'static str {
        "engine"
    }
    fn name(&self) -> &'static str {
        "Power plant"
    }
    fn fits(&self) -> &'static [SocketKind] {
        &[SocketKind::Internal]
    }
    fn params(&self) -> Vec<Param> {
        vec![
            Param {
                id: "power_kw",
                name: "Power",
                unit: "kW",
                min: 2.0,
                max: 50_000.0,
                default: 450.0,
                scale: Scale::Log,
                role: Role::Budgeted,
                help: "Output power. Mass grows in proportion; the technology sets how much per kilogram.",
            },
            Param {
                id: "tech",
                name: "Technology",
                unit: "",
                min: 0.0,
                max: 2.0,
                default: 0.0,
                scale: Scale::Linear,
                role: Role::Free,
                help: "0 diesel, 1 turbine, 2 fusion cell (when unlocked).",
            },
        ]
    }

    fn generate(&self, v: &Values, _lib: &MaterialLibrary) -> PartDef {
        let pw = v["power_kw"];
        let tech = tech_of(v);
        let mass = pw / TECH[tech].1;
        let vol = mass / DENSITY;
        // A block of proportions 1.3 : 0.7 : 1.0.
        let a = (vol / 0.91).cbrt();
        let (bw, bh, bl) = (1.3 * a, 0.7 * a, 1.0 * a);
        let mut shapes = vec![style::bx([bw, bh * 0.8, bl], p(0.0, bh * 0.4, 0.0), "machinery", Slot::Metal, 0.05 * a)];
        match tech {
            0 => {
                // Cylinder heads.
                for s in [-1.0, 1.0] {
                    shapes.push(style::bx([0.32 * bw, 0.2 * bh, 0.9 * bl], p(s * 0.24 * bw, 0.9 * bh, 0.0), "machinery", Slot::Dark, 0.02 * a));
                }
            }
            1 => {
                shapes.push(style::cyl(0.33 * bh, 1.05 * bl, Axis::Z, 12, 1.0, p(0.0, 0.95 * bh, 0.0), "machinery", Slot::Dark, None, 0.02 * a));
            }
            _ => {
                // A glowing containment core with dark rings.
                shapes.push(style::cyl(0.3 * bh, 0.9 * bl, Axis::Z, 12, 1.0, p(0.0, 0.95 * bh, 0.0), "fittings", Slot::Glow, None, 0.0));
                for k in 0..3 {
                    let z = -0.3 * bl + 0.3 * bl * k as f64;
                    shapes.push(style::cyl(0.36 * bh, 0.08 * bl, Axis::Z, 12, 1.0, p(0.0, 0.95 * bh, z), "machinery", Slot::Dark, None, 0.0));
                }
            }
        }
        PartDef {
            id: format!("engine_{}_{pw:.0}kw", TECH[tech].0.replace(' ', "_")),
            name: format!("{pw:.0} kW {}", TECH[tech].0),
            category: Category::Engine,
            size: SizeClass::Medium,
            tags: vec!["engine".into()],
            palette: None,
            voxels: Some(48),
            vital: Some(false),
            shapes,
            sockets: vec![SocketDef {
                name: "mount".into(),
                kind: SocketKind::Mount,
                size: SizeClass::Medium,
                at: [0.0; 3],
                normal: [0.0, -1.0, 0.0],
                forward: [0.0, 0.0, -1.0],
                hints: Default::default(),
            }],
            function: Function { power_kw: pw, ..Default::default() },
        }
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        vec![
            stat("engine mass", built.mass.mass_kg / 1000.0, "t"),
            stat("specific power", TECH[tech_of(v)].1, "kW/kg"),
        ]
    }
}
