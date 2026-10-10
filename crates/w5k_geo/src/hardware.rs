//! Small hardware shared by the mount and weapon families: bevelled boxes, cylinders on any axis and surfaces of revolution about Y, all
//! closed and outward-facing, in the frame of the part that uses them.

use crate::loft::{bevel_ring, loft_beveled, Section};
use crate::mesh::Mesh;
use crate::wheel::{cylinder_x, revolve};
use w5k_math::{Transform, Vec3};

/// A box of full size `size` (x, y, z) centred on `at`, with its edges cut by `corner_m` and vertex rows `band_m` beside them.
pub fn bevel_box(at: Vec3, size: [f64; 3], corner_m: f64, band_m: f64) -> Mesh {
    let [w, h, l] = size;
    let ring = |z: f64| Section { z_m: z, ring: bevel_ring(w, h, 0.0, corner_m, band_m) };
    // const-ok: a vertex at least every 10 m along the hard edges, far beyond any box here, so no subdivision
    let m = loft_beveled(&[ring(-l / 2.0), ring(l / 2.0)], corner_m, band_m, &[], 10.0);
    m.transformed(&Transform::from_pos(at))
}

/// A cylinder of radius `r` along Z from `z0` to `z1`, centred on the axis through (`x`, `y`).
pub fn cyl_z(x: f64, y: f64, r: f64, z0: f64, z1: f64, n: u32) -> Mesh {
    let mut m = cylinder_x(r, z0, z1, n);
    m.v = m.v.iter().map(|p| Vec3::new(p.y + x, p.z + y, p.x)).collect();
    m
}

/// A cylinder of radius `r` along Y from `y0` to `y1`, centred on the axis through (`x`, `z`).
pub fn cyl_y(x: f64, z: f64, r: f64, y0: f64, y1: f64, n: u32) -> Mesh {
    let mut m = cylinder_x(r, y0, y1, n);
    m.v = m.v.iter().map(|p| Vec3::new(p.z + x, p.x, p.y + z)).collect();
    m
}

/// A cylinder of radius `r` along X from `x0` to `x1`, centred on the axis through (`y`, `z`).
pub fn cyl_x(y: f64, z: f64, r: f64, x0: f64, x1: f64, n: u32) -> Mesh {
    let mut m = cylinder_x(r, x0, x1, n);
    m.v = m.v.iter().map(|p| Vec3::new(p.x, p.y + y, p.z + z)).collect();
    m
}

/// Revolve a closed (rho, y) profile about the +Y axis.
pub fn revolve_y(profile: &[(f64, f64)], n: u32) -> Mesh {
    let m = revolve(profile, n);
    Mesh { v: m.v.iter().map(|p| Vec3::new(p.z, p.x, p.y)).collect(), t: m.t }
}
