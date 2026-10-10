//! Weapon families. A gun is one parametric generator with presets (`shapes/weapons.ron`): a receiver box, a barrel with an optional
//! jacket and a muzzle device, ammunition boxes, spade grips or a sight, and trunnion pins that reach the mount's cradle. It is authored
//! in its own frame (trunnion axis at the origin, bore along -Z, +Y up). The receiver, pins, boxes, grips and sight swing with the gun
//! (role `GunPitch`); the barrel, jacket and muzzle device slide back on firing (role `Recoil`). As a module it mounts on a `Trunnion`
//! socket by its trunnion block, whose width is the receiver's: a cradle takes it only if the receiver fits between the uprights.

use crate::hardware::{bevel_box, cyl_x, cyl_y, cyl_z};
use crate::mesh::Mesh;
use crate::module::{frame, Module, ModuleKind, Socket, SocketKind};
use crate::part::{Part, Side};
use crate::wheel::segments_for;
use serde::Deserialize;
use w5k_contract::render::{NodeRole, SlotKind};
use w5k_math::{Transform, Vec3};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmmoBox {
    pub size: (f64, f64, f64),
    pub at: (f64, f64, f64),
    pub mirror: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grips {
    pub bar_radius_m: f64,
    pub half_span_m: f64,
    pub handle_x_m: f64,
    pub handle_radius_m: f64,
    pub handle_drop_m: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GunDims {
    pub name: String,
    pub receiver_m: (f64, f64, f64),
    pub pivot_back_frac: f64,
    pub barrel_radius_m: f64,
    pub barrel_len_m: f64,
    pub jacket: Option<(f64, f64)>,
    pub muzzle_m: (f64, f64),
    pub pin_radius_m: f64,
    pub ammo: Vec<AmmoBox>,
    pub grips: Option<Grips>,
    pub sight: Option<(f64, f64, f64)>,
    pub sight_z_m: f64,
    pub overlap_m: f64,
    pub corner_m: f64,
    pub band_m: f64,
}

impl GunDims {
    /// A preset of `shapes/weapons.ron` by name.
    pub fn preset(name: &str) -> Option<GunDims> {
        let all: Vec<GunDims> = ron::from_str(include_str!("../shapes/weapons.ron")).expect("weapons.ron parses");
        all.into_iter().find(|g| g.name == name)
    }
}

/// The gun as a module. `cradle_w_m` is the clear width of the cradle it will sit in (the hint `cradle_w_m` of the trunnion socket): the
/// pins are as long as it takes to reach the uprights, and a few millimetres into them.
pub fn gun_module(d: &GunDims, cradle_w_m: f64, detail: u8) -> Module {
    let n = segments_for(detail) / 4; // const-ok: a quarter of the wheel segments for small cylinders
    let (e, o, band) = (d.corner_m, d.overlap_m, d.band_m);
    let (rw, rh, rl) = d.receiver_m;
    let (rear, front) = (rl * d.pivot_back_frac, rl * d.pivot_back_frac - rl);
    let part = |name: &str, role: NodeRole, slot: SlotKind, side: Side, mesh: Mesh| Part {
        name: name.into(),
        role,
        station: None,
        side,
        slot,
        fitting: false,
        mesh: mesh.finished(),
        pose: Transform::IDENTITY,
        placement: None,
    };
    let (pitch, recoil) = (NodeRole::GunPitch, NodeRole::Recoil);
    let mut parts = vec![
        part(
            "receiver",
            pitch,
            SlotKind::Metal,
            Side::Centre,
            bevel_box(Vec3::new(0.0, 0.0, (rear + front) / 2.0), [rw, rh, rl], e, band),
        ),
        // the pins pass through the receiver and end a few millimetres inside the cradle's uprights
        part(
            "pins",
            pitch,
            SlotKind::Metal,
            Side::Centre,
            cyl_x(0.0, 0.0, d.pin_radius_m, -cradle_w_m / 2.0 - o, cradle_w_m / 2.0 + o, n),
        ),
    ];
    let root = front + o; // the barrel and jacket start inside the receiver
    parts.push(part(
        "barrel",
        recoil,
        SlotKind::Metal,
        Side::Centre,
        cyl_z(0.0, 0.0, d.barrel_radius_m, front - d.barrel_len_m, root, n),
    ));
    if let Some((len, radius)) = d.jacket {
        parts.push(part(
            "jacket",
            recoil,
            SlotKind::Metal,
            Side::Centre,
            cyl_z(0.0, 0.0, radius, front - len, root, n),
        ));
    }
    let (mlen, mradius) = d.muzzle_m;
    let tip = front - d.barrel_len_m;
    parts.push(part("muzzle", recoil, SlotKind::Metal, Side::Centre, cyl_z(0.0, 0.0, mradius, tip - mlen, tip + o, n)));
    for (i, b) in d.ammo.iter().enumerate() {
        let (size, at) = ([b.size.0, b.size.1, b.size.2], Vec3::new(b.at.0, b.at.1, b.at.2));
        let can = bevel_box(at, size, e, band);
        if b.mirror {
            parts.push(part(&format!("ammo{i}.r"), pitch, SlotKind::Paint, Side::Right, can.clone()));
            parts.push(part(&format!("ammo{i}.l"), pitch, SlotKind::Paint, Side::Left, can.mirrored_x()));
        } else {
            parts.push(part(&format!("ammo{i}"), pitch, SlotKind::Paint, Side::Centre, can));
        }
    }
    if let Some(g) = &d.grips {
        // a cross-bar just behind the receiver and a handle at each end, reaching down from the bar
        let z = rear + o;
        parts.push(part(
            "grip_bar",
            pitch,
            SlotKind::Metal,
            Side::Centre,
            cyl_x(0.0, z, g.bar_radius_m, -g.half_span_m, g.half_span_m, n),
        ));
        for (name, side, sx) in [("grip.r", Side::Right, 1.0), ("grip.l", Side::Left, -1.0)] {
            let handle = cyl_y(sx * g.handle_x_m, z, g.handle_radius_m, -g.handle_drop_m, g.bar_radius_m, n);
            parts.push(part(name, pitch, SlotKind::Metal, side, handle));
        }
    }
    if let Some((sw, sh, sl)) = d.sight {
        parts.push(part(
            "sight",
            pitch,
            SlotKind::Metal,
            Side::Centre,
            bevel_box(Vec3::new(0.0, rh / 2.0 + sh / 2.0 - o, d.sight_z_m), [sw, sh, sl], e, band),
        ));
    }
    let mount = Socket {
        name: "mount".into(),
        kind: SocketKind::Trunnion,
        side: Side::Centre,
        pose: frame(Vec3::ZERO, Vec3::Z, Vec3::Y),
        size_m: rw,
        station: None,
        carrier: NodeRole::Hull,
        owner: None,
        hints: Vec::new(),
    };
    Module {
        name: d.name.clone(),
        kind: ModuleKind::Weapon,
        parts,
        sockets: Vec::new(),
        mount: Some(mount),
        symmetric: false,
    }
}
