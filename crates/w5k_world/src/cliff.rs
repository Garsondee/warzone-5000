//! Cliffs (steep scarps), the chute beside them and the switchback road that zig-zags up their face.
//!
//! A heightfield has one height per (x, z), so a cliff is a *scarp*: a step of `height_m` whose face runs over a horizontal depth
//! `1.5 H / face_grade` (the steepest slope of a smoothstep is 1.5 H / depth, so this gives exactly the stated grade). It is added to the
//! hills as a separate layer, so the course's ordinary grade limit still governs the hills and the cliff carries its own.
//!
//! Frame of a cliff: `s` runs from the foot (0) up the face toward the plateau, `t` runs along the face. `rise_dir_deg` is the direction of
//! +s as a compass bearing in the course frame: 0 = -Z (the driving direction), 90 = +X.

use serde::{Deserialize, Serialize};
use w5k_contract::param::Param;
use w5k_math::scalar;

/// A steep line straight up the face: a notch with a gentler grade that a capable vehicle can attempt cross-country.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChuteDef {
    /// Position along the face, m.
    pub t_m: f64,
    pub width_m: Param,
    /// Slope of the chute, rise over run: stated here so it can be graded against traction.
    pub grade: Param,
    /// Material laid in the chute (loose gravel makes it a traction test).
    pub surface: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CliffDef {
    /// Where the foot line crosses the course plan, m.
    pub x_m: f64,
    pub z_m: f64,
    pub rise_dir_deg: Param,
    pub height_m: Param,
    pub face_grade: Param,
    /// Length of the full-height face along `t`, m; it is centred on `x_m, z_m`.
    pub length_m: Param,
    /// Each end of the face eases down over this distance (the long way round).
    pub end_taper_m: Param,
    #[serde(default)]
    pub chute: Option<ChuteDef>,
}

/// A zig-zag road up a cliff face, between two road waypoints (`after_waypoint` and the next).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwitchbackDef {
    pub after_waypoint: usize,
    /// Index into `CourseDef::cliffs`.
    pub cliff: usize,
    /// Centre of the zig-zag along the face, m.
    pub t_centre_m: f64,
    /// Length of each leg along the face, m (longer legs mean fewer hairpins).
    pub span_m: Param,
    /// Radius of every hairpin (a semicircle at the end of each leg), m. Must exceed the vehicle's turning radius; the lanes of
    /// successive legs are `2 R` apart at the turn, so `depth / legs` must be at least `2 R`.
    pub hairpin_radius_m: Param,
}

/// Slope of the scarp's smoothstep rise relative to its mean: the steepest point is 1.5 H / depth.
const SMOOTHSTEP_PEAK: f64 = 1.5; // const-ok: d/dx of 3x^2 - 2x^3 at x = 1/2

impl CliffDef {
    pub fn check(&self, name: &str) -> Result<(), String> {
        for (l, p) in [
            ("rise_dir_deg", &self.rise_dir_deg),
            ("height_m", &self.height_m),
            ("face_grade", &self.face_grade),
            ("length_m", &self.length_m),
            ("end_taper_m", &self.end_taper_m),
        ] {
            p.check(&format!("{name}.{l}"))?;
        }
        if let Some(c) = &self.chute {
            c.width_m.check(&format!("{name}.chute.width_m"))?;
            c.grade.check(&format!("{name}.chute.grade"))?;
            if c.grade.v >= self.face_grade.v {
                return Err(format!("{name}.chute.grade must be gentler than the face grade"));
            }
        }
        Ok(())
    }

    /// Unit vectors of the cliff frame in plan `(x, z)`: (+s, +t).
    pub fn axes(&self) -> ((f64, f64), (f64, f64)) {
        let a = scalar::deg_to_rad(self.rise_dir_deg.v);
        ((scalar::sin(a), -scalar::cos(a)), (scalar::cos(a), scalar::sin(a)))
    }

    pub fn to_st(&self, x: f64, z: f64) -> (f64, f64) {
        let (s, t) = self.axes();
        let (dx, dz) = (x - self.x_m, z - self.z_m);
        (dx * s.0 + dz * s.1, dx * t.0 + dz * t.1)
    }

    pub fn to_xz(&self, s: f64, t: f64) -> (f64, f64) {
        let (sa, ta) = self.axes();
        (self.x_m + s * sa.0 + t * ta.0, self.z_m + s * sa.1 + t * ta.1)
    }

    /// Grade of the face at position `t` along it: the face grade, blending to the chute grade inside the chute.
    fn grade_at(&self, t: f64) -> f64 {
        match &self.chute {
            Some(c) => {
                let w =
                    1.0 - scalar::smoothstep(c.width_m.v * 0.5, c.width_m.v * 0.5 + CHUTE_BLEND_M, (t - c.t_m).abs());
                self.face_grade.v + (c.grade.v - self.face_grade.v) * w
            }
            None => self.face_grade.v,
        }
    }

    /// Horizontal depth of the face of the main wall.
    pub fn depth_m(&self) -> f64 {
        SMOOTHSTEP_PEAK * self.height_m.v / self.face_grade.v
    }

    /// Depth of the longest face (the chute's).
    pub fn max_depth_m(&self) -> f64 {
        SMOOTHSTEP_PEAK * self.height_m.v / self.chute.as_ref().map_or(self.face_grade.v, |c| c.grade.v)
    }

    /// Height of the scarp layer at a plan position (0 at the foot, `height_m` on the plateau, easing to 0 beyond the ends).
    pub fn height_at(&self, x: f64, z: f64) -> f64 {
        let (s, t) = self.to_st(x, z);
        if s <= 0.0 {
            return 0.0;
        }
        let depth = SMOOTHSTEP_PEAK * self.height_m.v / self.grade_at(t);
        let rise = scalar::smoothstep(0.0, 1.0, s / depth);
        let along =
            1.0 - scalar::smoothstep(self.length_m.v * 0.5, self.length_m.v * 0.5 + self.end_taper_m.v, t.abs());
        self.height_m.v * rise * along
    }

    /// True where the cliff's own (steeper) grade limit applies: the face, its ends and `margin_m` around them.
    pub fn in_zone(&self, x: f64, z: f64, margin_m: f64) -> bool {
        let (s, t) = self.to_st(x, z);
        s >= -margin_m
            && s <= self.max_depth_m() + margin_m
            && t.abs() <= self.length_m.v * 0.5 + self.end_taper_m.v + margin_m
    }

    /// True inside the chute's footprint (its notch in the face).
    pub fn in_chute(&self, x: f64, z: f64) -> bool {
        let Some(c) = &self.chute else { return false };
        let (s, t) = self.to_st(x, z);
        s >= 0.0 && s <= self.max_depth_m() && (t - c.t_m).abs() <= c.width_m.v * 0.5
    }

    /// Steepest grade the face can reach (the main wall; the chute is gentler).
    pub fn steepest_grade(&self) -> f64 {
        self.face_grade.v
    }
}

/// Width over which the chute's grade blends into the wall's, m (sets how sharp the chute's side walls are).
const CHUTE_BLEND_M: f64 = 3.0; // const-ok: shape of the chute edges

/// Inverse of `smoothstep(0, 1, x)` on [0, 1] by bisection (deterministic, fixed iteration count).
fn inv_smoothstep(y: f64) -> f64 {
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..60 {
        // const-ok: bisection steps, far below f64 resolution
        let mid = 0.5 * (lo + hi);
        if scalar::smoothstep(0.0, 1.0, mid) < y {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

/// The zig-zag road up the face as a dense plan path (about one point per metre), bottom to top: straight legs joined by semicircular
/// hairpins of radius `hairpin_radius_m`. Each leg climbs `H / legs` (the stations are placed where the face is at `k / legs` of its
/// height, so the legs are equal in rise). Refuses, with the numbers, a layout whose legs are too steep or too close together to hold
/// the road, or whose hairpins do not fit between the lanes.
pub fn switchback_path(
    cliff: &CliffDef,
    sb: &SwitchbackDef,
    road_width_m: f64,
    shoulder_m: f64,
    road_grade: f64,
    approach_m: f64,
) -> Result<Vec<(f64, f64)>, String> {
    let (h, depth, span, r) = (cliff.height_m.v, cliff.depth_m(), sb.span_m.v, sb.hairpin_radius_m.v);
    let want = road_grade * ROAD_GRADE_USE;
    let mut legs = None;
    for n in 1..=MAX_LEGS {
        // Each leg climbs h / n over a run of about hypot(span, rise of s per leg); the first n that is gentle enough wins.
        let run = scalar::hypot(span, depth / n as f64);
        if h / n as f64 / run <= want {
            legs = Some(n);
            break;
        }
    }
    let n = legs.ok_or_else(|| {
        format!(
            "switchback: even {MAX_LEGS} legs of {span} m cannot climb {h} m at grade {road_grade}; lengthen span_m"
        )
    })?;
    let spacing = depth / n as f64;
    let need = road_width_m + shoulder_m;
    if spacing < need {
        return Err(format!(
            "switchback: {n} legs on a {depth:.1} m deep face leave {spacing:.1} m between lanes but the road needs {need:.1} m; lengthen span_m or deepen the face"
        ));
    }
    if spacing < 2.0 * r || span < 4.0 * r {
        return Err(format!(
            "switchback: hairpins of radius {r} m need {:.1} m between lanes (have {spacing:.1}) and legs of at least {:.1} m (have {span}); reduce hairpin_radius_m",
            2.0 * r,
            4.0 * r
        ));
    }
    let side = |k: usize| if k.is_multiple_of(2) { -0.5 } else { 0.5 };
    let tk = |k: usize| sb.t_centre_m + side(k) * span;
    let stations: Vec<f64> =
        (0..=n).map(|k| if k == 0 { 0.0 } else { depth * inv_smoothstep(k as f64 / n as f64) }).collect();
    let mut out: Vec<(f64, f64)> = Vec::new();
    // Straight run from the current end to `to`, sampled every PATH_STEP_M.
    let line = |out: &mut Vec<(f64, f64)>, to: (f64, f64)| {
        let from = out.last().copied().unwrap_or(to);
        let len = scalar::hypot(to.0 - from.0, to.1 - from.1);
        let steps = (len / PATH_STEP_M).ceil().max(1.0) as usize;
        for i in 1..=steps {
            let f = i as f64 / steps as f64;
            out.push((from.0 + (to.0 - from.0) * f, from.1 + (to.1 - from.1) * f));
        }
    };
    out.push((-approach_m, tk(0)));
    for (k, &station) in stations.iter().enumerate().take(n).skip(1) {
        let dir = if side(k) > 0.0 { 1.0 } else { -1.0 };
        let tc = tk(k) - dir * r; // hairpin centre line along the face
        line(&mut out, (station - r, tc));
        let steps = ((std::f64::consts::PI * r) / PATH_STEP_M).ceil() as usize;
        for i in 1..=steps {
            let phi = std::f64::consts::PI * (1.0 - i as f64 / steps as f64);
            out.push((station + r * scalar::cos(phi), tc + dir * r * scalar::sin(phi)));
        }
    }
    line(&mut out, (stations[n], tk(n)));
    line(&mut out, (depth + approach_m, tk(n)));
    Ok(out.into_iter().map(|(s, t)| cliff.to_xz(s, t)).collect())
}

/// Spacing of the points of an authored path, m (one grid cell).
const PATH_STEP_M: f64 = 1.0; // const-ok: sampling interval of an authored road path
/// Longest leg count tried by the switchback layout.
const MAX_LEGS: usize = 12; // const-ok: more hairpins than this is not a road
/// Fraction of the stated road grade a layout may use, leaving margin for the profile's smoothing.
const ROAD_GRADE_USE: f64 = 0.9; // const-ok: safety factor on the stated grade
