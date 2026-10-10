//! `w5k drive --vehicle FILE.ron --course FILE.ron [--vehicles-dir DIR] [--web DIR] [--port N] [--open] [--no-assist] [--record DIR]`:
//! the live test-drive server (ARCH). The real simulation (FORGE rig, DRIVE powertrain, CHASSIS vehicle, WORLD course) runs in real time on
//! its own thread; a tiny loopback HTTP server serves a prebuilt page and a small JSON API (`docs/swarm/requests/arch-drive-protocol.md`).
//! A five-year-old drives with arrow keys, a gamepad or big buttons; the kid assists live in `content/physics/arch/drive_assist.ron`.
//!
//! Stage 2 of the series: the paced live loop (no server yet).
//!
//! This is tooling: it paces the simulation against the wall clock, which the simulation crates may not (the results of a tick never
//! depend on it).
#![allow(clippy::disallowed_methods)] // std::time::Instant::now paces the loop; no simulated quantity reads it
#![allow(dead_code)] // temporary: wired up by the later changes of this series (live loop, server)

mod assist;
mod live;
mod session;

use std::path::PathBuf;
use std::sync::Arc;

use live::{Recorder, Runner, Shared};
use session::{Car, Scene};

const USAGE: &str = "usage: w5k drive --vehicle FILE.ron --course FILE.ron [--vehicles-dir DIR] [--web DIR] [--port N] [--open] [--no-assist] [--record DIR]";
const TUNING: &str = "content/physics/arch/drive_assist.ron";
const DEFAULT_VEHICLES_DIR: &str = "content/vehicles/game";
const DEFAULT_PORT: u16 = 8787; // const-ok: the default port of the local page

/// Parsed command line.
pub(crate) struct Opts {
    pub vehicle: Option<String>,
    pub vehicles_dir: String,
    pub course: String,
    pub web: Option<PathBuf>,
    pub port: u16,
    pub open: bool,
    pub assist: bool,
    pub record: Option<PathBuf>,
    pub root: PathBuf,
}

impl Opts {
    fn parse(args: &[String]) -> Result<Opts, String> {
        let opt = |key: &str| args.iter().position(|a| a == key).and_then(|i| args.get(i + 1)).map(String::as_str);
        let port = opt("--port")
            .map_or(Ok(DEFAULT_PORT), |p| p.parse().map_err(|_| format!("--port: not a port number\n{USAGE}")))?;
        Ok(Opts {
            vehicle: opt("--vehicle").map(String::from),
            vehicles_dir: opt("--vehicles-dir").unwrap_or(DEFAULT_VEHICLES_DIR).to_string(),
            course: opt("--course").ok_or(USAGE)?.to_string(),
            web: opt("--web").map(PathBuf::from),
            port,
            open: args.iter().any(|a| a == "--open"),
            assist: !args.iter().any(|a| a == "--no-assist"),
            record: opt("--record").map(PathBuf::from),
            root: PathBuf::from("."),
        })
    }
}

/// A running simulation thread.
pub(crate) struct Started {
    /// Lets a test stop the simulation thread (the command line runs until the process ends).
    #[cfg_attr(not(test), allow(dead_code))]
    pub shared: Arc<Shared>,
}

/// Load everything and start the simulation thread.
pub(crate) fn start(o: &Opts) -> Result<Started, String> {
    let scene = Arc::new(Scene::load(&o.root, TUNING, &o.course)?);
    let mut garage = Car::load_dir(&o.root, &scene, &o.vehicles_dir)?;
    let mut first_id = None;
    if let Some(file) = &o.vehicle {
        let car = Car::load(&o.root, &scene, file)?;
        first_id = Some(car.id.clone());
        match garage.iter().position(|c| c.id == car.id) {
            Some(i) => garage[i] = Arc::new(car),
            None => garage.push(Arc::new(car)),
        }
    }
    let first = first_id
        .and_then(|id| garage.iter().find(|c| c.id == id))
        .or(garage.first())
        .cloned()
        .ok_or("no drivable vehicle found")?;
    let shared = Arc::new(Shared::default());
    let rec = o.record.clone().map(|d| Recorder::new(d, &scene));
    let runner = Runner::new(shared.clone(), garage.clone(), scene.clone(), first, o.assist, rec)?;
    std::thread::spawn(move || runner.run());
    Ok(Started { shared })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::session::testing::root;
    use super::*;

    fn opts(args: &[&str]) -> Result<Opts, String> {
        Opts::parse(&args.iter().map(|a| a.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn the_command_line_has_defaults_and_flags() {
        let o = opts(&["--course", "c.ron"]).expect("parses");
        assert!(
            o.assist
                && !o.open
                && o.port == DEFAULT_PORT
                && o.vehicles_dir == DEFAULT_VEHICLES_DIR
                && o.record.is_none()
        );
        let o =
            opts(&["--course", "c.ron", "--no-assist", "--port", "0", "--record", "out", "--open"]).expect("parses");
        assert!(!o.assist && o.open && o.port == 0 && o.record == Some(PathBuf::from("out")));
        assert!(opts(&["--vehicle", "v.ron"]).is_err(), "--course is required");
        assert!(opts(&["--course", "c.ron", "--port", "http"]).is_err());
    }

    #[test]
    fn the_live_loop_publishes_about_thirty_frames_per_wall_clock_second_and_no_more() {
        let o = Opts {
            vehicle: Some("content/vehicles/game/scout_4x4.ron".into()),
            vehicles_dir: DEFAULT_VEHICLES_DIR.into(),
            course: "content/world/courses/slice.ron".into(),
            web: None,
            port: 0,
            open: false,
            assist: true,
            record: None,
            root: root(),
        };
        let s = start(&o).expect("starts");
        let t0 = std::time::Instant::now();
        let mut seen = 0;
        let mut next = s.shared.subscribe();
        while t0.elapsed() < Duration::from_secs(2) {
            let (frames, n) = s.shared.frames_since(next, Duration::from_millis(100));
            seen += frames.len();
            next = n;
        }
        s.shared.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let per_s = seen as f64 / t0.elapsed().as_secs_f64();
        assert!((20.0..=33.0).contains(&per_s), "{per_s} frames per second");
    }
}
