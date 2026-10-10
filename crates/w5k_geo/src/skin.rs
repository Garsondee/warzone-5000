//! Skins: the wheeled-truck hulls of the game garage, each a hull family (`truck::TruckKind`) with the wheel modules on its axles, built
//! either from stand-in dimensions (`Skin::placeholder`, `PROVISIONAL(C-002)`) or from FORGE's physics definition (`Skin::from_def`), so
//! the wheel radii, track, wheelbase and hull box of the picture are the physics rig's. Every skin has the joint layout of the export:
//! four spins, two steers, four travels, front axle left then right, rear axle left then right.

use crate::part::Part;
use crate::truck::{truck_assembly, TruckKind, UtilityDims};
use crate::wheel::WheelDims;
use serde::Deserialize;
use w5k_contract::def::{RunningGearDef, VehicleDef};

/// Every skin id, smallest vehicle first.
pub const IDS: [&str; 3] = ["scout_4x4", "utility_4x4", "hauler_4x4"];

/// A hull family with its dimensions and the axles it is cut for (z in the hull frame, front first).
#[derive(Clone, Debug)]
pub struct Skin {
    pub kind: TruckKind,
    pub dims: UtilityDims,
    pub axles_z: Vec<f64>,
    pub steered: Vec<bool>,
}

/// The stand-in file of a family: the dimensions, the axles measured from the front and which of them steer.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SkinSpec {
    dims: UtilityDims,
    axles_from_front_m: Vec<f64>,
    steered: Vec<bool>,
}

impl Skin {
    /// The skin of `id` from stand-in dimensions; `None` for an id that is not a skin.
    pub fn for_id(id: &str) -> Option<Skin> {
        match id {
            "utility_4x4" => {
                let d = UtilityDims::placeholder();
                Some(Skin {
                    kind: TruckKind::Utility,
                    dims: d,
                    axles_z: vec![-d.wheelbase_m / 2.0, d.wheelbase_m / 2.0],
                    steered: vec![true, false],
                })
            }
            "scout_4x4" => Some(Skin::from_spec(TruckKind::Scout, include_str!("../shapes/placeholder_scout_4x4.ron"))),
            "hauler_4x4" => {
                Some(Skin::from_spec(TruckKind::Hauler, include_str!("../shapes/placeholder_hauler_4x4.ron")))
            }
            _ => None,
        }
    }

    fn from_spec(kind: TruckKind, text: &str) -> Skin {
        let spec: SkinSpec =
            ron::from_str(text).unwrap_or_else(|e| panic!("the stand-in of {} parses: {e}", kind.id()));
        let axles_z = spec.axles_from_front_m.iter().map(|f| f - spec.dims.length_m / 2.0).collect();
        Skin { kind, dims: spec.dims, axles_z, steered: spec.steered }
    }

    /// The skin of a physics definition: the hull box, the axle positions, the track and the tyre are the definition's. The game garage's
    /// ids select the family (`scout_4x4`, `hauler_4x4`); the Mule (`mule_4x4`) wears the utility skin, whose dossier-based box is kept.
    pub fn from_def(def: &VehicleDef) -> Result<Skin, String> {
        let kind = match def.id.as_str() {
            "scout_4x4" => TruckKind::Scout,
            "hauler_4x4" => TruckKind::Hauler,
            "mule_4x4" | "utility_4x4" => {
                return Skin::for_id("utility_4x4").ok_or_else(|| "no utility skin".to_string())
            }
            other => return Err(format!("{other} has no skin")),
        };
        let RunningGearDef::Wheeled(gear) = &def.running_gear else { return Err(format!("{} is tracked", def.id)) };
        let (first, last) = (gear.axles.first().ok_or("no axles")?, gear.axles.last().ok_or("no axles")?);
        let h = &def.hull;
        // the wheel's rim and lug depth are the stand-in wheel's proportions at this radius, and it has at most as many lugs (the triangle budget)
        let (radius, base) = (gear.tyre.outer_diameter_m.v / 2.0, WheelDims::placeholder());
        let k = radius / base.outer_radius_m;
        let wheel = WheelDims {
            outer_radius_m: radius,
            width_m: gear.tyre.section_width_m.v,
            rim_radius_m: base.rim_radius_m * k,
            lug_depth_m: base.lug_depth_m * k,
            lugs_around: ((f64::from(base.lugs_around) * k).round() as u32).min(base.lugs_around),
        };
        let dims = UtilityDims {
            length_m: h.length_m.v,
            width_m: h.width_m.v,
            height_m: h.height_m.v,
            wheelbase_m: last.from_front_m.v - first.from_front_m.v,
            track_m: first.track_width_m.v,
            ground_clearance_m: h.ground_clearance_m.v,
            wheel,
            regions_m: None,
        };
        let axles_z = gear.axles.iter().map(|a| a.from_front_m.v - dims.length_m / 2.0).collect();
        Ok(Skin { kind, dims, axles_z, steered: gear.axles.iter().map(|a| a.steered).collect() })
    }

    /// The parts of the skin at a `detail` level, in the hull frame with the wheels in their node frames.
    pub fn parts(&self, detail: u8) -> Vec<Part> {
        truck_assembly(self.kind, &self.dims, &self.axles_z, &self.steered, detail).parts
    }
}
