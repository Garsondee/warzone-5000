//! Between the player's pedals and the powertrain: the kid assists. All numbers are `Param`s in `content/physics/arch/drive_assist.ron`.
//!
//! Speed cap (throttle eases off towards the cap, a brake catches an overshoot), throttle and brake ramps, speed-dependent steering
//! authority and slew rate, always the automatic gearbox, a gentle brake when no pedal is down (and a hold brake near standstill, so the
//! torque converter cannot creep), and the three reasons to put the vehicle back on the road: rolled over, stuck, off the map.
//! With assists off the pedals pass straight through (adults).

use serde::Deserialize;
use w5k_contract::{DriveInputs, GearRequest, Param};
use w5k_math::scalar;

const PEDAL_EPS: f64 = 0.02; // const-ok: a pedal below 2 % counts as released (a key or a stick at rest)

/// What the player asks for (the body of `POST /api/input`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Raw {
    pub throttle: f64,
    pub brake: f64,
    /// -1 left .. +1 right.
    pub steer: f64,
    pub reverse: bool,
}

impl Raw {
    /// Everything clamped to its range (a page may send 1.2 or -0.1); non-finite numbers count as zero.
    pub(crate) fn clamped(self) -> Raw {
        let c = |v: f64, lo: f64| if v.is_finite() { scalar::clamp(v, lo, 1.0) } else { 0.0 };
        Raw {
            throttle: c(self.throttle, 0.0),
            brake: c(self.brake, 0.0),
            steer: c(self.steer, -1.0),
            reverse: self.reverse,
        }
    }
}

/// What the assists may look at.
pub(crate) struct Obs {
    pub speed_m_s: f64,
    /// Angle between the hull's up axis and world up, rad.
    pub tilt_rad: f64,
    /// Outside the playable terrain (or fallen through it).
    pub off_map: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AssistTuning {
    pub speed_cap_m_s: Param,
    pub cap_soft_start: Param,
    pub cap_brake_per_m_s: Param,
    pub throttle_rise_per_s: Param,
    pub throttle_fall_per_s: Param,
    pub brake_rise_per_s: Param,
    pub brake_fall_per_s: Param,
    pub steer_full_authority_m_s: Param,
    pub steer_min_authority: Param,
    pub steer_rate_slow_per_s: Param,
    pub steer_rate_fast_per_s: Param,
    pub idle_brake: Param,
    pub hold_brake: Param,
    pub hold_speed_m_s: Param,
    pub roll_limit_rad: Param,
    pub roll_dwell_s: Param,
    pub stuck_speed_m_s: Param,
    pub stuck_throttle: Param,
    pub stuck_dwell_s: Param,
    pub bounds_margin_m: Param,
    pub fall_depth_m: Param,
    /// Set by `--no-speed-limit`: the throttle is no longer eased and the brake no longer catches an overshoot at the cap.
    /// Steering keeps its speed-dependent authority (it is scheduled against the cap), so a fast vehicle stays gentle to steer.
    #[serde(skip)]
    pub no_speed_limit: bool,
}

impl AssistTuning {
    pub(crate) fn check(&self) -> Result<(), String> {
        let ps = [
            ("speed_cap_m_s", &self.speed_cap_m_s),
            ("cap_soft_start", &self.cap_soft_start),
            ("cap_brake_per_m_s", &self.cap_brake_per_m_s),
            ("throttle_rise_per_s", &self.throttle_rise_per_s),
            ("throttle_fall_per_s", &self.throttle_fall_per_s),
            ("brake_rise_per_s", &self.brake_rise_per_s),
            ("brake_fall_per_s", &self.brake_fall_per_s),
            ("steer_full_authority_m_s", &self.steer_full_authority_m_s),
            ("steer_min_authority", &self.steer_min_authority),
            ("steer_rate_slow_per_s", &self.steer_rate_slow_per_s),
            ("steer_rate_fast_per_s", &self.steer_rate_fast_per_s),
            ("idle_brake", &self.idle_brake),
            ("hold_brake", &self.hold_brake),
            ("hold_speed_m_s", &self.hold_speed_m_s),
            ("roll_limit_rad", &self.roll_limit_rad),
            ("roll_dwell_s", &self.roll_dwell_s),
            ("stuck_speed_m_s", &self.stuck_speed_m_s),
            ("stuck_throttle", &self.stuck_throttle),
            ("stuck_dwell_s", &self.stuck_dwell_s),
            ("bounds_margin_m", &self.bounds_margin_m),
            ("fall_depth_m", &self.fall_depth_m),
        ];
        ps.iter().try_for_each(|(n, p)| p.check(&format!("drive_assist.assist.{n}")))
    }

    /// How much of full lock the wheel may turn at `speed_m_s`: 1 at walking pace, `steer_min_authority` at the speed cap.
    pub(crate) fn steer_authority(&self, speed_m_s: f64) -> f64 {
        scalar::lerp(1.0, self.steer_min_authority.v, self.speed_fraction(speed_m_s))
    }

    /// How fast the wheel may move, fraction of full lock per second: quick when parking, slow at the cap.
    pub(crate) fn steer_rate(&self, speed_m_s: f64) -> f64 {
        scalar::lerp(self.steer_rate_slow_per_s.v, self.steer_rate_fast_per_s.v, self.speed_fraction(speed_m_s))
    }

    fn speed_fraction(&self, speed_m_s: f64) -> f64 {
        scalar::smoothstep(self.steer_full_authority_m_s.v, self.speed_cap_m_s.v, speed_m_s.abs())
    }
}

/// Move `x` towards `target` by at most `rise` (up) or `fall` (down), both already multiplied by the time step.
fn slew(x: f64, target: f64, rise: f64, fall: f64) -> f64 {
    if target > x {
        (x + rise).min(target)
    } else {
        (x - fall).max(target)
    }
}

/// Why the vehicle was put back on the road.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Recovery {
    RolledOver,
    Stuck,
    OffMap,
}

pub(crate) struct Assist {
    pub on: bool,
    /// Short status for the page: `""`, or which assist is acting.
    pub message: &'static str,
    throttle: f64,
    brake: f64,
    steer: f64,
    rolled_s: f64,
    stuck_s: f64,
}

impl Assist {
    pub(crate) fn new(on: bool) -> Assist {
        Assist { on, message: "", throttle: 0.0, brake: 0.0, steer: 0.0, rolled_s: 0.0, stuck_s: 0.0 }
    }

    /// Forget all state (after a recovery).
    pub(crate) fn reset(&mut self) {
        *self = Assist::new(self.on);
    }

    /// The powertrain inputs for this tick.
    pub(crate) fn apply(&mut self, raw: &Raw, obs: &Obs, t: &AssistTuning, dt_s: f64) -> DriveInputs {
        let raw = raw.clamped();
        let gear = if raw.reverse { GearRequest::Reverse } else { GearRequest::Auto };
        if !self.on {
            return DriveInputs {
                throttle: raw.throttle,
                brake: raw.brake,
                steer: raw.steer,
                gear,
                ..DriveInputs::default()
            };
        }
        let (speed, cap) = (obs.speed_m_s.abs(), t.speed_cap_m_s.v);
        let ease = if t.no_speed_limit { 1.0 } else { 1.0 - scalar::smoothstep(t.cap_soft_start.v * cap, cap, speed) };
        let over_cap = if t.no_speed_limit { 0.0 } else { (speed - cap) * t.cap_brake_per_m_s.v };
        let mut brake = raw.brake.max(scalar::clamp(over_cap, 0.0, 1.0));
        self.message = if raw.throttle > PEDAL_EPS && ease < 0.5 { "speed limit" } else { "" }; // const-ok: halfway through the easing
        if raw.throttle < PEDAL_EPS && raw.brake < PEDAL_EPS {
            brake = brake.max(if speed < t.hold_speed_m_s.v { t.hold_brake.v } else { t.idle_brake.v });
            self.message = "stopping";
        }
        self.throttle =
            slew(self.throttle, raw.throttle * ease, t.throttle_rise_per_s.v * dt_s, t.throttle_fall_per_s.v * dt_s);
        self.brake = slew(self.brake, brake, t.brake_rise_per_s.v * dt_s, t.brake_fall_per_s.v * dt_s);
        let wheel = raw.steer * t.steer_authority(speed);
        let step = t.steer_rate(speed) * dt_s;
        self.steer = slew(self.steer, wheel, step, step);
        DriveInputs { throttle: self.throttle, brake: self.brake, steer: self.steer, gear, ..DriveInputs::default() }
    }

    /// Time the three trouble conditions and say when one has lasted long enough. Call once per tick after the physics.
    pub(crate) fn watch(&mut self, raw: &Raw, obs: &Obs, t: &AssistTuning, dt_s: f64) -> Option<Recovery> {
        let timer = |s: &mut f64, on: bool| *s = if on { *s + dt_s } else { 0.0 };
        timer(&mut self.rolled_s, obs.tilt_rad > t.roll_limit_rad.v);
        timer(&mut self.stuck_s, obs.speed_m_s.abs() < t.stuck_speed_m_s.v && raw.throttle > t.stuck_throttle.v);
        if !self.on {
            None
        } else if obs.off_map {
            Some(Recovery::OffMap)
        } else if self.rolled_s >= t.roll_dwell_s.v {
            Some(Recovery::RolledOver)
        } else if self.stuck_s >= t.stuck_dwell_s.v {
            Some(Recovery::Stuck)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::arch_drive::session::testing::{root, TUNING};
    use crate::cmd::arch_drive::session::Tuning;

    const DT: f64 = 1.0 / 60.0;

    fn tuning() -> AssistTuning {
        Tuning::load(&root(), TUNING).expect("tuning").assist
    }

    fn at(speed_m_s: f64) -> Obs {
        Obs { speed_m_s, tilt_rad: 0.0, off_map: false }
    }

    #[test]
    fn steering_authority_and_slew_rate_shrink_as_speed_grows() {
        let t = tuning();
        let speeds: Vec<f64> = (0..=14).map(|k| f64::from(k) * 0.5).collect();
        let auth: Vec<f64> = speeds.iter().map(|&v| t.steer_authority(v)).collect();
        let rate: Vec<f64> = speeds.iter().map(|&v| t.steer_rate(v)).collect();
        assert!(auth.windows(2).all(|w| w[1] <= w[0]), "authority must never grow with speed: {auth:?}");
        assert!(rate.windows(2).all(|w| w[1] <= w[0]), "slew rate must never grow with speed: {rate:?}");
        assert!((auth[0] - 1.0).abs() < 1e-12, "full lock when parking");
        assert!((t.steer_authority(t.speed_cap_m_s.v) - t.steer_min_authority.v).abs() < 1e-9);
        assert!(t.steer_authority(t.speed_cap_m_s.v) < 0.5 * auth[0], "at the cap the wheel turns at most half as far");
    }

    #[test]
    fn a_tap_on_steer_at_speed_moves_the_wheel_no_faster_than_the_slew_rate() {
        let t = tuning();
        let mut a = Assist::new(true);
        let v = t.speed_cap_m_s.v;
        let out = a.apply(&Raw { steer: 1.0, throttle: 0.5, ..Raw::default() }, &at(v), &t, DT);
        assert!(out.steer <= t.steer_rate(v) * DT + 1e-12, "snapped to {}", out.steer);
        let mut last = out.steer;
        for _ in 0..600 {
            last = a.apply(&Raw { steer: 1.0, throttle: 0.5, ..Raw::default() }, &at(v), &t, DT).steer;
        }
        assert!((last - t.steer_authority(v)).abs() < 1e-9, "settles at the authority limit, got {last}");
    }

    #[test]
    fn the_throttle_pedal_cannot_jerk() {
        let t = tuning();
        let mut a = Assist::new(true);
        let mut last = 0.0;
        for k in 1..=12 {
            let out = a.apply(&Raw { throttle: 1.0, ..Raw::default() }, &at(0.0), &t, DT);
            assert!(
                out.throttle - last <= t.throttle_rise_per_s.v * DT + 1e-12,
                "tick {k}: {last} -> {}",
                out.throttle
            );
            last = out.throttle;
        }
        assert!(last < 0.3, "a fifth of a second after a full press the pedal is at {last}");
    }

    #[test]
    fn the_throttle_is_eased_to_zero_at_the_speed_cap_and_the_brake_catches_an_overshoot() {
        let t = tuning();
        let mut a = Assist::new(true);
        let cap = t.speed_cap_m_s.v;
        let mut out = a.apply(&Raw { throttle: 1.0, ..Raw::default() }, &at(cap), &t, DT);
        for _ in 0..120 {
            out = a.apply(&Raw { throttle: 1.0, ..Raw::default() }, &at(cap), &t, DT);
        }
        assert!(out.throttle < 1e-6 && out.brake < 1e-6, "at the cap: throttle {} brake {}", out.throttle, out.brake);
        assert_eq!(a.message, "speed limit");
        for _ in 0..120 {
            out = a.apply(&Raw { throttle: 1.0, ..Raw::default() }, &at(cap + 2.0), &t, DT);
        }
        assert!(out.brake > 0.5 && out.throttle < 1e-6, "2 m/s over: throttle {} brake {}", out.throttle, out.brake);
    }

    #[test]
    fn with_the_speed_limit_off_a_full_press_is_never_eased_or_braked_above_the_old_cap() {
        let mut t = tuning();
        t.no_speed_limit = true;
        let mut a = Assist::new(true);
        let cap = t.speed_cap_m_s.v;
        let mut out = a.apply(&Raw { throttle: 1.0, ..Raw::default() }, &at(2.0 * cap), &t, DT);
        for _ in 0..120 {
            out = a.apply(&Raw { throttle: 1.0, ..Raw::default() }, &at(2.0 * cap), &t, DT);
        }
        assert!(
            out.throttle > 0.99 && out.brake < 1e-6,
            "twice the old cap: throttle {} brake {}",
            out.throttle,
            out.brake
        );
        assert_ne!(a.message, "speed limit");
        assert!(t.steer_authority(2.0 * cap) < 0.5, "steering stays gentle at speed");
    }

    #[test]
    fn with_no_pedal_down_the_brake_is_gentle_while_rolling_and_firmer_near_standstill() {
        let t = tuning();
        let mut a = Assist::new(true);
        let mut rolling = a.apply(&Raw::default(), &at(5.0), &t, DT);
        for _ in 0..120 {
            rolling = a.apply(&Raw::default(), &at(5.0), &t, DT);
        }
        assert!((rolling.brake - t.idle_brake.v).abs() < 1e-9, "{}", rolling.brake);
        assert_eq!(a.message, "stopping");
        let mut held = rolling;
        for _ in 0..120 {
            held = a.apply(&Raw::default(), &at(0.1), &t, DT);
        }
        assert!((held.brake - t.hold_brake.v).abs() < 1e-9 && held.brake > rolling.brake, "{}", held.brake);
        let go =
            (0..30).map(|_| a.apply(&Raw { throttle: 1.0, ..Raw::default() }, &at(0.1), &t, DT)).last().expect("ticks");
        assert!(
            go.brake < 1e-9,
            "the hold brake lets go within half a second of the throttle going down: {}",
            go.brake
        );
    }

    #[test]
    fn the_gearbox_is_automatic_unless_reverse_is_asked_for() {
        let t = tuning();
        let mut a = Assist::new(true);
        assert_eq!(a.apply(&Raw::default(), &at(0.0), &t, DT).gear, GearRequest::Auto);
        assert_eq!(a.apply(&Raw { reverse: true, ..Raw::default() }, &at(0.0), &t, DT).gear, GearRequest::Reverse);
    }

    #[test]
    fn with_assists_off_the_pedals_pass_straight_through() {
        let t = tuning();
        let mut a = Assist::new(false);
        let raw = Raw { throttle: 1.0, brake: 0.0, steer: 1.0, reverse: false };
        let out = a.apply(&raw, &at(30.0), &t, DT);
        assert_eq!((out.throttle, out.brake, out.steer), (1.0, 0.0, 1.0));
        let tilted = Obs { speed_m_s: 0.0, tilt_rad: 3.0, off_map: true };
        assert_eq!(a.watch(&raw, &tilted, &t, 10.0), None, "adults are never teleported");
    }

    #[test]
    fn rolled_over_for_one_second_stuck_for_three_or_off_the_map_each_asks_for_a_recovery() {
        let t = tuning();
        let ticks = |s: f64| (s / DT).round() as u32;
        // rolled: 0.9 s is not enough, 1.0 s is
        let mut a = Assist::new(true);
        let rolled = Obs { speed_m_s: 5.0, tilt_rad: 1.5, off_map: false };
        assert!((0..ticks(0.9)).all(|_| a.watch(&Raw::default(), &rolled, &t, DT).is_none()));
        assert!((0..ticks(0.2)).any(|_| a.watch(&Raw::default(), &rolled, &t, DT) == Some(Recovery::RolledOver)));
        // a bounce that ends resets the timer
        let mut a = Assist::new(true);
        for _ in 0..ticks(0.9) {
            a.watch(&Raw::default(), &rolled, &t, DT);
        }
        a.watch(&Raw::default(), &at(5.0), &t, DT);
        assert!((0..ticks(0.9)).all(|_| a.watch(&Raw::default(), &rolled, &t, DT).is_none()));
        // stuck: slow with the throttle down for 3 s; not with the throttle up
        let mut a = Assist::new(true);
        let push = Raw { throttle: 0.5, ..Raw::default() };
        assert!((0..ticks(2.9)).all(|_| a.watch(&push, &at(0.1), &t, DT).is_none()));
        assert!((0..ticks(0.2)).any(|_| a.watch(&push, &at(0.1), &t, DT) == Some(Recovery::Stuck)));
        let mut a = Assist::new(true);
        assert!(
            (0..ticks(10.0)).all(|_| a.watch(&Raw::default(), &at(0.0), &t, DT).is_none()),
            "standing still with no pedal is not stuck"
        );
        // off the map: at once
        let off = Obs { speed_m_s: 5.0, tilt_rad: 0.0, off_map: true };
        assert_eq!(Assist::new(true).watch(&Raw::default(), &off, &t, DT), Some(Recovery::OffMap));
    }
}
