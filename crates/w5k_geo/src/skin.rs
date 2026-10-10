//! Skins: the wheeled-truck hulls of the game garage, each a hull family (`truck::TruckKind`) with the wheel modules on its axles, built
//! either from stand-in dimensions (`Skin::placeholder`, `PROVISIONAL(C-002)`) or from FORGE's physics definition (`Skin::from_def`), so
//! the wheel radii, track, wheelbase and hull box of the picture are the physics rig's. Every wheeled skin has the joint layout of the
//! export: four spins, two steers, four travels, front axle left then right, rear axle left then right. The tracked carrier is a skin too
//! (`carrier_tracked`), built from the compiled rig (`Skin::from_rig`): its joint layout is that rig's.

use crate::carrier::{placeholder_dims as carrier_dims, tracked_assembly};
use crate::export::{render_rig_with, Overrides};
use crate::flags::FlagParams;
use crate::part::Part;
use crate::track::{pitch_radius_m, Belt, LinkSpec, RunSpec, RunWheel, StationIds};
use crate::truck::{truck_assembly, TruckKind, UtilityDims};
use crate::wheel::WheelDims;
use serde::Deserialize;
use w5k_contract::def::{RunningGearDef, VehicleDef};
use w5k_contract::render::RenderRig;
use w5k_contract::rig::{PhysRig, Side, WheelKind};

/// Every wheeled skin id, smallest vehicle first (`carrier_tracked` is the tracked skin and has the joint layout of its own rig).
pub const IDS: [&str; 3] = ["scout_4x4", "utility_4x4", "hauler_4x4"];

/// A hull family with its dimensions and the axles it is cut for (z in the hull frame, front first).
#[derive(Clone, Debug)]
pub struct Skin {
    pub kind: TruckKind,
    pub dims: UtilityDims,
    pub axles_z: Vec<f64>,
    pub steered: Vec<bool>,
    /// The running gear of a tracked skin (`None` for a wheeled one).
    pub tracks: Option<TrackGear>,
}

/// A tracked skin's running gear, and what its physics rig adds to the parts: the travel axes and joint count of the export, and the notes on
/// where the rig and the drawn belt disagree (none for a consistent rig).
#[derive(Clone, Debug)]
pub struct TrackGear {
    pub run: RunSpec,
    pub overrides: Overrides,
    pub notes: Vec<String>,
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
                    tracks: None,
                })
            }
            "carrier_tracked" => Some(Skin {
                kind: TruckKind::Carrier,
                dims: carrier_dims(),
                axles_z: Vec::new(),
                steered: Vec::new(),
                tracks: Some(TrackGear {
                    run: RunSpec::placeholder(),
                    overrides: Overrides::default(),
                    notes: Vec::new(),
                }),
            }),
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
        Skin { kind, dims: spec.dims, axles_z, steered: spec.steered, tracks: None }
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
        Ok(Skin { kind, dims, axles_z, steered: gear.axles.iter().map(|a| a.steered).collect(), tracks: None })
    }

    /// The parts of the skin at a `detail` level, in the hull frame with the wheels in their node frames.
    pub fn parts(&self, detail: u8) -> Vec<Part> {
        match &self.tracks {
            Some(t) => {
                tracked_assembly(&self.dims, &t.run, &LinkSpec::standard(), detail)
                    .expect("the belt of a tracked skin was checked when it was built")
                    .parts
            }
            None => truck_assembly(self.kind, &self.dims, &self.axles_z, &self.steered, detail).parts,
        }
    }

    /// The skin as a `RenderRig` at a `detail` level, with the physics rig's travel axes and joint count when it was built from one.
    pub fn rig(&self, detail: u8, flags: &FlagParams) -> RenderRig {
        let over = self.tracks.as_ref().map(|t| t.overrides.clone()).unwrap_or_default();
        render_rig_with(self.kind.id(), &self.parts(detail), flags, &over)
    }

    /// The skin of a compiled tracked rig: the hull box is the definition's, every wheel (road wheels, sprocket, idler, return rollers) is
    /// exactly where the rig puts it, with its radius and width, the belt is the band round them (thickness, pitch and teeth from the track),
    /// and the joint coordinates follow the rig's `joint_names()`. An error says what in the rig the carrier cannot be drawn from (a wheel the
    /// belt does not touch, an asymmetric rig, no belt thickness); `TrackGear::notes` lists where the drawn belt disagrees with the rig's
    /// numbers (belt length, contact length) without refusing it.
    pub fn from_rig(rig: &PhysRig, def: &VehicleDef) -> Result<Skin, String> {
        if !matches!(def.running_gear, RunningGearDef::Tracked(_)) {
            return Err(format!("{} is not tracked", def.id));
        }
        let track = |side: Side| {
            rig.tracks.iter().find(|t| t.side == side).ok_or_else(|| format!("{} has no {side:?} track", rig.id))
        };
        let (tr, tl) = (track(Side::Right)?, track(Side::Left)?);
        if tr.stations.len() != tl.stations.len() {
            return Err("the two tracks wrap different numbers of wheels".into());
        }
        let station = |i: usize| rig.stations.get(i).ok_or_else(|| format!("track station {i} is not in the rig"));
        let x = station(tr.stations[0])?.rest_pos_m.x;
        let (mut wheels, tol) = (Vec::new(), 1e-6); // const-ok: positions of a mirrored rig agree to a micrometre
        for (&ri, &li) in tr.stations.iter().zip(&tl.stations) {
            let (r, l) = (station(ri)?, station(li)?);
            let (p, q) = (r.rest_pos_m, l.rest_pos_m);
            let mirrored = (p.x + q.x).abs() < tol
                && (p.y - q.y).abs() < tol
                && (p.z - q.z).abs() < tol
                && r.wheel.kind == l.wheel.kind;
            if !mirrored || (p.x - x).abs() > tol || x <= 0.0 {
                return Err(format!("{} and {}: the carrier skin needs a mirror-symmetric rig with the right track at +x and one centre line", r.name, l.name));
            }
            wheels.push(RunWheel {
                kind: r.wheel.kind,
                z_m: p.z,
                y_m: p.y,
                radius_m: r.wheel.radius_m,
                width_m: r.wheel.width_m,
            });
        }
        let first_road = wheels.iter().find(|w| w.kind == WheelKind::RoadWheel).copied().ok_or("no road wheels")?;
        let sprocket = wheels.iter().find(|w| w.kind == WheelKind::Sprocket).copied().ok_or("no sprocket")?;
        if tr.thickness_m <= 0.0 || tr.pitch_m <= 0.0 || tr.sprocket_teeth == 0 {
            return Err(format!("{}: the belt needs a thickness, a pitch and a sprocket tooth count", tr.name));
        }
        let pitch_radius = pitch_radius_m(tr.pitch_m, u32::from(tr.sprocket_teeth));
        if (sprocket.radius_m / pitch_radius - 1.0).abs() > 0.02 {
            // const-ok: the contract says the sprocket's radius is its pitch radius; 2% for rounding in an authored number
            return Err(format!(
                "the sprocket's radius {} m is not the pitch radius {pitch_radius:.4} m of {} teeth of pitch {} m",
                sprocket.radius_m, tr.sprocket_teeth, tr.pitch_m
            ));
        }
        let (mut ids_r, mut ids_l) = (
            tr.stations.iter().map(|&i| i as u8).collect::<Vec<_>>(),
            tl.stations.iter().map(|&i| i as u8).collect::<Vec<_>>(),
        );
        // the band is walked counter-clockwise in (z, y): the top run leaves the sprocket towards the idler, towards -z for a rear sprocket; a
        // front sprocket's loop runs the other way, so reverse it
        let idler = wheels.iter().find(|w| w.kind == WheelKind::Idler).copied().ok_or("no idler")?;
        if idler.z_m > sprocket.z_m {
            wheels.reverse();
            ids_r.reverse();
            ids_l.reverse();
        }
        let run = RunSpec {
            track_x_m: x,
            belt_width_m: tr.width_m,
            belt_thickness_m: tr.thickness_m,
            pitch_m: tr.pitch_m,
            sprocket_teeth: u32::from(tr.sprocket_teeth),
            wheels,
            stations: Some(StationIds { right: ids_r, left: ids_l }),
        };
        let belt = Belt::round(&run.circles())?;
        let mut notes = Vec::new();
        if tr.belt_length_m > 0.0 && (belt.length_m / tr.belt_length_m - 1.0).abs() > 0.03 {
            // const-ok: 3%, the chordal action and an authored length
            notes.push(format!(
                "the belt round the wheels is {:.2} m, the rig's belt_length_m is {:.2} m",
                belt.length_m, tr.belt_length_m
            ));
        }
        let (z0, z1) = run
            .wheels
            .iter()
            .filter(|w| w.kind == WheelKind::RoadWheel)
            .fold((f64::MAX, f64::MIN), |(a, b), w| (a.min(w.z_m), b.max(w.z_m)));
        if tr.contact_length_m > 0.0 && ((z1 - z0) / tr.contact_length_m - 1.0).abs() > 0.05 {
            // const-ok: 5%, the straight ground run between the first and last road wheel against the rig's contact length
            notes.push(format!(
                "the straight ground run is {:.2} m, the rig's contact_length_m is {:.2} m",
                z1 - z0,
                tr.contact_length_m
            ));
        }
        let h = &def.hull;
        let base = WheelDims::placeholder();
        let k = first_road.radius_m / base.outer_radius_m;
        let dims = UtilityDims {
            length_m: h.length_m.v,
            width_m: h.width_m.v,
            height_m: h.height_m.v,
            wheelbase_m: z1 - z0,
            track_m: 2.0 * x,
            ground_clearance_m: h.ground_clearance_m.v,
            wheel: WheelDims {
                outer_radius_m: first_road.radius_m,
                width_m: first_road.width_m,
                rim_radius_m: base.rim_radius_m * k,
                lug_depth_m: 0.0,
                lugs_around: 0,
            },
            regions_m: None,
        };
        if (rig.ride_height_m - dims.ride_height_m()).abs() > 0.01 {
            // const-ok: 1 cm between the definition's hull box and the rig's ride height
            notes.push(format!(
                "the definition's ride height is {:.3} m, the rig's is {:.3} m",
                dims.ride_height_m(),
                rig.ride_height_m
            ));
        }
        let overrides = Overrides {
            travel_axes: rig.stations.iter().map(|s| s.bump_dir).collect(),
            joint_count: Some(rig.joint_names().len()),
        };
        Ok(Skin {
            kind: TruckKind::Carrier,
            dims,
            axles_z: Vec::new(),
            steered: Vec::new(),
            tracks: Some(TrackGear { run, overrides, notes }),
        })
    }
}
