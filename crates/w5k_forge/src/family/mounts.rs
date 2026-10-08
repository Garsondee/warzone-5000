//! The standard mounting points a hull offers, so that any running gear can go on any hull that has the right
//! kind of socket (docs/design/04-parametric-components.md).
//!
//! | Socket | Kind | Where | Used by |
//! |---|---|---|---|
//! | `gear_r`, `gear_l` | Gear | one long mount on each side | track units |
//! | `station_r1..n`, `station_l1..n` | Station (or Hip on round bodies) | evenly spaced along each side | wheels, legs, pods, rotor arms |
//! | `keel_1..n` | Keel | centre line under the hull | rail bogies |
//! | `belly` | Belly | the whole underside | air cushions, anti-gravity plates |
//!
//! Every socket carries hints (`ctx.*`) that tell what attaches how much room it has: `ctx.hip_height` (height of
//! the socket above the ground), `ctx.spacing` and `ctx.length` (room along the hull), `ctx.width`, and for track
//! units `ctx.top` and `ctx.bottom` (how far the unit reaches above and below the socket).

use std::collections::BTreeMap;

use super::style::p;
use crate::geom::V3;
use crate::schema::{SizeClass, SocketDef, SocketKind};

/// Where a hull offers running gear.
#[derive(Clone, Copy, Debug)]
pub struct Chassis {
    /// Front and rear ends of the underbody along z (the front is negative).
    pub z0: f64,
    pub z1: f64,
    /// Underside height (ground clearance) and the height of the side mounts, both above the ground.
    pub y_under: f64,
    pub y_side: f64,
    /// Top of a track run above the ground.
    pub track_top: f64,
    /// x of the side mounts (half the width where the sides meet running gear).
    pub half_w: f64,
    /// Overall underbody width (rail gauge, cushion size).
    pub width: f64,
    /// Number of discrete stations on each side (and of keel mounts).
    pub stations: usize,
    /// Centre and length of the full-length side mount used by track units (`None`: this hull has no tracks).
    pub gear: Option<(f64, f64)>,
}

/// Numeric code of a socket kind, passed to families as the hint `ctx.kind` (families that must build different
/// geometry for different kinds of mount read it).
pub fn kind_code(kind: SocketKind) -> f64 {
    match kind {
        SocketKind::Station => KIND_STATION,
        SocketKind::Keel => KIND_KEEL,
        SocketKind::Belly => KIND_BELLY,
        SocketKind::Gear => KIND_GEAR,
        SocketKind::Hip => KIND_HIP,
        SocketKind::Mast => KIND_MAST,
        SocketKind::TurretRing => KIND_RING,
        SocketKind::Internal => KIND_INTERNAL,
        _ => 9.0,
    }
}
pub const KIND_STATION: f64 = 0.0;
pub const KIND_KEEL: f64 = 1.0;
pub const KIND_BELLY: f64 = 2.0;
pub const KIND_GEAR: f64 = 3.0;
pub const KIND_HIP: f64 = 4.0;
pub const KIND_MAST: f64 = 5.0;
pub const KIND_RING: f64 = 6.0;
pub const KIND_INTERNAL: f64 = 7.0;

/// A socket with hints (and the `ctx.kind` code).
pub fn sock(name: &str, kind: SocketKind, at: V3, normal: V3, forward: V3, hints: &[(&str, f64)]) -> SocketDef {
    let mut h: BTreeMap<String, f64> = hints.iter().map(|(k, v)| (k.to_string(), *v)).collect();
    h.insert("ctx.kind".into(), kind_code(kind));
    SocketDef { name: name.into(), kind, size: SizeClass::Medium, at: at.arr(), normal: normal.arr(), forward: forward.arr(), hints: h }
}

/// Segment centres along the underbody: `n` equal segments of the span `[z0, z1]`.
pub fn segments(z0: f64, z1: f64, n: usize) -> (f64, Vec<f64>) {
    let seg = (z1 - z0) / n as f64;
    (seg, (0..n).map(|i| z0 + seg * (i as f64 + 0.5)).collect())
}

/// The running-gear sockets of a straight-sided hull.
pub fn running_gear(c: &Chassis) -> Vec<SocketDef> {
    let fwd = p(0.0, 0.0, -1.0);
    let n = c.stations.max(1);
    let (seg, zs) = segments(c.z0, c.z1, n);
    let mut out = Vec::new();
    if let Some((gz, glen)) = c.gear {
        let hints = [("ctx.length", glen), ("ctx.top", c.track_top - c.y_side), ("ctx.bottom", c.y_side), ("ctx.hip_height", c.y_side), ("ctx.hull_width", c.width)];
        out.push(sock("gear_r", SocketKind::Gear, p(c.half_w, c.y_side, gz), p(1.0, 0.0, 0.0), fwd, &hints));
        out.push(sock("gear_l", SocketKind::Gear, p(-c.half_w, c.y_side, gz), p(-1.0, 0.0, 0.0), fwd, &hints));
    }
    let hints = [
        ("ctx.hip_height", c.y_side),
        ("ctx.spacing", seg),
        ("ctx.length", 0.94 * seg),
        ("ctx.top", c.track_top - c.y_side),
        ("ctx.bottom", c.y_side),
        ("ctx.hull_width", c.width),
        ("ctx.stations", n as f64),
    ];
    for (i, z) in zs.iter().enumerate() {
        out.push(sock(&format!("station_r{}", i + 1), SocketKind::Station, p(c.half_w, c.y_side, *z), p(1.0, 0.0, 0.0), fwd, &hints));
        out.push(sock(&format!("station_l{}", i + 1), SocketKind::Station, p(-c.half_w, c.y_side, *z), p(-1.0, 0.0, 0.0), fwd, &hints));
    }
    for (i, z) in zs.iter().enumerate() {
        let keel_hints = [("ctx.hip_height", c.y_under), ("ctx.spacing", seg), ("ctx.length", 0.94 * seg), ("ctx.width", c.width), ("ctx.stations", n as f64), ("ctx.index", i as f64)];
        out.push(sock(&format!("keel_{}", i + 1), SocketKind::Keel, p(0.0, c.y_under, *z), p(0.0, -1.0, 0.0), fwd, &keel_hints));
    }
    out.push(sock(
        "belly",
        SocketKind::Belly,
        p(0.0, c.y_under, 0.5 * (c.z0 + c.z1)),
        p(0.0, -1.0, 0.0),
        fwd,
        &[("ctx.length", c.z1 - c.z0), ("ctx.width", c.width), ("ctx.hip_height", c.y_under), ("ctx.top", (c.track_top - c.y_under).max(0.3))],
    ));
    out
}
