//! Directional armour and silhouette tables.
//!
//! For each attack direction a grid of parallel rays is fired at the pieces. Each ray is intersected
//! **exactly** with every convex piece (clipping a line against its planes), so thickness is not limited by
//! any voxel size: a 60 mm plate reads as 60 mm.
//!
//! A shell piece of thickness `s` is the region between its outer planes and the same planes moved inward by
//! `s` (for a convex shape, moving every face inward is exactly the inset shape). Along a ray it therefore
//! contributes material `[entry, inner entry)` and `[inner exit, exit)` with interior between. We add up
//! material thickness (weighted by hardness) from where the ray first meets the vehicle to the first point
//! that is interior and not covered by other material. Because a ray crosses a plate of thickness `t` tilted
//! by angle `a` over a path `t / cos(a)`, **sloped armour emerges from the geometry** with no special rule.
//!
//! Combat later samples these tables instead of firing rays per shot.

use crate::build::Piece;
use crate::geom::{v3, Plane, V3};
use crate::schema::MaterialLibrary;

/// Elevations of the table rows (degrees above the horizon, attacker relative to the target).
pub const ELEVATIONS: [f64; 8] = [-30.0, -10.0, 0.0, 10.0, 25.0, 45.0, 65.0, 90.0];
/// Number of azimuth columns (0 = attacker in front, increasing clockwise seen from above: 90 = right side).
pub const AZIMUTHS: usize = 32;
/// Rays across the bounding sphere's diameter for each direction.
pub const RAYS_ACROSS: usize = 80;

#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DirStats {
    /// Projected area that a shot from this direction can hit (m^2).
    pub area_m2: f64,
    /// Part of that area where a shot reaches the interior (m^2).
    pub vital_m2: f64,
    /// Mean steel-equivalent thickness protecting the interior (mm). Sensitive to rare, very long paths (a ray
    /// running down the length of a track), so the median is the better "typical" figure.
    pub mean_mm: f64,
    /// Median steel-equivalent thickness protecting the interior (mm): what a typical hit faces.
    #[serde(default)]
    pub median_mm: f64,
    /// 10th-percentile thickness: the weak spots (mm).
    pub weak_mm: f64,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ArmourTable {
    pub elevations: Vec<f64>,
    pub azimuths: usize,
    /// `rows[elevation][azimuth]`.
    pub rows: Vec<Vec<DirStats>>,
}

impl ArmourTable {
    /// Stats for the nearest tabulated direction.
    pub fn at(&self, azimuth_deg: f64, elevation_deg: f64) -> &DirStats {
        let row = self
            .elevations
            .iter()
            .enumerate()
            .min_by(|a, b| (a.1 - elevation_deg).abs().total_cmp(&(b.1 - elevation_deg).abs()))
            .map(|(i, _)| i)
            .unwrap_or(0);
        let col = ((azimuth_deg.rem_euclid(360.0) / 360.0 * self.azimuths as f64).round() as usize) % self.azimuths;
        &self.rows[row][col]
    }
}

/// Unit vector from the target toward the attacker.
pub fn direction(azimuth_deg: f64, elevation_deg: f64) -> V3 {
    let (sa, ca) = azimuth_deg.to_radians().sin_cos();
    let (se, ce) = elevation_deg.to_radians().sin_cos();
    v3(sa * ce, se, -ca * ce)
}

/// A piece prepared for ray casting.
struct Target {
    centre: V3,
    radius: f64,
    lo: V3,
    hi: V3,
    outer: Vec<Plane>,
    /// Inset planes of a shell piece (`None` for solid pieces).
    inner: Option<Vec<Plane>>,
    /// Whether the shell's inside is vital space (otherwise it is air between two walls).
    vital: bool,
    hardness: f64,
}

/// Pieces prepared for ray casting. Pieces made of non-protective materials (guns, engines, electronics) and
/// scenery are transparent here.
fn targets_of(pieces: &[Piece], lib: &MaterialLibrary) -> Vec<Target> {
    pieces
        .iter()
        .filter(|p| !p.is_scenery() && lib.materials.get(&p.mat).map(|m| m.armour).unwrap_or(true))
        .map(|p| {
            let (lo, hi) = p.poly.aabb();
            let centre = (lo + hi) * 0.5;
            let radius = p.poly.verts.iter().fold(0.0f64, |m, v| m.max((*v - centre).len()));
            Target {
                centre,
                radius,
                lo,
                hi,
                outer: p.convex.planes.clone(),
                inner: p.inner().map(|c| c.planes),
                vital: p.vital,
                hardness: lib.materials.get(&p.mat).map(|m| m.hardness).unwrap_or(1.0),
            }
        })
        .collect()
}

pub fn armour_table(pieces: &[Piece], lib: &MaterialLibrary) -> ArmourTable {
    let targets = targets_of(pieces, lib);

    // Rows are independent, so each elevation runs on its own thread; the result does not depend on scheduling.
    let rows = std::thread::scope(|s| {
        let handles: Vec<_> = ELEVATIONS
            .iter()
            .map(|&el| {
                let targets = &targets;
                s.spawn(move || {
                    (0..AZIMUTHS)
                        .map(|a| cast_direction(targets, direction(a as f64 * 360.0 / AZIMUTHS as f64, el), RAYS_ACROSS))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().expect("armour thread panicked")).collect()
    });
    ArmourTable { elevations: ELEVATIONS.to_vec(), azimuths: AZIMUTHS, rows }
}

/// The four directions the vehicle sheet reads: from the front, the right side, the rear and from above.
#[derive(Clone, Copy, Debug, Default)]
pub struct ArmourSummary {
    pub front: DirStats,
    pub side: DirStats,
    pub rear: DirStats,
    pub top: DirStats,
}

impl ArmourSummary {
    pub fn from_table(t: &ArmourTable) -> ArmourSummary {
        ArmourSummary { front: *t.at(0.0, 0.0), side: *t.at(90.0, 0.0), rear: *t.at(180.0, 0.0), top: *t.at(0.0, 90.0) }
    }
}

/// A fast armour summary: only the four directions of [`ArmourSummary`], with fewer rays. About a hundred times
/// cheaper than the full table, and accurate to a few per cent: for sampling thousands of designs.
pub fn quick(pieces: &[Piece], lib: &MaterialLibrary) -> ArmourSummary {
    let targets = targets_of(pieces, lib);
    const N: usize = 40;
    ArmourSummary {
        front: cast_direction(&targets, direction(0.0, 0.0), N),
        side: cast_direction(&targets, direction(90.0, 0.0), N),
        rear: cast_direction(&targets, direction(180.0, 0.0), N),
        top: cast_direction(&targets, direction(0.0, 90.0), N),
    }
}

/// Fire a grid of parallel rays travelling along `-f` over the vehicle's projected outline (about `n` x `n` rays,
/// laid out over the outline's bounding rectangle so long, thin vehicles are sampled as finely as compact ones).
fn cast_direction(targets: &[Target], f: V3, n: usize) -> DirStats {
    if targets.is_empty() {
        return DirStats::default();
    }
    let up = if f.y.abs() < 0.99 { v3(0.0, 1.0, 0.0) } else { v3(1.0, 0.0, 0.0) };
    let u = up.cross(f).norm();
    let w = f.cross(u);
    // Bounding rectangle of the projection (from the pieces' bounding boxes) and the overall extent.
    let (mut u0, mut u1, mut w0, mut w1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    let (mut lo, mut hi) = (v3(f64::MAX, f64::MAX, f64::MAX), v3(f64::MIN, f64::MIN, f64::MIN));
    for t in targets {
        lo = lo.min(t.lo);
        hi = hi.max(t.hi);
        for k in 0..8 {
            let c = v3(if k & 1 == 0 { t.lo.x } else { t.hi.x }, if k & 2 == 0 { t.lo.y } else { t.hi.y }, if k & 4 == 0 { t.lo.z } else { t.hi.z });
            let (pu, pw) = (c.dot(u), c.dot(w));
            u0 = u0.min(pu);
            u1 = u1.max(pu);
            w0 = w0.min(pw);
            w1 = w1.max(pw);
        }
    }
    let radius = (hi - lo).len() * 0.5;
    let centre = (lo + hi) * 0.5;
    let (ru, rw) = ((u1 - u0).max(1e-6), (w1 - w0).max(1e-6));
    let spacing = (ru * rw / (n * n) as f64).sqrt();
    let (nu, nw) = (((ru / spacing).ceil() as usize).max(1), ((rw / spacing).ceil() as usize).max(1));
    let (du, dw) = (ru / nu as f64, rw / nw as f64);
    let (cu, cw) = (centre.dot(u), centre.dot(w));
    let mut hit = 0usize;
    let mut thick: Vec<f64> = Vec::new();
    for j in 0..nw {
        for i in 0..nu {
            let (x, y) = (u0 + (i as f64 + 0.5) * du - cu, w0 + (j as f64 + 0.5) * dw - cw);
            let o = centre + u * x + w * y + f * (radius * 2.0);
            match cast_ray(targets, o, -f) {
                Ray::Miss => {}
                Ray::External => hit += 1,
                Ray::Vital(t) => {
                    hit += 1;
                    thick.push(t);
                }
            }
        }
    }
    thick.sort_by(f64::total_cmp);
    let mean = if thick.is_empty() { 0.0 } else { thick.iter().sum::<f64>() / thick.len() as f64 };
    let weak = if thick.is_empty() { 0.0 } else { thick[thick.len() / 10] };
    let median = if thick.is_empty() { 0.0 } else { thick[thick.len() / 2] };
    let ray_area = du * dw;
    DirStats {
        area_m2: hit as f64 * ray_area,
        vital_m2: thick.len() as f64 * ray_area,
        mean_mm: mean * 1000.0,
        median_mm: median * 1000.0,
        weak_mm: weak * 1000.0,
    }
}

pub(crate) enum Ray {
    Miss,
    /// Hit material but never reached the interior (an external fitting such as a gun barrel or a track).
    External,
    /// Reached the interior after this much steel-equivalent material (m).
    Vital(f64),
}

/// The part of the line `o + t d` inside every half-space, as `[t0, t1)`.
pub(crate) fn clip(planes: &[Plane], o: V3, d: V3) -> Option<(f64, f64)> {
    let mut t0 = f64::NEG_INFINITY;
    let mut t1 = f64::INFINITY;
    for pl in planes {
        let denom = pl.n.dot(d);
        let room = pl.d - pl.n.dot(o); // inside while t * denom <= room
        if denom.abs() < 1e-12 {
            if room < 0.0 {
                return None;
            }
            continue;
        }
        let t = room / denom;
        if denom > 0.0 {
            t1 = t1.min(t);
        } else {
            t0 = t0.max(t);
        }
        if t0 >= t1 {
            return None;
        }
    }
    Some((t0, t1))
}

/// A segment along a ray, `[a, b)`, with the hardness of the piece it belongs to.
struct Seg {
    a: f64,
    b: f64,
    h: f64,
}

fn covers(segs: &[Seg], x: f64) -> Option<f64> {
    segs.iter().filter(|s| x >= s.a && x < s.b).map(|s| s.h).reduce(f64::max)
}

/// Classify the ray with the same rule as the voxel grid: solid pieces are always material; otherwise any
/// shell's interior wins (overlapping hollow pieces form one hollow volume); otherwise a shell's outer volume
/// is wall material.
fn cast_ray(targets: &[Target], o: V3, d: V3) -> Ray {
    let mut solids: Vec<Seg> = Vec::new();
    let mut walls: Vec<Seg> = Vec::new();
    let mut ints: Vec<Seg> = Vec::new();
    for t in targets {
        // Cheap rejection: distance from the sphere centre to the ray.
        let oc = t.centre - o;
        let along = oc.dot(d);
        if (oc - d * along).len() > t.radius {
            continue;
        }
        let Some((a0, a1)) = clip(&t.outer, o, d) else { continue };
        match &t.inner {
            None => solids.push(Seg { a: a0, b: a1, h: t.hardness }),
            Some(inner) => match clip(inner, o, d) {
                Some((b0, b1)) if t.vital => {
                    walls.push(Seg { a: a0, b: a1, h: t.hardness });
                    ints.push(Seg { a: b0, b: b1, h: 0.0 });
                }
                // A non-vital hollow: two walls with air between.
                Some((b0, b1)) => {
                    walls.push(Seg { a: a0, b: b0, h: t.hardness });
                    walls.push(Seg { a: b1, b: a1, h: t.hardness });
                }
                None => walls.push(Seg { a: a0, b: a1, h: t.hardness }),
            },
        }
    }
    if solids.is_empty() && walls.is_empty() {
        return Ray::Miss;
    }
    // The first vital point is where an interior starts, or where a solid ends inside an interior. Each candidate is
    // classified a hair past the boundary, so pieces that merely touch (a plate laid against an inner wall) are
    // judged the same way whichever way rounding falls.
    const EPS: f64 = 1e-6;
    let mut cands: Vec<f64> = ints.iter().map(|i| i.a).chain(solids.iter().map(|s| s.b)).collect();
    cands.sort_by(f64::total_cmp);
    let vital = cands.into_iter().find(|&c| covers(&ints, c + EPS).is_some() && covers(&solids, c + EPS).is_none());
    let Some(vital) = vital else { return Ray::External };
    // Steel-equivalent thickness before `vital`; where pieces overlap, the hardest material counts.
    let mut cuts: Vec<f64> = solids.iter().chain(&walls).flat_map(|s| [s.a, s.b]).filter(|&x| x < vital).chain([vital]).collect();
    cuts.sort_by(f64::total_cmp);
    let mut acc = 0.0;
    for wnd in cuts.windows(2) {
        let mid = 0.5 * (wnd[0] + wnd[1]);
        let h = covers(&solids, mid).or_else(|| if covers(&ints, mid).is_some() { None } else { covers(&walls, mid) }).unwrap_or(0.0);
        acc += (wnd[1] - wnd[0]) * h;
    }
    Ray::Vital(acc)
}
