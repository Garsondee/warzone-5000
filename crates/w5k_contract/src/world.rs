//! The world as the physics sees it: terrain height, surface materials (with soil parameters) and props.
//! Written by lane WORLD; read by vehicle dynamics, combat and the AI. `testing` has stand-ins (`FlatPlane`, `BumpStrip`).

use serde::{Deserialize, Serialize};
use w5k_math::Vec3;

use crate::param::Param;

/// Index into a [`MaterialTable`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MaterialId(pub u16);

/// Bekker-Wong soil parameters in SI (pressure-sinkage `p = (kc/b + kphi) z^n`, Mohr-Coulomb strength with Janosi-Hanamoto shear).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoilParams {
    /// Sinkage exponent n (dimensionless).
    pub n: f64,
    /// Cohesive modulus kc, Pa / m^(n-1).
    pub kc_pa_m_n1: f64,
    /// Frictional modulus kphi, Pa / m^n.
    pub kphi_pa_m_n: f64,
    /// Cohesion c, Pa.
    pub cohesion_pa: f64,
    /// Angle of internal friction phi, rad.
    pub friction_angle_rad: f64,
    /// Shear deformation modulus K, m.
    pub shear_k_m: f64,
}

/// A surface as the physics uses it (plain numbers, no pedigree: fast to read every substep).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub name: String,
    /// Peak friction coefficient of a rubber tyre or track shoe on this surface (dimensionless).
    pub mu_peak: f64,
    /// Sliding friction coefficient.
    pub mu_slide: f64,
    /// Rolling resistance coefficient of a hard wheel on it (dimensionless).
    pub rolling_coeff: f64,
    /// RMS height of micro-roughness the heightfield does not resolve, m (adds ride excitation).
    pub roughness_rms_m: f64,
    /// `None` for rigid ground (asphalt, rock); soft ground carries soil parameters.
    pub soil: Option<SoilParams>,
}

/// A surface as authored in RON: every number is a [`Param`] with its pedigree. [`MaterialDef::bake`] flattens it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialDef {
    pub name: String,
    pub mu_peak: Param,
    pub mu_slide: Param,
    pub rolling_coeff: Param,
    pub roughness_rms_m: Param,
    #[serde(default)]
    pub soil: Option<SoilDef>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoilDef {
    pub n: Param,
    pub kc_pa_m_n1: Param,
    pub kphi_pa_m_n: Param,
    pub cohesion_pa: Param,
    /// Authored in degrees because every soil table prints degrees.
    pub friction_angle_deg: Param,
    pub shear_k_m: Param,
}

impl MaterialDef {
    /// Check every parameter and flatten into the runtime form.
    pub fn bake(&self) -> Result<Material, String> {
        let n = &self.name;
        self.mu_peak.check(&format!("{n}.mu_peak"))?;
        self.mu_slide.check(&format!("{n}.mu_slide"))?;
        self.rolling_coeff.check(&format!("{n}.rolling_coeff"))?;
        self.roughness_rms_m.check(&format!("{n}.roughness_rms_m"))?;
        let soil = match &self.soil {
            None => None,
            Some(s) => {
                s.n.check(&format!("{n}.soil.n"))?;
                s.kc_pa_m_n1.check(&format!("{n}.soil.kc"))?;
                s.kphi_pa_m_n.check(&format!("{n}.soil.kphi"))?;
                s.cohesion_pa.check(&format!("{n}.soil.cohesion"))?;
                s.friction_angle_deg.check(&format!("{n}.soil.friction_angle"))?;
                s.shear_k_m.check(&format!("{n}.soil.shear_k"))?;
                Some(SoilParams {
                    n: s.n.v,
                    kc_pa_m_n1: s.kc_pa_m_n1.v,
                    kphi_pa_m_n: s.kphi_pa_m_n.v,
                    cohesion_pa: s.cohesion_pa.v,
                    friction_angle_rad: w5k_math::scalar::deg_to_rad(s.friction_angle_deg.v),
                    shear_k_m: s.shear_k_m.v,
                })
            }
        };
        Ok(Material {
            name: self.name.clone(),
            mu_peak: self.mu_peak.v,
            mu_slide: self.mu_slide.v,
            rolling_coeff: self.rolling_coeff.v,
            roughness_rms_m: self.roughness_rms_m.v,
            soil,
        })
    }
}

/// The surfaces of a world, indexed by [`MaterialId`].
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MaterialTable {
    pub materials: Vec<Material>,
}

impl MaterialTable {
    pub fn get(&self, id: MaterialId) -> &Material {
        // A world only ever hands out ids that exist; fall back to material 0 rather than panic mid-simulation.
        self.materials.get(id.0 as usize).unwrap_or(&self.materials[0])
    }

    pub fn id_of(&self, name: &str) -> Option<MaterialId> {
        self.materials.iter().position(|m| m.name == name).map(|i| MaterialId(i as u16))
    }

    pub fn push(&mut self, m: Material) -> MaterialId {
        self.materials.push(m);
        MaterialId((self.materials.len() - 1) as u16)
    }
}

/// What a probe of the ground returns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundSample {
    pub height_m: f64,
    /// Unit surface normal (points up).
    pub normal: Vec3,
    pub material: MaterialId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PropId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropKind {
    Tree,
    Barricade,
    Building,
    Wall,
    Rock,
    Wreck,
    Target,
}

/// Collision shape of a prop, in the prop's local frame (primitives only; the world lane may extend this by CCR).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PropShape {
    Sphere { radius_m: f64 },
    /// A vertical cylinder standing on `base` (a tree trunk, a pillar), `height_m` tall.
    Cylinder { radius_m: f64, height_m: f64 },
    Box { half_m: Vec3 },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PropRef {
    pub id: PropId,
    pub kind: PropKind,
    pub shape: PropShape,
    pub transform: w5k_math::Transform,
    /// Breaks (is removed or toppled) when hit with more than this impulse, N s. `f64::INFINITY` = never.
    pub break_impulse_ns: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit {
    pub distance_m: f64,
    pub point: Vec3,
    pub normal: Vec3,
    pub material: MaterialId,
    pub prop: Option<PropId>,
}

/// The query interface of a world. Implementations must be deterministic and side-effect free (`&self`): vehicles are stepped
/// against an immutable world.
pub trait WorldQuery {
    /// Terrain height at plan position `(x, z)`, m.
    fn height_m(&self, x: f64, z: f64) -> f64;
    /// Unit terrain normal at `(x, z)`.
    fn normal(&self, x: f64, z: f64) -> Vec3;
    /// The surface material at `(x, z)`.
    fn material_id_at(&self, x: f64, z: f64) -> MaterialId;
    fn materials(&self) -> &MaterialTable;
    /// First hit of the ray with terrain or props within `max_m`.
    fn raycast(&self, origin: Vec3, dir: Vec3, max_m: f64) -> Option<RayHit>;
    /// Append the props overlapping the axis-aligned box `[min, max]` to `out`.
    fn props_in_aabb(&self, min: Vec3, max: Vec3, out: &mut Vec<PropRef>);
    /// Axis-aligned bounds of the playable area `(min, max)`.
    fn bounds(&self) -> (Vec3, Vec3);

    fn material_at(&self, x: f64, z: f64) -> &Material {
        self.materials().get(self.material_id_at(x, z))
    }

    fn sample(&self, x: f64, z: f64) -> GroundSample {
        GroundSample { height_m: self.height_m(x, z), normal: self.normal(x, z), material: self.material_id_at(x, z) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::param::Param;

    fn mud() -> MaterialDef {
        MaterialDef {
            name: "mud".into(),
            mu_peak: Param::estimate(0.45, 0.3, 0.6, "stand-in order of magnitude for a wet clay; WORLD and VALIDATION replace it with a cited value"),
            mu_slide: Param::estimate(0.4, 0.3, 0.5, "stand-in; to be sourced"),
            rolling_coeff: Param::estimate(0.12, 0.08, 0.2, "stand-in; to be sourced"),
            roughness_rms_m: Param::estimate(0.03, 0.01, 0.06, "stand-in; to be sourced"),
            soil: Some(SoilDef {
                n: Param::estimate(0.8, 0.5, 1.1, "stand-in; to be sourced"),
                kc_pa_m_n1: Param::estimate(13_190.0, 5_000.0, 30_000.0, "stand-in order of magnitude for a clayey soil; to be sourced"),
                kphi_pa_m_n: Param::estimate(692_200.0, 400_000.0, 1_200_000.0, "stand-in order of magnitude for a clayey soil; to be sourced"),
                cohesion_pa: Param::estimate(4_140.0, 2_000.0, 8_000.0, "stand-in; to be sourced"),
                friction_angle_deg: Param::estimate(13.0, 8.0, 20.0, "stand-in; to be sourced"),
                shear_k_m: Param::estimate(0.025, 0.01, 0.05, "stand-in; to be sourced"),
            }),
        }
    }

    #[test]
    fn a_material_def_bakes_and_converts_degrees() {
        let m = mud().bake().expect("valid");
        let s = m.soil.expect("soft");
        assert!((s.friction_angle_rad - 13.0f64.to_radians()).abs() < 1e-12);
        assert!((m.mu_peak - 0.45).abs() < 1e-15);
    }

    #[test]
    fn a_bad_param_is_named_in_the_error() {
        let mut d = mud();
        d.mu_peak = Param::spec(0.5, "");
        let e = d.bake().unwrap_err();
        assert!(e.contains("mud.mu_peak"), "{e}");
    }
}
