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
    pub edge_band_m: f64,
    pub smooth_angle_deg: f64,
    pub max_edge_m: f64,
    pub cavity_rays: u32,
    pub preview_cavity_rays: u32,
    pub cavity_cap_m: f64,
    pub cavity_lift_m: f64,
}

impl FlagParams {
    pub fn default_params() -> FlagParams {
        ron::from_str(include_str!("../shapes/flags.ron")).expect("shapes/flags.ron parses")
    }

    /// The same bake with `rays` cavity rays per vertex. The cost is proportional to it, the positions, triangles and edge flags do not depend on
    /// it, and the cavity only gets noisier as it falls: a per-call knob for a caller that wants a cheaper skin.
    pub fn with_cavity_rays(self, rays: u32) -> FlagParams {
        FlagParams { cavity_rays: rays, ..self }
    }

    /// The bake for a preview skin (a Workshop slider moving): `preview_cavity_rays` rays, a small fraction of the default's cost.
    pub fn preview() -> FlagParams {
        let full = FlagParams::default_params();
        full.with_cavity_rays(full.preview_cavity_rays)
    }
}

/// One value in 0..1 per vertex, for each of `edge` and `cavity`.
pub struct Flags {
    pub edge: Vec<f64>,
    pub cavity: Vec<f64>,
}

/// Each part is first subdivided to `max_edge_m` (the flags need vertices where they vary), then `edge` is computed per part and `cavity`
/// on the union of all parts (they occlude each other). Returns the refined meshes with their flags.
pub fn bake(parts: &[&Mesh], p: &FlagParams) -> Vec<(Mesh, Flags)> {
    let parts: Vec<Mesh> = parts.iter().map(|m| m.subdivided(p.max_edge_m)).collect();
    let mut all = Mesh::default();
    for m in &parts {
        all.append(m);
    }
    let bvh = Bvh::build(&all);
    let rad = |d: f64| d * std::f64::consts::PI / 180.0; // const-ok: degrees to radians at the authoring edge
    parts
        .iter()
        .map(|m| {
            let normals = m.vertex_normals();
            let flags = Flags {
                edge: edge_values(m, rad(p.edge_ramp_lo_deg), rad(p.edge_ramp_hi_deg)).1,
                cavity: m
                    .v
                    .iter()
                    .zip(&normals)
                    .map(|(&v, &n)| cavity_at(&bvh, v, n, p.cavity_rays, p.cavity_cap_m, p.cavity_lift_m))
                    .collect(),
            };
            (m.clone(), flags)
        })
        .collect()
}
