//! Baking the per-vertex `edge` and `cavity` flags for a set of parts in their assembled pose.

use crate::bvh::Bvh;
use crate::cavity::cavity_at;
use crate::edge::edge_values;
use crate::mesh::Mesh;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlagParams {
    pub edge_ramp_lo_deg: f64,
    pub edge_ramp_hi_deg: f64,
    pub cavity_rays: u32,
    pub cavity_cap_m: f64,
    pub cavity_lift_m: f64,
}

impl FlagParams {
    pub fn default_params() -> FlagParams {
        ron::from_str(include_str!("../shapes/flags.ron")).expect("shapes/flags.ron parses")
    }
}

/// One value in 0..1 per vertex, for each of `edge` and `cavity`.
pub struct Flags {
    pub edge: Vec<f64>,
    pub cavity: Vec<f64>,
}

/// `edge` per part (its own geometry); `cavity` on the union of all parts (they occlude each other).
pub fn bake(parts: &[&Mesh], p: &FlagParams) -> Vec<Flags> {
    let mut all = Mesh::default();
    for m in parts {
        all.append(m);
    }
    let bvh = Bvh::build(&all);
    let rad = |d: f64| d * std::f64::consts::PI / 180.0; // const-ok: degrees to radians at the authoring edge
    parts
        .iter()
        .map(|m| {
            let normals = m.vertex_normals();
            Flags {
                edge: edge_values(m, rad(p.edge_ramp_lo_deg), rad(p.edge_ramp_hi_deg)).1,
                cavity: m
                    .v
                    .iter()
                    .zip(&normals)
                    .map(|(&v, &n)| cavity_at(&bvh, v, n, p.cavity_rays, p.cavity_cap_m, p.cavity_lift_m))
                    .collect(),
            }
        })
        .collect()
}
