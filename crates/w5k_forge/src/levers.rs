//! The lever API of the Design Impact Matrix (`docs/validation/IMPACT-MATRIX.md`): perturb a `VehicleDef` by lever name and factor.
//! Each lever scales the design numbers a designer would touch; the compile turns the result into a rig (or a clear rejection). A
//! perturbed number may leave its authored band: the band is widened to include it, because the perturbation is an experiment, not a
//! design claim. Provenance and source are kept.

use w5k_contract::def::{RunningGearDef, VehicleDef};
use w5k_contract::param::Param;

/// One lever: its name, what it scales, and the compiled quantity that moves with it (what the tests check).
pub struct LeverInfo {
    pub name: &'static str,
    pub scales: &'static str,
    pub compiled_effect: &'static str,
}

pub const LEVERS: &[LeverInfo] = &[
    LeverInfo { name: "engine_peak_power", scales: "powertrain.engine.peak_power_w and peak_torque_nm together (same curve shape)", compiled_effect: "peak of torque x omega on the engine curve, x factor; peak torque x factor" },
    LeverInfo { name: "torque_peak_rpm", scales: "peak_torque_rpm and peak_power_rpm together, with peak_power_w following (the same torque curve moved along the rpm axis)", compiled_effect: "rpm of the torque-curve maximum, x factor" },
    LeverInfo { name: "final_drive", scales: "powertrain.final_drive_ratio", compiled_effect: "axle differential ratio, x factor" },
    LeverInfo { name: "first_gear", scales: "powertrain.gearbox.forward_ratios[0]", compiled_effect: "gearbox first ratio, x factor" },
    LeverInfo { name: "brake_torque", scales: "brakes.service_decel_g", compiled_effect: "total brake torque capacity, x factor (rejected above the tyre's friction)" },
    LeverInfo { name: "brake_thermal_mass", scales: "brakes.thermal_mass_kj_k", compiled_effect: "brake thermal mass, x factor" },
    LeverInfo { name: "mass", scales: "hull.mass_kg", compiled_effect: "hull (sprung) mass, x factor; springs, dampers and preloads follow; total brake torque is HELD (service_decel_g scales by 1/total-mass ratio)" },
    LeverInfo { name: "com_height", scales: "hull.com_height_m", compiled_effect: "COM height above the ground, x factor" },
    LeverInfo { name: "ground_clearance", scales: "hull.ground_clearance_m", compiled_effect: "hull underside height above the ground, x factor" },
    LeverInfo { name: "wheelbase", scales: "axle positions about the front axle (COM keeps its fraction between the axles; hull length keeps the rear overhang)", compiled_effect: "distance between the first and last station rows, x factor" },
    LeverInfo { name: "track_gauge", scales: "running_gear.axles[*].track_width_m", compiled_effect: "lateral station offset, x factor" },
    LeverInfo { name: "ride_frequency", scales: "suspension.{front,rear}_ride_frequency_hz", compiled_effect: "ride rate (series spring and tyre), x factor squared" },
    LeverInfo { name: "spring_rate", scales: "ride frequency by sqrt(factor), so the rate scales by factor", compiled_effect: "ride rate, x factor" },
    LeverInfo { name: "damping", scales: "suspension.damping_ratio", compiled_effect: "mean damper coefficient, x factor" },
    LeverInfo { name: "suspension_travel", scales: "suspension.{bump,droop}_travel_m", compiled_effect: "station bump and droop travel, x factor" },
    LeverInfo { name: "tyre_width", scales: "running_gear.tyre.section_width_m", compiled_effect: "wheel width, x factor; contact patch length x 1/factor" },
    LeverInfo { name: "tyre_pressure", scales: "running_gear.tyre.inflation_pa", compiled_effect: "contact patch length, x 1/factor (vertical stiffness is its own number: see the request file)" },
    LeverInfo { name: "tyre_friction", scales: "running_gear.tyre.mu_peak_ref", compiled_effect: "tyre mu_scale, x factor" },
    LeverInfo { name: "frontal_area", scales: "aero.frontal_area_m2", compiled_effect: "aero frontal area, x factor" },
    LeverInfo { name: "drag_coeff", scales: "aero.drag_coeff", compiled_effect: "aero drag coefficient, x factor" },
];

fn scale(p: &mut Param, f: f64) {
    p.v *= f;
    p.lo = p.lo.map(|lo| lo.min(p.v));
    p.hi = p.hi.map(|hi| hi.max(p.v));
}

/// Perturb `def` by `lever` and `factor` (> 0, finite). Unknown lever or a def the lever does not apply to is an error with a reason.
pub fn apply(def: &VehicleDef, lever: &str, factor: f64) -> Result<VehicleDef, String> {
    if !(factor.is_finite() && factor > 0.0) {
        return Err(format!("lever factor {factor} must be finite and positive"));
    }
    let mut d = def.clone();
    let RunningGearDef::Wheeled(w) = &mut d.running_gear else {
        return Err(format!("lever {lever:?}: tracked vehicles have no levers yet (the tracked compile is not built)"));
    };
    match lever {
        "engine_peak_power" => {
            // An engine of the same character, bigger or smaller: power AND torque scale together, so the curve keeps its shape. Power alone
            // would change the curve's shape and can make the two peaks inconsistent (power would peak earlier than stated).
            scale(&mut d.powertrain.engine.peak_power_w, factor);
            scale(&mut d.powertrain.engine.peak_torque_nm, factor);
        }
        "torque_peak_rpm" => {
            // The whole curve moves along the rpm axis (both peaks), which keeps its shape; moving only the torque peak toward the power
            // peak changes the shape and can make the peaks inconsistent. A power peak pushed past the redline is rejected by the compile.
            scale(&mut d.powertrain.engine.peak_torque_rpm, factor);
            scale(&mut d.powertrain.engine.peak_power_rpm, factor);
            // The torque values stay, so the peak power moves with the rpm (P = T omega).
            scale(&mut d.powertrain.engine.peak_power_w, factor);
        }
        "final_drive" => scale(&mut d.powertrain.final_drive_ratio, factor),
        "first_gear" => {
            scale(d.powertrain.gearbox.forward_ratios.first_mut().ok_or("the gearbox has no forward ratio")?, factor)
        }
        "brake_torque" => scale(&mut d.brakes.service_decel_g, factor),
        "brake_thermal_mass" => scale(&mut d.brakes.thermal_mass_kj_k, factor),
        "mass" => {
            // The brakes are hardware: more mass must not buy more braking torque. The def states brake capability as a deceleration, so
            // the lever scales it by the inverse of the total-mass ratio to hold the torque constant (a heavier vehicle brakes less hard).
            let unsprung = w.axles.len() as f64 * 2.0 * w.tyre.unsprung_mass_kg.v;
            let (before, after) = (d.hull.mass_kg.v + unsprung, d.hull.mass_kg.v * factor + unsprung);
            scale(&mut d.hull.mass_kg, factor);
            scale(&mut d.brakes.service_decel_g, before / after);
        }
        "com_height" => scale(&mut d.hull.com_height_m, factor),
        "ground_clearance" => scale(&mut d.hull.ground_clearance_m, factor),
        "wheelbase" => {
            let front = w.axles.first().ok_or("no axles")?.from_front_m.v;
            let rear = w.axles.last().ok_or("no axles")?.from_front_m.v;
            let span = rear - front;
            let new_span = span * factor;
            // Interior axles keep their fraction of the span; the COM keeps its fraction between the axles; the rear overhang is kept.
            for a in w.axles.iter_mut().skip(1) {
                let frac = (a.from_front_m.v - front) / span;
                let v = front + frac * new_span;
                a.from_front_m.v = v;
                a.from_front_m.lo = a.from_front_m.lo.map(|l| l.min(v));
                a.from_front_m.hi = a.from_front_m.hi.map(|h| h.max(v));
            }
            let com_frac = (d.hull.com_from_front_m.v - front) / span;
            let (com, len) = (front + com_frac * new_span, d.hull.length_m.v + (new_span - span));
            d.hull.com_from_front_m.v = com;
            d.hull.com_from_front_m.lo = d.hull.com_from_front_m.lo.map(|l| l.min(com));
            d.hull.com_from_front_m.hi = d.hull.com_from_front_m.hi.map(|h| h.max(com));
            d.hull.length_m.v = len;
            d.hull.length_m.lo = d.hull.length_m.lo.map(|l| l.min(len));
            d.hull.length_m.hi = d.hull.length_m.hi.map(|h| h.max(len));
        }
        "track_gauge" => w.axles.iter_mut().for_each(|a| scale(&mut a.track_width_m, factor)),
        "ride_frequency" | "spring_rate" => {
            let f = if lever == "spring_rate" { w5k_math::scalar::sqrt(factor) } else { factor };
            scale(&mut d.suspension.front_ride_frequency_hz, f);
            scale(&mut d.suspension.rear_ride_frequency_hz, f);
        }
        "damping" => scale(&mut d.suspension.damping_ratio, factor),
        "suspension_travel" => {
            scale(&mut d.suspension.bump_travel_m, factor);
            scale(&mut d.suspension.droop_travel_m, factor);
        }
        "tyre_width" => scale(&mut w.tyre.section_width_m, factor),
        "tyre_pressure" => scale(&mut w.tyre.inflation_pa, factor),
        "tyre_friction" => scale(&mut w.tyre.mu_peak_ref, factor),
        "frontal_area" => scale(&mut d.aero.frontal_area_m2, factor),
        "drag_coeff" => scale(&mut d.aero.drag_coeff, factor),
        other => {
            return Err(format!(
                "unknown lever {other:?}; the levers are: {}",
                LEVERS.iter().map(|l| l.name).collect::<Vec<_>>().join(", ")
            ))
        }
    }
    Ok(d)
}
