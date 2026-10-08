//! The vehicle sheet: totals, budgets and performance of an assembled vehicle.
//!
//! Everything here is a closed-form physical model fed by the parts' `Function` data and by measured mass and
//! armour. The same function serves the full build (voxel mass, full armour table) and the quick path used to
//! sample thousands of designs (exact-volume mass, four-direction armour).

use crate::armour::ArmourSummary;
use crate::assemble::{self, power_balance_speed, Assembly, VehicleSheet, WeaponLine};
use crate::geom::V3;
use crate::schema::{Category, Locomotion};
use crate::{voxel, Forge};

const G: f64 = 9.81;
const RHO_AIR: f64 = 1.225;
/// Radius of the "virtual" world the horizon is computed on (m). Deliberately large, so a 2 m tank sees about a
/// kilometre and a 50 m titan about five (docs/design/01-technical-architecture.md).
pub const WORLD_RADIUS_M: f64 = 250_000.0;
/// Height of the standard target (m).
pub const TARGET_H: f64 = 3.0;
/// Range of the plain optics every vehicle has (m).
pub const BASE_SIGHT_M: f64 = 2500.0;

/// Distance to the horizon for an observer `eye_height_m` above the ground, to a standard 3 m target.
pub fn horizon_m(eye_height_m: f64) -> f64 {
    (2.0 * WORLD_RADIUS_M * eye_height_m.max(0.0)).sqrt() + (2.0 * WORLD_RADIUS_M * TARGET_H).sqrt()
}

/// What the sheet needs to know about the vehicle as a whole.
pub struct SheetInput {
    pub mass_kg: f64,
    pub com: V3,
    pub armour: ArmourSummary,
    /// Extents of the vehicle (m).
    pub lo: V3,
    pub hi: V3,
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        0.0
    } else {
        v.iter().sum::<f64>() / v.len() as f64
    }
}

impl Forge {
    pub fn make_sheet(&self, asm: &Assembly, inp: &SheetInput) -> VehicleSheet {
        let mut s = VehicleSheet { mass_kg: inp.mass_kg, problems: asm.errors.clone(), ..Default::default() };
        let weight_n = inp.mass_kg * G;
        let size = inp.hi - inp.lo;
        s.length_m = size.z;
        s.width_m = size.x;
        s.height_m = size.y;
        s.lift_height_m = asm.parts.first().map(|(_, x)| x.t.y).unwrap_or(0.0);
        let a = &inp.armour;
        s.frontal_m2 = a.front.area_m2;
        s.side_area_m2 = a.side.area_m2;
        s.top_area_m2 = a.top.area_m2;
        s.armour_front_mm = a.front.median_mm;
        s.armour_side_mm = a.side.median_mm;
        s.armour_rear_mm = a.rear.median_mm;
        s.armour_top_mm = a.top.median_mm;
        s.armour_front_weak_mm = a.front.weak_mm;

        let mut rolling = Vec::new();
        let mut efficiency = Vec::new();
        let mut gear_limit = f64::INFINITY;
        let mut disc_area = 0.0;
        let mut contact = 0.0;
        let (mut cu_area, mut cu_perim, mut cu_gap, mut cu_n) = (0.0, 0.0, 0.0, 0usize);
        let (mut grav_lift, mut grav_kwt, mut grav_ride) = (0.0f64, 0.0f64, 0.0f64);
        let mut steps = Vec::new();
        let mut rail = false;
        let mut track_len = 0.0f64;
        let (mut wheel_z0, mut wheel_z1) = (f64::MAX, f64::MIN);
        let mut stance = 0.0f64;
        let mut sensor_best: Option<(f64, f64)> = None; // (eye height, range)
        let mut weapons = Vec::new();

        for (i, (id, x)) in asm.parts.iter().enumerate() {
            let part = &self.parts[id];
            let f = &part.def.function;
            s.power_kw += f.power_kw;
            s.draw_kw += f.draw_kw;
            s.load_kg += f.load_kg;
            contact += f.contact_m2;
            disc_area += std::f64::consts::PI * f.rotor_radius_m * f.rotor_radius_m;
            if f.cushion_area_m2 > 0.0 {
                cu_area += f.cushion_area_m2;
                cu_perim += f.cushion_perimeter_m;
                cu_gap += f.cushion_gap_m;
                cu_n += 1;
            }
            if f.grav_kw_per_t > 0.0 {
                grav_lift += f.load_kg;
                grav_kwt += f.grav_kw_per_t * f.load_kg;
                grav_ride = grav_ride.max(f.ride_height_m.unwrap_or(0.0));
            }
            if let Some(l) = f.locomotion {
                if !s.locomotion.contains(&l) {
                    s.locomotion.push(l);
                }
                let default_rolling = match l {
                    Locomotion::Wheels => 0.03,
                    Locomotion::Tracks | Locomotion::HalfTracks => 0.06,
                    Locomotion::Legs => 0.6,
                    Locomotion::Rail => 0.0015,
                    Locomotion::Hover => 0.03,
                    Locomotion::AntiGrav | Locomotion::Rotor | Locomotion::Jet => 0.0,
                };
                rolling.push(f.rolling.unwrap_or(default_rolling));
                efficiency.push(assemble::drive_efficiency(l));
                gear_limit = gear_limit.min(f.max_kmh.unwrap_or(f64::INFINITY));
                rail |= f.rail_bound || l == Locomotion::Rail;
                if f.step_m > 0.0 {
                    steps.push(f.step_m);
                }
                if matches!(l, Locomotion::Tracks | Locomotion::HalfTracks) {
                    track_len = track_len.max(f.contact_len_m);
                }
                if matches!(l, Locomotion::Wheels) {
                    let z = x.point(V3::ZERO).z;
                    wheel_z0 = wheel_z0.min(z);
                    wheel_z1 = wheel_z1.max(z);
                }
                if matches!(l, Locomotion::Legs) {
                    stance = stance.max(f.ride_height_m.unwrap_or(0.0));
                }
            }
            if let Some(w) = &f.weapon {
                weapons.push((id.clone(), w.clone()));
            }
            if let Some(se) = &f.sensor {
                let top = asm.pieces.iter().filter(|p| p.part as usize == i).map(|p| p.poly.aabb().1.y).fold(f64::MIN, f64::max);
                let eye = if top > f64::MIN { top } else { x.t.y + se.height_m };
                if sensor_best.map(|(_, r)| se.range_m > r).unwrap_or(true) {
                    sensor_best = Some((eye, se.range_m));
                }
            }
            s.repair_kg_s += f.repair_kg_s;
            s.repair_reach_m = s.repair_reach_m.max(f.repair_reach_m);
        }
        s.power_to_weight_kw_t = if s.mass_kg > 0.0 { s.power_kw / (s.mass_kg / 1000.0) } else { 0.0 };
        s.ground_pressure_kpa = if contact > 0.0 { weight_n / contact / 1000.0 } else { 0.0 };
        s.step_m = steps.iter().cloned().fold(f64::INFINITY, f64::min);
        if !s.step_m.is_finite() {
            s.step_m = 0.0;
        }
        s.movement = if rail {
            "rail"
        } else if s.locomotion.iter().any(|l| matches!(l, Locomotion::Rotor | Locomotion::Jet | Locomotion::AntiGrav)) {
            "air"
        } else if s.locomotion.contains(&Locomotion::Hover) {
            "hover"
        } else {
            "ground"
        }
        .into();

        // --- Checks that make a design invalid.
        if s.locomotion.is_empty() {
            s.problems.push("no locomotion".into());
        }
        if s.power_kw <= 0.0 {
            s.problems.push("no engine".into());
        }
        if s.load_kg > 0.0 && s.mass_kg > s.load_kg {
            s.problems.push(format!("overloaded: {:.0} kg on running gear rated {:.0} kg", s.mass_kg, s.load_kg));
        }
        if s.draw_kw > s.power_kw {
            s.problems.push(format!("power deficit: draws {:.0} kW of {:.0} kW", s.draw_kw, s.power_kw));
        }
        for pl in &asm.placements {
            let Some(parent) = pl.parent else { continue };
            let part = &self.parts[&asm.parts[pl.part_index].0];
            let parent_def = &self.parts[&asm.parts[parent].0].def;
            let Some(sock) = parent_def.sockets.iter().find(|k| k.name == pl.socket) else { continue };
            let f = &part.def.function;
            if let Some(&max) = sock.hints.get("ctx.ring_max") {
                if f.ring_m > max * 1.001 {
                    s.problems.push(format!("turret ring {:.2} m is wider than the {:.2} m ring on '{}'", f.ring_m, max, pl.socket));
                }
            }
            if part.def.category == Category::Engine {
                let (lo, hi) = voxel::bounds(&part.pieces);
                let d = hi - lo;
                let fits = |key: &str, v: f64| sock.hints.get(key).map(|&b| v <= b * 1.001).unwrap_or(true);
                if !(fits("ctx.bay_width", d.x) && fits("ctx.bay_height", d.y) && fits("ctx.bay_length", d.z)) {
                    s.problems.push(format!(
                        "engine {:.1} x {:.1} x {:.1} m does not fit the {:.1} x {:.1} x {:.1} m bay",
                        d.x,
                        d.y,
                        d.z,
                        sock.hints.get("ctx.bay_width").unwrap_or(&0.0),
                        sock.hints.get("ctx.bay_height").unwrap_or(&0.0),
                        sock.hints.get("ctx.bay_length").unwrap_or(&0.0)
                    ));
                }
            }
        }

        // --- Lift: rotors, air cushion and anti-gravity all spend power just to stay up.
        let spare0 = (s.power_kw - s.draw_kw).max(0.0);
        let mut lift_kw = 0.0;
        if disc_area > 0.0 {
            let kw = assemble::hover_power_w(weight_n, disc_area) / 1000.0;
            if kw > spare0 {
                s.problems.push(format!("cannot hover: rotors need {kw:.1} kW, {spare0:.1} kW available"));
            }
            lift_kw += kw;
        }
        if cu_area > 0.0 {
            let p = weight_n / cu_area;
            let q = cu_perim * (cu_gap / cu_n as f64) * (2.0 * p / RHO_AIR).sqrt();
            let kw = p * q / assemble::FAN_EFFICIENCY / 1000.0;
            if kw > spare0 {
                s.problems.push(format!("cannot float: the air cushion needs {kw:.1} kW, {spare0:.1} kW available"));
            }
            lift_kw += kw;
        }
        if grav_lift > 0.0 {
            let kw = (grav_kwt / grav_lift) * (inp.mass_kg / 1000.0) * (1.0 + grav_ride / 25.0);
            if kw > spare0 {
                s.problems.push(format!("cannot rise: anti-gravity needs {kw:.0} kW, {spare0:.0} kW available"));
            }
            lift_kw += kw;
        }
        s.lift_kw = lift_kw;
        let spare = (spare0 - lift_kw).max(0.0);

        // --- Speed: the power balance P = C m g v + 1/2 rho Cd A v^3, capped by what the running gear allows.
        let net_kw = spare * mean(&efficiency);
        let c1 = mean(&rolling) * weight_n;
        let c3 = 0.5 * RHO_AIR * 0.9 * s.frontal_m2;
        let by_power = if s.locomotion.is_empty() { 0.0 } else { power_balance_speed(net_kw * 1000.0, c1, c3) * 3.6 };
        if by_power > gear_limit {
            s.top_speed_kmh = gear_limit;
            s.speed_limited_by = "running gear".into();
        } else {
            s.top_speed_kmh = by_power;
            s.speed_limited_by = "power".into();
        }
        let v_ms = s.top_speed_kmh / 3.6;

        // --- Turning, by type of running gear.
        let len = s.length_m.max(1.0);
        s.turn_rate_dps = match s.locomotion.first() {
            // Skid steering: turning on the spot drags the whole contact patch sideways, a resisting moment
            // of mu W L / 4 that the drive must overcome (docs/design/04-parametric-components.md).
            Some(Locomotion::Tracks | Locomotion::HalfTracks) => {
                let l = if track_len > 0.0 { track_len } else { 0.8 * len };
                let m_r = 0.6 * weight_n * l / 4.0;
                ((0.6 * net_kw * 1000.0) / m_r.max(1.0)).to_degrees().min(120.0)
            }
            Some(Locomotion::Wheels) => {
                let wheelbase = if wheel_z1 > wheel_z0 { wheel_z1 - wheel_z0 } else { 0.5 * len };
                (v_ms.min(5.5) / (1.9 * wheelbase.max(0.5))).to_degrees().min(90.0)
            }
            Some(Locomotion::Legs) => (v_ms.min(6.0) / (1.5 * stance.max(0.5))).to_degrees().min(90.0),
            Some(Locomotion::Hover) => (45.0 * (5.0 / len).sqrt()).min(90.0),
            Some(Locomotion::AntiGrav) => (40.0 * (5.0 / len).sqrt()).min(90.0),
            Some(Locomotion::Rotor | Locomotion::Jet) => (120.0 * (3.0 / len).sqrt()).min(180.0),
            Some(Locomotion::Rail) | None => 0.0,
        };

        // --- Weapons.
        let burst: f64 = weapons.iter().map(|(_, w)| w.burst_kw).sum();
        let duty = if burst > 0.0 { (spare / burst).min(1.0) } else { 1.0 };
        let mut max_recoil = 0.0f64;
        for (id, w) in &weapons {
            let salvo = if w.salvo > 0.0 { w.salvo } else { 1.0 };
            let line = WeaponLine {
                name: self.parts[id].def.name.clone(),
                kind: w.kind.clone(),
                energy_mj: w.energy_j / 1e6,
                shots_per_min: w.shots_per_min,
                penetration_mm: w.penetration_mm,
                range_km: w.range_m / 1000.0,
            };
            let d = if w.kind == "beam" { duty } else { 1.0 };
            s.firepower_kw += w.energy_j * w.shots_per_min / 60.0 / 1000.0 * d;
            s.alpha_mj += w.energy_j * salvo / 1e6;
            s.best_pen_mm = s.best_pen_mm.max(w.penetration_mm);
            s.range_km = s.range_km.max(w.range_m / 1000.0);
            max_recoil = max_recoil.max(w.recoil_ns);
            s.weapons.push(line);
        }
        s.recoil_mps = if s.mass_kg > 0.0 { max_recoil / s.mass_kg } else { 0.0 };

        // --- Sight: the horizon grows with the height of the sensor.
        let top_y = inp.hi.y.max(1.0);
        let (eye, range) = match sensor_best {
            Some((h, r)) if r > BASE_SIGHT_M => (h.max(top_y), r),
            _ => (top_y, BASE_SIGHT_M),
        };
        s.eye_height_m = eye;
        s.sight_km = range.min(horizon_m(eye)) / 1000.0;

        // --- Legal, but worse.
        if s.recoil_mps > 0.5 {
            s.warnings.push(format!("recoil shoves it back at {:.1} m/s", s.recoil_mps));
        }
        if s.ground_pressure_kpa > 100.0 {
            s.warnings.push(format!("bogs down in soft ground ({:.0} kPa)", s.ground_pressure_kpa));
        }
        if rail {
            s.warnings.push("rail-bound: only goes where rails go".into());
        }
        if s.range_km > s.sight_km * 1.05 && s.range_km > 0.0 {
            s.warnings.push(format!("outranges its own sight ({:.1} km against {:.1} km): needs spotters", s.range_km, s.sight_km));
        }
        s
    }
}
