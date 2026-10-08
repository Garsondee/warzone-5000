//! Replays: what a run looked like, in a form any viewer can play.
//!
//! The simulation records one [`Frame`] per tick (20 Hz). A frame is 16 bytes of integers (millimetres, milliradians,
//! centimetres per second...), so a three-minute run is under 60 KB and the viewers never touch fixed point. The run also
//! carries a state hash every second and at the end: replaying a recorded run on another machine must reproduce them
//! exactly (this is how desyncs and platform differences are caught).

use serde::{Deserialize, Serialize};

/// Ticks per second.
pub const HZ: u32 = 20;
/// Size of one encoded frame.
pub const FRAME_BYTES: usize = 16;

/// Run state, the low nibble of [`Frame::state`].
pub mod run_state {
    pub const RUNNING: u8 = 0;
    pub const FINISHED: u8 = 1;
    pub const STOPPED: u8 = 2;
    pub const NOT_STARTED: u8 = 3;
}

/// What limited the vehicle during a tick, the high nibble of [`Frame::state`]. These are what the viewers use to explain
/// a run: "power-limited on the climb", "grip-limited at the valley entry", "stalled in the valley".
pub mod limit {
    /// Nothing recorded (parade laps, coasting).
    pub const NONE: u8 = 0;
    /// The engine could not push harder: force = power / speed.
    pub const POWER: u8 = 1;
    /// The ground could not take more: force = grip x load.
    pub const GRIP: u8 = 2;
    /// At the running gear's rated speed.
    pub const RATING: u8 = 3;
    /// The soil's resistance dominated (sinking, compaction).
    pub const SOIL: u8 = 4;
    /// The vehicle is not making progress.
    pub const STALLED: u8 = 5;
}

/// One tick of one vehicle.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Frame {
    /// Distance along the course (mm); the vehicle's reference point is the centre of its support.
    pub s_mm: i32,
    /// Height of that point (mm), on the course datum (the start pad is 0).
    pub y_mm: i32,
    /// Nose-up pitch (milliradians).
    pub pitch_mrad: i16,
    /// Speed along the course (cm/s).
    pub v_cms: i16,
    /// How far the vehicle has sunk into the ground (mm).
    pub sink_mm: u16,
    /// Wheel or track slip (percent).
    pub slip_pct: u8,
    /// `limit << 4 | run_state`.
    pub state: u8,
}

impl Frame {
    pub fn run_state(&self) -> u8 {
        self.state & 0x0f
    }

    pub fn limit(&self) -> u8 {
        self.state >> 4
    }

    pub fn to_bytes(&self) -> [u8; FRAME_BYTES] {
        let mut b = [0u8; FRAME_BYTES];
        b[0..4].copy_from_slice(&self.s_mm.to_le_bytes());
        b[4..8].copy_from_slice(&self.y_mm.to_le_bytes());
        b[8..10].copy_from_slice(&self.pitch_mrad.to_le_bytes());
        b[10..12].copy_from_slice(&self.v_cms.to_le_bytes());
        b[12..14].copy_from_slice(&self.sink_mm.to_le_bytes());
        b[14] = self.slip_pct;
        b[15] = self.state;
        b
    }

    pub fn from_bytes(b: &[u8; FRAME_BYTES]) -> Frame {
        Frame {
            s_mm: i32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            y_mm: i32::from_le_bytes([b[4], b[5], b[6], b[7]]),
            pitch_mrad: i16::from_le_bytes([b[8], b[9]]),
            v_cms: i16::from_le_bytes([b[10], b[11]]),
            sink_mm: u16::from_le_bytes([b[12], b[13]]),
            slip_pct: b[14],
            state: b[15],
        }
    }
}

/// Why a run stopped short.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cause {
    /// Stuck in soft ground: the soil resists more than the tracks or wheels can push.
    Bogged,
    /// Not enough engine or grip for the slope.
    Stalled,
    /// Still driving when the clock ran out.
    TimedOut,
}

/// How a run ended. Failing to finish is an expected, informative result: every outcome says why.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    /// Crossed the finish line: the time and the time at each checkpoint after the start (in ticks).
    Finished { ticks: u32, splits: Vec<u32> },
    /// Did not finish: the cause, how far it got (mm), when it stopped (ticks) and the numbers behind it.
    Dnf { cause: Cause, s_mm: i32, ticks: u32, detail: String },
    /// Did not start: the design cannot run this course (no rails, invalid design...).
    Dns { cause: String },
}

impl Outcome {
    /// Finishing time in seconds, if it finished.
    pub fn time_s(&self) -> Option<f64> {
        match self {
            Outcome::Finished { ticks, .. } => Some(*ticks as f64 / HZ as f64),
            _ => None,
        }
    }
}

/// One vehicle's run over the course.
#[derive(Clone, Debug)]
pub struct Run {
    pub id: String,
    pub outcome: Outcome,
    pub frames: Vec<Frame>,
    /// State hash after every `HZ` ticks (once a second).
    pub checkpoint_hashes: Vec<u64>,
    /// State hash after the last tick.
    pub final_hash: u64,
}

impl Run {
    /// A run that never left the line.
    pub fn did_not_start(id: &str, cause: &str) -> Run {
        Run { id: id.to_string(), outcome: Outcome::Dns { cause: cause.to_string() }, frames: Vec::new(), checkpoint_hashes: Vec::new(), final_hash: 0 }
    }

    /// All frames as one byte string (16 bytes each, little-endian).
    pub fn frame_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.frames.len() * FRAME_BYTES);
        for f in &self.frames {
            out.extend_from_slice(&f.to_bytes());
        }
        out
    }
}
