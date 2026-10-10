//! The remote weapon station: an enclosed turret for a roof ring. A collar fixed to the hull (role `Hull`, a fitting), a turntable that yaws on
//! it and carries a housing (a lofted box whose front face leans back), a sight head, an ammunition bin, a smoke-launcher bank each side and a
//! cradle of two uprights on the front face that holds a gun's trunnion (all role `Turret`, one node frame at the centre of the collar's top
//! face). As a module it mounts on a `Ring` socket (sized by its ring diameter) and offers one `Trunnion` socket (sized by the cradle's clear
//! width, also published as the hint `cradle_w_m`), exactly as the open ring mount does, so the same guns fit either. Authored standing up:
//! mount normal down, reference forward.

use crate::hardware::{bevel_box, cyl_y, revolve_y};
use crate::loft::{bevel_ring, loft_beveled, Section};
use crate::mesh::Mesh;
use crate::module::{frame, Module, ModuleKind, Socket, SocketKind};
use crate::part::{Part, Side};
use crate::wheel::segments_for;
use serde::Deserialize;
use w5k_contract::render::{NodeRole, SlotKind};
use w5k_math::{Transform, Vec3};

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurretDims {
    pub ring_m: f64,
    pub collar_height_m: f64,
    pub collar_width_m: f64,
    pub turntable_m: f64,
    pub housing_w_m: f64,
    pub housing_l_m: f64,
    pub housing_h_m: f64,
    pub housing_z_m: f64,
    pub front_slope_m: f64,
    pub front_foot_m: f64,
    pub cradle_width_m: f64,
    pub cradle_height_m: f64,
    pub cradle_z_m: f64,
    pub upright_thickness_m: f64,
    pub upright_length_m: f64,
    pub sight_m: (f64, f64, f64),
    pub sight_at_m: (f64, f64),
    pub bin_m: (f64, f64, f64),
    pub bin_at_m: (f64, f64, f64),
    pub smoke_m: (f64, f64, f64),
    pub smoke_at_m: (f64, f64, f64),
    pub overlap_m: f64,
    pub corner_m: f64,
    pub band_m: f64,
}

impl TurretDims {
    /// `shapes/rws_turret.ron`.
    pub fn standard() -> TurretDims {
        ron::from_str(include_str!("../shapes/rws_turret.ron")).expect("rws_turret.ron parses")
    }
}

/// The turret module (frame: origin at the centre of the collar's bottom face on the hull, +Y up, -Z forward).
pub fn rws_turret(d: &TurretDims, detail: u8) -> Module {
    let (n, r, e, o, band) = (segments_for(detail) / 2, d.ring_m / 2.0, d.corner_m, d.overlap_m, d.band_m); // const-ok: half the wheel segments for a ring this size
    let (ch, cw, t) = (d.collar_height_m, d.collar_width_m, d.turntable_m);
    let part = |name: &str, role: NodeRole, slot: SlotKind, side: Side, mesh: Mesh| Part {
        name: name.into(),
        role,
        station: None,
        side,
        slot,
        fitting: role == NodeRole::Hull,
        mesh: mesh.finished(),
        pose: if role == NodeRole::Hull { Transform::IDENTITY } else { Transform::from_pos(Vec3::new(0.0, ch, 0.0)) },
        placement: None,
    };
    let collar = revolve_y(&[(r - cw, 0.0), (r, 0.0), (r, ch - e), (r - e, ch), (r - cw, ch)], n);
    // the housing, in the turntable's frame: a loft along z whose front section is the low foot of the front face and whose second section is
    // the full height `front_slope_m` further back (the leaning front), then the box to the rear face
    let (w, l, h) = (d.housing_w_m, d.housing_l_m, t + d.housing_h_m); // h: the roof of the housing in the node frame
    let (z_front, z_rear) = (d.housing_z_m - l / 2.0, d.housing_z_m + l / 2.0);
    // a section from the turntable's top (overlapping it) up to `top`
    let sec = |z: f64, width: f64, top: f64| Section {
        z_m: z,
        ring: bevel_ring(width, top + o - t, (top + t - o) / 2.0, e, band),
    };
    let housing = loft_beveled(
        &[
            sec(z_front, 0.82 * w, t + d.front_foot_m),
            sec(z_front + d.front_slope_m, w, h),
            sec(z_rear - 0.05, w, h),
            sec(z_rear, 0.94 * w, t + 0.94 * d.housing_h_m),
        ],
        e,
        band,
        &[],
        0.5, // const-ok: a vertex at least every half metre along a hard edge (spike S-G)
    );
    let (sw, sh, sl) = d.sight_m;
    let sight = bevel_box(Vec3::new(d.sight_at_m.0, h + sh / 2.0 - o, d.sight_at_m.1), [sw, sh, sl], e, band);
    let (bw, bh, bl) = d.bin_m;
    let bin = bevel_box(Vec3::new(d.bin_at_m.0, d.bin_at_m.1, d.bin_at_m.2), [bw, bh, bl], e, band);
    let (mw, mh, ml) = d.smoke_m;
    let smoke = bevel_box(Vec3::new(d.smoke_at_m.0, d.smoke_at_m.1, d.smoke_at_m.2), [mw, mh, ml], e, band);
    let (ut, ul) = (d.upright_thickness_m, d.upright_length_m);
    let pivot = Vec3::new(0.0, d.cradle_height_m, d.cradle_z_m);
    let top = pivot.y + ut;
    let upright =
        bevel_box(Vec3::new(d.cradle_width_m / 2.0 + ut / 2.0, (top - o) / 2.0, pivot.z), [ut, top + o, ul], e, band);
    let mut parts = vec![
        part("collar", NodeRole::Hull, SlotKind::Metal, Side::Centre, collar),
        part("turntable", NodeRole::Turret, SlotKind::Metal, Side::Centre, cyl_y(0.0, 0.0, r - cw / 2.0, -o, t, n)),
        part("housing", NodeRole::Turret, SlotKind::Paint, Side::Centre, housing),
        part("sight_head", NodeRole::Turret, SlotKind::Metal, Side::Centre, sight),
        part("ammo_bin", NodeRole::Turret, SlotKind::Paint, Side::Right, bin),
    ];
    for (name, slot, mesh) in [("smoke", SlotKind::Metal, smoke), ("upright", SlotKind::Metal, upright)] {
        parts.push(part(&format!("{name}.r"), NodeRole::Turret, slot, Side::Right, mesh.clone()));
        parts.push(part(&format!("{name}.l"), NodeRole::Turret, slot, Side::Left, mesh.mirrored_x()));
    }
    let mount = Socket {
        name: "mount".into(),
        kind: SocketKind::Ring,
        side: Side::Centre,
        pose: frame(Vec3::ZERO, -Vec3::Y, -Vec3::Z),
        size_m: d.ring_m,
        station: None,
        carrier: NodeRole::Hull,
        owner: None,
        hints: Vec::new(),
    };
    let trunnion = Socket {
        name: "trunnion".into(),
        kind: SocketKind::Trunnion,
        side: Side::Centre,
        pose: frame(Vec3::new(0.0, ch + pivot.y, pivot.z), -Vec3::Z, Vec3::Y),
        size_m: d.cradle_width_m,
        station: None,
        carrier: NodeRole::Turret, // the trunnion rides on the turntable
        owner: None,
        hints: vec![("cradle_w_m".into(), d.cradle_width_m)],
    };
    Module {
        name: "rws_turret".into(),
        kind: ModuleKind::Mount,
        parts,
        sockets: vec![trunnion],
        mount: Some(mount),
        symmetric: true,
    }
}
