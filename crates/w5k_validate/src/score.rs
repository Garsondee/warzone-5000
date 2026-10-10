//! Scoring: a design sheet and a measured replay against a dossier, one row per scored dossier quantity.

use w5k_contract::def::VehicleDef;

use crate::dossier::Dossier;
use crate::measure::Measured;
use crate::verdict::{judge, not_measured, Class, Row};

/// What the replay is a test of. A replay's peak speed is only the vehicle's top speed if the drive was a top-speed run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunKind {
    TopSpeed,
    Other,
}

/// Score every non-cross quantity of `dossier`. Quantities no scenario can measure yet come back as `NotMeasured`: the dashboard
/// lists them, so the gap is visible.
pub fn score(dossier: &Dossier, def: &VehicleDef, m: &Measured, kind: RunKind) -> Vec<Row> {
    dossier
        .scored()
        .map(|q| match q.id.as_str() {
            "static.length_m" => judge(q, def.hull.length_m.v, Class::Static),
            "static.width_m" => judge(q, def.hull.width_m.v, Class::Static),
            "static.height_m" => judge(q, def.hull.height_m.v, Class::Static),
            "powertrain.top_speed_m_s" if kind == RunKind::TopSpeed => judge(q, m.peak_speed_m_s, Class::TopSpeed),
            "powertrain.top_speed_m_s" => {
                not_measured(q, &format!("this replay is not a top-speed run (peak {:.1} m/s is only a lower bound)", m.peak_speed_m_s))
            }
            "static.mass_curb_kg" | "static.mass_gross_kg" => {
                not_measured(q, "the design sheet's hull mass excludes unsprung mass and the loading differs; needs a total-mass accessor")
            }
            _ => not_measured(q, "no scenario measures this yet"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dossier::Dossier;
    use crate::verdict::Light;

    fn m998() -> Dossier {
        Dossier::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/dossier/m998.ron"))
            .expect("m998")
    }

    fn mule() -> VehicleDef {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/vehicles/game/mule_4x4.ron"),
        )
        .expect("mule");
        ron::from_str(&text).expect("parse mule")
    }

    fn measured(peak: f64) -> Measured {
        Measured { peak_speed_m_s: peak, time_to_32kmh_s: None, distance_m: 0.0, frame_dt_s: 1.0 / 30.0 }
    }

    #[test]
    fn every_scored_dossier_quantity_gets_a_row_and_unmeasured_ones_say_so() {
        let d = m998();
        let rows = score(&d, &mule(), &measured(10.0), RunKind::Other);
        assert_eq!(rows.len(), d.scored().count());
        assert!(rows.iter().filter(|r| r.light == Light::NotMeasured).all(|r| !r.note.is_empty()));
    }

    #[test]
    fn harness_reports_red_for_a_deliberately_wrong_model() {
        // Negative control: a model that tops out at twice the published governed top speed must not pass the top-speed check.
        let d = m998();
        let published =
            d.quantities.iter().find(|q| q.id == "powertrain.top_speed_m_s").expect("top speed").param.band().1;
        let honest = score(&d, &mule(), &measured(published), RunKind::TopSpeed);
        let wrong = score(&d, &mule(), &measured(2.0 * published), RunKind::TopSpeed);
        let light =
            |rows: &[crate::verdict::Row]| rows.iter().find(|r| r.id == "powertrain.top_speed_m_s").expect("row").light;
        assert_eq!(light(&honest), Light::Green);
        assert_eq!(light(&wrong), Light::Red);
    }

    #[test]
    fn a_top_speed_check_is_not_scored_from_a_replay_that_was_not_a_top_speed_run() {
        let rows = score(&m998(), &mule(), &measured(10.0), RunKind::Other);
        assert_eq!(rows.iter().find(|r| r.id == "powertrain.top_speed_m_s").expect("row").light, Light::NotMeasured);
    }
}
