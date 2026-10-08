//! Soft ground: how a footprint sinks into it, and what that costs.
//!
//! Two classic laws of terramechanics, in their simplest forms:
//!
//! * **Bekker's pressure-sinkage law.** Soil is a spring that gets stiffer the narrower the footprint on it: p = (kc / b + kphi) z.
//!   (The soil beside a narrow footprint helps to carry it; under a wide one it cannot, which is why tracks float where a tyre
//!   of the same pressure sinks.) The general law has an exponent, z^n; here n = 1, so the spring is linear and the sinkage is
//!   just `z = p / k` for a pressure `p = load / contact area`. No powers, so nothing platform-dependent in the simulation.
//! * **Mohr-Coulomb shear.** Soil gives way when the shear stress passes its strength, `tau = c + p tan(phi)`: cohesion (what
//!   sticks the grains together) plus friction (which grows with the pressing load). The most thrust a footprint can get from
//!   the ground is the strength times the area it shears: `H = A c + N tan(phi)`. Soft ground gives less than a grip coefficient
//!   would on concrete, and it gives it only as the soil is *sheared*: a long footprint shears a long way and gets close to the
//!   maximum, a short one gets less. We use the hyperbola `x / (x + 2)` of the usual exponential law (the same slope at the
//!   start, within about 12% after), with `x = 0.6 L / K`: the thrust at 60% slip, with `K` the soil's shear modulus.
//!
//! What sinking costs: the **compaction resistance** (the work of pressing the ground down: `R_c = 1/2 B p z` for rolling
//! footprints of total width `B`; for stepping feet, the weight times the sinkage per stride), and the **hull dragging** when
//! the sinkage nears the ground clearance (a cubic ramp, not a cliff, so a design near the edge slows before it stops).
//!
//! The constants are tuned for a game, not a survey: real soil tables are far too stiff or far too soft to give a course
//! where designs at different ground pressures finish, slow down and bog (see `content/terrain.ron`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use w5k_math::{Fx, StateHasher};

/// A surface as written in `content/terrain.ron`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SurfaceDef {
    /// Concrete, rock: the ground does not give. Running gear grips with its own coefficient.
    Rigid,
    /// Soft ground (n = 1 Bekker, Mohr-Coulomb shear).
    Soft {
        /// Cohesive modulus of sinkage (kPa).
        kc: f64,
        /// Frictional modulus of sinkage (kPa / m).
        kphi: f64,
        /// Cohesion (kPa).
        cohesion: f64,
        /// Angle of internal friction (degrees).
        friction_deg: f64,
        /// Shear deformation modulus (m).
        shear_k: f64,
    },
}

/// The terrain table: what each surface name means.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TerrainDef {
    pub surfaces: BTreeMap<String, SurfaceDef>,
}

/// Soft ground in fixed point.
#[derive(Clone, Copy, Debug)]
pub struct Soil {
    kc: Fx,
    kphi: Fx,
    cohesion: Fx,
    tan_phi: Fx,
    shear_k: Fx,
}

/// Where a vehicle meets the ground, in fixed point.
#[derive(Clone, Copy, Debug, Default)]
pub struct Footprint {
    /// How many separate contact units (two tracks, six wheels, eight feet).
    pub units: Fx,
    /// Width `b` of one unit (m): the footprint's smaller dimension, which sets the soil's stiffness under it.
    pub width: Fx,
    /// Length `L` of one unit along the direction of travel (m).
    pub length: Fx,
    /// Total contact area (m^2): the load over this is the ground pressure.
    pub area: Fx,
    /// Feet only: distance travelled between two steps of one foot (m). Zero for tracks and wheels, which roll.
    pub stride: Fx,
    /// Hull belly above the ground (m); zero when it does not matter.
    pub clearance: Fx,
}

/// What a soil does to a footprint, at full cover (the mover scales it by the fraction of the footprint on that soil).
#[derive(Clone, Copy, Debug, Default)]
pub struct SoilAct {
    /// Sinkage (m).
    pub sink: Fx,
    /// Compaction resistance (kN).
    pub compaction: Fx,
    /// Hull dragging resistance (kN).
    pub hull: Fx,
    /// Most thrust the soil gives at the working slip (kN).
    pub shear: Fx,
    /// Strength of the soil under the footprint (kN): `A c + N tan(phi)`.
    pub saturated: Fx,
    /// Shear modulus (m).
    pub shear_k: Fx,
}

/// Smallest footprint width the sinkage law is evaluated at (m): keeps `kc / b` finite.
const MIN_WIDTH: Fx = Fx::from_ratio(1, 20);
/// Hull dragging at a sinkage equal to the clearance, as a fraction of the weight.
const HULL_DRAG: Fx = Fx::from_ratio(1, 4);
/// The sinkage over the clearance beyond which the hull ramp stops growing (the vehicle is long since stuck).
const HULL_RATIO_MAX: Fx = Fx::from_ratio(5, 2);
/// Slip at which the thrust of a footprint is evaluated.
const WORKING_SLIP: Fx = Fx::from_ratio(6, 10);
/// Shear modulus of rigid ground (m): the slip of a tyre on concrete before it grips fully.
pub const HARD_SHEAR_K: Fx = Fx::from_ratio(1, 500);

impl Soil {
    /// Convert a table entry (the one place floats enter: at load).
    pub fn from_def(kc: f64, kphi: f64, cohesion: f64, friction_deg: f64, shear_k: f64) -> Result<Soil, String> {
        if kc < 0.0 || kphi <= 0.0 || cohesion < 0.0 || shear_k <= 0.0 || !(0.0..90.0).contains(&friction_deg) {
            return Err("soil constants must be positive (kphi, shear_k) or non-negative, and the friction angle below 90 degrees".into());
        }
        Ok(Soil {
            kc: Fx::from_f64(kc),
            kphi: Fx::from_f64(kphi),
            cohesion: Fx::from_f64(cohesion),
            tan_phi: Fx::from_f64(friction_deg.to_radians().tan()),
            shear_k: Fx::from_f64(shear_k),
        })
    }

    /// Feed the constants to a state hasher (regression tests of baked courses).
    pub fn hash_into(&self, h: &mut StateHasher) {
        for v in [self.kc, self.kphi, self.cohesion, self.tan_phi, self.shear_k] {
            h.write_fx(v);
        }
    }

    /// Stiffness of the ground under a footprint of width `b` (kPa / m).
    pub fn stiffness(&self, b: Fx) -> Fx {
        self.kc / b.max(MIN_WIDTH) + self.kphi
    }

    /// Sinkage under a pressure `p` (kPa) on a footprint of width `b` (m).
    pub fn sinkage(&self, p: Fx, b: Fx) -> Fx {
        p / self.stiffness(b)
    }

    /// Strength of the soil under a footprint: `A c + N tan(phi)` (kN), for area `A` (m^2) and normal load `N` (kN).
    pub fn strength(&self, area: Fx, normal: Fx) -> Fx {
        area * self.cohesion + normal * self.tan_phi
    }

    /// Thrust fraction of the strength a footprint of length `L` reaches at the working slip: `x / (x + 2)`, `x = 0.6 L / K`.
    pub fn thrust_fraction(&self, length: Fx) -> Fx {
        let x = WORKING_SLIP * length / self.shear_k;
        x / (x + Fx::TWO)
    }

    /// The forces this soil puts on a footprint carrying `normal` kN (the weight component pressing on the ground) of a vehicle
    /// weighing `weight` kN.
    pub fn act(&self, fp: &Footprint, normal: Fx, weight: Fx) -> SoilAct {
        if fp.area <= Fx::ZERO || normal <= Fx::ZERO {
            return SoilAct { shear_k: self.shear_k, ..SoilAct::default() };
        }
        let p = normal / fp.area;
        let z = self.sinkage(p, fp.width);
        let compaction = if fp.stride > Fx::ZERO {
            // Each foot is pressed in once per stride: the energy of the whole set over one stride is N z.
            normal * z / fp.stride
        } else {
            Fx::HALF * fp.units * fp.width * p * z
        };
        let hull = if fp.clearance > Fx::ZERO {
            let r = (z / fp.clearance).min(HULL_RATIO_MAX);
            HULL_DRAG * weight * r * r * r
        } else {
            Fx::ZERO
        };
        let saturated = self.strength(fp.area, normal);
        SoilAct { sink: z, compaction, hull, shear: saturated * self.thrust_fraction(fp.length), saturated, shear_k: self.shear_k }
    }
}
