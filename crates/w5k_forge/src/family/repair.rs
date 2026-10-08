//! Repair rig: a crane arm with a welding and fabricating head, as one parametric family.
//!
//! * The rig restores **structure at a rate** (kg of hull per second) and reaches an **arm's length** around it,
//!   so it can mend allies and itself within reach. In the auto-battler this is battlefield sustain.
//! * Rebuilding material costs energy: about 25 kW per kg/s, drawn from the vehicle.
//! * A long arm is a cantilever: the bending moment at its root grows with reach x load, so the root sections
//!   thicken quickly with reach, and the rig gets heavy.

use super::style::{self, p};
use super::{stat, Family, Host, Param, Role, Scale, Stat, Values};
use crate::schema::{Axis, Category, Function, MaterialLibrary, Node, PartDef, SizeClass, SocketDef, SocketKind, Slot};
use crate::Built;

pub struct Repair;

#[allow(clippy::too_many_arguments)]
fn param(id: &'static str, name: &'static str, unit: &'static str, min: f64, max: f64, default: f64, scale: Scale, role: Role, help: &'static str) -> Param {
    Param { id, name, unit, min, max, default, scale, role, help }
}

impl Family for Repair {
    fn id(&self) -> &'static str {
        "repair"
    }
    fn name(&self) -> &'static str {
        "Repair rig"
    }
    fn params(&self) -> Vec<Param> {
        vec![
            param("reach_m", "Reach", "m", 1.5, 30.0, 6.0, Scale::Log, Role::Budgeted, "Arm length: how far from the rig it can mend. The arm is a cantilever, so it gets heavy fast."),
            param("rate", "Repair rate", "kg/s", 0.05, 40.0, 1.0, Scale::Log, Role::Budgeted, "Structure restored per second (about 25 kW of power for each kg/s)."),
        ]
    }
    fn fits(&self) -> &'static [SocketKind] {
        &[SocketKind::Mast]
    }
    fn host(&self) -> Option<Host> {
        Some(Host { hull: "hull_lancer", hull_params: &[("length_m", 7.0), ("width_m", 3.2), ("height_m", 1.1)], socket: "mast_2", kw_per_t: 18.0 })
    }

    fn generate(&self, v: &Values, _lib: &MaterialLibrary) -> PartDef {
        let (reach, rate) = (v["reach_m"], v["rate"]);
        let mut shapes: Vec<Node> = Vec::new();
        // Turntable base, shoulder, boom, jib and tool head, folded up in a rest pose.
        let rb = 0.12 + 0.03 * reach + 0.02 * rate.sqrt();
        let h0 = 0.25 + 0.04 * reach;
        shapes.push(style::cyl(rb, 0.12 * rb + 0.05, Axis::Y, 14, 1.0, p(0.0, 0.5 * (0.12 * rb + 0.05), 0.0), "machinery", Slot::Dark, None, 0.0));
        shapes.push(style::cyl(0.8 * rb, h0, Axis::Y, 14, 0.8, p(0.0, 0.5 * h0 + 0.06 * rb, 0.0), "steel", Slot::Secondary, Some(0.01 + 0.004 * reach), 0.0));
        let l1 = 0.55 * reach;
        let l2 = 0.5 * reach;
        let sh = p(0.0, h0 + 0.06 * rb, 0.0);
        let a1 = 62.0f64.to_radians();
        let el = sh + p(0.0, l1 * a1.sin(), -l1 * a1.cos() * 0.8);
        let a2 = 14.0f64.to_radians();
        let wr = el + p(0.0, -l2 * a2.sin() * 0.6, -l2 * a2.cos());
        let w1 = 0.05 + 0.012 * reach + 0.03 * rate.sqrt();
        let w2 = 0.7 * w1;
        shapes.push(style::sphere(0.9 * w1, sh, "machinery", Slot::Dark));
        shapes.push(style::beam(sh, el, [w1, w1 * 1.2], p(1.0, 0.0, 0.0), "steel", Slot::Primary, 0.12 * w1));
        shapes.push(style::sphere(0.8 * w1, el, "machinery", Slot::Dark));
        shapes.push(style::beam(el, wr, [w2, w2 * 1.2], p(1.0, 0.0, 0.0), "steel", Slot::Primary, 0.12 * w2));
        // A hydraulic ram from the base to the boom, and a stripe of Trim on each segment.
        shapes.push(style::beam(sh + p(0.0, -0.1 * h0, 0.5 * rb), sh + (el - sh) * 0.45 + p(0.0, -0.4 * w1, 0.0), [0.35 * w1, 0.35 * w1], p(1.0, 0.0, 0.0), "machinery", Slot::Metal, 0.0));
        shapes.push(style::beam(sh + (el - sh) * 0.25, sh + (el - sh) * 0.7, [w1 * 1.04, w1 * 0.25], p(1.0, 0.0, 0.0), "fittings", Slot::Trim, 0.0));
        // Tool head: a fabricator block with a glowing welding nozzle pointing down and forward.
        let hs = 0.8 * w2 + 0.08;
        shapes.push(style::bx([1.6 * hs, 1.2 * hs, 1.8 * hs], wr + p(0.0, -0.3 * hs, -0.3 * hs), "machinery", Slot::Secondary, 0.1 * hs));
        shapes.push(style::beam(wr + p(0.0, -0.5 * hs, -0.9 * hs), wr + p(0.0, -1.5 * hs, -1.4 * hs), [0.5 * hs, 0.5 * hs], p(1.0, 0.0, 0.0), "machinery", Slot::Dark, 0.0));
        shapes.push(style::sphere(0.3 * hs, wr + p(0.0, -1.55 * hs, -1.45 * hs), "glass", Slot::Glow));
        PartDef {
            id: format!("repair_{reach:.1}m_{rate:.2}"),
            name: format!("Repair rig {reach:.1} m"),
            category: Category::Utility,
            size: SizeClass::Small,
            tags: vec!["repair".into()],
            palette: None,
            voxels: Some(96),
            vital: Some(false),
            shapes,
            sockets: vec![SocketDef { name: "mount".into(), kind: SocketKind::Mount, size: SizeClass::Small, at: [0.0; 3], normal: [0.0, -1.0, 0.0], forward: [0.0, 0.0, -1.0], hints: Default::default() }],
            function: Function { draw_kw: 25.0 * rate, mast_m: 2.0 * rb, repair_kg_s: rate, repair_reach_m: reach, ..Default::default() },
        }
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        vec![
            stat("rig mass", built.mass.mass_kg, "kg"),
            stat("repair rate", v["rate"], "kg/s"),
            stat("reach", v["reach_m"], "m"),
            stat("power draw", 25.0 * v["rate"], "kW"),
            // Time to rebuild one tonne of structure.
            stat("minutes per tonne", 1000.0 / v["rate"] / 60.0, "min"),
        ]
    }
}
