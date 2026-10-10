//! The bake's quality knob: a preview bake is the full bake with fewer cavity rays, so a caller that wants a cheap skin changes nothing but the
//! cavity's sampling noise.

use w5k_geo::flags::{bake, FlagParams};
use w5k_geo::wheel::{segments_for, wheel, WheelDims};

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

#[test]
fn a_preview_bake_is_the_full_bake_with_a_sixteenth_of_the_cavity_rays_and_nothing_else_changed_but_sampling_noise() {
    let (full, preview) = (FlagParams::default_params(), FlagParams::preview());
    assert!(preview.cavity_rays * 8 <= full.cavity_rays, "{} rays against {}", preview.cavity_rays, full.cavity_rays);
    assert_eq!(preview.cavity_rays, full.preview_cavity_rays);
    let w = wheel(&WheelDims::placeholder(), segments_for(0));
    let parts = [&w.tyre, &w.rim, &w.lugs];
    let (a, b) = (bake(&parts, &full), bake(&parts, &preview));
    assert_eq!(a.len(), b.len());
    for ((ma, fa), (mb, fb)) in a.iter().zip(&b) {
        assert!(ma.v == mb.v && ma.t == mb.t, "the meshes do not depend on the ray count");
        assert!(fa.edge == fb.edge, "the edge flags do not depend on the ray count");
        assert_eq!(fa.cavity.len(), fb.cavity.len());
        let per_vertex =
            fa.cavity.iter().zip(&fb.cavity).map(|(x, y)| (x - y).abs()).sum::<f64>() / fa.cavity.len() as f64;
        let bias = (mean(&fa.cavity) - mean(&fb.cavity)).abs();
        assert!(bias < 0.04, "the preview's mean cavity differs by {bias:.3}: a bias, not noise");
        assert!(
            per_vertex < 0.2,
            "the preview's cavity differs by {per_vertex:.3} a vertex: more than 16 rays of noise"
        );
    }
}

#[test]
fn the_ray_count_is_a_per_call_parameter() {
    let one = FlagParams::default_params().with_cavity_rays(1);
    assert_eq!((one.cavity_rays, one.edge_band_m), (1, FlagParams::default_params().edge_band_m));
}
