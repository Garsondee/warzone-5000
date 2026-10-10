//! The reference soils of `content/physics/tracks/reference_soils.ron`: textbook cases with `n != 1` for the benches and the oracle tests.

use w5k_contract::world::MaterialDef;
use w5k_contract::Material;

/// `lete_sand` (n 0.79), `clayey` (n 0.5), `snow` (n 1.6), in that order. UNVALIDATED (see the file).
pub fn reference_soils() -> Vec<Material> {
    let defs: Vec<MaterialDef> = ron::from_str(include_str!("../../../content/physics/tracks/reference_soils.ron"))
        .expect("reference_soils.ron parses");
    defs.iter().map(|d| d.bake().expect("reference soil has valid Params")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_soils_load_with_exponents_other_than_one() {
        let s = reference_soils();
        let ns: Vec<f64> = s.iter().map(|m| m.soil.unwrap().n).collect();
        assert_eq!(ns, vec![0.79, 0.5, 1.6]);
    }
}
