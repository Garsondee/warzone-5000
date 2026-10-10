//! The data-driven bump strip: the same features and names as the contract stand-in `BumpStrip`, authored in RON
//! (`content/world/strips/*.ron`) with every number a `Param`, so CHASSIS and VALIDATION can use it from day one of M1.
//!
//! The strip is uniform across X, so every wheel of an axle sees the same height. Heights are closed-form functions of z
//! (half-sines, a cosine washboard, smoothstep ramps); nothing is sampled, so there is no grid error to explain away.

use serde::{Deserialize, Serialize};
use w5k_contract::param::Param;
use w5k_contract::world::{MaterialDef, MaterialId, MaterialTable, PropRef, RayHit, WorldQuery};
use w5k_math::{scalar, Vec3};

/// One feature as authored. Same variants and field names as the stand-in `testing::world::Feature`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum FeatureDef {
    /// A half-sine bump (speed hump) `height_m` tall, `length_m` long, centred at `z_m`.
    Bump { z_m: Param, length_m: Param, height_m: Param },
    /// `count` sinusoidal ripples of amplitude `height_m` and `wavelength_m`, starting at `z_m` and running toward -Z.
    Washboard { z_m: Param, wavelength_m: Param, height_m: Param, count: u32 },
    /// A smooth step up of `height_m` over `ramp_m`, held for `hold_m`, then stepped down.
    Plateau { z_m: Param, ramp_m: Param, hold_m: Param, height_m: Param },
    /// A pothole (negative half-sine).
    Hole { z_m: Param, length_m: Param, depth_m: Param },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StripDef {
    pub name: String,
    pub features: Vec<FeatureDef>,
    /// The mud patch runs from `mud_start_z_m` down to `mud_end_z_m` (start > end: driving toward -Z).
    pub mud_start_z_m: Param,
    pub mud_end_z_m: Param,
}

/// A feature with its parameters checked and flattened.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Feature {
    Bump { z: f64, length: f64, height: f64 },
    Washboard { z: f64, wavelength: f64, height: f64, count: u32 },
    Plateau { z: f64, ramp: f64, hold: f64, height: f64 },
    Hole { z: f64, length: f64, depth: f64 },
}

impl Feature {
    fn height(&self, z: f64) -> f64 {
        match *self {
            Feature::Bump { z: z0, length, height } => {
                let u = (z - z0) / length + 0.5;
                if (0.0..=1.0).contains(&u) {
                    height * scalar::sin(scalar::PI * u)
                } else {
                    0.0
                }
            }
            Feature::Washboard { z: z0, wavelength, height, count } => {
                let d = z0 - z;
                if d >= 0.0 && d <= wavelength * count as f64 {
                    0.5 * height * (1.0 - scalar::cos(scalar::TAU * d / wavelength))
                } else {
                    0.0
                }
            }
            Feature::Plateau { z: z0, ramp, hold, height } => {
                let d = z0 - z;
                if d < 0.0 {
                    0.0
                } else if d < ramp {
                    height * scalar::smoothstep(0.0, ramp, d)
                } else if d < ramp + hold {
                    height
                } else if d < 2.0 * ramp + hold {
                    height * (1.0 - scalar::smoothstep(0.0, ramp, d - ramp - hold))
                } else {
                    0.0
                }
            }
            Feature::Hole { z: z0, length, depth } => {
                let u = (z - z0) / length + 0.5;
                if (0.0..=1.0).contains(&u) {
                    -depth * scalar::sin(scalar::PI * u)
                } else {
                    0.0
                }
            }
        }
    }
}

/// The materials every strip uses (`content/world/materials.ron`), baked and checked.
pub fn standard_material_table() -> Result<MaterialTable, String> {
    let defs: Vec<MaterialDef> = ron::from_str(include_str!("../../../content/world/materials.ron"))
        .map_err(|e| format!("materials.ron: {e}"))?;
    let mut t = MaterialTable::default();
    for d in &defs {
        t.push(d.bake()?);
    }
    Ok(t)
}

/// A strip of ground along -Z with the stand-in's features, queried in closed form.
#[derive(Clone, Debug)]
pub struct DataStrip {
    pub name: String,
    features: Vec<Feature>,
    mud: (f64, f64),
    materials: MaterialTable,
}

impl DataStrip {
    /// The standard first-light strip, from the committed RON.
    pub fn standard() -> DataStrip {
        DataStrip::from_ron(include_str!("../../../content/world/strips/standard.ron"))
            .expect("content/world/strips/standard.ron is valid")
    }

    pub fn from_ron(text: &str) -> Result<DataStrip, String> {
        let def: StripDef = ron::from_str(text).map_err(|e| format!("strip: {e}"))?;
        DataStrip::bake(&def, standard_material_table()?)
    }

    pub fn bake(def: &StripDef, materials: MaterialTable) -> Result<DataStrip, String> {
        let n = &def.name;
        let mut features = Vec::new();
        let p = |label: &str, x: &Param| -> Result<f64, String> {
            x.check(&format!("{n}.{label}"))?;
            Ok(x.v)
        };
        for (k, f) in def.features.iter().enumerate() {
            features.push(match f {
                FeatureDef::Bump { z_m, length_m, height_m } => Feature::Bump {
                    z: p(&format!("features[{k}].z_m"), z_m)?,
                    length: pos(
                        &format!("{n}.features[{k}].length_m"),
                        p(&format!("features[{k}].length_m"), length_m)?,
                    )?,
                    height: p(&format!("features[{k}].height_m"), height_m)?,
                },
                FeatureDef::Washboard { z_m, wavelength_m, height_m, count } => Feature::Washboard {
                    z: p(&format!("features[{k}].z_m"), z_m)?,
                    wavelength: pos(
                        &format!("{n}.features[{k}].wavelength_m"),
                        p(&format!("features[{k}].wavelength_m"), wavelength_m)?,
                    )?,
                    height: p(&format!("features[{k}].height_m"), height_m)?,
                    count: *count,
                },
                FeatureDef::Plateau { z_m, ramp_m, hold_m, height_m } => Feature::Plateau {
                    z: p(&format!("features[{k}].z_m"), z_m)?,
                    ramp: pos(&format!("{n}.features[{k}].ramp_m"), p(&format!("features[{k}].ramp_m"), ramp_m)?)?,
                    hold: p(&format!("features[{k}].hold_m"), hold_m)?,
                    height: p(&format!("features[{k}].height_m"), height_m)?,
                },
                FeatureDef::Hole { z_m, length_m, depth_m } => Feature::Hole {
                    z: p(&format!("features[{k}].z_m"), z_m)?,
                    length: pos(
                        &format!("{n}.features[{k}].length_m"),
                        p(&format!("features[{k}].length_m"), length_m)?,
                    )?,
                    depth: p(&format!("features[{k}].depth_m"), depth_m)?,
                },
            });
        }
        let mud = (p("mud_start_z_m", &def.mud_start_z_m)?, p("mud_end_z_m", &def.mud_end_z_m)?);
        if mud.0 < mud.1 {
            return Err(format!("{n}: mud_start_z_m must be greater than mud_end_z_m (driving toward -Z)"));
        }
        if materials.id_of("mud").is_none() {
            return Err(format!("{n}: the material table has no `mud`"));
        }
        Ok(DataStrip { name: def.name.clone(), features, mud, materials })
    }

    /// `(start, end)` z of the mud patch.
    pub fn mud_range_z_m(&self) -> (f64, f64) {
        self.mud
    }

    fn h(&self, z: f64) -> f64 {
        self.features.iter().map(|f| f.height(z)).sum()
    }
}

fn pos(what: &str, v: f64) -> Result<f64, String> {
    if v > 0.0 {
        Ok(v)
    } else {
        Err(format!("{what}: must be positive"))
    }
}

impl WorldQuery for DataStrip {
    fn height_m(&self, _x: f64, z: f64) -> f64 {
        self.h(z)
    }

    fn normal(&self, _x: f64, z: f64) -> Vec3 {
        let e = 0.05; // const-ok: finite-difference half step for the closed-form profile, m
        let dz = (self.h(z + e) - self.h(z - e)) / (2.0 * e);
        Vec3::new(0.0, 1.0, -dz).normalized_or_zero()
    }

    fn material_id_at(&self, _x: f64, z: f64) -> MaterialId {
        // `bake` checked that `mud` exists.
        let mud = self.materials.id_of("mud").unwrap_or(MaterialId(0));
        if z <= self.mud.0 && z >= self.mud.1 {
            mud
        } else {
            self.materials.id_of("asphalt").unwrap_or(MaterialId(0))
        }
    }

    fn materials(&self) -> &MaterialTable {
        &self.materials
    }

    fn raycast(&self, origin: Vec3, dir: Vec3, max_m: f64) -> Option<RayHit> {
        let dir = dir.try_normalize(1e-12)?; // const-ok: zero-length direction tolerance
        if origin.y - self.h(origin.z) < 0.0 {
            return None;
        }
        let step = 0.05; // const-ok: march step, m; finer than the narrowest feature (0.8 m hump)
        let (mut prev_d, mut d) = (0.0, step);
        while d <= max_m + step {
            let p = origin + dir * d;
            if p.y - self.h(p.z) < 0.0 {
                let (mut lo, mut hi) = (prev_d, d);
                for _ in 0..40 {
                    let mid = 0.5 * (lo + hi);
                    let q = origin + dir * mid;
                    if q.y - self.h(q.z) < 0.0 {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                if hi > max_m {
                    return None;
                }
                let point = origin + dir * hi;
                return Some(RayHit {
                    distance_m: hi,
                    point,
                    normal: self.normal(point.x, point.z),
                    material: self.material_id_at(point.x, point.z),
                    prop: None,
                });
            }
            prev_d = d;
            d += step;
        }
        None
    }

    fn props_in_aabb(&self, _min: Vec3, _max: Vec3, _out: &mut Vec<PropRef>) {}

    fn bounds(&self) -> (Vec3, Vec3) {
        let lowest = self.mud.1.min(-300.0); // const-ok: the stand-in's extent, m
        (Vec3::new(-50.0, -5.0, lowest), Vec3::new(50.0, 20.0, 50.0)) // const-ok: the stand-in's extent, m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::testing::world::BumpStrip;

    #[test]
    fn data_strip_matches_the_stand_in_strip_everywhere() {
        let (a, b) = (DataStrip::standard(), BumpStrip::standard());
        let mut z = 20.0;
        while z > -260.0 {
            assert!((a.height_m(0.0, z) - b.height_m(0.0, z)).abs() < 1e-12, "height at z={z}");
            assert!((a.normal(0.0, z) - b.normal(0.0, z)).length() < 1e-12, "normal at z={z}");
            assert_eq!(a.material_id_at(0.0, z).0 == 2, b.material_id_at(0.0, z).0 == 2, "mud at z={z}");
            z -= 0.037;
        }
        assert_eq!(a.bounds(), b.bounds());
    }

    #[test]
    fn strip_has_the_stated_peaks_and_the_mud_is_where_it_says() {
        let w = DataStrip::standard();
        assert!((w.height_m(0.0, -40.0) - 0.10).abs() < 1e-9);
        assert!((w.height_m(0.0, -130.0) - 0.2).abs() < 1e-9);
        assert!((w.height_m(0.0, -170.0) + 0.12).abs() < 1e-9);
        assert!(w.material_at(0.0, -220.0).soil.is_some());
        assert!(w.material_at(0.0, -100.0).soil.is_none());
    }

    #[test]
    fn strip_ron_round_trips_and_every_param_checks() {
        let text = include_str!("../../../content/world/strips/standard.ron");
        let def: StripDef = ron::from_str(text).expect("parse");
        let again: StripDef = ron::from_str(&ron::to_string(&def).expect("serialise")).expect("re-parse");
        assert_eq!(def, again);
        assert!(DataStrip::bake(&def, standard_material_table().expect("table")).is_ok());
        assert_eq!(standard_material_table().expect("table").materials.len(), 6);
    }

    #[test]
    fn a_strip_with_a_bad_param_is_refused_with_its_name() {
        let text = include_str!("../../../content/world/strips/standard.ron")
            .replace("prov: Spec, src: \"test-course layout; a speed hump\"", "prov: Spec, src: \"\"");
        let e = DataStrip::from_ron(&text).unwrap_err();
        assert!(e.contains("height_m"), "{e}");
    }

    #[test]
    fn strip_raycast_lands_on_the_surface_over_the_plateau() {
        let w = DataStrip::standard();
        let hit = w.raycast(Vec3::new(0.0, 5.0, -130.0), -Vec3::Y, 10.0).expect("hit");
        assert!((hit.distance_m - (5.0 - 0.2)).abs() < 1e-9);
        let slanted = w.raycast(Vec3::new(0.0, 1.0, -30.0), Vec3::new(0.0, -0.2, -1.0), 30.0).expect("hit");
        assert!((slanted.point.y - w.height_m(0.0, slanted.point.z)).abs() < 1e-9);
    }
}
