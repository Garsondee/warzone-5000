//! Anti-gravity pods and plates: running gear that holds the hull up on a field instead of touching the ground.
//!
//! * A pod's **rated lift** is what its coils can hold up; a vehicle needs the total rating to exceed its weight.
//! * The field costs power in proportion to the weight it holds, `kw_per_t` per tonne, whatever the size of the
//!   pod: unlike a rotor or an air cushion there is no W^1.5 penalty, so it is the way to float titans, but the
//!   baseline is high. A better field (fewer kW/t) needs more coil turns, so a heavier, bulkier pod.
//! * The higher the hull floats, the more the field has to reach: cost grows by 4 % per metre of ride height.
//!   A tall ride clears rocks, trenches and walls; the ground pressure is nil.
//!
//! Part space: the mount is on the hull (a flank station, a round-body hip or the underside). On a station or hip
//! the pod hangs outward on a short strut; on the belly it is a plate under the hull. The ground is `ride_m` below
//! the mount, and the glowing rings and beam drawn down to it are `scenery` (no mass, no armour, no bounds).

use super::mounts::{KIND_BELLY, KIND_STATION};
use super::style::{self, p};
use super::{ctx, stat, Family, Host, Param, Role, Scale, Stat, Values};
use crate::schema::{Axis, Category, Function, Locomotion, MaterialLibrary, Node, PartDef, SizeClass, SocketDef, SocketKind, Slot};
use crate::Built;

pub struct AntiGrav;

#[allow(clippy::too_many_arguments)]
fn param(id: &'static str, name: &'static str, unit: &'static str, min: f64, max: f64, default: f64, scale: Scale, role: Role, help: &'static str) -> Param {
    Param { id, name, unit, min, max, default, scale, role, help }
}

/// Power needed to hold a given load (kW) at a ride height: the sheet's formula.
pub fn hold_kw(lift_t: f64, kw_per_t: f64, ride_m: f64) -> f64 {
    kw_per_t * lift_t * (1.0 + ride_m / 25.0)
}

/// Coil rings stacked in the pod: efficient fields need more of them.
fn rings(kw_per_t: f64) -> usize {
    let t = ((kw_per_t / 8.0).ln() / (90.0f64 / 8.0).ln()).clamp(0.0, 1.0);
    (4.0 - 3.0 * t).round() as usize
}

impl Family for AntiGrav {
    fn id(&self) -> &'static str {
        "antigrav"
    }
    fn name(&self) -> &'static str {
        "Anti-grav pod"
    }
    fn params(&self) -> Vec<Param> {
        vec![
            param("lift_t", "Rated lift", "t", 0.2, 600.0, 6.0, Scale::Log, Role::Budgeted, "Mass this pod holds up (every pod counts: a vehicle needs the total to exceed its weight)."),
            param("kw_per_t", "Field cost", "kW/t", 8.0, 90.0, 30.0, Scale::Log, Role::Budgeted, "Power per tonne held. A cheaper field needs more coils: a heavier, bulkier pod."),
            param("ride_m", "Ride height", "m", 0.3, 40.0, 3.0, Scale::Log, Role::Free, "How high the hull floats: it clears rougher ground, at +4% power per metre."),
        ]
    }
    fn fits(&self) -> &'static [SocketKind] {
        &[SocketKind::Station, SocketKind::Hip, SocketKind::Belly]
    }
    fn host(&self) -> Option<Host> {
        Some(Host { hull: "hull_skiff", hull_params: &[("length_m", 12.0), ("width_m", 5.0), ("height_m", 1.4), ("stations", 2.0), ("front_mm", 30.0), ("side_mm", 15.0)], socket: "station_*", kw_per_t: 140.0 })
    }

    fn generate(&self, v: &Values, _lib: &MaterialLibrary) -> PartDef {
        let lift = v["lift_t"];
        let ride = v["ride_m"];
        let kw_t = v["kw_per_t"];
        let kind = ctx(v, "ctx.kind", KIND_STATION);
        let belly = (kind - KIND_BELLY).abs() < 0.5;
        let n = rings(kw_t);
        let mut shapes: Vec<Node> = Vec::new();
        // Size: the pod grows with the cube root of the lift it must hold; a plate fills the underside instead.
        let r_pod = 0.28 + 0.26 * lift.cbrt();
        let (r, c) = if belly {
            let room = 0.46 * ctx(v, "ctx.width", 4.0).min(ctx(v, "ctx.length", 8.0));
            (r_pod.max(0.5 * room).min(room), p(0.0, 0.0, 0.0))
        } else {
            let arm = r_pod + 0.12;
            (r_pod, p(arm, -0.12 * r_pod, 0.0))
        };
        let slab = 0.28 * r;
        let ring_h = 0.1 * r;
        let sh = (0.01 + 0.004 * r).min(0.05);
        let top = c.y + 0.5 * (slab + n as f64 * ring_h * 1.35);
        if belly {
            // A plate hanging flush under the hull: housing, coil rings and a bright emitter lens in the middle.
            let y0 = -0.5 * slab;
            shapes.push(style::cyl(r, slab, Axis::Y, 24, 1.0, p(0.0, y0, 0.0), "steel", Slot::Secondary, Some(sh), 0.03 * r));
            for k in 0..n {
                let rr = r * (0.9 - 0.18 * k as f64);
                shapes.push(style::cyl(rr, 0.5 * ring_h, Axis::Y, 24, 1.0, p(0.0, y0 - 0.5 * slab - 0.25 * ring_h - 0.02 * r, 0.0), "steel", Slot::Dark, Some(sh), 0.0));
                shapes.push(style::cyl(rr - 0.05 * r, 0.7 * ring_h, Axis::Y, 24, 1.0, p(0.0, y0 - 0.5 * slab - 0.35 * ring_h - 0.02 * r, 0.0), "fittings", Slot::Glow, None, 0.0));
            }
        } else {
            // A pod on a strut: housing, a stack of coil rings with glowing seams, and the emitter lens beneath.
            shapes.push(style::beam(p(0.0, 0.0, 0.0), p(c.x - 0.6 * r, c.y, 0.0), [0.34 * r, 0.3 * r], p(0.0, 1.0, 0.0), "steel", Slot::Dark, 0.02 * r));
            shapes.push(style::cyl(r, slab, Axis::Y, 20, 1.0, c + p(0.0, 0.5 * (n as f64 * ring_h * 1.35), 0.0), "steel", Slot::Secondary, Some(sh), 0.04 * r));
            for k in 0..n {
                let y = c.y - 0.5 * slab + 0.35 * ring_h - k as f64 * ring_h * 1.35;
                shapes.push(style::cyl(1.12 * r, ring_h, Axis::Y, 20, 1.0, p(c.x, y, 0.0), "steel", Slot::Primary, Some(sh), 0.03 * r));
                shapes.push(style::cyl(1.15 * r, 0.22 * ring_h, Axis::Y, 20, 1.0, p(c.x, y - 0.62 * ring_h, 0.0), "fittings", Slot::Glow, None, 0.0));
            }
            let y_lens = c.y - 0.5 * slab - (n as f64) * ring_h * 1.35 + 0.2 * ring_h;
            shapes.push(style::cyl(0.7 * r, 0.1 * r, Axis::Y, 16, 1.0, p(c.x, y_lens, 0.0), "fittings", Slot::Glow, None, 0.0));
            let _ = top;
        }
        // The field made visible: a thin glowing beam from the pod to the ground and two rings where it lands.
        let ground = -ride;
        let (fx, fy0) = if belly { (0.0, -0.5 * slab - 1.35 * ring_h * n as f64) } else { (c.x, c.y - 0.5 * slab - 1.35 * ring_h * n as f64) };
        if fy0 - ground > 0.4 {
            shapes.push(style::cyl(0.1 * r, fy0 - ground, Axis::Y, 8, 3.2, p(fx, ground + 0.5 * (fy0 - ground), 0.0), "scenery", Slot::Glow, Some(0.003), 0.0));
        }
        for (k, f) in [1.5, 1.05].iter().enumerate() {
            let rr = f * r * (1.0 + 0.2 * (ride / (ride + 6.0)));
            shapes.push(style::flat_ring(rr, 0.05 + 0.03 * r, 0.02, (24.0 + 8.0 * rr).min(64.0) as u32, p(fx, ground + 0.012 + 0.012 * k as f64, 0.0), "scenery", Slot::Glow));
        }
        PartDef {
            id: format!("antigrav_{lift:.1}t_{kw_t:.0}kw_{ride:.1}m"),
            name: format!("Anti-grav {lift:.1} t"),
            category: Category::Locomotion,
            size: SizeClass::Medium,
            tags: vec!["antigrav".into()],
            palette: None,
            voxels: Some(64),
            vital: Some(false),
            shapes,
            sockets: vec![SocketDef {
                name: "mount".into(),
                kind: SocketKind::Mount,
                size: SizeClass::Medium,
                at: [0.0; 3],
                normal: if belly { [0.0, 1.0, 0.0] } else { [-1.0, 0.0, 0.0] },
                forward: [0.0, 0.0, -1.0],
                hints: Default::default(),
            }],
            function: Function {
                locomotion: Some(Locomotion::AntiGrav),
                load_kg: lift * 1000.0,
                grav_kw_per_t: kw_t,
                ride_height_m: Some(ride),
                max_kmh: Some(240.0),
                step_m: 0.9 * ride,
                ..Default::default()
            },
        }
    }

    fn performance(&self, v: &Values, built: &Built) -> Vec<Stat> {
        vec![
            stat("pod mass", built.mass.mass_kg, "kg"),
            stat("rated lift", v["lift_t"], "t"),
            stat("mass / lift", 100.0 * built.mass.mass_kg / (v["lift_t"] * 1000.0), "%"),
            stat("power to hold the rating", hold_kw(v["lift_t"], v["kw_per_t"], v["ride_m"]), "kW"),
            stat("steps obstacles", 0.9 * v["ride_m"], "m"),
        ]
    }

    fn fit_to_load(&self, v: &mut Values, load_kg: f64, _lib: &MaterialLibrary) {
        v.insert("lift_t".into(), (load_kg * 1.15 / 1000.0).clamp(0.2, 600.0));
    }
}
