//! The design audit (`w5k.audit.v1`): consequences with units, provenance and because-lines, in the shape the Workshop reads.

use w5k_forge::audit::{audit, Audit, Row, SCHEMA};
use w5k_forge::compile::{compile, parse_def, parse_extras};
use w5k_forge::levers::apply_both;

const IDS: [&str; 4] = ["scout_4x4", "mule_4x4", "hauler_4x4", "carrier_tracked"];

fn audit_of(id: &str, lever: Option<(&str, f64)>) -> Audit {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../content/vehicles/game/");
    let mut d = parse_def(&std::fs::read_to_string(format!("{dir}{id}.ron")).unwrap()).unwrap();
    let mut x = parse_extras(&std::fs::read_to_string(format!("{dir}{id}.extras.ron")).unwrap()).unwrap();
    if let Some((name, f)) = lever {
        (d, x) = apply_both(&d, &x, name, f).unwrap();
    }
    let c = compile(&d, &x).unwrap_or_else(|e| panic!("{e:?}"));
    audit(&d, &x, &c)
}

fn row<'a>(a: &'a Audit, id: &str) -> &'a Row {
    a.consequences.iter().find(|r| r.id == id).unwrap_or_else(|| panic!("no row {id}"))
}

#[test]
fn every_row_has_a_unit_a_because_line_a_provenance_and_a_unique_id() {
    for id in IDS {
        let a = audit_of(id, None);
        assert_eq!(a.schema, SCHEMA);
        let mut ids: Vec<&str> = a.consequences.iter().map(|r| r.id.as_str()).collect();
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n, "{id}: ids are unique");
        for r in &a.consequences {
            assert!(!r.because.is_empty() && !r.label.is_empty(), "{id} {}", r.id);
            assert!(["ESTIMATE", "TUNED", "MEASURED", "SPEC"].contains(&r.provenance), "{id} {}", r.id);
            assert!(r.value.is_finite(), "{id} {}", r.id);
        }
    }
}

#[test]
fn the_json_has_the_shape_the_workshop_reads() {
    let a = audit_of("mule_4x4", None);
    let v: serde_json::Value = serde_json::to_value(&a).unwrap();
    assert_eq!(v["schema"], "w5k.audit.v1");
    let rows = v["consequences"].as_array().unwrap();
    assert!(!rows.is_empty() && v["flags"].is_array());
    for r in rows {
        for key in ["id", "label", "value", "unit", "because"] {
            assert!(r.get(key).is_some(), "row lacks {key}: {r}");
        }
    }
    // A row of several numbers carries `text`, and `value` is the first of them.
    let share = rows.iter().find(|r| r["id"] == "balance.front_share").unwrap();
    assert!(share["text"].as_str().unwrap().contains(" / "));
}

#[test]
fn the_mass_rows_sum_to_the_sprung_mass_and_the_total_follows() {
    for id in IDS {
        let a = audit_of(id, None);
        let parts: f64 = a
            .consequences
            .iter()
            .filter(|r| r.id.starts_with("mass.") && !["mass.total", "mass.sprung"].contains(&r.id.as_str()))
            .map(|r| r.value)
            .sum();
        let sprung = row(&a, "mass.sprung").value;
        assert!((parts - sprung).abs() < 1e-6 * sprung, "{id}: parts {parts} vs sprung {sprung}");
        assert!(row(&a, "mass.total").value >= sprung, "{id}");
    }
}

#[test]
fn a_heavier_engine_moves_the_front_axle_share_toward_the_front() {
    let base = audit_of("mule_4x4", None);
    let big = audit_of("mule_4x4", Some(("engine_peak_power", 1.5)));
    assert!(row(&big, "balance.front_share").value > row(&base, "balance.front_share").value);
    assert!(row(&big, "mass.engine").value > 1.49 * row(&base, "mass.engine").value);
    assert!(row(&big, "mass.total").value > row(&base, "mass.total").value);
}

#[test]
fn power_to_weight_is_peak_power_over_total_mass_and_says_so() {
    let a = audit_of("hauler_4x4", None);
    let r = row(&a, "power.to_weight");
    let total = row(&a, "mass.total").value;
    assert!((r.value - 135.0 / (total / 1e3)).abs() < 1e-9, "{}", r.value);
    assert!(r.because.contains("135 kW") && r.because.contains("total mass"), "{}", r.because);
}

#[test]
fn the_gyration_row_matches_the_tensor_and_the_ride_rows_carry_their_bands() {
    let a = audit_of("scout_4x4", None);
    let g = row(&a, "balance.gyration");
    assert!(g.text.as_ref().unwrap().contains("roll") && g.because.contains("parallel-axis"));
    let f = row(&a, "ride.frequency");
    assert!(f.lo.unwrap() <= f.value && f.value <= f.hi.unwrap());
    // Bump capacity is travel over static sag.
    let sag = row(&a, "ride.static_sag").value;
    assert!((row(&a, "ride.bump_capacity").value - 0.22 / sag).abs() < 1e-9);
}
