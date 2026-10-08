//! Combat data of a rig: armour, damage modules, removable mass and sights. **PROVISIONAL** (`PROVISIONAL(COMBAT settling round)`): the
//! vocabulary and the conventions are fixed now so that FORGE, GEOMETRY and COMBAT build to one shape; COMBAT's design note refines the
//! fields by CCR before the freeze. Every list is empty for a vehicle that has no combat data yet.
//!
//! Conventions (the contract fixes these now; see `docs/architecture/CONTRACTS.md`):
//! * Data is addressed by `Option<usize>` articulation joint indices, `None` = the hull; volumes and solids sit in that body's frame.
//! * **A body's mass includes its removable items at as-loaded fill** (a full ammunition rack); `MassItemDef` says how much of it can leave.
//! * Damage never mutates a rig: COMBAT's `degrade(&PhysRig, &Health) -> PhysRig` returns a rig whose numbers differ only where the damaged
//!   module governs, and the vehicle swaps it in (`VehicleModel::swap_rig`); `PhysRig::rig_hash` changes with it.
//! * One shape vocabulary for collision proxies, armour solids and module volumes: [`crate::rig::ProxyShape`].
//! * Armour authoring: FORGE compiles per-face thickness sliders of the `VehicleDef` onto GEOMETRY's closed shells into `armour`; COMBAT owns
//!   the material table and the penetration kernels. Armour is never read from visual meshes.

use serde::{Deserialize, Serialize};
use w5k_math::{Transform, Vec3};

use crate::rig::{CrewRole, ProxyShape, Side};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CombatDef {
    #[serde(default)]
    pub materials: Vec<ArmourMaterial>,
    #[serde(default)]
    pub armour: Vec<ArmourSolid>,
    #[serde(default)]
    pub modules: Vec<ModuleDef>,
    #[serde(default)]
    pub mass_items: Vec<MassItemDef>,
    #[serde(default)]
    pub sensors: Vec<SensorDef>,
}

/// Protection of a material per metre of thickness, as metres of rolled homogeneous armour (RHA) equivalent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArmourMaterial {
    pub name: String,
    pub density_kg_m3: f64,
    /// Against kinetic-energy rounds (m RHA per m).
    pub rha_kinetic: f64,
    /// Against shaped charges (m RHA per m).
    pub rha_chemical: f64,
}

/// A closed solid that stops shells. The line-of-sight thickness of a hit is the chord of the shot's path through it (`t / cos(obliquity)`
/// for a plate).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArmourSolid {
    pub name: String,
    /// Body whose frame the solid is in (`None` = the hull).
    pub body: Option<usize>,
    pub pose: Transform,
    pub shape: ProxyShape,
    /// Index into `CombatDef::materials`.
    pub material: usize,
    /// The module that owns this solid (reactive armour, a skirt) and takes it away when destroyed.
    #[serde(default)]
    pub module: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ModuleKind {
    Engine,
    Transmission,
    FuelTank,
    Ammunition,
    Crew(CrewRole),
    /// A track of one side (a thrown or cut track).
    Track(Side),
    TurretRing,
    Gun,
    Sight,
    Other,
}

/// What a destroyed (or damaged) module does to the rig: the contract between COMBAT's `degrade()` and the rig fields it may touch, so
/// CHASSIS, DRIVE and TRACKS need to know nothing about damage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DerateTarget {
    /// The engine's torque curve at every rpm.
    EnginePower,
    Gearbox,
    Brakes,
    /// The output of that side's sprocket (a thrown track).
    Track(Side),
    Station(usize),
    /// A servo joint (a jammed ring): its rates and effort.
    Joint(usize),
    Muzzle(usize),
    Sensor(usize),
    /// Every servo whose actuator is that crew role.
    Crew(CrewRole),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ModuleEffect {
    /// Multiply the capability of `target` by `factor` (0 = destroyed).
    Derate { target: DerateTarget, factor: f64 },
    /// Chance that the stored ammunition or fuel goes up when this module is hit and destroyed (wet stowage lowers it); `vented` = blow-off panels
    /// keep the crew compartment safe.
    CookOff { chance: f64, vented: bool },
    /// Starts a fire (heat per second, W) while the module is destroyed.
    Fire { power_w: f64 },
}

/// A volume that belongs to a module, in a body's frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModuleVolume {
    pub body: Option<usize>,
    pub pose: Transform,
    pub shape: ProxyShape,
}

/// A part of the vehicle that can be hit and damaged. A shot's residual path walks the volumes in order and deposits its energy in each.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModuleDef {
    pub name: String,
    pub kind: ModuleKind,
    /// Energy that destroys it, J.
    pub health_j: f64,
    pub volumes: Vec<ModuleVolume>,
    pub effects: Vec<ModuleEffect>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MassKind {
    Ammunition,
    Fuel,
    Crew,
    /// Add-on armour, skirts, reactive tiles.
    AddOn,
}

/// Mass inside a body's mass that can leave (ammunition fired, fuel burned, a skirt shot off), with where it sits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MassItemDef {
    pub name: String,
    pub kind: MassKind,
    pub body: Option<usize>,
    /// Position in that body's frame, m.
    pub pos_m: Vec3,
    /// Mass at as-loaded fill (already inside the body's mass), kg.
    pub full_mass_kg: f64,
    /// Rounds at as-loaded fill (ammunition).
    pub rounds: u32,
    /// Time to bring a round from this store to the gun (ready rack 0 s, hull rack 6 s).
    pub handling_s: f64,
    #[serde(default)]
    pub module: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SensorKind {
    DaySight,
    Thermal,
    Periscope,
    Rangefinder,
}

/// A sight or sensor: its optical axis is -Z of `pose` (never a private convention).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SensorDef {
    pub name: String,
    pub kind: SensorKind,
    pub body: Option<usize>,
    pub pose: Transform,
    pub fov_rad: f64,
    /// The muzzle (index into `PhysRig::muzzles`) this sight is boresighted to; `None` = independent.
    #[serde(default)]
    pub boresight_to: Option<usize>,
}
