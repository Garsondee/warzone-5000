//! The course: height and surface along the track.
//!
//! A course is written as a list of **segments** (length, grade, surface), like a road profile: "30 m of flat concrete, 80 m
//! down at 8 % on concrete, 140 m flat on soft earth...". Real roads do not change grade at a point, they ease through a
//! vertical curve, and a vehicle that has to bridge a kink behaves badly (and looks worse), so every change of grade is
//! blended over `blend_m` metres: the slope changes *linearly* across the blend, which makes the height a parabola there.
//!
//! The segments are baked once, at load, into tables at 0.5 m spacing (height, slope, surface). After that every lookup is
//! integer arithmetic. The terrain is a profile extruded across the lane for now; the movement code only ever asks for the
//! height and surface at a distance `s`, so a real 2D heightfield can replace the tables later without touching it.

use serde::{Deserialize, Serialize};
use w5k_math::{Fx, StateHasher};

use crate::soil::{Soil, SurfaceDef, TerrainDef};

/// Table spacing: 0.5 m, exactly representable in Q32.32.
const DS: Fx = Fx::from_raw(1 << 31);
/// A quarter, for the trapezoid rule over one table cell (half a cell width, halved again for the mean of two slopes).
const QUARTER: Fx = Fx::from_raw(1 << 30);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SegmentDef {
    pub length_m: f64,
    /// Rise over run: +0.06 climbs 6 m in 100 m.
    pub grade: f64,
    /// Surface name; the terrain table (`content/terrain.ron`) gives it a meaning.
    pub surface: String,
}

fn d_pad() -> f64 {
    40.0
}
fn d_run_out() -> f64 {
    100.0
}
fn d_blend() -> f64 {
    12.0
}
fn d_limit() -> f64 {
    180.0
}

/// A course as written in `content/courses/*.ron`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CourseDef {
    pub name: String,
    /// Flat pad behind the start line (m), so long vehicles start fully on the course.
    #[serde(default = "d_pad")]
    pub pad_behind_m: f64,
    /// Flat run-out after the finish line (m), where vehicles brake to a stop.
    #[serde(default = "d_run_out")]
    pub run_out_m: f64,
    /// Length of the vertical curve at every change of grade (m).
    #[serde(default = "d_blend")]
    pub blend_m: f64,
    #[serde(default = "d_limit")]
    pub time_limit_s: f64,
    pub segments: Vec<SegmentDef>,
    /// Timing gates (m from the start line). Empty means just start and finish.
    #[serde(default)]
    pub checkpoints_m: Vec<f64>,
}

#[derive(Clone, Copy, Debug)]
struct Piece {
    s0: Fx,
    s1: Fx,
    grade: Fx,
    surface: u8,
}

/// A baked course.
#[derive(Clone, Debug)]
pub struct Course {
    pub name: String,
    surface_names: Vec<String>,
    pieces: Vec<Piece>,
    s_min: Fx,
    height: Vec<Fx>,
    slope: Vec<Fx>,
    surf: Vec<u8>,
    finish: Fx,
    checkpoints: Vec<Fx>,
    time_limit_ticks: u32,
    /// The soil of each surface (by surface id); `None` where the ground is rigid.
    soils: Vec<Option<Soil>>,
}

fn intern(names: &mut Vec<String>, name: &str) -> Result<u8, String> {
    if let Some(i) = names.iter().position(|n| n == name) {
        return Ok(i as u8);
    }
    if names.len() >= 255 {
        return Err("too many distinct surfaces".into());
    }
    names.push(name.to_string());
    Ok((names.len() - 1) as u8)
}

/// Slope at `s`: the grade of the piece that contains it, eased linearly across each vertical curve.
fn slope_at(pieces: &[Piece], blend: Fx, s: Fx) -> Fx {
    let mut g = pieces[0].grade;
    for p in pieces {
        if s >= p.s0 {
            g = p.grade;
        }
    }
    if blend > Fx::ZERO {
        let half = blend * Fx::HALF;
        for j in 1..pieces.len() {
            let k = pieces[j].s0;
            let (g0, g1) = (pieces[j - 1].grade, pieces[j].grade);
            if g0 != g1 && s > k - half && s < k + half {
                let t = (s - (k - half)) / blend;
                return g0 + (g1 - g0) * t;
            }
        }
    }
    g
}

impl Course {
    /// Bake a course definition into fixed-point tables, with every surface rigid (no soil). This is the one place floats enter
    /// the simulation, with [`Course::bake_with`]: each is converted exactly once (`Fx::from_f64` is IEEE arithmetic, so the
    /// result is the same everywhere).
    pub fn bake(def: &CourseDef) -> Result<Course, String> {
        Course::bake_inner(def, None)
    }

    /// Bake a course whose surfaces mean what the terrain table says. Every surface the course uses must be in the table.
    pub fn bake_with(def: &CourseDef, terrain: &TerrainDef) -> Result<Course, String> {
        Course::bake_inner(def, Some(terrain))
    }

    fn bake_inner(def: &CourseDef, terrain: Option<&TerrainDef>) -> Result<Course, String> {
        if def.segments.is_empty() {
            return Err("a course needs at least one segment".into());
        }
        let blend = Fx::from_f64(def.blend_m.max(0.0));
        let pad = Fx::from_f64(def.pad_behind_m.max(0.0));
        let run_out = Fx::from_f64(def.run_out_m.max(0.0));
        let mut surface_names: Vec<String> = Vec::new();

        let first = intern(&mut surface_names, &def.segments[0].surface)?;
        let mut pieces: Vec<Piece> = Vec::new();
        if pad > Fx::ZERO {
            pieces.push(Piece { s0: -pad, s1: Fx::ZERO, grade: Fx::ZERO, surface: first });
        }
        let mut s = Fx::ZERO;
        for seg in &def.segments {
            if seg.length_m <= 0.0 {
                return Err("segment lengths must be positive".into());
            }
            if seg.length_m < def.blend_m {
                return Err(format!("a {:.1} m segment is shorter than the {:.1} m vertical curve", seg.length_m, def.blend_m));
            }
            let len = Fx::from_f64(seg.length_m);
            let surface = intern(&mut surface_names, &seg.surface)?;
            pieces.push(Piece { s0: s, s1: s + len, grade: Fx::from_f64(seg.grade), surface });
            s += len;
        }
        let finish = s;
        if run_out > Fx::ZERO {
            let last = pieces[pieces.len() - 1].surface;
            pieces.push(Piece { s0: finish, s1: finish + run_out, grade: Fx::ZERO, surface: last });
        }

        let s_min = pieces[0].s0;
        let s_max = pieces[pieces.len() - 1].s1;
        let n = (s_max - s_min).mul_int(2).floor_int() as usize + 1;
        let mut slope = Vec::with_capacity(n);
        let mut surf = Vec::with_capacity(n);
        for i in 0..n {
            let at = s_min + DS.mul_int(i as i64);
            slope.push(slope_at(&pieces, blend, at));
            let mut sid = pieces[0].surface;
            for p in &pieces {
                if at >= p.s0 {
                    sid = p.surface;
                }
            }
            surf.push(sid);
        }
        // The slope is piecewise linear with knots on the 0.5 m grid, so the trapezoid rule integrates it exactly.
        let mut height = Vec::with_capacity(n);
        height.push(Fx::ZERO);
        for i in 0..n - 1 {
            let h = height[i] + (slope[i] + slope[i + 1]) * QUARTER;
            height.push(h);
        }

        let mut checkpoints: Vec<Fx> = def.checkpoints_m.iter().map(|&c| Fx::from_f64(c)).collect();
        if checkpoints.is_empty() {
            checkpoints = vec![Fx::ZERO, finish];
        }
        let time_limit_ticks = Fx::from_f64(def.time_limit_s).mul_int(crate::replay::HZ as i64).floor_int().max(1) as u32;
        let mut soils = Vec::with_capacity(surface_names.len());
        for name in &surface_names {
            soils.push(match terrain {
                None => None,
                Some(t) => match t.surfaces.get(name) {
                    None => return Err(format!("surface '{name}' is not in the terrain table")),
                    Some(SurfaceDef::Rigid) => None,
                    Some(SurfaceDef::Soft { kc, kphi, cohesion, friction_deg, shear_k }) => {
                        Some(Soil::from_def(*kc, *kphi, *cohesion, *friction_deg, *shear_k).map_err(|e| format!("surface '{name}': {e}"))?)
                    }
                },
            });
        }
        Ok(Course { name: def.name.clone(), surface_names, pieces, s_min, height, slope, surf, finish, checkpoints, time_limit_ticks, soils })
    }

    /// The soft surfaces of this course and their soils.
    pub fn soft_surfaces(&self) -> impl Iterator<Item = (u8, &Soil)> {
        self.soils.iter().enumerate().filter_map(|(i, s)| s.as_ref().map(|s| (i as u8, s)))
    }

    /// Whether any part of the course is soft ground.
    pub fn has_soft_ground(&self) -> bool {
        self.soils.iter().any(|s| s.is_some())
    }

    /// Table index and fraction for a distance (clamped to the table).
    fn locate(&self, s: Fx) -> (usize, Fx) {
        let u = (s - self.s_min).mul_int(2);
        if u <= Fx::ZERO {
            return (0, Fx::ZERO);
        }
        let last = self.height.len() - 1;
        let i = u.floor_int();
        if i as usize >= last {
            return (last, Fx::ZERO);
        }
        (i as usize, u.frac())
    }

    /// Height of the ground at distance `s` from the start line (m). The start pad is at 0.
    pub fn height(&self, s: Fx) -> Fx {
        let (i, f) = self.locate(s);
        if f == Fx::ZERO {
            self.height[i]
        } else {
            self.height[i].lerp(self.height[i + 1], f)
        }
    }

    /// Slope (rise over run) at `s`.
    pub fn slope(&self, s: Fx) -> Fx {
        let (i, f) = self.locate(s);
        if f == Fx::ZERO {
            self.slope[i]
        } else {
            self.slope[i].lerp(self.slope[i + 1], f)
        }
    }

    /// Surface id at `s` (an index into [`Course::surface_names`]).
    pub fn surface(&self, s: Fx) -> u8 {
        let (i, _) = self.locate(s);
        self.surf[i]
    }

    /// Fraction (0 to 1) of the stretch `s0..s1` that lies on surface `id`. Exact: the surface changes at the piece
    /// boundaries, so the fraction slides smoothly as a vehicle crosses one.
    pub fn fraction(&self, id: u8, s0: Fx, s1: Fx) -> Fx {
        let (lo, hi) = (s0.min(s1).max(self.s_min), s0.max(s1).min(self.s_max()));
        if hi - lo <= Fx::EPSILON.mul_int(4) {
            return if self.surface(lo) == id { Fx::ONE } else { Fx::ZERO };
        }
        let mut on = Fx::ZERO;
        for p in &self.pieces {
            if p.surface == id {
                let (a, b) = (p.s0.max(lo), p.s1.min(hi));
                if b > a {
                    on += b - a;
                }
            }
        }
        (on / (hi - lo)).clamp(Fx::ZERO, Fx::ONE)
    }

    pub fn surface_names(&self) -> &[String] {
        &self.surface_names
    }

    pub fn surface_id(&self, name: &str) -> Option<u8> {
        self.surface_names.iter().position(|n| n == name).map(|i| i as u8)
    }

    pub fn s_min(&self) -> Fx {
        self.s_min
    }

    pub fn s_max(&self) -> Fx {
        self.s_min + DS.mul_int(self.height.len() as i64 - 1)
    }

    /// Distance of the finish line from the start line (m).
    pub fn finish(&self) -> Fx {
        self.finish
    }

    /// Timing gates (m): the start line, any intermediate gates, and the finish line.
    pub fn checkpoints(&self) -> &[Fx] {
        &self.checkpoints
    }

    pub fn time_limit_ticks(&self) -> u32 {
        self.time_limit_ticks
    }

    /// Height and surface every `step_m` metres from `s_min` to `s_max`, as floats, for viewers and charts only.
    pub fn profile(&self, step_m: f64) -> (Vec<f64>, Vec<u8>) {
        let (a, b) = (self.s_min.to_f64(), self.s_max().to_f64());
        let n = ((b - a) / step_m).floor() as usize + 1;
        let mut h = Vec::with_capacity(n);
        let mut s_ids = Vec::with_capacity(n);
        for k in 0..n {
            let s = Fx::from_f64(a + k as f64 * step_m);
            h.push(self.height(s).to_f64());
            s_ids.push(self.surface(s));
        }
        (h, s_ids)
    }

    /// Hash of the baked tables (regression tests).
    pub fn table_hash(&self) -> u64 {
        let mut h = StateHasher::new();
        for (&y, &sid) in self.height.iter().zip(&self.surf) {
            h.write_fx(y);
            h.write_u8(sid);
        }
        for soil in self.soils.iter().flatten() {
            soil.hash_into(&mut h);
        }
        h.finish()
    }
}
