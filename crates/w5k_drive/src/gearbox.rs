//! The gearbox: ratios as a lever, an automatic shift map with hysteresis, and the torque interruption of a shift.
//!
//! Principle: a gear pair multiplies torque by the ratio and divides speed by it (power in = power out, less the mesh losses). The shift
//! map is a Schmitt trigger: the upshift and downshift points are different, and a shift is refused if it would land next to the opposite
//! point, so the box cannot hunt between two gears.

use serde::{Deserialize, Serialize};
use w5k_contract::command::GearRequest;
use w5k_contract::rig::GearboxDef;
use w5k_contract::Param;
use w5k_math::scalar::{exp, lerp};
use w5k_math::StateHasher;

use crate::coupling::Downstream;

/// rad/s per rpm.
const RPM_TO_RAD_S: f64 = core::f64::consts::PI / 30.0; // const-ok: unit conversion, mathematical

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShiftTuning {
    pub light_throttle_shift_scale: Param,
    pub kickdown_throttle: Param,
    pub hysteresis_margin_rpm: Param,
    pub min_gear_dwell_s: Param,
    /// Time constant of the pedal filter the shift map reads, s: a blip of the pedal does not move the shift points or trigger a kick-down.
    pub pedal_filter_s: Param,
    /// How much the pedal moves the downshift point (0 = it stays at the light-throttle value, 1 = it follows the pedal like the upshift point).
    /// Kept low so a pedal swing at one road speed can never turn an upshift into a downshift: that is what makes a box hunt.
    pub downshift_pedal_influence: Param,
    /// Smoothed brake pedal above which the automatic holds or drops a gear for engine braking (a long descent), 0..1.
    pub engine_brake_pedal: Param,
    /// Time constant of the brake-pedal filter, s: only sustained braking (a descent) asks for engine braking, not a stab into a corner.
    pub engine_brake_filter_s: Param,
    /// An upshift is refused unless the engine could pull the road load in the higher gear with this much to spare (a margin over 1): a truck
    /// on a steep hill stays in the gear that can climb it instead of shifting up and rolling back.
    pub upshift_pull_margin: Param,
    /// Below this road speed the pull margin applies (a torque interruption can stall or roll back a slow, loaded truck); above it an
    /// upshift only needs the higher gear to hold the load, m/s.
    pub hill_hold_speed_m_s: Param,
    /// A downshift for engine braking may land up to this fraction of the full-throttle upshift point.
    pub engine_brake_limit_fraction: Param,
    pub reverse_engage_max_speed_m_s: Param,
    /// The upshift point is capped at this fraction of the engine's redline, so a schedule written for another engine cannot pin the box.
    pub upshift_max_fraction_of_redline: Param,
}

impl ShiftTuning {
    pub fn check(&self) -> Result<(), String> {
        self.light_throttle_shift_scale.check("light_throttle_shift_scale")?;
        self.kickdown_throttle.check("kickdown_throttle")?;
        self.hysteresis_margin_rpm.check("hysteresis_margin_rpm")?;
        self.min_gear_dwell_s.check("min_gear_dwell_s")?;
        self.pedal_filter_s.check("pedal_filter_s")?;
        self.downshift_pedal_influence.check("downshift_pedal_influence")?;
        self.engine_brake_pedal.check("engine_brake_pedal")?;
        self.engine_brake_filter_s.check("engine_brake_filter_s")?;
        self.upshift_pull_margin.check("upshift_pull_margin")?;
        self.hill_hold_speed_m_s.check("hill_hold_speed_m_s")?;
        self.engine_brake_limit_fraction.check("engine_brake_limit_fraction")?;
        self.reverse_engage_max_speed_m_s.check("reverse_engage_max_speed_m_s")?;
        self.upshift_max_fraction_of_redline.check("upshift_max_fraction_of_redline")
    }
}

/// Road speed for an engine speed through a total reduction `ratio` (engine / wheel) and a rolling radius: `v = w r / ratio`.
pub fn road_speed_m_s(engine_omega_rad_s: f64, total_ratio: f64, wheel_radius_m: f64) -> f64 {
    engine_omega_rad_s * wheel_radius_m / total_ratio
}

/// What the gearbox tells the rest of the powertrain this step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShiftOut {
    /// 0 neutral, positive forward, negative reverse.
    pub gear: i8,
    pub shifting: bool,
    /// Multiplies the coupling's capacity: 0 while the torque is interrupted, else 1.
    pub capacity_scale: f64,
}

#[derive(Clone, Debug)]
pub struct Gearbox {
    forward: Vec<f64>,
    reverse: Vec<f64>,
    efficiency: f64,
    inertia: f64,
    up_rpm: f64,
    down_rpm: f64,
    shift_time_s: f64,
    light_scale: f64,
    kickdown: f64,
    margin_rpm: f64,
    dwell_min_s: f64,
    pedal_tau_s: f64,
    down_influence: f64,
    brake_pedal_min: f64,
    brake_tau_s: f64,
    pull_margin: f64,
    hill_speed: f64,
    road_speed: f64,
    /// The engine's full-load torque curve `(rpm, N m)`, if the powertrain gave it (enables the hill check on upshifts).
    curve: Vec<(f64, f64)>,
    load_nm: f64,
    brake_limit: f64,
    brake: f64,
    pedal: f64,
    pedal_seeded: bool,
    reverse_max_speed: f64,
    gear: i8,
    pending: Option<i8>,
    shift_left_s: f64,
    dwell_s: f64,
    driving: bool,
}

impl Gearbox {
    pub fn new(def: &GearboxDef, redline_rpm: f64, tuning: &ShiftTuning) -> Result<Gearbox, String> {
        tuning.check()?;
        if def.forward_ratios.is_empty()
            || def.forward_ratios.iter().chain(&def.reverse_ratios).any(|r| r.is_nan() || *r <= 0.0)
        {
            return Err("gearbox needs at least one forward ratio and all ratios > 0".into());
        }
        if def.forward_ratios.windows(2).any(|p| p[1] >= p[0]) {
            return Err("forward ratios must fall from first gear to top".into());
        }
        if !(def.efficiency > 0.0 && def.efficiency <= 1.0 && def.inertia_kg_m2 > 0.0) {
            return Err("gearbox efficiency must be in (0, 1] and inertia > 0".into());
        }
        if def.forward_ratios.len() > 100 {
            return Err("gearbox: more than 100 forward gears".into()); // const-ok: gear index must fit i8 with reverse
        }
        let s = &def.shift;
        Ok(Gearbox {
            forward: def.forward_ratios.clone(),
            reverse: def.reverse_ratios.clone(),
            efficiency: def.efficiency,
            inertia: def.inertia_kg_m2,
            // the schedule is data per vehicle, but an upshift point the engine can never reach would pin the box in a low gear
            up_rpm: s.upshift_rpm.min(tuning.upshift_max_fraction_of_redline.v * redline_rpm),
            down_rpm: s.downshift_rpm,
            shift_time_s: s.shift_time_s.max(0.0),
            light_scale: tuning.light_throttle_shift_scale.v,
            kickdown: tuning.kickdown_throttle.v,
            margin_rpm: tuning.hysteresis_margin_rpm.v,
            dwell_min_s: tuning.min_gear_dwell_s.v,
            pedal_tau_s: tuning.pedal_filter_s.v,
            down_influence: tuning.downshift_pedal_influence.v,
            brake_pedal_min: tuning.engine_brake_pedal.v,
            brake_tau_s: tuning.engine_brake_filter_s.v,
            pull_margin: tuning.upshift_pull_margin.v,
            hill_speed: tuning.hill_hold_speed_m_s.v,
            road_speed: 0.0,
            curve: Vec::new(),
            load_nm: 0.0,
            brake_limit: tuning.engine_brake_limit_fraction.v,
            brake: 0.0,
            pedal: 0.0,
            pedal_seeded: false,
            reverse_max_speed: tuning.reverse_engage_max_speed_m_s.v,
            gear: if def.forward_ratios.is_empty() { 0 } else { 1 },
            pending: None,
            shift_left_s: 0.0,
            dwell_s: tuning.min_gear_dwell_s.v,
            driving: true,
        })
    }

    /// Tell the shift logic where the brake pedal is (smoothed like the throttle). A driver who is braking on a descent wants engine braking:
    /// the automatic then refuses upshifts and may drop a gear, as grade-braking logic does.
    pub fn note_brake(&mut self, dt: f64, brake: f64) {
        self.brake += (brake.clamp(0.0, 1.0) - self.brake) * (1.0 - exp(-dt / self.brake_tau_s));
    }

    /// Give the gearbox the engine's full-load curve so it can tell whether the engine could pull the load in a higher gear.
    pub fn with_engine_curve(mut self, curve: &[(f64, f64)]) -> Gearbox {
        self.curve = curve.to_vec();
        self
    }

    /// Tell the shift logic the load at the gearbox output now (road load, grade, drag: positive resists), N m.
    pub fn note_load(&mut self, load_nm: f64) {
        self.load_nm = load_nm;
    }

    /// Could the engine pull the present load in `gear` at this output speed, with the margin to spare? True when the load helps, or when
    /// no curve was given.
    fn can_pull(&self, gear: i8, w_out: f64) -> bool {
        if self.curve.len() < 2 || self.load_nm <= 0.0 {
            return true;
        }
        let rpm = self.input_rpm(gear, w_out).max(self.curve[0].0);
        let tq = match self.curve.windows(2).find(|p| rpm <= p[1].0) {
            Some(p) => lerp(p[0].1, p[1].1, (rpm - p[0].0) / (p[1].0 - p[0].0)),
            None => self.curve[self.curve.len() - 1].1,
        };
        let margin = if self.road_speed < self.hill_speed { self.pull_margin } else { 1.0 };
        tq * self.efficiency * self.ratio_of(gear) >= margin * self.load_nm
    }

    pub fn gear(&self) -> i8 {
        self.gear
    }

    fn ratio_of(&self, gear: i8) -> f64 {
        match gear {
            0 => 1.0,
            g if g > 0 => self.forward[usize::from(g.unsigned_abs() - 1)],
            g => self
                .reverse
                .get(usize::from(g.unsigned_abs() - 1))
                .copied()
                .unwrap_or_else(|| self.reverse.last().copied().unwrap_or(1.0)),
        }
    }

    /// Signed reduction from engine side to output: negative in reverse (the output turns the other way). 1 in neutral (unused).
    pub fn signed_ratio(&self) -> f64 {
        let r = self.ratio_of(self.gear);
        if self.gear < 0 {
            -r
        } else {
            r
        }
    }

    /// Decide gear changes and report the torque interruption. The automatic map runs on the gearbox *input* speed in each candidate gear
    /// (output speed x ratio), not on engine rpm: that is exact for the gear we would land in, and does not wobble with converter slip.
    pub fn update(
        &mut self,
        dt: f64,
        request: GearRequest,
        throttle: f64,
        output_omega_rad_s: f64,
        road_speed_m_s: f64,
    ) -> ShiftOut {
        self.dwell_s += dt;
        self.road_speed = road_speed_m_s.abs();
        // the shift map reads a smoothed pedal: a driver who wobbles around a cruise must not wobble the gearbox
        if self.pedal_seeded {
            self.pedal += (throttle.clamp(0.0, 1.0) - self.pedal) * (1.0 - exp(-dt / self.pedal_tau_s));
        } else {
            self.pedal = throttle.clamp(0.0, 1.0);
            self.pedal_seeded = true;
        }
        if self.shift_left_s > 0.0 {
            self.shift_left_s -= dt;
            if self.shift_left_s <= 0.0 {
                self.shift_left_s = 0.0;
                if let Some(g) = self.pending.take() {
                    self.gear = g;
                    self.dwell_s = 0.0;
                }
            }
        } else if let Some(target) = self.choose(request, self.pedal, output_omega_rad_s, road_speed_m_s) {
            if target != self.gear {
                self.pending = Some(target);
                self.shift_left_s = self.shift_time_s;
                if self.shift_left_s <= 0.0 {
                    self.gear = target;
                    self.pending = None;
                    self.dwell_s = 0.0;
                }
            }
        }
        let shifting = self.shift_left_s > 0.0 || self.gear == 0;
        ShiftOut {
            gear: self.gear,
            shifting: self.shift_left_s > 0.0,
            capacity_scale: if shifting { 0.0 } else { 1.0 },
        }
    }

    fn choose(&self, request: GearRequest, throttle: f64, w_out: f64, speed: f64) -> Option<i8> {
        let top = i8::try_from(self.forward.len()).unwrap_or(i8::MAX);
        let slow = speed.abs() < self.reverse_max_speed;
        match request {
            GearRequest::Neutral => Some(0),
            GearRequest::Reverse => (slow && !self.reverse.is_empty()).then_some(-1),
            GearRequest::Hold => None,
            GearRequest::Up => (self.gear >= 0 && self.gear < top).then(|| self.gear + 1),
            GearRequest::Down => (self.gear > 1).then(|| self.gear - 1),
            GearRequest::Gear(n) => {
                (n >= 1 && (self.gear > 0 || slow)).then(|| i8::try_from(n).unwrap_or(top).min(top))
            }
            GearRequest::Auto => {
                if self.gear <= 0 {
                    return slow.then_some(1);
                }
                // a manual box under `Auto` is shifted by the driver model: the same schedule, with the clutch opened for the shift time
                if self.dwell_s < self.dwell_min_s {
                    return None;
                }
                let g = self.gear;
                let braking = self.brake > self.brake_pedal_min;
                if !braking && self.wants_upshift(g, w_out, throttle) {
                    Some(g + 1)
                } else if self.wants_downshift(g, w_out, throttle, true) {
                    Some(g - 1)
                } else if braking && g > 1 && self.input_rpm(g - 1, w_out) <= self.brake_limit * self.up_rpm {
                    // braking on a descent: a lower gear takes some of the load off the brakes
                    Some(g - 1)
                } else {
                    None
                }
            }
        }
    }

    /// Upshift and downshift points at a pedal position, as gearbox input rpm. Both rise with the pedal, but the downshift point only by
    /// `downshift_pedal_influence`, so the downshift curve stays under the upshift curve at every pedal position.
    fn shift_points(&self, pedal: f64) -> (f64, f64) {
        let p = pedal.clamp(0.0, 1.0);
        (
            self.up_rpm * lerp(self.light_scale, 1.0, p),
            self.down_rpm * lerp(self.light_scale, 1.0, self.down_influence * p),
        )
    }

    fn input_rpm(&self, gear: i8, w_out: f64) -> f64 {
        w_out.abs() / RPM_TO_RAD_S * self.ratio_of(gear)
    }

    /// Would the automatic go from `gear` to the next one up at this output speed and pedal (ignoring the dwell timer)?
    pub fn wants_upshift(&self, gear: i8, w_out: f64, pedal: f64) -> bool {
        let top = i8::try_from(self.forward.len()).unwrap_or(i8::MAX);
        let (up, down) = self.shift_points(pedal);
        gear >= 1
            && gear < top
            && self.input_rpm(gear, w_out) > up
            && self.input_rpm(gear + 1, w_out) >= down + self.margin_rpm
            && self.can_pull(gear + 1, w_out)
    }

    /// Would it drop a gear? Below the downshift point, or (with `kickdown`) on a pedal past the kick-down threshold; either way the
    /// lower gear must sit clear of the upshift point at this pedal, which is what stops a downshift being undone by the next upshift.
    pub fn wants_downshift(&self, gear: i8, w_out: f64, pedal: f64, kickdown: bool) -> bool {
        let (up, down) = self.shift_points(pedal);
        gear > 1
            && self.input_rpm(gear - 1, w_out) <= up - self.margin_rpm
            && (self.input_rpm(gear, w_out) < down || (kickdown && pedal > self.kickdown))
    }

    /// The driven side as the coupling sees it (everything reflected through the ratio to the gearbox input).
    pub fn reflect(&self, out: &Downstream) -> Downstream {
        let r = self.signed_ratio();
        let k = if self.driving { 1.0 / self.efficiency } else { self.efficiency };
        Downstream {
            omega_rad_s: r * out.omega_rad_s,
            inertia_kg_m2: out.inertia_kg_m2 / (r * r) + self.inertia,
            ext_torque_nm: out.ext_torque_nm / r * k,
            ext_slope_nm_s_rad: out.ext_slope_nm_s_rad / (r * r),
        }
    }

    /// Torque at the output for a torque `t_in` delivered to the gearbox input: times the ratio, less the mesh loss in the direction of flow.
    pub fn output_torque(&mut self, t_in: f64) -> f64 {
        self.driving = t_in >= 0.0;
        let k = if self.driving { self.efficiency } else { 1.0 / self.efficiency };
        t_in * self.signed_ratio() * k
    }

    pub fn hash_state(&self, h: &mut StateHasher) {
        h.write_u8(self.gear.to_le_bytes()[0]);
        h.write_f64(self.shift_left_s);
        h.write_f64(self.dwell_s);
        h.write_f64(self.pedal);
        h.write_f64(self.brake);
        h.write_u8(u8::from(self.driving));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::shift_tuning;
    use w5k_contract::rig::ShiftDef;

    fn def() -> GearboxDef {
        GearboxDef {
            forward_ratios: vec![2.48, 1.48, 1.0, 0.73],
            reverse_ratios: vec![2.0],
            efficiency: 0.95,
            inertia_kg_m2: 0.05,
            shift: ShiftDef { automatic: true, upshift_rpm: 3200.0, downshift_rpm: 1500.0, shift_time_s: 0.3 },
        }
    }

    fn gb() -> Gearbox {
        Gearbox::new(&def(), 4000.0, &shift_tuning()).unwrap()
    }

    #[test]
    fn speed_in_each_gear_at_redline_matches_the_ratio_arithmetic() {
        let (w, radius, final_drive) = (4000.0 * core::f64::consts::PI / 30.0, 0.4, 4.1);
        let mut g = gb();
        for (i, r) in [2.48, 1.48, 1.0, 0.73].into_iter().enumerate() {
            g.gear = i8::try_from(i + 1).unwrap();
            let v = road_speed_m_s(w, g.signed_ratio() * final_drive, radius);
            assert!((v - w * radius / (r * final_drive)).abs() < 1e-12, "gear {}", i + 1);
        }
        // and it is a lever: output speed is input speed over the ratio, output torque is input torque times it (less 5% loss)
        let mut g = gb();
        let out = Downstream { omega_rad_s: 100.0, inertia_kg_m2: 10.0, ..Default::default() };
        assert!((g.reflect(&out).omega_rad_s - 248.0).abs() < 1e-9);
        assert!((g.output_torque(100.0) - 100.0 * 2.48 * 0.95).abs() < 1e-9);
    }

    const RPM: f64 = core::f64::consts::PI / 30.0;

    #[test]
    fn automatic_upshifts_at_the_stated_rpm_and_downshifts_with_hysteresis() {
        let dt = 1.0 / 120.0;
        let (mut g, ratios) = (gb(), [2.48, 1.48, 1.0, 0.73]);
        let scale = 0.45 + (1.0 - 0.45) * 0.8; // pedal 0.8 is below kick-down: the points scale with the pedal
        let down_scale = 0.45 + (1.0 - 0.45) * 0.3 * 0.8; // the downshift point follows the pedal only by the stated influence (0.3)
        let (up, down) = (3200.0 * scale, 1500.0 * down_scale);
        let mut shifts: Vec<(f64, i8)> = Vec::new();
        let mut last = g.gear();
        // the road (output) speed in rpm rises to 1300 over 10 s and falls back; the gearbox input follows through the current ratio
        for k in 0..2400 {
            let t = f64::from(k) * dt;
            let out_rpm = if t < 10.0 { 130.0 * t } else { 1300.0 - 130.0 * (t - 10.0) };
            let o = g.update(dt, GearRequest::Auto, 0.8, out_rpm * RPM, 5.0);
            if o.gear != last {
                shifts.push((out_rpm, o.gear));
                last = o.gear;
            }
        }
        let _ = ratios;
        assert_eq!(shifts.len(), 2, "{shifts:?}");
        // 1 -> 2 when the input passes the upshift point (taking effect one 0.3 s interruption later)
        assert_eq!(shifts[0].1, 2);
        let up_rpm = shifts[0].0 * 2.48;
        assert!(up_rpm > up && up_rpm < up + 130.0 * 2.48 * 0.3 + 5.0, "upshift at {up_rpm} rpm, stated {up}");
        // 2 -> 1 below the downshift point: at a lower road speed than the upshift (the hysteresis)
        assert_eq!(shifts[1].1, 1);
        let down_rpm = shifts[1].0 * 1.48;
        assert!(
            down_rpm < down && down_rpm > down - 130.0 * 1.48 * 0.3 - 5.0,
            "downshift at {down_rpm} rpm, stated {down}"
        );
        assert!(shifts[1].0 < shifts[0].0);
    }

    #[test]
    fn shift_interrupts_torque_for_the_stated_time() {
        let dt = 1.0 / 1000.0;
        let mut g = gb();
        let mut off = 0.0;
        for _ in 0..200 {
            g.update(dt, GearRequest::Auto, 1.0, 100.0 * RPM, 5.0);
        }
        for _ in 0..900 {
            let o = g.update(dt, GearRequest::Auto, 1.0, 1400.0 * RPM, 5.0); // first-gear input 3472 rpm, above the 3200 upshift point
            if o.capacity_scale == 0.0 {
                off += dt;
            }
        }
        assert!((off - 0.3).abs() < 2.0 * dt, "interrupted for {off} s");
        assert_eq!(g.gear(), 2);
    }

    #[test]
    fn reverse_gears_work() {
        let mut g = gb();
        // refused while rolling, accepted when nearly stopped
        g.update(0.5, GearRequest::Reverse, 0.0, 800.0, 8.0);
        for _ in 0..100 {
            g.update(0.01, GearRequest::Reverse, 0.0, 800.0, 8.0);
        }
        assert_eq!(g.gear(), 1);
        for _ in 0..100 {
            g.update(0.01, GearRequest::Reverse, 0.0, 800.0, 0.2);
        }
        assert_eq!(g.gear(), -1);
        assert!(g.signed_ratio() < 0.0 && (g.signed_ratio() + 2.0).abs() < 1e-12);
        let drive = g.output_torque(100.0);
        assert!(drive < 0.0 && (drive + 100.0 * 2.0 * 0.95).abs() < 1e-9, "reverse drives backwards");
        let out = Downstream { omega_rad_s: -10.0, inertia_kg_m2: 4.0, ..Default::default() };
        assert!((g.reflect(&out).omega_rad_s - 20.0).abs() < 1e-9, "backwards output turns the input forwards");
    }

    #[test]
    fn the_automatic_does_not_hunt_when_cruising_between_two_gears() {
        let mut g = gb();
        g.gear = 2;
        let mut shifts = 0;
        let mut last = 2;
        for _ in 0..6000 {
            let o = g.update(0.01, GearRequest::Auto, 0.5, 2300.0 / 1.48 * RPM, 10.0);
            if o.gear != last {
                shifts += 1;
                last = o.gear;
            }
        }
        assert_eq!(shifts, 0);
    }

    /// A driver on a speed controller that is bang-bang around its target (lift when above, press when below) sits right on the line where
    /// a first-cut map flips: at a light pedal the upshift point is low, at a moderate pedal the downshift point is higher.
    fn bang_bang_shifts(target_kmh: f64, press: f64) -> usize {
        let mut def = def();
        def.forward_ratios = vec![2.48, 1.48, 1.0, 0.75];
        def.shift.upshift_rpm = 3500.0;
        def.shift.downshift_rpm = 1400.0;
        def.shift.shift_time_s = 0.35;
        let mut g = Gearbox::new(&def, 4000.0, &shift_tuning()).unwrap();
        g.gear = 2;
        let (dt, wheel_r, final_drive) = (0.01, 0.4, 5.13);
        let (mut v, mut shifts, mut last) = (target_kmh / 3.6, 0, 2);
        for _ in 0..12000 {
            let pedal = if v > target_kmh / 3.6 { 0.0 } else { press };
            v += dt * if pedal > 0.0 { 0.25 } else { -0.15 }; // gentle acceleration under power, coasting down without
            let o = g.update(dt, GearRequest::Auto, pedal, v / wheel_r * final_drive, v);
            if o.gear != last {
                shifts += 1;
                last = o.gear;
            }
        }
        shifts
    }

    #[test]
    fn a_bang_bang_speed_driver_does_not_make_the_box_hunt() {
        for target in [28.0, 30.0, 31.0, 32.0, 34.0, 36.0, 40.0] {
            let n = bang_bang_shifts(target, 0.5);
            assert!(n <= 1, "{n} shifts in 120 s at {target} km/h");
        }
    }

    #[test]
    fn no_road_speed_can_trigger_an_upshift_at_one_pedal_and_a_downshift_at_another() {
        // the property that makes a box hunt when it fails: the downshift curve must sit under the upshift curve at every pair of pedals
        let mut def = def();
        def.forward_ratios = vec![2.48, 1.48, 1.0, 0.75];
        def.shift.upshift_rpm = 3500.0;
        def.shift.downshift_rpm = 1400.0;
        let g = Gearbox::new(&def, 4000.0, &shift_tuning()).unwrap();
        for gear in 1..=3_i8 {
            for k in 0..4000 {
                let w = f64::from(k) * 0.1; // output shaft speed, 0 to 400 rad/s
                for pa in (0..=9).map(|i| f64::from(i) * 0.1) {
                    if g.wants_upshift(gear, w, pa) {
                        for pb in (0..=9).map(|i| f64::from(i) * 0.1) {
                            assert!(
                                !g.wants_downshift(gear + 1, w, pb, false),
                                "gear {gear}->{} at {w} rad/s, pedals {pa} then {pb}",
                                gear + 1
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_short_pedal_blip_does_not_kick_down() {
        let mut def = def();
        def.forward_ratios = vec![2.48, 1.48, 1.0, 0.75];
        let mut g = Gearbox::new(&def, 4000.0, &shift_tuning()).unwrap();
        g.gear = 3;
        let w_out = 1300.0 * RPM; // input 1300 rpm in third: comfortably above the downshift point
        for _ in 0..500 {
            g.update(0.01, GearRequest::Auto, 0.3, w_out, 9.0);
        }
        let mut changes = 0;
        for k in 0..800 {
            let pedal = if (100..140).contains(&k) { 1.0 } else { 0.3 }; // a 0.4 s stab at full throttle
            changes += usize::from(g.update(0.01, GearRequest::Auto, pedal, w_out, 9.0).gear != 3);
        }
        assert_eq!(changes, 0, "a 0.4 s blip must not downshift");
        // a held full pedal does kick down
        for _ in 0..400 {
            g.update(0.01, GearRequest::Auto, 1.0, w_out, 9.0);
        }
        assert_eq!(g.gear(), 2, "a held full pedal must kick down");
    }

    #[test]
    fn braking_down_a_grade_drops_a_gear_for_engine_braking_and_refuses_upshifts() {
        let mut def = def();
        def.forward_ratios = vec![2.48, 1.48, 1.0, 0.75];
        let mut g = Gearbox::new(&def, 4000.0, &shift_tuning()).unwrap();
        g.gear = 4;
        let w_out = 1280.0 * RPM / 0.75; // input 1280 rpm in fourth: 1707 in third, 2527 in second (just under 0.8 x the 3200 upshift point), 4233 in first
                                         // coasting with no brake: the box stays where it is
        for _ in 0..800 {
            g.note_brake(0.01, 0.0);
            g.update(0.01, GearRequest::Auto, 0.0, w_out, 12.0);
        }
        assert_eq!(g.gear(), 4);
        // brake held: it steps down (one gear per dwell) to where the engine can take it, and no further
        for _ in 0..2000 {
            g.note_brake(0.01, 0.6);
            g.update(0.01, GearRequest::Auto, 0.0, w_out, 12.0);
        }
        assert_eq!(g.gear(), 2, "should settle where the next gear down would exceed the engine-braking limit");
    }
}
