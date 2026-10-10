//! Bridges: a deck laid across a river on a road, with rails as props and a load rating as course data.
//!
//! A heightfield has one height per (x, z), so nothing can pass *under* a bridge; that is fine, because nothing drives under one. The
//! deck is ground raised to deck level over the water (after the grade clamp, which would otherwise build ramps into the channel);
//! the river keeps its water on either side of it. A bridge's load rating is not enforced by physics (no collapse: NOT-MODELLED), it
//! is data for the course survey and for a scenario to flag an overloaded crossing.

use serde::{Deserialize, Serialize};
use w5k_contract::param::Param;
use w5k_contract::world::{PropId, PropKind, PropRef, PropShape};
use w5k_math::{scalar, Quat, Transform, Vec3};

use crate::river::RiverDef;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BridgeKind {
    /// Narrow, rough planks, a low load rating, rails that break.
    Wooden,
    /// Wide, laid in the road's own surface, a high rating, rails that do not.
    Road,
}

/// A bridge on a road: after waypoint `after_waypoint` the road leaves its A* path, runs straight across the river at `at_fraction` of
/// the way along it, and picks up again on the far bank.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BridgeDef {
    pub after_waypoint: usize,
    /// Index into `CourseDef::rivers`.
    pub river: usize,
    pub at_fraction: Param,
    pub kind: BridgeKind,
    pub width_m: Param,
    /// Greatest vehicle mass the bridge is rated for, kg.
    pub load_limit_kg: Param,
    pub rail_height_m: Param,
}

/// A bridge as laid: where it runs and what it is rated for.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedBridge {
    pub kind: BridgeKind,
    /// The two ends of the straight run, plan `(x, z)`, a little beyond the banks.
    pub a: (f64, f64),
    pub b: (f64, f64),
    /// Length of the deck itself (bank top to bank top), m.
    pub deck_len_m: f64,
    pub width_m: f64,
    pub deck_y_m: f64,
    pub load_limit_kg: f64,
    pub rail_height_m: f64,
}

/// How far the straight run extends beyond each bank top, m (so the deck meets the road flush).
pub const RUNOUT_M: f64 = 2.0; // const-ok: overlap of the deck with the approach road
/// Thickness of a rail, m.
const RAIL_THICKNESS_M: f64 = 0.2; // const-ok: rail section
/// Impulse that snaps a wooden rail, N s (UNVALIDATED placeholder; COMBAT/VALIDATION to source).
const WOODEN_RAIL_BREAK_NS: f64 = 3.0e4; // const-ok: placeholder, see the doc comment

impl BridgeDef {
    pub fn check(&self, name: &str) -> Result<(), String> {
        for (l, p) in [
            ("at_fraction", &self.at_fraction),
            ("width_m", &self.width_m),
            ("load_limit_kg", &self.load_limit_kg),
            ("rail_height_m", &self.rail_height_m),
        ] {
            p.check(&format!("{name}.{l}"))?;
        }
        Ok(())
    }
}

/// Lay a bridge across `river` (carved with `river_seed`), arriving from the side nearer to `from`. `surface_m` is the water level at the
/// crossing and `freeboard_m` the river's bank height, so the deck sits level with the floodplain.
pub fn place(def: &BridgeDef, river: &RiverDef, river_seed: u64, from: (f64, f64), surface_m: f64) -> PlacedBridge {
    let line = river.centreline(river_seed);
    let total = line[line.len() - 1].2;
    let k = line.iter().position(|p| p.2 >= def.at_fraction.v * total).unwrap_or(1).clamp(1, line.len() - 2);
    let (p, q) = (line[k - 1], line[k + 1]);
    let len = scalar::hypot(q.0 - p.0, q.1 - p.1);
    let nrm = (-(q.1 - p.1) / len, (q.0 - p.0) / len);
    let top = river.bank_top_m();
    let reach = top + RUNOUT_M;
    let c = (line[k].0, line[k].1);
    let (e1, e2) = ((c.0 - nrm.0 * reach, c.1 - nrm.1 * reach), (c.0 + nrm.0 * reach, c.1 + nrm.1 * reach));
    let near_first = scalar::hypot(e1.0 - from.0, e1.1 - from.1) <= scalar::hypot(e2.0 - from.0, e2.1 - from.1);
    let (a, b) = if near_first { (e1, e2) } else { (e2, e1) };
    PlacedBridge {
        kind: def.kind,
        a,
        b,
        deck_len_m: 2.0 * top,
        width_m: def.width_m.v,
        deck_y_m: surface_m + river.freeboard_m.v,
        load_limit_kg: def.load_limit_kg.v,
        rail_height_m: def.rail_height_m.v,
    }
}

/// Rails along both edges of the deck as `Wall` props.
pub fn rails(first_id: u32, b: &PlacedBridge) -> Vec<PropRef> {
    let len = scalar::hypot(b.b.0 - b.a.0, b.b.1 - b.a.1);
    let dir = ((b.b.0 - b.a.0) / len, (b.b.1 - b.a.1) / len);
    let side = (-dir.1, dir.0);
    let mid = ((b.a.0 + b.b.0) * 0.5, (b.a.1 + b.b.1) * 0.5);
    let yaw = Quat::from_yaw(scalar::atan2(-dir.0, -dir.1)); // the box's local -Z runs along the bridge
    let impulse = if b.kind == BridgeKind::Wooden { WOODEN_RAIL_BREAK_NS } else { f64::INFINITY };
    [-1.0, 1.0]
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let off = s * (b.width_m * 0.5 - RAIL_THICKNESS_M * 0.5);
            PropRef {
                id: PropId(first_id + k as u32),
                kind: PropKind::Wall,
                shape: PropShape::Box {
                    half_m: Vec3::new(RAIL_THICKNESS_M * 0.5, b.rail_height_m * 0.5, b.deck_len_m * 0.5),
                },
                transform: Transform::new(
                    Vec3::new(mid.0 + side.0 * off, b.deck_y_m + b.rail_height_m * 0.5, mid.1 + side.1 * off),
                    yaw,
                ),
                break_impulse_ns: impulse,
            }
        })
        .collect()
}
