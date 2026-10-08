//! The integration spine's test: the first-light scenario must run, stay finite, behave sensibly and hash exactly as the golden says.
//!
//! The golden (`tests/golden/first_light.json`) is a hash chain of the simulation state, one entry per simulated second. It must be
//! identical on Linux and Windows (CI compares them). It changes only when the simulation changes on purpose:
//! `W5K_BLESS=1 cargo test -p w5k_sim --test first_light`, then say why in the PR description with a `Golden-Change:` line.
//! Never bless to make a failing test pass.

use std::path::PathBuf;

use w5k_sim::scenario::{first_light, FirstLightParts};

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("golden").join("first_light.json")
}

fn hex(h: u64) -> String {
    format!("{h:016x}")
}

#[test]
fn first_light_on_stand_ins_runs_and_behaves() {
    let r = first_light(FirstLightParts::stand_ins());
    let s = &r.summary;
    assert!(s.all_finite, "a frame contained a NaN or an infinity");
    assert_eq!(s.frames, 1200, "40 s at 30 Hz");
    assert!(s.distance_m > 150.0, "the truck should drive well down the strip, drove {} m", s.distance_m);
    assert!(s.max_speed_m_s > 8.0 && s.max_speed_m_s < 60.0, "top speed {} m/s", s.max_speed_m_s);
    assert!(s.final_speed_m_s < 0.2, "it should have braked to a stop, final speed {} m/s", s.final_speed_m_s);
    assert!(s.max_abs_travel_m > 0.01, "the suspension should move over the bumps");
    assert!(s.max_abs_pitch_deg > 0.2, "the body should pitch over the bumps");
    assert_eq!(r.header.state_hashes.len(), 40);
    assert_eq!(r.header.vehicles[0].joint_names.len(), r.frames[0].vehicles[0].joints.len());
}

#[test]
fn first_light_is_reproducible_within_a_process() {
    let a = first_light(FirstLightParts::stand_ins());
    let b = first_light(FirstLightParts::stand_ins());
    assert_eq!(a.final_hash, b.final_hash);
    assert_eq!(a.header.state_hashes, b.header.state_hashes);
}

#[test]
fn first_light_matches_the_golden_hash_chain() {
    let r = first_light(FirstLightParts::stand_ins());
    let now = serde_json::json!({
        "note": "Hash chain of the first-light simulation state, one entry per simulated second. See tests/first_light.rs before changing.",
        "final_hash": hex(r.final_hash),
        "state_hashes": r.header.state_hashes.iter().map(|h| hex(*h)).collect::<Vec<_>>(),
    });
    let path = golden_path();
    if std::env::var("W5K_BLESS").as_deref() == Ok("1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_string_pretty(&now).unwrap() + "\n").unwrap();
        eprintln!("blessed {}", path.display());
        return;
    }
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("missing golden {}: {e}. Bless it with W5K_BLESS=1.", path.display()));
    let golden: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        golden["state_hashes"], now["state_hashes"],
        "the first-light state hash chain changed. If that is deliberate, bless it (W5K_BLESS=1) and put a `Golden-Change:` line in the PR; otherwise this is a regression or a platform difference."
    );
    assert_eq!(golden["final_hash"], now["final_hash"]);
}
