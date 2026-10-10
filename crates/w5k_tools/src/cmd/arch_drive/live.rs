//! The real-time half of `w5k drive`: the state shared between the sockets and the simulation thread, the recorder, and the paced loop.
//!
//! Pacing: the loop wakes about every 2 ms, runs the ticks that are due on the wall clock (at most `max_ticks_per_wake`, so a slow
//! machine slows the simulation down instead of spiralling), and publishes every `publish_every_ticks`-th tick as a stream frame.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use w5k_contract::frame::{Frame, ReplayHeader, VehicleHeader, WorldHeader, REPLAY_VERSION};
use w5k_contract::rig::TICK_HZ;
use w5k_replay::ReplayFile;

use super::assist::Raw;
use super::session::{Car, Scene, Session};

const RING_FRAMES: usize = 64; // const-ok: stream frames kept for slow clients (about 2 s at 30 Hz)
const WAKE: Duration = Duration::from_millis(2); // const-ok: longest sleep between wakes, bounds the added input latency

/// A request for the simulation thread.
pub(crate) enum Cmd {
    Select(String),
    Reset,
    /// Write the recording; the reply is the file name (`None` when not recording or nothing recorded).
    Finish(Sender<Result<Option<String>, String>>),
}

#[derive(Default)]
struct Ring {
    next_seq: u64,
    frames: VecDeque<(u64, Arc<str>)>,
}

/// What the HTTP threads and the simulation thread share.
#[derive(Default)]
pub(crate) struct Shared {
    input: Mutex<Option<(Raw, Instant)>>,
    cmds: Mutex<VecDeque<Cmd>>,
    ring: Mutex<Ring>,
    cv: Condvar,
    /// Connected `/api/stream` clients.
    pub clients: AtomicUsize,
    pub stop: AtomicBool,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Shared {
    pub(crate) fn set_input(&self, raw: Raw) {
        *lock(&self.input) = Some((raw.clamped(), Instant::now()));
    }

    /// The pedals as of `now`: released when nothing has been received for `timeout_s`.
    fn current(&self, now: Instant, timeout_s: f64) -> Raw {
        match *lock(&self.input) {
            Some((raw, at)) if now.saturating_duration_since(at).as_secs_f64() <= timeout_s => raw,
            _ => Raw::default(),
        }
    }

    pub(crate) fn push_cmd(&self, c: Cmd) {
        lock(&self.cmds).push_back(c);
    }

    fn publish(&self, json: String) {
        let mut r = lock(&self.ring);
        let seq = r.next_seq;
        r.next_seq += 1;
        r.frames.push_back((seq, json.into()));
        while r.frames.len() > RING_FRAMES {
            r.frames.pop_front();
        }
        self.cv.notify_all();
    }

    /// The sequence number a new client starts at (the next frame).
    pub(crate) fn subscribe(&self) -> u64 {
        lock(&self.ring).next_seq
    }

    /// Frames with sequence number `>= from` (waiting up to `wait` for the first), and the number to ask for next.
    pub(crate) fn frames_since(&self, from: u64, wait: Duration) -> (Vec<Arc<str>>, u64) {
        let mut g = lock(&self.ring);
        if g.next_seq <= from {
            g = self.cv.wait_timeout(g, wait).unwrap_or_else(PoisonError::into_inner).0;
        }
        (g.frames.iter().filter(|(s, _)| *s >= from).map(|(_, j)| j.clone()).collect(), g.next_seq)
    }
}

/// The session as a replay (one vehicle; segments are written when the vehicle changes or the session ends).
pub(crate) struct Recorder {
    dir: PathBuf,
    frames: Vec<Frame>,
    hashes: Vec<u64>,
    header: Option<(VehicleHeader, String)>,
    frame_dt_s: f64,
    course: String,
    seed: u64,
    written: usize,
}

impl Recorder {
    pub(crate) fn new(dir: PathBuf, scene: &Scene) -> Recorder {
        let frame_dt_s = f64::from(scene.assist_tuning.publish_every_ticks) / TICK_HZ;
        Recorder {
            dir,
            frames: Vec::new(),
            hashes: Vec::new(),
            header: None,
            frame_dt_s,
            course: scene.course.clone(),
            seed: scene.seed,
            written: 0,
        }
    }

    fn begin(&mut self, car: &Car) {
        let v = VehicleHeader {
            name: car.id.clone(),
            rig_id: car.rig.id.clone(),
            joint_names: car.rig.joint_names(),
            contact_names: car.rig.contact_names(),
            livery: None,
        };
        self.header = Some((v, car.id.clone()));
    }

    fn capture(&mut self, s: &Session) {
        self.frames.push(Frame {
            t_s: s.time_s(),
            vehicles: vec![s.frame()],
            events: Vec::new(),
            projectiles: Vec::new(),
        });
        if s.ticks.is_multiple_of(TICK_HZ as u64) {
            self.hashes.push(s.state_hash());
        }
    }

    /// Write what was recorded since the last flush; `replay.w5kr` first, then `replay_2.w5kr`, ...
    pub(crate) fn flush(&mut self) -> Result<Option<String>, String> {
        let Some((vehicle, id)) = self.header.clone().filter(|_| !self.frames.is_empty()) else {
            return Ok(None);
        };
        let replay = ReplayFile {
            header: ReplayHeader {
                version: REPLAY_VERSION,
                scenario: format!("drive: {id} on {}", self.course),
                frame_dt_s: self.frame_dt_s,
                vehicles: vec![vehicle],
                world: WorldHeader { course: self.course.clone(), seed: self.seed, terrain: None },
                state_hashes: std::mem::take(&mut self.hashes),
            },
            frames: std::mem::take(&mut self.frames),
        };
        self.written += 1;
        let name = if self.written == 1 { "replay.w5kr".to_string() } else { format!("replay_{}.w5kr", self.written) };
        std::fs::create_dir_all(&self.dir).map_err(|e| format!("cannot create {}: {e}", self.dir.display()))?;
        let path = self.dir.join(name);
        w5k_replay::write_bin(&path, &replay)?;
        Ok(Some(path.to_string_lossy().into_owned()))
    }
}

/// How many ticks are due at `now` (at most `cap`); advances `next`, and when still behind after the cap drops the backlog so the
/// simulation slows down on a slow machine instead of spiralling.
fn due_ticks(next: &mut Instant, now: Instant, step: Duration, cap: u32) -> u32 {
    let mut n = 0;
    while *next <= now && n < cap {
        *next += step;
        n += 1;
    }
    if *next <= now {
        *next = now;
    }
    n
}

/// Owns the session and runs it against the wall clock.
pub(crate) struct Runner {
    shared: Arc<Shared>,
    garage: Vec<Arc<Car>>,
    scene: Arc<Scene>,
    session: Session,
    assist_on: bool,
    rec: Option<Recorder>,
    busy: Duration,
    ticks_run: u64,
    quiet_since: Option<Instant>,
    seen_client: bool,
}

impl Runner {
    pub(crate) fn new(
        shared: Arc<Shared>,
        garage: Vec<Arc<Car>>,
        scene: Arc<Scene>,
        first: Arc<Car>,
        assist_on: bool,
        rec: Option<Recorder>,
    ) -> Result<Runner, String> {
        let session = Session::new(scene.clone(), first, assist_on)?;
        let mut r = Runner {
            shared,
            garage,
            scene,
            session,
            assist_on,
            rec,
            busy: Duration::ZERO,
            ticks_run: 0,
            quiet_since: None,
            seen_client: false,
        };
        r.begin_recording();
        Ok(r)
    }

    fn begin_recording(&mut self) {
        if let Some(rec) = &mut self.rec {
            rec.begin(&self.session.car);
        }
    }

    /// Simulated seconds per busy wall-clock second (above 1 means the machine keeps up in real time with room to spare).
    pub(crate) fn real_time_factor(&self) -> f64 {
        self.ticks_run as f64 / TICK_HZ / self.busy.as_secs_f64().max(f64::MIN_POSITIVE)
    }

    fn finish(&mut self) -> Result<Option<String>, String> {
        eprintln!("w5k drive: real-time factor {:.1}x ({} ticks)", self.real_time_factor(), self.ticks_run);
        self.rec.as_mut().map_or(Ok(None), Recorder::flush)
    }

    fn handle(&mut self, c: Cmd) {
        match c {
            Cmd::Reset => self.session.recover(),
            Cmd::Finish(tx) => {
                let _ = tx.send(self.finish());
            }
            Cmd::Select(id) => {
                let Some(car) = self.garage.iter().find(|c| c.id == id).cloned() else { return };
                if let Some(rec) = &mut self.rec {
                    if let Err(e) = rec.flush() {
                        eprintln!("w5k drive: cannot write the recording: {e}");
                    }
                }
                match Session::new(self.scene.clone(), car, self.assist_on) {
                    Ok(s) => self.session = s,
                    Err(e) => eprintln!("w5k drive: {e}"),
                }
                self.begin_recording();
            }
        }
    }

    /// One tick with `raw`; every `publish_every_ticks`-th tick is published (and recorded).
    pub(crate) fn tick(&mut self, raw: &Raw) {
        let started = Instant::now();
        self.session.step(raw);
        self.ticks_run += 1;
        if self.session.ticks.is_multiple_of(u64::from(self.scene.assist_tuning.publish_every_ticks)) {
            let json = self.session.stream_json();
            self.shared.publish(json);
            if let Some(rec) = &mut self.rec {
                rec.capture(&self.session);
            }
        }
        self.busy += started.elapsed();
    }

    /// Stop the recording when the last client has been gone for `idle_finish_s`.
    fn watch_clients(&mut self, now: Instant) {
        if self.shared.clients.load(Ordering::Relaxed) > 0 {
            (self.seen_client, self.quiet_since) = (true, None);
        } else if self.seen_client && self.quiet_since.is_none() {
            self.quiet_since = Some(now);
        }
        let gone = self
            .quiet_since
            .is_some_and(|q| now.duration_since(q).as_secs_f64() >= self.scene.assist_tuning.idle_finish_s.v);
        if gone {
            (self.seen_client, self.quiet_since) = (false, None);
            match self.finish() {
                Ok(Some(f)) => eprintln!("w5k drive: last client gone, wrote {f}"),
                Ok(None) => {}
                Err(e) => eprintln!("w5k drive: cannot write the recording: {e}"),
            }
        }
    }

    /// The paced loop; returns when `shared.stop` is set.
    pub(crate) fn run(mut self) {
        let t = self.scene.assist_tuning.clone();
        let step = Duration::from_secs_f64(1.0 / TICK_HZ);
        let mut next = Instant::now();
        while !self.shared.stop.load(Ordering::Relaxed) {
            let cmd = lock(&self.shared.cmds).pop_front();
            if let Some(c) = cmd {
                self.handle(c);
                continue;
            }
            let now = Instant::now();
            for _ in 0..due_ticks(&mut next, now, step, t.max_ticks_per_wake) {
                let raw = self.shared.current(now, t.input_timeout_s.v);
                self.tick(&raw);
            }
            self.watch_clients(now);
            std::thread::sleep(next.saturating_duration_since(Instant::now()).min(WAKE));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::arch_drive::session::testing::{car, flat_scene};

    fn runner(rec_dir: Option<PathBuf>) -> Runner {
        let scene = flat_scene(|_| ());
        let rec = rec_dir.map(|d| Recorder::new(d, &scene));
        Runner::new(
            Arc::new(Shared::default()),
            vec![car("scout_4x4"), car("mule_4x4")],
            scene,
            car("scout_4x4"),
            true,
            rec,
        )
        .expect("runner")
    }

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("w5k_drive_{name}_{}", std::process::id()))
    }

    #[test]
    fn a_slow_machine_is_capped_at_four_ticks_per_wake_and_drops_the_backlog() {
        let (step, now) = (Duration::from_secs_f64(1.0 / TICK_HZ), Instant::now());
        let mut next = now - Duration::from_secs(10);
        assert_eq!(due_ticks(&mut next, now, step, 4), 4);
        assert_eq!(next, now, "ten seconds of backlog must be dropped, not worked off");
        let mut next = now + step;
        assert_eq!(due_ticks(&mut next, now, step, 4), 0, "nothing is due before its time");
        let mut next = now - step * 2;
        assert_eq!(due_ticks(&mut next, now, step, 4), 3, "ticks at -2, -1 and 0 steps are due");
    }

    #[test]
    fn pedals_are_released_when_no_input_arrives_for_the_timeout() {
        let s = Shared::default();
        let now = Instant::now();
        assert_eq!(s.current(now, 0.5), Raw::default(), "before any input");
        s.set_input(Raw { throttle: 1.5, brake: 0.0, steer: -3.0, reverse: false });
        let held = s.current(Instant::now(), 0.5);
        assert_eq!((held.throttle, held.steer), (1.0, -1.0), "clamped and held");
        assert_eq!(s.current(Instant::now() + Duration::from_millis(600), 0.5), Raw::default());
    }

    #[test]
    fn a_stream_client_receives_every_published_frame_in_order_and_a_new_one_starts_at_the_next() {
        let s = Shared::default();
        let from = s.subscribe();
        for k in 0..5 {
            s.publish(format!("{{\"k\":{k}}}"));
        }
        let (frames, next) = s.frames_since(from, Duration::from_millis(1));
        assert_eq!(
            frames.iter().map(|f| &**f).collect::<Vec<_>>(),
            ["{\"k\":0}", "{\"k\":1}", "{\"k\":2}", "{\"k\":3}", "{\"k\":4}"]
        );
        assert_eq!(s.subscribe(), next, "a new client misses the old frames");
        assert!(s.frames_since(next, Duration::from_millis(1)).0.is_empty());
        for k in 0..(RING_FRAMES + 10) {
            s.publish(k.to_string());
        }
        assert_eq!(
            s.frames_since(0, Duration::ZERO).0.len(),
            RING_FRAMES,
            "a client that fell far behind loses the oldest"
        );
    }

    #[test]
    fn the_recording_is_written_as_a_replay_when_the_last_client_has_been_gone_for_ten_seconds() {
        let dir = temp("idle");
        let mut r = runner(Some(dir.clone()));
        for _ in 0..120 {
            r.tick(&Raw { throttle: 1.0, ..Raw::default() });
        }
        let t0 = Instant::now();
        r.shared.clients.store(1, Ordering::Relaxed);
        r.watch_clients(t0);
        r.shared.clients.store(0, Ordering::Relaxed);
        r.watch_clients(t0 + Duration::from_secs(1));
        r.watch_clients(t0 + Duration::from_secs(9));
        assert!(!dir.join("replay.w5kr").exists(), "nine seconds is not ten");
        r.watch_clients(t0 + Duration::from_secs(12));
        let replay = w5k_replay::read_bin(&dir.join("replay.w5kr")).expect("replay written");
        assert_eq!(replay.header.vehicles[0].name, "scout_4x4");
        assert_eq!(replay.frames.len(), 60, "120 ticks, one frame per two");
        assert!(replay.frames.iter().all(|f| f.vehicles[0].is_finite()));
        assert_eq!(replay.header.state_hashes.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn finish_writes_the_segments_and_selecting_another_vehicle_starts_a_new_one() {
        let dir = temp("finish");
        let mut r = runner(Some(dir.clone()));
        r.tick(&Raw::default());
        r.tick(&Raw::default());
        r.handle(Cmd::Select("mule_4x4".into()));
        assert!(dir.join("replay.w5kr").exists(), "the first vehicle's segment is written on select");
        assert_eq!(r.session.car.id, "mule_4x4");
        r.tick(&Raw::default());
        r.tick(&Raw::default());
        let (tx, rx) = std::sync::mpsc::channel();
        r.handle(Cmd::Finish(tx));
        let file = rx.recv().expect("reply").expect("written").expect("a file");
        assert!(file.ends_with("replay_2.w5kr"), "{file}");
        let (tx, rx) = std::sync::mpsc::channel();
        r.handle(Cmd::Finish(tx));
        assert_eq!(rx.recv().expect("reply"), Ok(None), "nothing new to write");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn selecting_an_unknown_vehicle_changes_nothing() {
        let mut r = runner(None);
        r.handle(Cmd::Select("nope".into()));
        assert_eq!(r.session.car.id, "scout_4x4");
    }
}
