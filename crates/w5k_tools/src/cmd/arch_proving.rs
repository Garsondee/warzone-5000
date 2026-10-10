//! `w5k scenario proving --vehicle a.ron[,b.ron...] --test <name|all> [--out PATH]`: ARCH's proving-ground runner.
//!
//! Each test is a scripted scenario on one vehicle (FORGE rig, CHASSIS wheeled chassis, DRIVE's real powertrain, a stand-in world from
//! `w5k_contract::testing`) that writes a `w5k.proving.result.v1` JSON (`w5k_validate::proving::ProvingResult`, spec in
//! `docs/validation/proving-ground.md` section 2). The `inputs` are the numbers the simulation actually used (mass, tyre friction,
//! driven load share, power), read back from the rig and the settled chassis, because VALIDATION's oracle is evaluated on them. A run
//! that ends early says why. Every test is run twice and the two state hashes and results must agree (determinism).
//!
//! `--out` ending in `.json` with one vehicle and one test is the result file; otherwise `--out` is a directory and each result lands in
//! `<out>/<vehicle>/<test>.json` next to `<test>.replay.w5kr`. Without `--out` the JSON goes to stdout. A test the current APIs cannot
//! run is reported as "not implemented" and writes nothing (see `docs/swarm/requests/arch-proving-gaps.md`); there is never a fake number.
//! Every driver number is in `content/physics/arch/proving.ron`.

mod facts;
mod longitudinal;
mod mobility;
mod sim;
mod traction;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use w5k_chassis::tuning::ChassisTuning;
use w5k_contract::frame::{Frame, ReplayHeader, VehicleHeader, WorldHeader, REPLAY_VERSION};
use w5k_contract::rig::{PhysRig, TICK_HZ};
use w5k_contract::testing::world::FlatPlane;
use w5k_contract::{Param, WorldQuery};
use w5k_replay::ReplayFile;
use w5k_validate::proving::{ProvingResult, SCHEMA, TESTS};

use super::arch_compare::{load_entry, unique_names};
use super::arch_course::read;
use sim::Sim;

const USAGE: &str = "usage: w5k scenario proving --vehicle a.ron[,b.ron,...] --test <name|all> [--out result.json | DIR] [--scenario FILE.ron]";
const DEFAULT_SCENARIO: &str = "content/physics/arch/proving.ron";

/// The scenario file: the two plain fields, then one `Param` (value, band, provenance, source) per driver number, all checked at load.
macro_rules! scenario_file {
    ($($param:ident),* $(,)?) => {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields, rename = "Proving")]
        pub(crate) struct Config {
            chassis_tuning: String,
            frame_every_ticks: u32,
            $($param: Param,)*
        }

        impl Config {
            fn check(&self) -> Result<(), String> {
                $(self.$param.check(concat!("proving.", stringify!($param)))?;)*
                Ok(())
            }
        }
    };
}

scenario_file!(
    settle_s,
    stopped_below_m_s,
    speed_gain_per_m_s,
    speed_integral_per_m,
    braking_speed_kmh,
    braking_approach_max_s,
    braking_hold_s,
    braking_hold_tol_m_s,
    braking_max_s,
    decel_window_s,
    accel_target_kmh,
    accel_max_s,
    tilt_rate_rad_s,
    tilt_max_rad,
    step_speed_m_s,
    step_distance_m,
    step_ramp_m,
    step_max_s,
    step_search_max_m,
    step_resolution_m,
    skid_radius_m,
    skid_start_speed_m_s,
    skid_ramp_m_s2,
    skid_warmup_s,
    skid_max_s,
    skid_slide_out_frac,
    skid_window_s,
    skid_plateau_gain_m_s,
    grade_hold_s,
    grade_progress_m,
    grade_search_max,
    grade_resolution,
);

/// What a test needs: the vehicle's rig, the chassis tuning and the driver numbers.
pub(crate) struct Ctx<'a> {
    pub rig: &'a PhysRig,
    pub tuning: &'a ChassisTuning,
    pub cfg: &'a Config,
}

/// What a test run leaves behind: the result maps (keys are the schema's, so static), the replay frames, the final state hash.
#[derive(Default)]
pub(crate) struct Outcome {
    inputs: BTreeMap<&'static str, f64>,
    labels: BTreeMap<&'static str, &'static str>,
    measured: BTreeMap<&'static str, f64>,
    ended_early: Option<String>,
    frames: Vec<Frame>,
    hash: u64,
    /// Simulated time at the end of the run, s.
    time_s: f64,
}

impl Outcome {
    pub fn from_sim(sim: Sim) -> Outcome {
        Outcome {
            ended_early: sim.early.clone(),
            hash: sim.hash(),
            time_s: sim.chassis.time_s,
            frames: sim.frames,
            ..Outcome::default()
        }
    }
}

/// The level plane of dry asphalt every flat-ground test runs on (the contract's stand-in world; its numbers match WORLD's table).
pub(crate) fn flat_world() -> Box<dyn WorldQuery> {
    Box::new(FlatPlane::new())
}

type Runner = fn(&Ctx) -> Result<Outcome, String>;

const RUNNERS: &[(&str, Runner)] = &[
    ("braking_50kmh", longitudinal::braking),
    ("accel_0_48kmh", longitudinal::acceleration),
    ("side_slope_rollover", mobility::side_slope),
    ("step_climb", mobility::step_climb),
    ("skidpad", traction::skidpad),
    ("gradeability", traction::gradeability),
];

/// The contract the result was produced against, e.g. `contract-v0.2`.
fn contract_pin() -> String {
    let v: Vec<&str> = w5k_contract::CONTRACT_VERSION.split('.').take(2).collect();
    format!("contract-v{}", v.join("."))
}

fn owned<V, W>(m: &BTreeMap<&'static str, V>, f: impl Fn(&V) -> W) -> BTreeMap<String, W> {
    m.iter().map(|(k, v)| ((*k).to_string(), f(v))).collect()
}

/// Run one test twice (identical hashes and results are required) and assemble the result and the replay.
fn run_test(ctx: &Ctx, vehicle: &str, test: &str, runner: Runner) -> Result<(ProvingResult, ReplayFile), String> {
    let (a, b) = (runner(ctx)?, runner(ctx)?);
    if a.hash != b.hash || a.inputs != b.inputs || a.measured != b.measured || a.ended_early != b.ended_early {
        return Err(format!("{vehicle}/{test}: the two runs differ: the simulation is not deterministic"));
    }
    let replay = ReplayFile {
        header: ReplayHeader {
            version: REPLAY_VERSION,
            scenario: format!("proving: {test} ({})", a.ended_early.as_deref().unwrap_or("finished")),
            frame_dt_s: f64::from(ctx.cfg.frame_every_ticks.max(1)) / TICK_HZ,
            vehicles: vec![VehicleHeader {
                name: vehicle.to_string(),
                rig_id: ctx.rig.id.clone(),
                joint_names: ctx.rig.joint_names(),
                contact_names: ctx.rig.contact_names(),
                livery: None,
            }],
            world: WorldHeader { course: "proving ground: flat asphalt".into(), seed: 0, terrain: None },
            state_hashes: vec![a.hash],
        },
        frames: a.frames,
    };
    let result = ProvingResult {
        schema: SCHEMA.to_string(),
        test: test.to_string(),
        vehicle: vehicle.to_string(),
        contract_pin: contract_pin(),
        inputs: owned(&a.inputs, |v| *v),
        labels: owned(&a.labels, |v| (*v).to_string()),
        measured: owned(&a.measured, |v| *v),
        ended_early: a.ended_early,
        replay: None,
    };
    Ok((result, replay))
}

/// Where a result and its replay go: `(json, replay)`; `None` = stdout and no replay.
fn paths(out: Option<&str>, single: bool, vehicle: &str, test: &str) -> (Option<PathBuf>, Option<PathBuf>) {
    match out {
        None => (None, None),
        Some(f) if single && f.ends_with(".json") => (Some(f.into()), Some(Path::new(f).with_extension("replay.w5kr"))),
        Some(d) => {
            let dir = Path::new(d).join(vehicle);
            (Some(dir.join(format!("{test}.json"))), Some(dir.join(format!("{test}.replay.w5kr"))))
        }
    }
}

/// Entry point for `w5k scenario proving`.
pub fn proving(args: &[String]) -> Result<(), String> {
    run(args, Path::new("."))
}

/// The command with the content root given (tests run from the crate folder).
fn run(args: &[String], root: &Path) -> Result<(), String> {
    let opt = |key: &str| args.iter().position(|a| a == key).and_then(|i| args.get(i + 1)).map(String::as_str);
    let (vehicles, test) = (opt("--vehicle").ok_or(USAGE)?, opt("--test").ok_or(USAGE)?);
    let scenario = opt("--scenario").unwrap_or(DEFAULT_SCENARIO);
    let cfg: Config = ron::from_str(&read(root, scenario)?).map_err(|e| format!("{scenario}: {e}"))?;
    cfg.check()?;
    let tuning = ChassisTuning::from_ron(&read(root, &cfg.chassis_tuning)?)
        .map_err(|e| format!("{}: {e}", cfg.chassis_tuning))?;
    let entries =
        vehicles.split(',').filter(|v| !v.is_empty()).map(|v| load_entry(root, v)).collect::<Result<Vec<_>, _>>()?;
    let tests: Vec<&str> = match test {
        "all" => TESTS.iter().map(|t| t.id).collect(),
        t if TESTS.iter().any(|s| s.id == t) => vec![t],
        t => {
            return Err(format!(
                "unknown test `{t}` (one of: all, {})",
                TESTS.iter().map(|s| s.id).collect::<Vec<_>>().join(", ")
            ))
        }
    };
    let runnable = tests.iter().filter(|t| RUNNERS.iter().any(|r| r.0 == **t)).count();
    let (out, single) = (opt("--out"), entries.len() * runnable <= 1);
    if out.is_none() && !single {
        return Err(format!("several results need --out DIR\n{USAGE}"));
    }
    let names = unique_names(&entries.iter().map(|e| e.def.id.clone()).collect::<Vec<_>>());
    for (entry, name) in entries.iter().zip(&names) {
        let compiled = w5k_forge::compile::compile(&entry.def, &entry.extras)
            .map_err(|r| format!("FORGE refused {name}: {r:?}"))?;
        let ctx = Ctx { rig: &compiled.rig, tuning: &tuning, cfg: &cfg };
        for t in &tests {
            let Some((_, runner)) = RUNNERS.iter().find(|r| r.0 == *t) else {
                println!("{name}/{t}: not implemented (docs/swarm/requests/arch-proving-gaps.md); nothing written");
                if test != "all" {
                    return Err(format!("{t}: not implemented (docs/swarm/requests/arch-proving-gaps.md)"));
                }
                continue;
            };
            let (mut result, replay) = run_test(&ctx, name, t, *runner).map_err(|e| format!("{name}/{t}: {e}"))?;
            let (json_path, replay_path) = paths(out, single, name, t);
            if let Some(p) = replay_path.as_ref().filter(|_| !replay.frames.is_empty()) {
                w5k_replay::write_bin(p, &replay)?;
                result.replay = Some(p.display().to_string());
            }
            let text = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?;
            match json_path {
                Some(p) => {
                    std::fs::create_dir_all(p.parent().unwrap_or(Path::new("."))).map_err(|e| e.to_string())?;
                    std::fs::write(&p, format!("{text}\n"))
                        .map_err(|e| format!("cannot write {}: {e}", p.display()))?;
                    println!("{name}/{t}: {} -> {}", summary(&result), p.display());
                }
                None => println!("{text}"),
            }
        }
    }
    Ok(())
}

/// One line for the console: the measured numbers, or why the run ended early.
fn summary(r: &ProvingResult) -> String {
    match &r.ended_early {
        Some(why) => format!("ENDED EARLY: {why}"),
        None => r.measured.iter().map(|(k, v)| format!("{k} = {v:.3}")).collect::<Vec<_>>().join(", "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_math::scalar;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    const GARAGE: [&str; 3] = [
        "content/vehicles/game/scout_4x4.ron",
        "content/vehicles/game/mule_4x4.ron",
        "content/vehicles/game/hauler_4x4.ron",
    ];

    /// A compiled garage vehicle with the shipped scenario file, ready to run tests on.
    struct Fixture {
        rig: PhysRig,
        tuning: ChassisTuning,
        cfg: Config,
        engine_peak_w: f64,
    }

    impl Fixture {
        fn load(vehicle: &str) -> Fixture {
            let cfg: Config = ron::from_str(&read(&root(), DEFAULT_SCENARIO).expect("scenario")).expect("parses");
            cfg.check().expect("every driver number has its provenance");
            let tuning = ChassisTuning::from_ron(&read(&root(), &cfg.chassis_tuning).expect("tuning")).expect("tuning");
            let entry = load_entry(&root(), vehicle).expect("vehicle");
            let rig = w5k_forge::compile::compile(&entry.def, &entry.extras).expect("compiles").rig;
            Fixture { rig, tuning, cfg, engine_peak_w: entry.def.powertrain.engine.peak_power_w.v }
        }

        fn ctx(&self) -> Ctx<'_> {
            Ctx { rig: &self.rig, tuning: &self.tuning, cfg: &self.cfg }
        }

        fn run(&self, test: &str) -> ProvingResult {
            let runner = RUNNERS.iter().find(|r| r.0 == test).expect("runner").1;
            run_test(&self.ctx(), &self.rig.id, test, runner).expect("runs twice with identical hashes").0
        }
    }

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("w5k_proving_{name}_{}", std::process::id()))
    }

    #[test]
    fn braking_distance_matches_v2_over_2mu_g_within_the_oracle_band_for_the_mule() {
        let r = Fixture::load(GARAGE[1]).run("braking_50kmh");
        r.check().expect("the scorer's parser accepts it");
        w5k_validate::oracle::score_braking(&r).expect("VALIDATION's scorer accepts the result");
        let (v, mu, d) = (r.inputs["speed_m_s"], r.inputs["mu"], r.measured["stop_distance_m"]);
        let oracle_m = v * v / (2.0 * mu * scalar::G);
        // const-ok: test bands. A sim cannot stop shorter than friction allows (3 % for the oracle's own idealisations), and the
        // oracle class is red beyond +20 %.
        assert!(d >= 0.97 * oracle_m && d <= 1.2 * oracle_m, "stopped in {d} m, v^2/(2 mu g) = {oracle_m} m");
        assert!(r.measured["peak_decel_g"] <= mu * 1.05, "peak decel {} g with mu {mu}", r.measured["peak_decel_g"]);
        // const-ok: test band
    }

    #[test]
    fn no_garage_vehicle_stops_shorter_than_friction_allows_and_the_entry_speed_is_fifty_kmh() {
        for v in GARAGE {
            let r = Fixture::load(v).run("braking_50kmh");
            r.check().expect("valid");
            let (v0, mu, d) = (r.inputs["speed_m_s"], r.inputs["mu"], r.measured["stop_distance_m"]);
            assert!((v0 - scalar::kmh_to_ms(50.0)).abs() < 0.3, "{v}: entered at {v0} m/s"); // const-ok: test values
            assert!(
                d >= 0.97 * v0 * v0 / (2.0 * mu * scalar::G),
                "{v}: stopped in {d} m, shorter than friction allows"
            );
            // const-ok: test band
        }
    }

    #[test]
    fn no_garage_vehicle_reaches_48_kmh_faster_than_its_energy_and_traction_bound() {
        for v in GARAGE {
            let f = Fixture::load(v);
            let r = f.run("accel_0_48kmh");
            r.check().expect("valid");
            let (m, p, mu, frac) =
                (r.inputs["mass_kg"], r.inputs["power_w"], r.inputs["mu"], r.inputs["driven_load_fraction"]);
            let v48 = r.inputs["speed_m_s"];
            let t_min = (m * v48 * v48 / (2.0 * p)).max(v48 / (mu * frac * scalar::G));
            let t = r.measured["t_0_48_s"];
            assert!(t >= t_min, "{v}: {t} s beats the bound {t_min} s");
            assert!(t <= 3.0 * t_min, "{v}: {t} s is more than 3x the bound {t_min} s"); // const-ok: the spec's green band
            assert!(p <= f.engine_peak_w * 1.05, "{v}: wheel power {p} W above the engine's {} W", f.engine_peak_w);
            // const-ok: test band
        }
    }

    #[test]
    fn the_result_json_round_trips_through_the_scorers_parser_and_two_runs_agree_to_the_byte() {
        let f = Fixture::load(GARAGE[0]);
        let (a, b) = (f.run("braking_50kmh"), f.run("braking_50kmh"));
        let text = serde_json::to_string_pretty(&a).expect("serialise");
        assert_eq!(text, serde_json::to_string_pretty(&b).expect("serialise"));
        let back = ProvingResult::from_json(&text).expect("parses");
        assert_eq!(back, a);
        assert!(back.check().is_ok() && back.contract_pin.starts_with("contract-v"));
    }

    #[test]
    fn a_run_that_cannot_reach_its_entry_speed_ends_early_with_a_reason_and_still_parses() {
        let mut f = Fixture::load(GARAGE[1]);
        f.cfg.braking_approach_max_s.v = 1.0; // const-ok: far too short to reach 50 km/h
        let r = f.run("braking_50kmh");
        assert!(r.ended_early.as_deref().is_some_and(|m| m.contains("could not hold")), "{:?}", r.ended_early);
        assert!(r.measured.is_empty());
        assert!(r.check().is_ok(), "an early end may omit its measurements");
    }

    #[test]
    fn test_all_writes_one_valid_file_per_implemented_test_and_nothing_for_the_rest() {
        let out = temp("all");
        let args: Vec<String> =
            ["--vehicle", GARAGE[0], "--test", "all", "--out", out.to_str().expect("utf8")].map(String::from).to_vec();
        run(&args, &root()).expect("runs");
        let dir = out.join("scout_4x4");
        let mut files: Vec<String> = std::fs::read_dir(&dir)
            .expect("dir")
            .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".json"))
            .collect();
        files.sort();
        let mut expect: Vec<String> = RUNNERS.iter().map(|r| format!("{}.json", r.0)).collect();
        expect.sort();
        assert_eq!(files, expect);
        for f in &files {
            let r = ProvingResult::from_json(&std::fs::read_to_string(dir.join(f)).expect("read")).expect("parses");
            r.check().expect("valid");
            // the skidpad bench returns no frames, so it has no replay; every other test has one
            assert_eq!(r.replay.is_none(), r.test == "skidpad", "{}", r.test);
            if let Some(replay) = r.replay {
                assert!(w5k_replay::read_bin(Path::new(&replay)).expect("readable replay").frames.len() > 10);
                // const-ok: test bound
            }
        }
        let _ = std::fs::remove_dir_all(&out);
    }

    #[test]
    fn a_single_test_with_a_json_out_path_writes_exactly_that_file_and_a_test_without_a_runner_is_an_error() {
        let out = temp("one").with_extension("json");
        let args = |test: &str, out: &Path| -> Vec<String> {
            ["--vehicle", GARAGE[1], "--test", test, "--out", out.to_str().expect("utf8")].map(String::from).to_vec()
        };
        run(&args("braking_50kmh", &out), &root()).expect("runs");
        let r = ProvingResult::from_json(&std::fs::read_to_string(&out).expect("file")).expect("parses");
        assert_eq!((r.test.as_str(), r.vehicle.as_str()), ("braking_50kmh", "mule_4x4"));
        let _ = std::fs::remove_file(&out);
        let _ = std::fs::remove_file(out.with_extension("replay.w5kr"));
        let e = run(&args("ride_washboard", &out), &root()).expect_err("no runner yet");
        assert!(e.contains("not implemented"), "{e}");
        assert!(!out.exists(), "a test that cannot run must not leave a file");
        let e = run(&args("teleport", &out), &root()).expect_err("unknown");
        assert!(e.contains("unknown test"), "{e}");
    }

    #[test]
    fn the_side_slope_angle_is_the_smaller_of_the_rigid_tipping_and_sliding_angles_and_never_above_it() {
        for v in GARAGE {
            let r = Fixture::load(v).run("side_slope_rollover");
            r.check().expect("valid");
            w5k_validate::oracle::score_side_slope(&r).expect("VALIDATION's scorer accepts the result");
            let (t, h, mu) = (r.inputs["track_m"], r.inputs["cg_height_m"], r.inputs["mu"]);
            let (tip, slide) = (scalar::atan(t / (2.0 * h)), scalar::atan(mu));
            let a = r.measured["slope_angle_rad"];
            assert_eq!(r.labels["mode"], if tip < slide { "roll" } else { "slide" }, "{v}: tip {tip}, slide {slide}");
            assert!(a <= 1.03 * tip.min(slide), "{v}: {a} rad is above the rigid bound {}", tip.min(slide)); // const-ok: the scorer's slack
            assert!(a >= 0.75 * tip.min(slide), "{v}: {a} rad is far below the bound {}", tip.min(slide));
            // const-ok: the spec's green band
        }
    }

    #[test]
    fn the_first_wheel_lifts_at_the_angle_chassiss_own_tilt_table_reports() {
        let f = Fixture::load(GARAGE[2]);
        let mut drive =
            w5k_drive::powertrain::Powertrain::new(&f.rig.drivetrain, &w5k_drive::powertrain::Tunings::shipped())
                .expect("drive");
        let table = w5k_chassis::bench::TiltTable {
            rate_rad_s: f.cfg.tilt_rate_rad_s.v,
            settle_s: f.cfg.settle_s.v,
            max_angle_rad: f.cfg.tilt_max_rad.v,
        };
        let bench = w5k_chassis::bench::tilt_table(&f.rig, &f.tuning, flat_world().as_ref(), &mut drive, &table)
            .expect("bench");
        let ours = f.run("side_slope_rollover");
        assert_eq!(ours.labels["mode"], "roll");
        let (a, b) = (ours.measured["slope_angle_rad"], bench.lift_angle_rad.expect("the bench lifts"));
        assert!((a - b).abs() < 0.02, "runner {a} rad, bench {b} rad"); // const-ok: about one degree
    }

    #[test]
    fn no_truck_climbs_a_step_above_one_wheel_radius_and_the_scout_and_hauler_stop_near_the_traction_limit() {
        for v in GARAGE {
            let r = Fixture::load(v).run("step_climb");
            r.check().expect("valid");
            w5k_validate::oracle::score_step(&r).expect("VALIDATION's scorer accepts the result");
            let (radius, mu, h) = (r.inputs["wheel_radius_m"], r.inputs["mu"], r.measured["step_height_m"]);
            assert!(h <= radius, "{v}: climbed {h} m with a wheel of radius {radius} m");
            // A rigid wheel on the corner of a step needs the contact force to be able to point straight up: the corner normal leans
            // by theta with cos(theta) = (r - h) / r, and friction can recover atan(mu) of it, so h <= r (1 - 1 / sqrt(1 + mu^2)).
            let traction_limit = radius * (1.0 - 1.0 / scalar::sqrt(1.0 + mu * mu));
            if v != GARAGE[1] {
                // const-ok: the Mule's momentum at 2 m/s carries it past the quasi-static limit (see arch-proving-gaps.md); the others do not
                assert!(
                    h >= 0.5 * traction_limit && h <= 1.5 * traction_limit,
                    "{v}: {h} m against the traction limit {traction_limit} m"
                );
            }
        }
    }

    #[test]
    fn skidpad_lateral_grip_never_exceeds_mu_or_the_rollover_limit_and_every_garage_truck_understeers() {
        for v in GARAGE {
            let r = Fixture::load(v).run("skidpad");
            r.check().expect("valid");
            let (mu, t, h) = (r.inputs["mu"], r.inputs["track_m"], r.inputs["cg_height_m"]);
            let bound = mu.min(t / (2.0 * h));
            let ay = r.measured["max_lat_accel_g"];
            assert!(ay <= 1.03 * bound, "{v}: {ay} g is above min(mu, t/2h) = {bound}"); // const-ok: the scorer's slack
            assert!(ay >= 0.5 * bound, "{v}: {ay} g is far below the bound {bound}"); // const-ok: test band
            assert!(r.measured["understeer_gradient_rad_per_g"] > 0.0, "{v} oversteers");
            assert!(r.labels.contains_key("limited_by"));
        }
    }

    #[test]
    fn a_truck_on_half_the_friction_cannot_climb_more_than_mu_f_and_every_garage_truck_stays_under_its_bounds() {
        for v in GARAGE {
            let mut f = Fixture::load(v);
            let full = f.run("gradeability");
            f.rig.stations.iter_mut().filter_map(|s| s.wheel.tyre.as_mut()).for_each(|t| t.mu_scale *= 0.5); // const-ok: the negative control
            let slick = f.run("gradeability");
            for (r, name) in [(&full, "full"), (&slick, "half friction")] {
                r.check().expect("valid");
                let i = &r.inputs;
                let (traction, force_n) =
                    (i["mu"] * i["driven_load_fraction"], i["wheel_torque_crawl_nm"] / i["wheel_radius_m"]);
                // F = m g (sin a + c cos a), solved by bisection on the angle
                let (mut lo, mut hi) = (0.0, 1.5);
                for _ in 0..60 {
                    let a = 0.5 * (lo + hi);
                    if i["mass_kg"] * scalar::G * (scalar::sin(a) + i["rolling_resistance_coeff"] * scalar::cos(a))
                        < force_n
                    {
                        lo = a;
                    } else {
                        hi = a;
                    }
                }
                let bound = traction.min(scalar::tan(lo));
                let (g, m) = (r.measured["max_grade_ratio"], &r.measured);
                assert!(g <= 1.03 * bound, "{v} {name}: climbs {g} but min(mu f, torque) = {bound}"); // const-ok: the scorer's slack
                assert!(g > 0.2 * bound, "{v} {name}: climbs only {g} of {bound}"); // const-ok: test band
                assert!(m["bracket_hi"] - m["bracket_lo"] <= 0.01 + 1e-9, "{v}: bracket {m:?}");
                // const-ok: the spec's resolution
            }
            assert!((slick.inputs["mu"] - 0.5 * full.inputs["mu"]).abs() < 1e-6, "the echoed mu follows the tyre"); // const-ok: tolerance
            assert!(slick.measured["max_grade_ratio"] < full.measured["max_grade_ratio"] + 1e-9);
        }
    }
}
