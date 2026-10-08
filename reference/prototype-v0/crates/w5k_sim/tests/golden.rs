//! Golden replays: every design of the army, baked, run on the hill-and-valley course, and its final state hash compared with the one
//! committed in `fixtures/roster.json`.
//!
//! This is the determinism gate of `docs/design/02-determinism-rules.md`: a run must be bit-identical on every machine, every build
//! profile and every thread count. If a hash here changes, either the simulation changed (then the change must have been meant, and the
//! fixture regenerated with `w5k trial content --out DIR --golden crates/w5k_sim/tests/fixtures/roster.json`) or something that must
//! never vary (a platform, a thread count, an uninitialised value) did.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;
use w5k_sim::trial::run;
use w5k_sim::{Course, CourseDef, MoverSpec, Outcome, Run, TerrainDef};

#[derive(Deserialize)]
struct Entry {
    id: String,
    hash: String,
    outcome: serde_json::Value,
    ticks: usize,
    spec: MoverSpec,
}

#[derive(Deserialize)]
struct Golden {
    roster: Vec<Entry>,
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn golden() -> Golden {
    let text = std::fs::read_to_string(root().join("tests/fixtures/roster.json")).expect("fixture");
    serde_json::from_str(&text).expect("the fixture parses (a spec with a field the simulation does not know is refused)")
}

fn course() -> Course {
    let read = |p: &str| std::fs::read_to_string(root().join("../..").join(p)).unwrap_or_else(|e| panic!("{p}: {e}"));
    let def: CourseDef = ron::from_str(&read("content/courses/hill_valley.ron")).expect("course");
    let terrain: TerrainDef = ron::from_str(&read("content/terrain.ron")).expect("terrain");
    Course::bake_with(&def, &terrain).expect("the course and the terrain table agree")
}

fn hash(r: &Run) -> String {
    format!("{:016x}", r.final_hash)
}

#[test]
fn every_golden_run_replays_exactly() {
    let (g, course) = (golden(), course());
    assert!(g.roster.len() >= 20, "the whole army is pinned: {}", g.roster.len());
    for e in &g.roster {
        let r = run(&course, &e.spec);
        assert_eq!(hash(&r), e.hash, "{}: final hash {} differs from the golden {} (see the module docs before updating it)", e.id, hash(&r), e.hash);
        assert_eq!(r.frames.len(), e.ticks, "{}: length of the run", e.id);
        let kind = match &r.outcome {
            Outcome::Finished { .. } => "finished",
            Outcome::Dnf { .. } => "dnf",
            Outcome::Dns { .. } => "dns",
        };
        assert_eq!(e.outcome["kind"], kind, "{}", e.id);
        if let Outcome::Finished { ticks, .. } = &r.outcome {
            assert_eq!(e.outcome["ticks"], *ticks, "{}: finishing time", e.id);
        }
    }
}

#[test]
fn the_army_has_every_kind_of_end() {
    // The fixture would pin little if everything finished: it must hold winners, a bogged vehicle and one that never started.
    let g = golden();
    let kinds: Vec<&str> = g.roster.iter().map(|e| e.outcome["kind"].as_str().unwrap()).collect();
    for k in ["finished", "dnf", "dns"] {
        assert!(kinds.contains(&k), "no {k} in the golden roster: {kinds:?}");
    }
    assert!(g.roster.iter().any(|e| e.outcome["cause"] == "Bogged"), "a bogged run");
}

#[test]
fn the_hashes_do_not_depend_on_how_many_threads_run_them() {
    let (g, course) = (golden(), course());
    let want: BTreeMap<&str, String> = g.roster.iter().map(|e| (e.id.as_str(), e.hash.clone())).collect();
    for threads in [1usize, 2, 4, 8] {
        let mut got: BTreeMap<&str, String> = BTreeMap::new();
        std::thread::scope(|s| {
            let handles: Vec<_> = (0..threads)
                .map(|t| {
                    let (g, course) = (&g, &course);
                    s.spawn(move || g.roster.iter().skip(t).step_by(threads).map(|e| (e.id.as_str(), hash(&run(course, &e.spec)))).collect::<Vec<_>>())
                })
                .collect();
            for h in handles {
                got.extend(h.join().unwrap());
            }
        });
        assert_eq!(got, want, "{threads} threads");
    }
}

#[test]
fn a_vehicle_alone_equals_the_same_vehicle_in_a_batch() {
    // Runs share no state: whatever ran before, or beside, does not matter.
    let (g, course) = (golden(), course());
    let last = g.roster.last().unwrap();
    let alone = run(&course, &last.spec);
    for e in &g.roster {
        let _ = run(&course, &e.spec);
    }
    let after = run(&course, &last.spec);
    assert_eq!(alone.final_hash, after.final_hash);
    assert_eq!(alone.frame_bytes(), after.frame_bytes());
    assert_eq!(alone.checkpoint_hashes, after.checkpoint_hashes);
}

#[test]
fn the_hash_chain_checks_every_second() {
    let (g, course) = (golden(), course());
    let e = g.roster.iter().find(|e| e.outcome["kind"] == "finished").unwrap();
    let r = run(&course, &e.spec);
    // One hash per second of the run and the last one is the final hash's predecessor in time: the chain is as long as the run.
    assert_eq!(r.checkpoint_hashes.len(), (r.frames.len() - 1) / w5k_sim::HZ as usize, "{}", e.id);
    let mut distinct = r.checkpoint_hashes.clone();
    distinct.dedup();
    assert_eq!(distinct.len(), r.checkpoint_hashes.len(), "a moving vehicle never repeats a state");
}

#[test]
fn a_spec_with_a_field_the_simulation_does_not_know_is_refused() {
    let g = golden();
    let mut v = serde_json::to_value(&g.roster[0].spec).unwrap();
    v["wing_area_m2"] = serde_json::json!(3.0);
    let err = serde_json::from_value::<MoverSpec>(v).unwrap_err().to_string();
    assert!(err.contains("wing_area_m2"), "{err}");
}
