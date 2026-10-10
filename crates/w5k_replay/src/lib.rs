//! `w5k_replay`: lane VIEWER. Reading and writing replay v2, the interface between the simulation and every viewer.
//!
//! This is the **Launch Kit version**: plain JSON of `{header, frames}`, enough for the viewers and the first-light spine to start.
//! Lane VIEWER replaces the encoding with the compact one (quantised, about 30 Hz, a few tens of bytes per vehicle-frame, binary
//! with a JSON header) behind the same function names, and keeps this JSON form as the debugging format. The data model itself
//! (`Frame`, `ReplayHeader`) belongs to the contract (`w5k_contract::frame`), not to this crate.

pub mod binary;

use std::path::Path;

use serde::{Deserialize, Serialize};
use w5k_contract::frame::{Frame, ReplayHeader};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReplayFile {
    pub header: ReplayHeader,
    pub frames: Vec<Frame>,
}

pub fn to_json_string(r: &ReplayFile) -> Result<String, String> {
    serde_json::to_string(r).map_err(|e| format!("cannot serialise the replay: {e}"))
}

pub fn from_json_str(s: &str) -> Result<ReplayFile, String> {
    serde_json::from_str(s).map_err(|e| format!("cannot parse the replay: {e}"))
}

pub fn write_json(path: &Path, r: &ReplayFile) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    std::fs::write(path, to_json_string(r)?).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

pub fn read_json(path: &Path) -> Result<ReplayFile, String> {
    let s = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    from_json_str(&s)
}

pub fn write_bin(path: &Path, r: &ReplayFile) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    std::fs::write(path, binary::encode(r)?).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

pub fn read_bin(path: &Path) -> Result<ReplayFile, String> {
    binary::decode(&std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use w5k_contract::testing::truck_over_bumps;

    #[test]
    fn a_replay_survives_a_json_round_trip() {
        let (header, frames, _) = truck_over_bumps();
        let r = ReplayFile { header, frames };
        let back = from_json_str(&to_json_string(&r).unwrap()).unwrap();
        assert_eq!(back.header, r.header);
        assert_eq!(back.frames.len(), r.frames.len());
        for (a, b) in back.frames.iter().zip(&r.frames) {
            assert!((a.vehicles[0].pos_m - b.vehicles[0].pos_m).length() < 1e-9);
            assert_eq!(a.vehicles[0].joints.len(), b.vehicles[0].joints.len());
        }
    }
}
