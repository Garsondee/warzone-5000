//! `w5k drive --vehicle FILE.ron --course FILE.ron [--vehicles-dir DIR] [--web DIR] [--port N] [--open] [--no-assist] [--record DIR]`:
//! the live test-drive server (ARCH). The real simulation (FORGE rig, DRIVE powertrain, CHASSIS vehicle, WORLD course) runs in real time on
//! its own thread; a tiny loopback HTTP server serves a prebuilt page and a small JSON API (`docs/swarm/requests/arch-drive-protocol.md`).
//! A five-year-old drives with arrow keys, a gamepad or big buttons; the kid assists live in `content/physics/arch/drive_assist.ron`.
//!
//! This is tooling: it paces the simulation against the wall clock, which the simulation crates may not (the results of a tick never
//! depend on it).
#![allow(clippy::disallowed_methods)] // std::time::Instant::now paces the loop; no simulated quantity reads it

mod assist;
mod http;
mod live;
mod session;

use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use live::{Recorder, Runner, Shared};
use session::{Car, Scene};

const USAGE: &str = "usage: w5k drive --vehicle FILE.ron --course FILE.ron [--vehicles-dir DIR] [--web DIR] [--port N] [--open] [--no-assist] [--record DIR]";
const TUNING: &str = "content/physics/arch/drive_assist.ron";
const DEFAULT_VEHICLES_DIR: &str = "content/vehicles/game";
const DEFAULT_PORT: u16 = 8787; // const-ok: the default port of the local page
const DEFAULT_WEB: [&str; 2] = ["viewer", "tools/viewer/dist"];

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

/// Whether `args` (after `w5k drive`) are for the live server and not for lane DRIVE's own commands.
pub fn wants(args: &[String]) -> bool {
    args.first().is_some_and(|a| a.starts_with("--"))
}

/// The page folder: `--web`, else `viewer/` next to the executable, else `tools/viewer/dist`.
fn find_web(given: Option<&Path>, root: &Path) -> Option<PathBuf> {
    if given.is_some() {
        return given.map(Path::to_path_buf);
    }
    let beside_exe = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join(DEFAULT_WEB[0])));
    beside_exe.into_iter().chain(DEFAULT_WEB.iter().map(|d| root.join(d))).find(|p| p.join("index.html").is_file())
}

/// A running server: where it listens, and what to stop.
pub(crate) struct Started {
    pub addr: SocketAddr,
    /// Lets a test stop the simulation thread (the command line runs until the process ends).
    #[cfg_attr(not(test), allow(dead_code))]
    pub shared: Arc<Shared>,
}

/// Load everything, start the simulation thread and the accept thread, return once the port is bound.
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
    let listener =
        TcpListener::bind(("127.0.0.1", o.port)).map_err(|e| format!("cannot listen on port {}: {e}", o.port))?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?;
    let shared = Arc::new(Shared::default());
    let rec = o.record.clone().map(|d| Recorder::new(d, &scene));
    let runner = Runner::new(shared.clone(), garage.clone(), scene.clone(), first, o.assist, rec)?;
    let app = Arc::new(http::App {
        garage,
        terrain_json: scene.terrain_json.clone(),
        shared: shared.clone(),
        web: find_web(o.web.as_deref(), &o.root),
    });
    std::thread::spawn(move || runner.run());
    std::thread::spawn(move || http::serve(listener, app));
    Ok(Started { addr, shared })
}

/// Ask the default browser to open `url`; failures are ignored.
fn open_browser(url: &str) {
    let mut cmd = if cfg!(windows) {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", "", url]);
        c
    } else {
        let mut c = std::process::Command::new(if cfg!(target_os = "macos") { "open" } else { "xdg-open" });
        c.arg(url);
        c
    };
    let _ = cmd
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// Entry point for `w5k drive --vehicle ...`.
pub fn run(args: &[String]) -> Result<(), String> {
    let o = Opts::parse(args)?;
    let s = start(&o)?;
    let url = format!("http://{}/", s.addr);
    println!("w5k drive: open {url} in a browser (assist {}); Ctrl-C to quit", if o.assist { "on" } else { "off" });
    if o.open {
        open_browser(&url);
    }
    loop {
        std::thread::park();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpStream;

    use serde_json::Value;

    use super::session::testing::root;
    use super::*;

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("w5k_drive_{name}_{}", std::process::id()))
    }

    /// Start a server on an ephemeral port over the real garage and the slice course.
    fn server(web: Option<PathBuf>, record: Option<PathBuf>) -> Started {
        let o = Opts {
            vehicle: Some("content/vehicles/game/mule_4x4.ron".into()),
            vehicles_dir: DEFAULT_VEHICLES_DIR.into(),
            course: "content/world/courses/slice.ron".into(),
            web,
            port: 0,
            open: false,
            assist: true,
            record,
            root: root(),
        };
        start(&o).expect("server starts")
    }

    fn send(addr: SocketAddr, method: &str, path: &str, body: &str) -> TcpStream {
        let mut s = TcpStream::connect(addr).expect("connect");
        s.set_read_timeout(Some(std::time::Duration::from_secs(20))).expect("timeout");
        write!(
            s,
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .expect("send");
        s
    }

    fn http(addr: SocketAddr, method: &str, path: &str, body: &str) -> (u16, String) {
        let mut text = String::new();
        send(addr, method, path, body).read_to_string(&mut text).expect("reply");
        let (head, body) = text.split_once("\r\n\r\n").expect("header end");
        (head.split_whitespace().nth(1).expect("status").parse().expect("number"), body.to_string())
    }

    fn get_json(addr: SocketAddr, path: &str) -> Value {
        let (status, body) = http(addr, "GET", path, "");
        assert_eq!(status, 200, "{path}: {body}");
        serde_json::from_str(&body).unwrap_or_else(|e| panic!("{path}: {e}"))
    }

    /// The event stream of a running server, read frame by frame.
    struct Stream(BufReader<TcpStream>);

    impl Stream {
        fn open(addr: SocketAddr) -> Stream {
            let mut r = BufReader::new(send(addr, "GET", "/api/stream", ""));
            let mut line = String::new();
            loop {
                line.clear();
                assert!(r.read_line(&mut line).expect("read") > 0, "stream ended in the header");
                if line.trim().is_empty() {
                    return Stream(r);
                }
            }
        }

        fn next_frame(&mut self) -> Value {
            let mut line = String::new();
            loop {
                line.clear();
                assert!(self.0.read_line(&mut line).expect("read") > 0, "stream ended");
                if let Some(j) = line.strip_prefix("data: ") {
                    return serde_json::from_str(j).expect("frame is JSON");
                }
            }
        }

        /// Frames up to and including the first one that carries `message_event` (at most 300).
        fn until_event(&mut self) -> Vec<Value> {
            let mut out = Vec::new();
            for _ in 0..300 {
                out.push(self.next_frame());
                if out[out.len() - 1].get("message_event").is_some() {
                    break;
                }
            }
            out
        }
    }

    fn keys(v: &Value, into: &mut BTreeSet<String>) {
        match v {
            Value::Object(m) => m.iter().for_each(|(k, x)| {
                into.insert(k.clone());
                keys(x, into);
            }),
            Value::Array(a) => a.iter().for_each(|x| keys(x, into)),
            _ => {}
        }
    }

    #[test]
    fn the_server_lists_vehicles_serves_the_rig_and_world_and_streams_frames_of_the_selected_vehicle() {
        let web = temp("web");
        std::fs::create_dir_all(&web).expect("dir");
        std::fs::write(web.join("index.html"), "<h1>hello</h1>").expect("page");
        let s = server(Some(web.clone()), None);
        let a = s.addr;
        let list = get_json(a, "/api/vehicles");
        let ids: Vec<&str> = list.as_array().expect("array").iter().map(|v| v["id"].as_str().expect("id")).collect();
        for id in ["mule_4x4", "scout_4x4", "hauler_4x4"] {
            assert!(ids.contains(&id), "{id} missing from {ids:?}");
        }
        let scout = list.as_array().expect("array").iter().find(|v| v["id"] == "scout_4x4").expect("scout");
        assert!(scout["mass_kg"].as_f64().expect("mass") > 500.0 && scout["wheelbase_m"].as_f64().expect("wb") > 1.5);
        let joints = get_json(a, "/api/rig/scout_4x4")["joint_count"].as_u64().expect("joint_count") as usize;
        assert_eq!(http(a, "GET", "/api/rig/nope", "").0, 404);
        assert_eq!(get_json(a, "/api/world")["format"], "w5k-terrain-1");

        assert_eq!(http(a, "POST", "/api/select", "{\"vehicle\":\"nope\"}").0, 404);
        assert_eq!(http(a, "POST", "/api/select", "{\"vehicle\":\"scout_4x4\"}"), (200, "{\"ok\":true}".to_string()));
        assert_eq!(http(a, "POST", "/api/input", "not json").0, 400);
        assert_eq!(http(a, "POST", "/api/input", "{\"throttle\":1.0,\"steer\":0.3}").0, 200);

        let mut stream = Stream::open(a);
        let got: Vec<Value> =
            (0..300).map(|_| stream.next_frame()).filter(|f| f["vehicle"] == "scout_4x4").take(3).collect();
        assert_eq!(got.len(), 3, "never saw three frames of the scout");
        for f in &got {
            assert_eq!(f["pos_m"].as_array().expect("pos").len(), 3);
            assert_eq!(f["rot"].as_array().expect("rot").len(), 4);
            assert_eq!(f["joints"].as_array().expect("joints").len(), joints);
            assert_eq!(f["contacts"].as_array().expect("contacts").len(), 4);
            assert!(
                f["contacts"][0]["in_contact"].as_bool().expect("flag")
                    && f["contacts"][0]["normal_n"].as_f64().expect("n") > 0.0
            );
            assert!(f["engine_rpm"].as_f64().expect("rpm") > 0.0 && f["gear"].is_i64() && f["speed_m_s"].is_f64());
            assert_eq!(f["assist"]["on"], true);
            assert!(f["t_s"].as_f64().expect("t") > 0.0);
        }
        let dt = got[2]["t_s"].as_f64().expect("t") - got[0]["t_s"].as_f64().expect("t");
        assert!((dt - 2.0 / 30.0).abs() < 1e-3, "three consecutive frames are 1/30 s apart, got {dt} over two gaps");

        assert_eq!(http(a, "GET", "/", ""), (200, "<h1>hello</h1>".to_string()));
        assert_eq!(http(a, "GET", "/../Cargo.toml", "").0, 400, "no way out of the web folder");
        assert_eq!(http(a, "GET", "/missing.js", "").0, 404);
        assert_eq!(http(a, "DELETE", "/api/input", "").0, 405);
        s.shared.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = std::fs::remove_dir_all(&web);
    }

    #[test]
    fn a_reset_over_http_puts_the_vehicle_back_on_the_road_and_finish_writes_the_replay() {
        let rec = temp("rec");
        let s = server(None, Some(rec.clone()));
        let a = s.addr;
        let mut stream = Stream::open(a);
        assert_eq!(http(a, "POST", "/api/input", "{\"reset\":true}").0, 200);
        let seen = stream.until_event();
        let last = seen.last().expect("frames");
        assert_eq!(last["message_event"], "back on the road", "no message_event after a reset");
        assert!(last["pos_m"][1].as_f64().expect("y").is_finite());
        (0..20).for_each(|_| drop(stream.next_frame())); // a third of a second of driving to record
        let (status, body) = http(a, "POST", "/api/finish", "");
        assert_eq!(status, 200, "{body}");
        let file = serde_json::from_str::<Value>(&body).expect("json")["file"].as_str().expect("file").to_string();
        let replay = w5k_replay::read_bin(Path::new(&file)).expect("replay is readable");
        assert!(replay.frames.len() > 10 && replay.header.vehicles[0].name == "mule_4x4");
        s.shared.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = std::fs::remove_dir_all(&rec);
    }

    #[test]
    fn the_protocol_document_lists_every_key_the_server_emits_and_accepts() {
        let doc =
            std::fs::read_to_string(root().join("docs/swarm/requests/arch-drive-protocol.md")).expect("protocol doc");
        let rec = temp("doc");
        let s = server(None, Some(rec.clone()));
        let a = s.addr;
        let mut emitted = BTreeSet::new();
        keys(&get_json(a, "/api/vehicles"), &mut emitted);
        let mut stream = Stream::open(a);
        http(a, "POST", "/api/input", "{\"reset\":true}");
        let seen = stream.until_event();
        assert!(seen.last().expect("frames").get("message_event").is_some(), "never saw a message_event to check");
        keys(&Value::Array(seen), &mut emitted);
        keys(&serde_json::from_str(&http(a, "POST", "/api/finish", "").1).expect("json"), &mut emitted);
        keys(
            &serde_json::from_str(&http(a, "POST", "/api/select", "{\"vehicle\":\"nope\"}").1).expect("json"),
            &mut emitted,
        );
        emitted.extend(["throttle", "brake", "steer", "reset", "reverse", "vehicle"].map(String::from));
        // The terrain's own keys are documented in world-viewer-terrain.md, the rig's in the contract.
        assert!(emitted.len() > 20, "{emitted:?}");
        for k in &emitted {
            assert!(
                doc.contains(&format!("\"{k}\"")),
                "docs/swarm/requests/arch-drive-protocol.md does not list \"{k}\""
            );
        }
        s.shared.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let _ = std::fs::remove_dir_all(&rec);
    }
}
