//! From a design to the numbers a time trial runs on.
//!
//! The vehicle sheet *measures* a design (mass, power, running gear); [`Forge::mover_spec`] turns that measurement into a
//! [`MoverSpec`], the physical description the simulation drives (`w5k_sim::mover`). It adds no physics of its own: it decides
//! which of the sheet's numbers feed which term of the model, for each kind of running gear. The gear's own figures come
//! from [`Gear`], the same summary the sheet uses for its top speed, so the two cannot disagree about one vehicle.
//!
//! What each kind of running gear is in the model:
//!
//! | Gear | Pushes against the ground with | Pays for |
//! |---|---|---|
//! | tracks, wheels | grip x load (the family's traction coefficient) | rolling resistance x load |
//! | legs | grip x load; the gait's cost of transport comes out of the engine's power *first* | (inside the machine) |
//! | air cushion | fans: a fixed fraction of the weight | a skirt that drags harder the faster it goes |
//! | anti-gravity, rotor | thrust: a fixed fraction of the weight | nothing: they are off the ground |
//! | rail | n/a: the course has no rails | the vehicle does not start |

use w5k_sim::{GearClass, MoverSpec};

use crate::assemble::{Assembly, VehicleSheet};
use crate::schema::Locomotion;
use crate::sheet::Gear;
use crate::Forge;

/// Thrust of an air cushion's propulsion fans, as a fraction of the weight (a real hovercraft: 0.1 to 0.2).
pub const CUSHION_THRUST_W: f64 = 0.12;
/// Thrust of anti-gravity pods, as a fraction of the weight.
pub const ANTIGRAV_THRUST_W: f64 = 0.5;
/// Thrust of a rotor craft (tilting the disc forward), as a fraction of the weight.
pub const ROTOR_THRUST_W: f64 = 0.3;
/// Grip assumed for running gear that states none: the legs' pad coefficient.
const DEFAULT_GRIP: f64 = 0.8;
/// Drag coefficient of a vehicle's frontal area (the sheet's value: boxy).
const CD: f64 = 0.9;
/// A walker's stride (the distance between two steps of one foot) as a multiple of its stance.
const STRIDE_PER_STANCE: f64 = 1.6;

/// The kind of running gear that sets how the vehicle moves, if it has any. Anything that flies decides it; then floating, then
/// whatever ground gear was met first (the order the sheet uses to pick its turning model).
pub fn gear_class(gear: &Gear) -> Option<GearClass> {
    let has = |l: Locomotion| gear.locomotion.contains(&l);
    if gear.rail {
        Some(GearClass::Rail)
    } else if has(Locomotion::Rotor) || has(Locomotion::Jet) {
        Some(GearClass::Rotor)
    } else if has(Locomotion::AntiGrav) {
        Some(GearClass::AntiGrav)
    } else if has(Locomotion::Hover) {
        Some(GearClass::Cushion)
    } else {
        match gear.locomotion.first() {
            Some(Locomotion::Tracks | Locomotion::HalfTracks) => Some(GearClass::Tracks),
            Some(Locomotion::Wheels) => Some(GearClass::Wheels),
            Some(Locomotion::Legs) => Some(GearClass::Legs),
            _ => None,
        }
    }
}

/// The distance between the points a vehicle rests on, which sets its pitch on a slope and the slope it feels (m).
fn support_span(class: GearClass, gear: &Gear, length_m: f64) -> f64 {
    let ground = |z: (f64, f64), fallback: f64| if z.1 > z.0 { z.1 - z.0 } else { fallback };
    let span = match class {
        GearClass::Tracks => {
            if gear.track_len > 0.0 {
                gear.track_len
            } else {
                0.8 * length_m
            }
        }
        GearClass::Wheels => ground(gear.wheel_z, 0.5 * length_m),
        GearClass::Legs => ground(gear.leg_z, 0.5 * length_m),
        GearClass::Cushion => {
            if gear.cushion_len > 0.0 {
                gear.cushion_len
            } else {
                0.9 * length_m
            }
        }
        GearClass::AntiGrav | GearClass::Rotor | GearClass::Rail => 0.8 * length_m,
    };
    span.clamp(1.0, 60.0)
}

/// Build a vehicle's mover spec from its sheet and its running gear (separate from `Forge::mover_spec` so that it can be tested
/// without any content).
pub fn mover_from(id: &str, sheet: &VehicleSheet, gear: &Gear, clearance_m: f64) -> MoverSpec {
    let class = gear_class(gear);
    let mut spec = MoverSpec {
        id: id.to_string(),
        class: class.unwrap_or(GearClass::Tracks),
        mass_t: sheet.mass_kg / 1000.0,
        cd_a_m2: CD * sheet.frontal_m2,
        ..MoverSpec::default()
    };
    let Some(class) = class else {
        spec.dns = Some("no running gear".into());
        return spec;
    };
    if let Some(problem) = sheet.problems.first() {
        spec.dns = Some(format!("invalid design: {problem}"));
        return spec;
    }
    if class == GearClass::Rail {
        spec.dns = Some("rail-bound: this course has no rails".into());
        return spec;
    }

    // Power that reaches the running gear: what the engine makes, less what other systems draw and what lift costs, times the
    // efficiency of the drive. (The same arithmetic as the sheet's top speed.)
    let spare = ((sheet.power_kw - sheet.draw_kw).max(0.0) - sheet.lift_kw).max(0.0);
    spec.drive_kw = spare * Gear::mean(&gear.efficiency);
    spec.rated_ms = if gear.limit_kmh.is_finite() { gear.limit_kmh } else { (1.2 * sheet.top_speed_kmh).max(10.0) } / 3.6;
    spec.span_m = support_span(class, gear, sheet.length_m);
    let rolling = Gear::mean(&gear.rolling);
    let traction = if gear.traction.is_empty() { None } else { Some(Gear::mean(&gear.traction)) };
    match class {
        GearClass::Tracks | GearClass::Wheels => {
            spec.c_roll = rolling;
            spec.grip_mu = traction.unwrap_or(DEFAULT_GRIP);
        }
        GearClass::Legs => {
            // The "rolling" figure of legs is their cost of transport: power per weight per speed, spent in the gait.
            spec.c_internal = rolling;
            spec.grip_mu = traction.unwrap_or(DEFAULT_GRIP);
            // Each foot is pressed in once per stride; a stride is about 1.6 times the stance (the hip's height above the foot: a
            // part that does not state it is measured from the geometry, as the hull's clearance).
            spec.stride_m = STRIDE_PER_STANCE * gear.stance.max(clearance_m).max(0.5);
        }
        GearClass::Cushion => {
            spec.c_roll = rolling;
            spec.thrust_w = CUSHION_THRUST_W;
            spec.skirt_drag = true;
        }
        GearClass::AntiGrav => {
            spec.thrust_w = ANTIGRAV_THRUST_W;
            spec.altitude_m = gear.altitude_m;
        }
        GearClass::Rotor => {
            spec.thrust_w = ROTOR_THRUST_W;
            spec.altitude_m = gear.altitude_m;
        }
        GearClass::Rail => {}
    }
    if class.grounded() {
        if let Some((units, width, length, area)) = gear.footprint() {
            spec.contact_units = units;
            spec.contact_width_m = width;
            spec.contact_len_m = length;
            spec.contact_area_m2 = area;
            spec.clearance_m = clearance_m;
        }
    }
    spec
}

/// Height of the hull's underside above the ground (m): the lowest point of the hull over the lowest point of the running
/// gear. Zero when there is no hull or no gear to measure against.
pub fn hull_clearance(forge: &Forge, asm: &Assembly) -> f64 {
    let (mut gear_lo, mut hull_lo) = (f64::MAX, f64::MAX);
    for p in &asm.pieces {
        let lo = p.poly.aabb().0.y;
        if forge.parts[&asm.parts[p.part as usize].0].def.function.locomotion.is_some() {
            gear_lo = gear_lo.min(lo);
        } else if p.part == 0 {
            hull_lo = hull_lo.min(lo);
        }
    }
    if gear_lo < f64::MAX && hull_lo < f64::MAX {
        (hull_lo - gear_lo).max(0.0)
    } else {
        0.0
    }
}

impl Forge {
    /// The mover spec of an assembled vehicle (see the module docs).
    pub fn mover_spec(&self, id: &str, asm: &Assembly, sheet: &VehicleSheet) -> MoverSpec {
        let mut gear = Gear::default();
        for (part, x) in &asm.parts {
            gear.add(&self.parts[part].def.function, x);
        }
        mover_from(id, sheet, &gear, hull_clearance(self, asm))
    }
}
