//! `w5k scenario course-compare --course FILE --vehicles a.ron,b.ron,... --out DIR`: N vehicles over the SAME course, one replay.
//!
//! The driver is `mule-course`'s (`arch_course.rs`) with the same parameter file, so the only difference between runs is the vehicle.
//! The speed target comes from the course alone (cruise speed, a cornering cap from the road's curvature, the final approach); it is
//! never capped by what the vehicle can do, so a weaker vehicle simply falls behind the target. Each vehicle is simulated on its own
//! (own state, own double-run determinism check). Outputs in DIR: `replay.w5kr` (all vehicles in `vehicles[]`, frames at the common
//! step, a vehicle that has stopped holds its last pose with zero velocity, the replay lasts as long as the longest run),
//! `rig_<name>.json` per vehicle for the viewer, `*_<name>.csv` traces, and the report (see `report.rs`).

mod report;

use std::collections::BTreeSet;
use std::path::Path;

use w5k_contract::def::VehicleDef;
use w5k_contract::frame::{Frame, ReplayHeader, VehicleHeader, WorldHeader, REPLAY_VERSION};
use w5k_contract::rig::{PhysRig, TICK_HZ};
use w5k_forge::extras::Extras;
use w5k_math::{StateHasher, Vec3};
use w5k_replay::ReplayFile;

use super::arch_course::{read, RunResult, Setup};

const USAGE: &str =
    "usage: w5k scenario course-compare --course FILE.ron --vehicles a.ron,b.ron,... --out DIR [--scenario FILE.ron]";
const DEFAULT_SCENARIO: &str = "content/physics/arch/mule_course.ron";

/// A vehicle design to run (tests alter it in memory).
pub(crate) struct Entry {
    pub def: VehicleDef,
    pub extras: Extras,
}

/// One vehicle's finished run, under its unique name.
#[derive(Clone)]
pub(crate) struct VehicleRun {
    pub name: String,
    pub stem: String,
    pub run: RunResult,
    pub rig: PhysRig,
    pub hull_size_m: Vec3,
}

/// Read a vehicle file and its `<name>.extras.ron` sidecar (FORGE's PROVISIONAL extras).
pub(crate) fn load_entry(root: &Path, vehicle: &str) -> Result<Entry, String> {
    let sidecar =
        vehicle.strip_suffix(".ron").map(|b| format!("{b}.extras.ron")).ok_or(format!("{vehicle}: not a .ron file"))?;
    Ok(Entry {
        def: w5k_forge::compile::parse_def(&read(root, vehicle)?)?,
        extras: w5k_forge::compile::parse_extras(&read(root, &sidecar)?)?,
    })
}

/// Unique output names: ids that occur more than once get `#1`, `#2`, ... and anything still colliding gets underscores.
pub(crate) fn unique_names(ids: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut counter = std::collections::BTreeMap::new();
    ids.iter()
        .map(|id| {
            let k = counter.entry(id.clone()).or_insert(0usize);
            *k += 1;
            let mut name = if ids.iter().filter(|o| *o == id).count() > 1 { format!("{id}#{k}") } else { id.clone() };
            while !seen.insert(name.clone()) {
                name.push('_');
            }
            name
        })
        .collect()
}

/// A file-name stem for a vehicle name: anything but letters, digits, `_` and `-` becomes `_`.
fn stem_of(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect()
}

/// Compile and run every entry over the course, each on its own and each twice (hashes must agree).
pub(crate) fn run_all(setup: &Setup, entries: &[Entry]) -> Result<Vec<VehicleRun>, String> {
    let names = unique_names(&entries.iter().map(|e| e.def.id.clone()).collect::<Vec<_>>());
    let mut stems = BTreeSet::new();
    let mut runs: Vec<VehicleRun> = Vec::new();
    for (e, name) in entries.iter().zip(names) {
        let c = w5k_forge::compile::compile(&e.def, &e.extras).map_err(|r| format!("FORGE refused {name}: {r:?}"))?;
        let mut stem = stem_of(&name);
        while !stems.insert(stem.clone()) {
            stem.push('_');
        }
        let run = setup.run_checked(&c.rig).map_err(|m| format!("{name}: {m}"))?;
        runs.push(VehicleRun { name, stem, run, rig: c.rig, hull_size_m: c.hull_size_m });
    }
    Ok(runs)
}

/// All vehicles in one replay: frame `i` holds every vehicle's frame `i`, or its last frame (velocities zeroed) once its run is over.
pub(crate) fn combined_replay(setup: &Setup, runs: &[VehicleRun]) -> ReplayFile {
    let longest = runs.iter().map(|r| r.run.frames.len()).max().unwrap_or(0);
    let lead = runs.iter().find(|r| r.run.frames.len() == longest);
    let frames = (0..longest)
        .map(|i| {
            let vehicles = runs
                .iter()
                .zip(0u32..)
                .map(|(r, k)| {
                    let mut v = r.run.frames[i.min(r.run.frames.len() - 1)].vehicles[0].clone();
                    v.vehicle = k;
                    if i >= r.run.frames.len() {
                        v.lin_vel_m_s = Vec3::ZERO;
                        v.ang_vel_rad_s = Vec3::ZERO;
                    }
                    v
                })
                .collect();
            let t_s = lead.map_or(0.0, |l| l.run.frames[i].t_s);
            Frame { t_s, vehicles, events: Vec::new(), projectiles: Vec::new() }
        })
        .collect();
    let seconds = runs.iter().map(|r| r.run.hashes.len()).max().unwrap_or(0);
    let state_hashes = (0..seconds)
        .map(|j| {
            let mut h = StateHasher::new();
            for r in runs {
                h.write_u64(r.run.hashes[j.min(r.run.hashes.len() - 1)]);
            }
            h.finish()
        })
        .collect();
    ReplayFile {
        header: ReplayHeader {
            version: REPLAY_VERSION,
            scenario: format!("course-compare: {} vehicles on {}", runs.len(), setup.course_name),
            frame_dt_s: f64::from(runs.first().map_or(1, |r| r.run.frame_every_ticks)) / TICK_HZ,
            vehicles: runs
                .iter()
                .map(|r| VehicleHeader {
                    name: r.name.clone(),
                    rig_id: r.rig.id.clone(),
                    joint_names: r.rig.joint_names(),
                    contact_names: r.rig.contact_names(),
                    livery: None,
                })
                .collect(),
            world: WorldHeader { course: setup.course_name.clone(), seed: setup.course_seed, terrain: None },
            state_hashes,
        },
        frames,
    }
}

/// Entry point for `w5k scenario course-compare`.
pub fn course_compare(args: &[String]) -> Result<(), String> {
    let opt = |key: &str| args.iter().position(|a| a == key).and_then(|i| args.get(i + 1)).map(String::as_str);
    let (course, list, out) =
        (opt("--course").ok_or(USAGE)?, opt("--vehicles").ok_or(USAGE)?, Path::new(opt("--out").ok_or(USAGE)?));
    let root = Path::new(".");
    let setup = Setup::load(root, opt("--scenario").unwrap_or(DEFAULT_SCENARIO), Some(course))?;
    let entries =
        list.split(',').filter(|v| !v.is_empty()).map(|v| load_entry(root, v)).collect::<Result<Vec<_>, _>>()?;
    if entries.is_empty() {
        return Err(USAGE.into());
    }
    let runs = run_all(&setup, &entries)?;
    std::fs::create_dir_all(out).map_err(|e| format!("cannot create {}: {e}", out.display()))?;
    let replay = combined_replay(&setup, &runs);
    w5k_replay::write_bin(&out.join("replay.w5kr"), &replay)?;
    for r in &runs {
        let render = w5k_forge::render::render_rig(&r.rig, r.hull_size_m);
        let write = |name: String, text: &str| {
            std::fs::write(out.join(&name), text).map_err(|e| format!("cannot write {name}: {e}"))
        };
        write(format!("rig_{}.json", r.stem), &serde_json::to_string(&render).map_err(|e| e.to_string())?)?;
        for (kind, text) in [
            ("speed", &r.run.speed),
            ("rpm", &r.run.rpm),
            ("gear", &r.run.gear),
            ("track", &r.run.track),
            ("loads", &r.run.loads),
        ] {
            write(format!("{kind}_{}.csv", r.stem), text)?;
        }
    }
    report::write_all(&setup, &runs, out)?;
    println!(
        "wrote {}/replay.w5kr ({} vehicles, {} frames), rig_*.json, traces and the report",
        out.display(),
        runs.len(),
        replay.frames.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use super::*;

    pub(super) fn root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    const MULE: &str = "content/vehicles/game/mule_4x4.ron";
    const COURSE: &str = "content/world/courses/slice.ron";

    pub(super) fn setup() -> Setup {
        Setup::load(&root(), DEFAULT_SCENARIO, Some(COURSE)).expect("setup")
    }

    /// Three Mules, run once and shared by the tests (each vehicle is run twice inside `run_all`, which refuses differing hashes).
    pub(super) fn three_mules() -> &'static (Setup, Vec<VehicleRun>) {
        static CELL: OnceLock<(Setup, Vec<VehicleRun>)> = OnceLock::new();
        CELL.get_or_init(|| {
            let setup = setup();
            let entries: Vec<Entry> = (0..3).map(|_| load_entry(&root(), MULE).expect("mule")).collect();
            let runs = run_all(&setup, &entries).expect("three runs, each deterministic");
            (setup, runs)
        })
    }

    #[test]
    fn identical_vehicles_get_unique_names_and_identical_deterministic_runs() {
        let (_, runs) = three_mules();
        let names: Vec<&str> = runs.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["mule_4x4#1", "mule_4x4#2", "mule_4x4#3"]);
        assert!(runs.iter().all(|r| r.run.final_hash == runs[0].run.final_hash), "vehicles shared state");
        assert!(runs.iter().all(|r| r.run.stop_reason.starts_with("finished")), "{}", runs[0].run.stop_reason);
    }

    #[test]
    fn a_vehicle_with_half_the_engine_power_arrives_later_or_not_at_all_on_the_same_course() {
        let setup = setup();
        let full = load_entry(&root(), MULE).expect("mule");
        let mut half = load_entry(&root(), MULE).expect("mule");
        for p in [&mut half.def.powertrain.engine.peak_power_w, &mut half.def.powertrain.engine.peak_torque_nm] {
            p.v *= 0.5; // const-ok: the variant under test
            p.lo = p.lo.map(|v| v * 0.5); // const-ok: the band moves with the value
            p.hi = p.hi.map(|v| v * 0.5); // const-ok: the band moves with the value
        }
        half.def.id = "mule_half_power".into();
        let runs = run_all(&setup, &[full, half]).expect("runs");
        let (a, b) = (&runs[0].run, &runs[1].run);
        let b_finished = b.stop_reason.starts_with("finished");
        assert!(a.stop_reason.starts_with("finished"), "{}", a.stop_reason);
        assert!(!b_finished || b.time_s > a.time_s, "half power finished in {} s against {} s", b.time_s, a.time_s);
        assert!(b_finished || b.stopped_at_s_m < a.stopped_at_s_m, "{}", b.stop_reason);
    }

    #[test]
    fn the_combined_replay_round_trips_with_every_vehicle_in_every_frame() {
        let (setup, runs) = three_mules();
        let replay = combined_replay(setup, runs);
        let path = std::env::temp_dir().join(format!("w5k_compare_{}.w5kr", std::process::id()));
        w5k_replay::write_bin(&path, &replay).expect("write");
        let back = w5k_replay::read_bin(&path).expect("read");
        let _ = std::fs::remove_file(&path);
        assert_eq!(back.header.vehicles.len(), 3);
        assert_eq!(back.header.vehicles[1].name, "mule_4x4#2");
        assert_eq!(back.frames.len(), replay.frames.len());
        for f in &back.frames {
            let ids: Vec<u32> = f.vehicles.iter().map(|v| v.vehicle).collect();
            assert_eq!(ids, [0, 1, 2]);
            assert!(f.vehicles.iter().all(|v| v.is_finite()));
        }
    }

    #[test]
    fn a_vehicle_that_has_stopped_holds_its_last_pose_while_a_longer_run_continues() {
        let (setup, runs) = three_mules();
        let mut short = runs[0].clone();
        short.run.frames.truncate(10); // const-ok: cut the run short
        let replay = combined_replay(setup, &[short, runs[1].clone()]);
        let (held, last) = (&replay.frames[replay.frames.len() - 1].vehicles[0], &replay.frames[9].vehicles[0]);
        assert_eq!(held.pos_m, last.pos_m);
        assert_eq!(held.lin_vel_m_s, Vec3::ZERO);
        assert_eq!(replay.frames.len(), runs[1].run.frames.len());
    }

    #[test]
    fn duplicate_names_are_suffixed_and_the_result_is_unique() {
        let ids: Vec<String> = ["a", "b", "a", "a#1"].iter().map(|s| s.to_string()).collect();
        let names = unique_names(&ids);
        assert_eq!(names.iter().collect::<BTreeSet<_>>().len(), 4);
        assert_eq!(&names[..3], ["a#1", "b", "a#2"]);
    }
}
