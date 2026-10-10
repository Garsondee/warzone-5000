//! Wheels: a tyre carcass revolved from a profile, tread lugs, a disc rim and wheel nuts. Hub at the origin, axle along local X, the
//! outboard side towards +X (a left wheel is the mirror). The outer radius of the lugs is exactly the stated radius.

use crate::loft::{loft, rect_ring, Section};
use crate::mesh::Mesh;
use serde::Deserialize;
use w5k_math::{scalar, Transform, Vec3};

const TEMPLATE: &str = include_str!("../shapes/wheel.ron");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WheelTemplate {
    tyre_profile: Vec<(f64, f64)>,
    rim_profile: Vec<(f64, f64)>,
    lug_rows: Vec<(f64, f64)>,
    lug_length_frac: f64,
    lug_width_frac: f64,
    lug_taper: f64,
    nuts: u32,
    nut_pitch_frac: f64,
    nut_radius_frac: f64,
    nut_height_frac: f64,
}

/// Real dimensions, from the `VehicleDef`'s tyre sliders (metres) and the tread description.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WheelDims {
    pub outer_radius_m: f64,
    pub width_m: f64,
    pub rim_radius_m: f64,
    pub lug_depth_m: f64,
    pub lugs_around: u32,
}

impl WheelDims {
    /// Placeholder dimensions (`shapes/placeholder_wheel.ron`, `PROVISIONAL(C-002)`) until FORGE supplies the real tyre Params.
    pub fn placeholder() -> WheelDims {
        ron::from_str(include_str!("../shapes/placeholder_wheel.ron")).expect("shapes/placeholder_wheel.ron parses")
    }
}

pub struct Wheel {
    pub tyre: Mesh,
    pub lugs: Mesh,
    pub rim: Mesh,
    pub nuts: Mesh,
}

/// Circle segments for a `detail` level: 0 coarse, 1 default, 2 reference.
pub fn segments_for(detail: u8) -> u32 {
    [24, 48, 128][usize::from(detail.min(2))] // const-ok: tessellation levels (docs/lanes/geometry/design-note.md section 7)
}

/// Revolve a closed profile of (rho, x) points about the X axis; angle 0 is straight down (-Y).
pub fn revolve(profile: &[(f64, f64)], segments: u32) -> Mesh {
    let n = profile.len() as u32;
    let mut m = Mesh::default();
    for s in 0..segments {
        let (sin, cos) = scalar::sin_cos(2.0 * std::f64::consts::PI * f64::from(s) / f64::from(segments)); // const-ok: pi
        m.v.extend(profile.iter().map(|&(rho, x)| Vec3::new(x, -rho * cos, rho * sin)));
    }
    for s in 0..segments {
        let t = (s + 1) % segments;
        for i in 0..n {
            let j = (i + 1) % n;
            m.t.push([s * n + i, s * n + j, t * n + j]);
            m.t.push([s * n + i, t * n + j, t * n + i]);
        }
    }
    m.orient_outward();
    m
}

/// Place a mesh built in a local frame (x across, y tangent, z radial outward) on the wheel at angle `theta` (0 = down) and axial offset `x`.
fn on_wheel(local: &Mesh, theta: f64, x: f64, rho: f64) -> Mesh {
    let (sin, cos) = scalar::sin_cos(theta);
    let (radial, tangent) = (Vec3::new(0.0, -cos, sin), Vec3::new(0.0, sin, cos));
    Mesh {
        v: local.v.iter().map(|p| Vec3::X * (p.x + x) + tangent * p.y + radial * (p.z + rho)).collect(),
        t: local.t.clone(),
    }
}

/// A closed cylinder along +X from `x0` to `x1`.
pub fn cylinder_x(r: f64, x0: f64, x1: f64, n: u32) -> Mesh {
    let ring: Vec<[f64; 2]> = (0..n)
        .map(|i| {
            let (s, c) = scalar::sin_cos(-2.0 * std::f64::consts::PI * f64::from(i) / f64::from(n));
            [r * c, r * s]
        })
        .collect(); // const-ok: pi
    let m = Mesh::extrude_fan(&ring, 0, x1 - x0);
    let mut m = Mesh { v: m.v.iter().map(|p| Vec3::new(p.y + x0, p.x, p.z)).collect(), t: m.t };
    m.orient_outward();
    m
}

pub fn wheel(d: &WheelDims, segments: u32) -> Wheel {
    let tpl: WheelTemplate = ron::from_str(TEMPLATE).expect("shapes/wheel.ron parses");
    let tread_r = d.outer_radius_m - d.lug_depth_m;
    let tyre_pts: Vec<(f64, f64)> = tpl
        .tyre_profile
        .iter()
        .map(|&(h, x)| (d.rim_radius_m + h * (tread_r - d.rim_radius_m), x * d.width_m))
        .collect();
    let rim_pts: Vec<(f64, f64)> =
        tpl.rim_profile.iter().map(|&(r, x)| (r * d.rim_radius_m * 0.98, x * d.width_m)).collect(); // const-ok: seat clearance so the bead sits outside the rim
    let pitch = 2.0 * std::f64::consts::PI / f64::from(d.lugs_around); // const-ok: pi
    let lug = {
        let (len, wid) = (tread_r * pitch * tpl.lug_length_frac, d.width_m * tpl.lug_width_frac);
        let sec = |w: f64, l: f64, z: f64| Section { z_m: z, ring: rect_ring(w, l, 0.0) };
        let embed = d.lug_depth_m * 0.5; // const-ok: a lug sits half its depth inside the carcass
        loft(
            &[sec(wid, len, -embed), sec(wid, len, 0.0), sec(wid * tpl.lug_taper, len * tpl.lug_taper, d.lug_depth_m)],
            // const-ok: a step longer than any lug: no subdivision
            10.0,
        )
    };
    let mut lugs = Mesh::default();
    for &(xc, phase) in &tpl.lug_rows {
        for k in 0..d.lugs_around {
            lugs.append(&on_wheel(&lug, (f64::from(k) + phase) * pitch, xc * d.width_m, tread_r));
        }
    }
    let mut nuts = Mesh::default();
    for k in 0..tpl.nuts {
        let a = 2.0 * std::f64::consts::PI * f64::from(k) / f64::from(tpl.nuts); // const-ok: pi
        let nut = cylinder_x(tpl.nut_radius_frac * d.rim_radius_m, 0.0, tpl.nut_height_frac * d.width_m, 8); // const-ok: hexagon-ish nut with 8 sides
        nuts.append(&on_wheel(&nut, a, 0.28 * d.width_m, tpl.nut_pitch_frac * d.rim_radius_m));
    }
    Wheel { tyre: revolve(&tyre_pts, segments), lugs, rim: revolve(&rim_pts, segments), nuts }
}

impl Wheel {
    pub fn mirrored_x(&self) -> Wheel {
        Wheel {
            tyre: self.tyre.mirrored_x(),
            lugs: self.lugs.mirrored_x(),
            rim: self.rim.mirrored_x(),
            nuts: self.nuts.mirrored_x(),
        }
    }

    pub fn placed(&self, t: &Transform) -> Wheel {
        Wheel {
            tyre: self.tyre.transformed(t),
            lugs: self.lugs.transformed(t),
            rim: self.rim.transformed(t),
            nuts: self.nuts.transformed(t),
        }
    }
}
