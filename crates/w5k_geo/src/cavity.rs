//! The `cavity` flag: the blocked fraction of cosine-weighted Hammersley rays within a cap distance (ambient occlusion by ray casting).

use crate::bvh::Bvh;
use w5k_math::{scalar, Vec3};

/// Direction `i` of `n` in the local frame (y up), cosine-weighted: the (x, z) part is uniform on the unit disc.
fn local_dir(i: u32, n: u32) -> Vec3 {
    let u = (f64::from(i) + 0.5) / f64::from(n);
    let v = f64::from(i.reverse_bits()) / 4_294_967_296.0; // const-ok: 2^32, the base-2 radical inverse
    let (s, c) = scalar::sin_cos(2.0 * std::f64::consts::PI * v); // const-ok: pi
    let r = u.sqrt();
    Vec3::new(r * c, (1.0 - u).sqrt(), r * s)
}

/// Blocked fraction in 0..1 at point `p` with unit normal `nrm`, over `rays` rays, hits closer than `max_dist_m` count.
pub fn cavity_at(bvh: &Bvh, p: Vec3, nrm: Vec3, rays: u32, max_dist_m: f64, lift_m: f64) -> f64 {
    let helper = if nrm.x.abs() < 0.9 { Vec3::X } else { Vec3::Y }; // const-ok: pick the axis least aligned with the normal
    let t = nrm.cross(helper).normalized_or_zero();
    let b = nrm.cross(t);
    let o = p + nrm * lift_m;
    let blocked = (0..rays)
        .filter(|&i| {
            let l = local_dir(i, rays);
            bvh.any_hit(o, t * l.x + nrm * l.y + b * l.z, max_dist_m)
        })
        .count();
    blocked as f64 / f64::from(rays)
}

/// The closed form at the foot of a tall wall: the circular segment beyond `a = d / L`, as a fraction of the unit disc.
pub fn wall_foot_fraction(d_m: f64, cap_m: f64) -> f64 {
    let a = (d_m / cap_m).min(1.0);
    (scalar::acos(a) - a * (1.0 - a * a).sqrt()) / std::f64::consts::PI // const-ok: pi
}
