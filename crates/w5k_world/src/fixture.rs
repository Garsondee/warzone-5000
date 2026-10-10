//! Proving-ground fixtures: tiny closed-form courses for one question each (`docs/validation/proving-ground.md`).
//!
//! | fixture | question | stated parameter |
//! |---|---|---|
//! | `ramp` | what is the steepest grade it can start on and hold? (gradeability) | `grade` (rise over run) |
//! | `side_slope` | at what cross-slope does it slide or tip? (rollover) | `grade` across the track |
//! | `step` | what is the highest step it can climb? | `height_m` |
//! | `trench` | what is the widest ditch it can cross? | `width_m` (at a stated `depth_m`) |
//!
//! Frame: the vehicle drives toward -Z; the feature starts at `z = 0` (the run-in is `z > 0`, flat); `s = -z` is the distance driven past
//! the start. Every height is a closed-form function of position, so the runner can **bisect the stated parameter** (`set_primary`)
//! without regenerating anything, and the answer is exact. Real edges cannot be vertical in a heightfield: a step or a trench wall is eased
//! over a stated run (`edge_run_m`), steep and honest about it. The ground is one material everywhere.

use serde::{Deserialize, Serialize};
use w5k_contract::param::Param;
use w5k_contract::world::{MaterialId, MaterialTable, PropRef, RayHit, WorldQuery};
use w5k_math::{scalar, Vec3};

use crate::strip::standard_material_table;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum FixtureKind {
    /// A ramp of constant `grade` for `length_m` (eased in over `ease_m`), then a flat top.
    Ramp { grade: Param, length_m: Param, ease_m: Param },
    /// The ground tilts across the track: it falls toward +X (the right-hand side) by `grade` per metre, over `length_m`.
    SideSlope { grade: Param, length_m: Param, ease_m: Param },
    /// A step up of `height_m`, its face eased over `edge_run_m`.
    Step { height_m: Param, edge_run_m: Param },
    /// A trench `width_m` across (along the drive) and `depth_m` deep, its walls eased over `depth_m / wall_grade`.
    Trench { width_m: Param, depth_m: Param, wall_grade: Param },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureDef {
    pub name: String,
    pub kind: FixtureKind,
    /// A material of `content/world/materials.ron` (the surface friction and soil of the whole fixture).
    pub surface: String,
}

/// Half width of the fixture across the track, m (a fixture is wide enough for any vehicle; the ground ends there).
const HALF_WIDTH_M: f64 = 50.0; // const-ok: width of the proving ground
/// Length of the flat run-in before the feature, m.
const RUN_IN_M: f64 = 50.0; // const-ok: room to accelerate before the feature
/// Length of the flat run-out after the feature, m.
const RUN_OUT_M: f64 = 50.0; // const-ok: room to stop after the feature
/// Step of the finite differences giving the normal, m (well below any edge run).
const NORMAL_EPS_M: f64 = 1e-3; // const-ok: finite-difference step
/// March step of the ray cast, m (below the shortest edge run).
const RAY_STEP_M: f64 = 0.01; // const-ok: ray march step

#[derive(Clone, Debug)]
pub struct Fixture {
    def: FixtureDef,
    materials: MaterialTable,
    surface: MaterialId,
}

impl FixtureDef {
    pub fn from_ron(text: &str) -> Result<FixtureDef, String> {
        ron::from_str(text).map_err(|e| format!("fixture: {e}"))
    }

    /// Every number checked and sensible.
    pub fn check(&self) -> Result<(), String> {
        let n = &self.name;
        let pos = |l: &str, p: &Param| -> Result<(), String> {
            p.check(&format!("{n}.{l}"))?;
            if p.v > 0.0 {
                Ok(())
            } else {
                Err(format!("{n}.{l}: must be positive"))
            }
        };
        match &self.kind {
            FixtureKind::Ramp { grade, length_m, ease_m } | FixtureKind::SideSlope { grade, length_m, ease_m } => {
                pos("grade", grade)?;
                pos("length_m", length_m)?;
                pos("ease_m", ease_m)?;
                if ease_m.v * 2.0 > length_m.v {
                    return Err(format!("{n}: ease_m must be at most half of length_m"));
                }
            }
            FixtureKind::Step { height_m, edge_run_m } => {
                pos("height_m", height_m)?;
                pos("edge_run_m", edge_run_m)?;
            }
            FixtureKind::Trench { width_m, depth_m, wall_grade } => {
                pos("width_m", width_m)?;
                pos("depth_m", depth_m)?;
                pos("wall_grade", wall_grade)?;
                if width_m.v < 2.0 * depth_m.v / wall_grade.v {
                    return Err(format!(
                        "{n}: the trench is narrower than its two walls ({:.2} m)",
                        2.0 * depth_m.v / wall_grade.v
                    ));
                }
            }
        }
        Ok(())
    }
}

impl Fixture {
    pub fn new(def: FixtureDef) -> Result<Fixture, String> {
        def.check()?;
        let materials = standard_material_table()?;
        let surface =
            materials.id_of(&def.surface).ok_or_else(|| format!("{}: unknown surface `{}`", def.name, def.surface))?;
        Ok(Fixture { def, materials, surface })
    }

    pub fn from_ron(text: &str) -> Result<Fixture, String> {
        Fixture::new(FixtureDef::from_ron(text)?)
    }

    /// The committed fixtures (`content/world/fixtures/*.ron`).
    pub fn ramp() -> Fixture {
        Fixture::from_ron(include_str!("../../../content/world/fixtures/ramp.ron")).expect("ramp.ron is valid")
    }
    pub fn side_slope() -> Fixture {
        Fixture::from_ron(include_str!("../../../content/world/fixtures/side_slope.ron"))
            .expect("side_slope.ron is valid")
    }
    pub fn step() -> Fixture {
        Fixture::from_ron(include_str!("../../../content/world/fixtures/step.ron")).expect("step.ron is valid")
    }
    pub fn trench() -> Fixture {
        Fixture::from_ron(include_str!("../../../content/world/fixtures/trench.ron")).expect("trench.ron is valid")
    }

    pub fn def(&self) -> &FixtureDef {
        &self.def
    }

    /// The parameter the runner bisects: the grade, the cross-grade, the step height or the trench width.
    pub fn primary(&self) -> f64 {
        match &self.def.kind {
            FixtureKind::Ramp { grade, .. } | FixtureKind::SideSlope { grade, .. } => grade.v,
            FixtureKind::Step { height_m, .. } => height_m.v,
            FixtureKind::Trench { width_m, .. } => width_m.v,
        }
    }

    /// Set the primary parameter (and nothing else). The value must be positive.
    pub fn set_primary(&mut self, v: f64) -> Result<(), String> {
        let mut def = self.def.clone();
        match &mut def.kind {
            FixtureKind::Ramp { grade, .. } | FixtureKind::SideSlope { grade, .. } => grade.v = v,
            FixtureKind::Step { height_m, .. } => height_m.v = v,
            FixtureKind::Trench { width_m, .. } => width_m.v = v,
        }
        // A bisected value leaves its stated band: the band belongs to the committed default, not to a probe.
        let widen = |p: &mut Param| {
            p.lo = Some(p.lo.map_or(p.v, |l| l.min(p.v)));
            p.hi = Some(p.hi.map_or(p.v, |h| h.max(p.v)));
        };
        match &mut def.kind {
            FixtureKind::Ramp { grade, .. } | FixtureKind::SideSlope { grade, .. } => widen(grade),
            FixtureKind::Step { height_m, .. } => widen(height_m),
            FixtureKind::Trench { width_m, .. } => widen(width_m),
        }
        def.check()?;
        self.def = def;
        Ok(())
    }

    /// Length of the feature along the drive, m.
    pub fn feature_length_m(&self) -> f64 {
        match &self.def.kind {
            FixtureKind::Ramp { length_m, .. } | FixtureKind::SideSlope { length_m, .. } => length_m.v,
            FixtureKind::Step { edge_run_m, .. } => edge_run_m.v,
            FixtureKind::Trench { width_m, .. } => width_m.v,
        }
    }

    /// Ground height at distance `s` past the feature start and lateral position `x`.
    fn h(&self, x: f64, s: f64) -> f64 {
        match &self.def.kind {
            FixtureKind::Ramp { grade, length_m, ease_m } => {
                // Quadratic ease-in (slope rises linearly from 0 to the grade), constant grade, then a flat top.
                let (g, l, e) = (grade.v, length_m.v, ease_m.v);
                if s <= 0.0 {
                    0.0
                } else if s < e {
                    g * s * s / (2.0 * e)
                } else if s <= l {
                    g * (s - 0.5 * e)
                } else {
                    g * (l - 0.5 * e)
                }
            }
            FixtureKind::SideSlope { grade, length_m, ease_m } => {
                let w = scalar::smoothstep(0.0, ease_m.v, s)
                    * (1.0 - scalar::smoothstep(length_m.v - ease_m.v, length_m.v, s));
                -grade.v * x * w
            }
            FixtureKind::Step { height_m, edge_run_m } => height_m.v * scalar::smoothstep(0.0, edge_run_m.v, s),
            FixtureKind::Trench { width_m, depth_m, wall_grade } => {
                let wall = depth_m.v / wall_grade.v;
                -depth_m.v
                    * scalar::smoothstep(0.0, wall, s)
                    * (1.0 - scalar::smoothstep(width_m.v - wall, width_m.v, s))
            }
        }
    }
}

impl WorldQuery for Fixture {
    fn height_m(&self, x: f64, z: f64) -> f64 {
        self.h(x, -z)
    }

    fn normal(&self, x: f64, z: f64) -> Vec3 {
        let e = NORMAL_EPS_M;
        let gx = (self.h(x + e, -z) - self.h(x - e, -z)) / (2.0 * e);
        let gz = (self.h(x, -(z + e)) - self.h(x, -(z - e))) / (2.0 * e);
        Vec3::new(-gx, 1.0, -gz).normalized_or_zero()
    }

    fn material_id_at(&self, _x: f64, _z: f64) -> MaterialId {
        self.surface
    }

    fn materials(&self) -> &MaterialTable {
        &self.materials
    }

    fn raycast(&self, origin: Vec3, dir: Vec3, max_m: f64) -> Option<RayHit> {
        let dir = dir.try_normalize(1e-12)?; // const-ok: zero-length direction tolerance
        if origin.y < self.height_m(origin.x, origin.z) {
            return None;
        }
        let (mut prev, mut d) = (0.0, RAY_STEP_M);
        while d <= max_m + RAY_STEP_M {
            let p = origin + dir * d;
            if p.y < self.height_m(p.x, p.z) {
                let (mut lo, mut hi) = (prev, d);
                for _ in 0..40 {
                    // const-ok: bisection steps
                    let mid = 0.5 * (lo + hi);
                    let q = origin + dir * mid;
                    if q.y < self.height_m(q.x, q.z) {
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
                    material: self.surface,
                    prop: None,
                });
            }
            prev = d;
            d += RAY_STEP_M;
        }
        None
    }

    fn props_in_aabb(&self, _min: Vec3, _max: Vec3, _out: &mut Vec<PropRef>) {}

    fn bounds(&self) -> (Vec3, Vec3) {
        let l = self.feature_length_m();
        let (lo, hi) = match &self.def.kind {
            FixtureKind::Ramp { grade, length_m, ease_m } => (0.0, grade.v * (length_m.v - 0.5 * ease_m.v)),
            FixtureKind::SideSlope { grade, .. } => (-grade.v * HALF_WIDTH_M, grade.v * HALF_WIDTH_M),
            FixtureKind::Step { height_m, .. } => (0.0, height_m.v),
            FixtureKind::Trench { depth_m, .. } => (-depth_m.v, 0.0),
        };
        (Vec3::new(-HALF_WIDTH_M, lo, -(l + RUN_OUT_M)), Vec3::new(HALF_WIDTH_M, hi, RUN_IN_M))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn z_at(s: f64) -> f64 {
        -s
    }

    #[test]
    fn ramp_has_exactly_its_stated_grade_between_its_ease_and_its_end() {
        let f = Fixture::ramp();
        let FixtureKind::Ramp { grade, length_m, ease_m } = &f.def().kind else { panic!("ramp") };
        for s in [ease_m.v + 1.0, 20.0, length_m.v - 1.0] {
            let n = f.normal(0.0, z_at(s));
            assert!((n.z.abs() / n.y - grade.v).abs() < 1e-5, "grade at s={s}");
            assert!(n.z > 0.0, "the ground rises ahead (toward -Z) so the normal leans back toward +Z");
        }
        assert!(
            (f.height_m(3.0, z_at(length_m.v)) - grade.v * (length_m.v - 0.5 * ease_m.v)).abs() < 1e-9,
            "height at the top"
        );
        assert!(f.height_m(0.0, 10.0).abs() < 1e-12, "the run-in is flat at zero");
        assert!(
            (f.height_m(0.0, z_at(length_m.v + 30.0)) - f.height_m(0.0, z_at(length_m.v))).abs() < 1e-12,
            "a flat top beyond the ramp"
        );
    }

    #[test]
    fn ramp_eases_in_with_a_continuous_slope() {
        let f = Fixture::ramp();
        let slope = |s: f64| (f.height_m(0.0, z_at(s + 1e-4)) - f.height_m(0.0, z_at(s - 1e-4))) / 2e-4;
        let FixtureKind::Ramp { grade, ease_m, .. } = &f.def().kind else { panic!("ramp") };
        assert!(slope(-1.0).abs() < 1e-9 && slope(1e-3) < 0.01, "starts flat");
        assert!(
            (slope(ease_m.v - 1e-3) - grade.v).abs() < 1e-3 && (slope(ease_m.v + 1e-3) - grade.v).abs() < 1e-3,
            "no kink where the ease ends"
        );
    }

    #[test]
    fn side_slope_tilts_across_the_track_and_is_level_along_it() {
        let f = Fixture::side_slope();
        let FixtureKind::SideSlope { grade, .. } = &f.def().kind else { panic!("side slope") };
        let z = z_at(40.0);
        let n = f.normal(0.0, z);
        assert!(
            (n.x / n.y - grade.v).abs() < 1e-5,
            "the ground falls toward +X: its normal leans toward +X by the grade"
        );
        assert!(n.z.abs() < 1e-6, "level along the drive");
        assert!(
            (f.height_m(-2.0, z) - f.height_m(2.0, z) - 4.0 * grade.v).abs() < 1e-9,
            "the right side is lower by grade x width"
        );
        assert!(f.height_m(10.0, 10.0).abs() < 1e-12, "flat before the slope");
    }

    #[test]
    fn step_is_exactly_as_high_as_stated_beyond_its_edge() {
        let f = Fixture::step();
        let FixtureKind::Step { height_m, edge_run_m } = &f.def().kind else { panic!("step") };
        assert!(
            f.height_m(0.0, 1.0).abs() < 1e-12
                && (f.height_m(0.0, z_at(edge_run_m.v + 0.01)) - height_m.v).abs() < 1e-12
        );
        let face = f.normal(0.0, z_at(0.5 * edge_run_m.v));
        assert!(face.y < 0.25, "the face is steep ({})", face.y);
    }

    #[test]
    fn trench_is_as_wide_and_deep_as_stated_and_its_floor_is_flat() {
        let mut f = Fixture::trench();
        f.set_primary(2.0).expect("a 2 m trench");
        let FixtureKind::Trench { depth_m, wall_grade, .. } = &f.def().kind else { panic!("trench") };
        let wall = depth_m.v / wall_grade.v;
        for s in [wall + 0.01, 1.0, 2.0 - wall - 0.01] {
            assert!((f.height_m(0.0, z_at(s)) + depth_m.v).abs() < 1e-12, "floor at s={s}");
        }
        assert!(
            f.height_m(0.0, 0.5).abs() < 1e-12 && f.height_m(0.0, z_at(2.0 + 0.5)).abs() < 1e-12,
            "level on both sides"
        );
        assert!((f.height_m(0.0, z_at(0.5 * wall)) + 0.5 * depth_m.v).abs() < 1e-9, "mid-wall is half depth");
    }

    #[test]
    fn the_runner_can_bisect_the_primary_parameter_and_a_bad_value_is_refused() {
        let mut f = Fixture::ramp();
        for g in [0.05, 0.6, 1.2] {
            f.set_primary(g).expect("a grade");
            assert!((f.primary() - g).abs() < 1e-15);
            let n = f.normal(0.0, z_at(30.0));
            assert!((n.z / n.y - g).abs() < 1e-5);
        }
        assert!(f.set_primary(-0.1).is_err() && f.set_primary(f64::NAN).is_err());
        let mut t = Fixture::trench();
        assert!(t.set_primary(0.2).is_err(), "narrower than its own two walls");
    }

    #[test]
    fn fixtures_load_from_content_carry_their_surface_and_refuse_bad_numbers() {
        for f in [Fixture::ramp(), Fixture::side_slope(), Fixture::step(), Fixture::trench()] {
            assert_eq!(f.material_at(0.0, 0.0).name, "asphalt", "{}", f.def().name);
            assert!(f.def().check().is_ok());
            let again: FixtureDef = ron::from_str(&ron::to_string(f.def()).expect("serialise")).expect("re-parse");
            assert_eq!(*f.def(), again);
        }
        let bad = include_str!("../../../content/world/fixtures/ramp.ron")
            .replace("surface: \"asphalt\"", "surface: \"lava\"");
        assert!(Fixture::from_ron(&bad).expect_err("refused").contains("unknown surface"));
    }

    #[test]
    fn raycast_lands_on_the_ramp_at_the_analytic_distance() {
        let f = Fixture::ramp();
        let FixtureKind::Ramp { grade, ease_m, .. } = &f.def().kind else { panic!("ramp") };
        // Straight down from above s = 20: the height there is grade (s - ease / 2).
        let hit = f.raycast(Vec3::new(0.0, 30.0, z_at(20.0)), -Vec3::Y, 100.0).expect("hit");
        assert!((hit.distance_m - (30.0 - grade.v * (20.0 - 0.5 * ease_m.v))).abs() < 1e-6);
        assert!(f.raycast(Vec3::new(0.0, 30.0, 10.0), Vec3::Y, 100.0).is_none(), "up misses");
    }

    #[test]
    fn bounds_contain_the_feature() {
        for f in [Fixture::ramp(), Fixture::side_slope(), Fixture::step(), Fixture::trench()] {
            let (lo, hi) = f.bounds();
            for s in [0.0, 0.5 * f.feature_length_m(), f.feature_length_m()] {
                let h = f.height_m(0.0, z_at(s));
                assert!(
                    h >= lo.y - 1e-9 && h <= hi.y + 1e-9,
                    "{}: height {h} outside [{}, {}]",
                    f.def().name,
                    lo.y,
                    hi.y
                );
            }
        }
    }
}
