//! The mover: one vehicle's motion along the course.
//!
//! **The model.** Along the course a vehicle obeys Newton's second law, with the forces in kilonewtons and the mass in tonnes
//! (so kN / t = m/s^2):
//!
//! ```text
//!   m a = F_thrust - F_grade - F_rolling - F_drag - F_compaction - F_hull
//! ```
//!
//! * `F_grade = W sin(theta)`: the pull of the slope the body rests on.
//! * `F_rolling = C_rr N` (N is the load pressing on the ground, `W cos(theta)`), and for a cushion it grows with speed.
//! * `F_drag = 1/2 rho C_d A v^2`.
//! * `F_thrust` is the smaller of two limits. The **engine** limit is force = power / speed (after the gear's own cost of
//!   transport, which a walker pays out of its power, and which takes the thrust *negative* when the walker cannot afford
//!   its speed: a walker is not a free-rolling cart, gravity does not simply speed it up): a hyperbola, infinite at a
//!   standstill, which is why real drives have a lowest gear (here, a constant force below a fraction of the rated speed).
//!   The **ground** limit is `grip x N`: you cannot push on the ground harder than it pushes back. At low speed the ground
//!   limits a powerful vehicle; at high speed the engine does.
//! * A smooth **governor**, `1 - (v / v_rated)^32`, takes the thrust to zero at the running gear's rated speed instead of
//!   hitting a wall (a hard speed cap makes the engine irrelevant for most designs, which is what Warzone 2100 does). The
//!   exponent is high so that a vehicle the *engine* limits (not the gear) keeps all its power until close to the rating.
//!
//! * On **soft ground** (`soil.rs`) a footprint sinks by `z = p / k` (Bekker), which costs a compaction resistance and, when the
//!   hull nears the ground, a hull drag; and the ground gives only the thrust its shear strength allows (Mohr-Coulomb) in
//!   place of `grip x load`. A footprint half on soft ground gets half of each, so the effects ramp in as it crosses a boundary.
//!
//! The slope the vehicle feels is the height difference across the span of the points it rests on. Everything is fixed point and
//! the substeps are integer, so a run is bit-identical on every machine.

use w5k_math::Fx;

use crate::course::Course;
use crate::replay::limit;
use crate::soil::{Footprint, HARD_SHEAR_K};
use crate::spec::MoverSpec;

/// Standard gravity (m/s^2).
const GRAV: Fx = Fx::from_ratio(981, 100);
/// 1/2 x rho (1.225 kg/m^3) / 1000, so that `AIR x C_dA x v^2` is the drag in kN.
const AIR: Fx = Fx::from_ratio(6125, 10_000_000);
/// Below this speed, with nothing left to push, a vehicle counts as stuck (m/s).
pub const STUCK_SPEED: Fx = Fx::from_ratio(15, 100);

/// x^32 by five squarings: no `pow`, so nothing platform-dependent.
fn pow32(x: Fx) -> Fx {
    let x2 = x * x;
    let x4 = x2 * x2;
    let x8 = x4 * x4;
    let x16 = x8 * x8;
    x16 * x16
}

/// Where the vehicle is and how fast it is going.
#[derive(Clone, Copy, Debug, Default)]
pub struct MoverState {
    pub s: Fx,
    pub v: Fx,
}

/// The forces of one sub-step, in kN, and what limited the vehicle: what a replay records to explain a run.
#[derive(Clone, Copy, Debug, Default)]
pub struct Step {
    pub accel: Fx,
    /// Thrust actually applied.
    pub thrust: Fx,
    /// The most the ground (or the gear) lets it push with.
    pub cap: Fx,
    pub grade: Fx,
    pub roll: Fx,
    pub drag: Fx,
    /// Soil compaction resistance (kN).
    pub compaction: Fx,
    /// Hull dragging in soft ground (kN).
    pub hull: Fx,
    pub weight: Fx,
    pub slope: Fx,
    /// How far the vehicle has sunk (m) and how much its running gear slips (0 to 1): outputs, not part of the dynamics.
    pub sink: Fx,
    pub slip: Fx,
    /// Fraction (0 to 1) of the footprint on soft ground.
    pub soft: Fx,
    pub limit: u8,
}

impl Step {
    /// Everything the soil takes (kN).
    pub fn soil(&self) -> Fx {
        self.compaction + self.hull
    }
}

/// A vehicle ready to run: the spec in fixed point.
#[derive(Clone, Debug)]
pub struct Mover {
    mass: Fx,
    weight: Fx,
    drive: Fx,
    rated: Fx,
    floor: Fx,
    c_roll: Fx,
    c_internal: Fx,
    grip: Fx,
    thrust_w: Fx,
    skirt: bool,
    k_drag: Fx,
    span: Fx,
    slope_span: Fx,
    grounded: bool,
    foot: Footprint,
}

impl Mover {
    pub fn new(spec: &MoverSpec) -> Result<Mover, String> {
        if spec.mass_t <= 0.0 {
            return Err("no mass".into());
        }
        if spec.drive_kw <= 0.0 {
            return Err("no power reaches the running gear".into());
        }
        if spec.rated_ms <= 0.0 {
            return Err("no rated speed".into());
        }
        let mass = Fx::from_f64(spec.mass_t);
        let rated = Fx::from_f64(spec.rated_ms);
        let span = Fx::from_f64(spec.span_m).clamp(Fx::ONE, Fx::from_int(60));
        let slope_span = if spec.class.airborne() { span.max(Fx::from_f64(2.0 * spec.altitude_m)) } else { span };
        Ok(Mover {
            mass,
            weight: mass * GRAV,
            drive: Fx::from_f64(spec.drive_kw),
            rated,
            floor: (rated * Fx::from_f64(spec.launch_floor.clamp(0.01, 1.0))).max(Fx::from_ratio(3, 10)),
            c_roll: Fx::from_f64(spec.c_roll),
            c_internal: Fx::from_f64(spec.c_internal),
            grip: Fx::from_f64(spec.grip_mu),
            thrust_w: Fx::from_f64(spec.thrust_w),
            skirt: spec.skirt_drag,
            k_drag: AIR * Fx::from_f64(spec.cd_a_m2),
            span,
            slope_span,
            grounded: spec.class.grounded(),
            foot: Footprint {
                units: Fx::from_int(spec.contact_units as i32),
                width: Fx::from_f64(spec.contact_width_m),
                length: Fx::from_f64(spec.contact_len_m),
                area: Fx::from_f64(spec.contact_area_m2),
                stride: Fx::from_f64(spec.stride_m),
                clearance: Fx::from_f64(spec.clearance_m),
            },
        })
    }

    /// Distance between the support points that set the pose (m).
    pub fn span(&self) -> Fx {
        self.span
    }

    pub fn rated(&self) -> Fx {
        self.rated
    }

    /// Advance by `dt` seconds and report the forces of this step.
    pub fn step(&self, course: &Course, st: &mut MoverState, dt: Fx) -> Step {
        let half = self.slope_span * Fx::HALF;
        let slope = (course.height(st.s + half) - course.height(st.s - half)) / self.slope_span;
        let cos = Fx::ONE / (Fx::ONE + slope * slope).sqrt();
        let load = self.weight * cos;
        let grade = self.weight * slope * cos;

        // Soft ground: each soil under the footprint acts in proportion to the fraction of the footprint on it.
        let (mut soft, mut sink, mut compaction, mut hull) = (Fx::ZERO, Fx::ZERO, Fx::ZERO, Fx::ZERO);
        let (mut shear, mut strength, mut shear_k) = (Fx::ZERO, Fx::ZERO, Fx::ZERO);
        if self.grounded && self.foot.area > Fx::ZERO {
            for (id, soil) in course.soft_surfaces() {
                let f = course.fraction(id, st.s - half, st.s + half);
                if f > Fx::ZERO {
                    let a = soil.act(&self.foot, load, self.weight);
                    soft += f;
                    sink += f * a.sink;
                    compaction += f * a.compaction;
                    hull += f * a.hull;
                    shear += f * a.shear;
                    strength += f * a.saturated;
                    shear_k += f * a.shear_k;
                }
            }
        }
        let soft = soft.min(Fx::ONE);

        let x = (st.v / self.rated).min(Fx::ONE);
        let cut = pow32(x);
        let gov = Fx::ONE - cut;
        // Power left after the running gear's own cost of transport, as a force at this speed.
        let engine = (self.drive * gov - self.c_internal * load * st.v) / st.v.max(self.floor);
        let hard_grip = self.grip * load;
        // The most the ground gives: grip on the hard part of the footprint, shear strength on the soft part.
        let cap = if self.grip > Fx::ZERO {
            if soft > Fx::ZERO {
                (Fx::ONE - soft) * hard_grip + shear
            } else {
                hard_grip
            }
        } else {
            self.thrust_w * self.weight
        };
        // Only a gait can come out negative: when it costs more power than the engine has at this speed, the machine cannot
        // afford the speed and slows (the same energy balance that sets a walker's top speed). Everything else pushes or
        // coasts.
        let thrust = engine.clamp(-cap, cap);
        let roll = self.c_roll * load * if self.skirt { x } else { Fx::ONE };
        let drag = self.k_drag * st.v * st.v;
        let accel = (thrust - grade - roll - drag - compaction - hull) / self.mass;

        // Slip is an output: the share of the ground's strength in use, through the same hyperbola as the thrust law.
        let slip = if self.grounded && self.foot.length > Fx::ZERO && thrust > Fx::ZERO {
            let avail = (Fx::ONE - soft) * hard_grip + strength;
            let h = if avail > Fx::ZERO { (thrust / avail).min(Fx::from_ratio(999, 1000)) } else { Fx::ZERO };
            let k = (Fx::ONE - soft) * HARD_SHEAR_K + shear_k;
            (Fx::TWO * k / self.foot.length * h / (Fx::ONE - h)).min(Fx::ONE)
        } else {
            Fx::ZERO
        };

        // `v` is the speed along the slope; the course is measured along the horizontal, so progress is v cos(theta).
        st.v = (st.v + accel * dt).clamp(Fx::ZERO, self.rated);
        st.s += st.v * cos * dt;

        let soil_share = compaction + hull;
        let lim = if st.v < STUCK_SPEED && accel <= Fx::ZERO {
            limit::STALLED
        } else if cut >= Fx::from_ratio(1, 10) {
            limit::RATING
        } else if engine > cap {
            if self.grip <= Fx::ZERO {
                limit::THRUST
            } else if soft >= Fx::HALF {
                limit::SOIL
            } else {
                limit::GRIP
            }
        } else if soft >= Fx::HALF && soil_share.mul_int(4) >= grade.max(Fx::ZERO) + roll + drag + soil_share {
            // The soil is a quarter or more of everything the vehicle is pushing against.
            limit::SOIL
        } else {
            limit::POWER
        };
        Step { accel, thrust, cap, grade, roll, drag, compaction, hull, weight: self.weight, slope, sink, slip, soft, limit: lim }
    }
}
