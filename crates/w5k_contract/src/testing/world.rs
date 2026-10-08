//! Stand-in worlds. The driving direction of every scenario is -Z (forward), so features are laid along -Z and run across X.

use w5k_math::{scalar, Vec3};

use crate::world::{Material, MaterialId, MaterialTable, PropRef, RayHit, SoilParams, WorldQuery};

/// Three surfaces: `asphalt` (id 0, rigid), `dirt` (id 1, firm) and `mud` (id 2, soft). **Stand-in numbers**: the right order of magnitude
/// for a wet clay, nothing more. WORLD and VALIDATION replace them with cited values (`MaterialDef` with provenance).
pub fn standard_materials() -> MaterialTable {
    let mut t = MaterialTable::default();
    t.push(Material {
        name: "asphalt".into(),
        mu_peak: 0.9,
        mu_slide: 0.8,
        rolling_coeff: 0.0,
        roughness_rms_m: 0.002,
        soil: None,
    });
    t.push(Material {
        name: "dirt".into(),
        mu_peak: 0.65,
        mu_slide: 0.55,
        rolling_coeff: 0.025,
        roughness_rms_m: 0.015,
        soil: None,
    });
    t.push(Material {
        name: "mud".into(),
        mu_peak: 0.45,
        mu_slide: 0.4,
        rolling_coeff: 0.0, // the soil law supplies the compaction resistance
        roughness_rms_m: 0.03,
        soil: Some(SoilParams {
            n: 0.8,
            kc_pa_m_n1: 13_190.0,
            kphi_pa_m_n: 692_200.0,
            cohesion_pa: 4_140.0,
            friction_angle_rad: scalar::deg_to_rad(13.0),
            shear_k_m: 0.025,
        }),
    });
    t
}

/// A level plane of asphalt.
#[derive(Clone, Debug)]
pub struct FlatPlane {
    pub height_m: f64,
    materials: MaterialTable,
}

impl FlatPlane {
    pub fn new() -> FlatPlane {
        FlatPlane { height_m: 0.0, materials: standard_materials() }
    }
}

impl Default for FlatPlane {
    fn default() -> FlatPlane {
        FlatPlane::new()
    }
}

impl WorldQuery for FlatPlane {
    fn height_m(&self, _x: f64, _z: f64) -> f64 {
        self.height_m
    }
    fn normal(&self, _x: f64, _z: f64) -> Vec3 {
        Vec3::Y
    }
    fn material_id_at(&self, _x: f64, _z: f64) -> MaterialId {
        MaterialId(0)
    }
    fn materials(&self) -> &MaterialTable {
        &self.materials
    }
    fn raycast(&self, origin: Vec3, dir: Vec3, max_m: f64) -> Option<RayHit> {
        if dir.y >= -1e-12 {
            return None;
        }
        let d = (self.height_m - origin.y) / dir.y;
        if d < 0.0 || d > max_m {
            return None;
        }
        Some(RayHit { distance_m: d, point: origin + dir * d, normal: Vec3::Y, material: MaterialId(0), prop: None })
    }
    fn props_in_aabb(&self, _min: Vec3, _max: Vec3, _out: &mut Vec<PropRef>) {}
    fn bounds(&self) -> (Vec3, Vec3) {
        (Vec3::new(-1000.0, -10.0, -1000.0), Vec3::new(1000.0, 50.0, 1000.0))
    }
}

/// One feature laid across the strip.
#[derive(Clone, Copy, Debug)]
pub enum Feature {
    /// A half-sine bump (speed hump): `height_m` tall, `length_m` long, centred at `z_m`.
    Bump { z_m: f64, length_m: f64, height_m: f64 },
    /// Sinusoidal ripples (washboard) of amplitude `height_m` and `wavelength_m`, `count` of them, starting at `z_m` and running toward -Z.
    Washboard { z_m: f64, wavelength_m: f64, height_m: f64, count: u32 },
    /// A smooth step up of `height_m` over `length_m`, ending at `z_m + ...` (stays up afterwards for `hold_m`, then steps down).
    Plateau { z_m: f64, ramp_m: f64, hold_m: f64, height_m: f64 },
    /// A pothole (negative half-sine).
    Hole { z_m: f64, length_m: f64, depth_m: f64 },
}

/// A straight strip with a speed bump, a washboard, a plateau, a pothole and a patch of mud: the first-light test course.
/// Features are uniform across X (every wheel of an axle sees the same height), which keeps expected results easy to reason about.
#[derive(Clone, Debug)]
pub struct BumpStrip {
    pub features: Vec<Feature>,
    /// `(z_start, z_end)` of the mud patch (z decreasing: start > end).
    pub mud: (f64, f64),
    materials: MaterialTable,
}

impl BumpStrip {
    pub fn standard() -> BumpStrip {
        BumpStrip {
            features: vec![
                Feature::Bump { z_m: -40.0, length_m: 0.8, height_m: 0.10 },
                Feature::Washboard { z_m: -70.0, wavelength_m: 1.5, height_m: 0.03, count: 12 },
                Feature::Plateau { z_m: -120.0, ramp_m: 3.0, hold_m: 20.0, height_m: 0.2 },
                Feature::Hole { z_m: -170.0, length_m: 1.5, depth_m: 0.12 },
            ],
            mud: (-200.0, -240.0),
            materials: standard_materials(),
        }
    }

    fn feature_height(f: &Feature, z: f64) -> f64 {
        match *f {
            Feature::Bump { z_m, length_m, height_m } => {
                let u = (z - z_m) / length_m + 0.5;
                if (0.0..=1.0).contains(&u) {
                    height_m * scalar::sin(scalar::PI * u)
                } else {
                    0.0
                }
            }
            Feature::Washboard { z_m, wavelength_m, height_m, count } => {
                let d = z_m - z; // distance travelled into the board
                if d >= 0.0 && d <= wavelength_m * count as f64 {
                    0.5 * height_m * (1.0 - scalar::cos(scalar::TAU * d / wavelength_m))
                } else {
                    0.0
                }
            }
            Feature::Plateau { z_m, ramp_m, hold_m, height_m } => {
                let d = z_m - z;
                if d < 0.0 {
                    0.0
                } else if d < ramp_m {
                    height_m * scalar::smoothstep(0.0, ramp_m, d)
                } else if d < ramp_m + hold_m {
                    height_m
                } else if d < 2.0 * ramp_m + hold_m {
                    height_m * (1.0 - scalar::smoothstep(0.0, ramp_m, d - ramp_m - hold_m))
                } else {
                    0.0
                }
            }
            Feature::Hole { z_m, length_m, depth_m } => {
                let u = (z - z_m) / length_m + 0.5;
                if (0.0..=1.0).contains(&u) {
                    -depth_m * scalar::sin(scalar::PI * u)
                } else {
                    0.0
                }
            }
        }
    }
}

impl WorldQuery for BumpStrip {
    fn height_m(&self, _x: f64, z: f64) -> f64 {
        self.features.iter().map(|f| BumpStrip::feature_height(f, z)).sum()
    }

    fn normal(&self, x: f64, z: f64) -> Vec3 {
        let e = 0.05;
        let dz = (self.height_m(x, z + e) - self.height_m(x, z - e)) / (2.0 * e);
        Vec3::new(0.0, 1.0, -dz).normalized_or_zero()
    }

    fn material_id_at(&self, _x: f64, z: f64) -> MaterialId {
        if z <= self.mud.0 && z >= self.mud.1 {
            MaterialId(2)
        } else {
            MaterialId(0)
        }
    }

    fn materials(&self) -> &MaterialTable {
        &self.materials
    }

    fn raycast(&self, origin: Vec3, dir: Vec3, max_m: f64) -> Option<RayHit> {
        let dir = dir.try_normalize(1e-12)?;
        let step = 0.05;
        let mut prev_d = 0.0;
        let mut prev_above = origin.y - self.height_m(origin.x, origin.z);
        if prev_above < 0.0 {
            return None;
        }
        let mut d = step;
        while d <= max_m + step {
            let p = origin + dir * d;
            let above = p.y - self.height_m(p.x, p.z);
            if above < 0.0 {
                // Bisect the crossing between prev_d and d.
                let (mut lo, mut hi) = (prev_d, d);
                for _ in 0..30 {
                    let mid = 0.5 * (lo + hi);
                    let q = origin + dir * mid;
                    if q.y - self.height_m(q.x, q.z) < 0.0 {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                let hit = origin + dir * hi;
                if hi > max_m {
                    return None;
                }
                return Some(RayHit {
                    distance_m: hi,
                    point: hit,
                    normal: self.normal(hit.x, hit.z),
                    material: self.material_id_at(hit.x, hit.z),
                    prop: None,
                });
            }
            prev_d = d;
            prev_above = above;
            d += step;
        }
        let _ = prev_above;
        None
    }

    fn props_in_aabb(&self, _min: Vec3, _max: Vec3, _out: &mut Vec<PropRef>) {}

    fn bounds(&self) -> (Vec3, Vec3) {
        (Vec3::new(-50.0, -5.0, -300.0), Vec3::new(50.0, 20.0, 50.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_strip_is_flat_where_nothing_is_and_has_the_stated_peaks() {
        let w = BumpStrip::standard();
        assert!(w.height_m(0.0, 0.0).abs() < 1e-12);
        assert!((w.height_m(0.0, -40.0) - 0.10).abs() < 1e-9, "speed hump peak");
        assert!((w.height_m(0.0, -130.0) - 0.2).abs() < 1e-9, "plateau top");
        assert!((w.height_m(0.0, -170.0) + 0.12).abs() < 1e-9, "pothole bottom");
        assert!(w.height_m(0.0, -300.0).abs() < 1e-12);
    }

    #[test]
    fn mud_patch_is_soft_and_the_rest_is_asphalt() {
        let w = BumpStrip::standard();
        assert!(w.material_at(0.0, -220.0).soil.is_some());
        assert!(w.material_at(0.0, -10.0).soil.is_none());
    }

    #[test]
    fn normal_tilts_back_on_the_up_slope_of_a_bump() {
        let w = BumpStrip::standard();
        // Driving toward -Z, the front face of the hump is at z slightly > -40: terrain rises as z decreases, so dh/dz < 0 and the
        // normal leans toward +Z (back toward the driver).
        let n = w.normal(0.0, -39.8);
        assert!(n.z > 0.0 && n.y > 0.9);
    }

    #[test]
    fn rays_hit_the_terrain_at_the_height_field() {
        let w = BumpStrip::standard();
        let hit = w.raycast(Vec3::new(0.0, 5.0, -40.0), Vec3::new(0.0, -1.0, 0.0), 20.0).expect("hit");
        assert!((hit.point.y - 0.10).abs() < 1e-3, "{}", hit.point.y);
        let f = FlatPlane::new().raycast(Vec3::new(1.0, 3.0, 1.0), Vec3::new(0.0, -1.0, 0.0), 10.0).expect("hit");
        assert!((f.distance_m - 3.0).abs() < 1e-12);
    }
}
