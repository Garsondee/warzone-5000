//! Replay v2 binary encoding: `"W5KR" | u32 version | u32 header_len | header JSON | u32 frame_count | groups`.
//!
//! A group is `u32 len | payload` and holds [`KEYFRAME_INTERVAL`] frames, so a viewer can seek to any second. Every frame is flattened into
//! a list of integers (time in microseconds, position in mm, rotation as smallest-three, each joint in its own quantum, forces in
//! newtons, ...), and each list is written as zig-zag varint **differences from the previous frame's list** (from zeros at the
//! first frame of a group). Smooth motion therefore costs one or two bytes per field. Events and projectiles ride in a per-frame
//! extension block (JSON, length-prefixed), so readers that predate a new field can skip it.

use w5k_contract::frame::{ContactFrame, Event, Frame, ProjectileFrame, ReplayHeader, VehicleFrame, WeaponFrame};
use w5k_contract::vehicle::LimitingFactor;
use w5k_math::{scalar, Quat, Vec3};

use crate::ReplayFile;

const MAGIC: &[u8; 4] = b"W5KR";
/// Frames per group (one second at 30 Hz). const-ok: file layout.
pub const KEYFRAME_INTERVAL: usize = 30;
/// Position quantum 1 mm, velocity quantum 1 mm/s, travel quantum 0.05 mm, spin/steer/aim quantum 2 pi / 65536 rad (0.096 mrad).
/// const-ok: quantisation steps of the file format, not physical constants.
const MM: f64 = 1000.0;
const TRAVEL_STEPS_PER_M: f64 = 20_000.0; // const-ok: quantisation step of the file format
const ANGLE_STEPS_PER_RAD: f64 = 65536.0 / std::f64::consts::TAU; // const-ok: quantisation step of the file format
/// Smallest-three rotation: components lie within +-1/sqrt(2), mapped onto +-32767 (step 2.2e-5, under 0.003 degrees).
const ROT_STEPS: f64 = 32767.0 * std::f64::consts::SQRT_2; // const-ok: quantisation step of the file format
const LIMITING: [LimitingFactor; 10] = [
    LimitingFactor::None,
    LimitingFactor::Power,
    LimitingFactor::Grip,
    LimitingFactor::Soil,
    LimitingFactor::Sinkage,
    LimitingFactor::Brake,
    LimitingFactor::Suspension,
    LimitingFactor::Stuck,
    LimitingFactor::Overturned,
    LimitingFactor::None,
];

fn q(v: f64, steps: f64) -> i64 {
    (v * steps).round() as i64
}

/// How many steps per unit a joint coordinate uses: metres for travel and recoil, radians for the rest.
fn joint_steps(name: &str) -> f64 {
    if name.contains("travel") || name.contains("recoil") {
        TRAVEL_STEPS_PER_M
    } else {
        ANGLE_STEPS_PER_RAD
    }
}

fn put_vec(out: &mut Vec<i64>, v: Vec3, steps: f64) {
    out.extend([q(v.x, steps), q(v.y, steps), q(v.z, steps)]);
}

fn get_vec(it: &mut impl Iterator<Item = i64>, steps: f64) -> Result<Vec3, String> {
    Ok(Vec3 { x: next(it)? as f64 / steps, y: next(it)? as f64 / steps, z: next(it)? as f64 / steps })
}

fn next(it: &mut impl Iterator<Item = i64>) -> Result<i64, String> {
    it.next().ok_or_else(|| "the frame ends early".to_string())
}

fn put_rot(out: &mut Vec<i64>, r: Quat) {
    let r = r.normalized();
    let c = [r.w, r.x, r.y, r.z];
    let big = (0..4).fold(0, |b, i| if c[i].abs() > c[b].abs() { i } else { b });
    let sign = if c[big] < 0.0 { -1.0 } else { 1.0 };
    out.push(big as i64);
    out.extend((0..4).filter(|&i| i != big).map(|i| q(c[i] * sign, ROT_STEPS)));
}

fn get_rot(it: &mut impl Iterator<Item = i64>) -> Result<Quat, String> {
    let big = next(it)? as usize;
    if big > 3 {
        return Err("bad rotation index".to_string());
    }
    let mut c = [0.0; 4];
    let mut sum = 0.0;
    for (i, slot) in c.iter_mut().enumerate() {
        if i != big {
            *slot = next(it)? as f64 / ROT_STEPS;
            sum += *slot * *slot;
        }
    }
    c[big] = scalar::sqrt((1.0 - sum).max(0.0));
    Ok(Quat::new(c[0], c[1], c[2], c[3]))
}

fn flatten(f: &Frame, joint_names: &[Vec<String>]) -> Vec<i64> {
    let mut o = vec![q(f.t_s, 1e6), f.vehicles.len() as i64]; // const-ok: quantisation step of the file format
    for v in &f.vehicles {
        o.extend([
            v.vehicle as i64,
            v.joints.len() as i64,
            v.contacts.len() as i64,
            v.ledger_n.len() as i64,
            v.weapons.len() as i64,
        ]);
        put_vec(&mut o, v.pos_m, MM);
        put_rot(&mut o, v.rot);
        put_vec(&mut o, v.lin_vel_m_s, MM);
        put_vec(&mut o, v.ang_vel_rad_s, MM);
        let names = &joint_names[(v.vehicle as usize).min(joint_names.len() - 1)];
        for (k, j) in v.joints.iter().enumerate() {
            o.push(q(f64::from(*j), names.get(k).map_or(ANGLE_STEPS_PER_RAD, |n| joint_steps(n))));
        }
        o.extend([
            q(f64::from(v.engine_rpm), 1.0),
            i64::from(v.gear),
            LIMITING.iter().position(|l| *l == v.limiting).unwrap_or(0) as i64,
        ]);
        for c in &v.contacts {
            o.extend([
                i64::from(c.flags),
                q(f64::from(c.normal_force_n), 0.25),
                q(f64::from(c.sinkage_m), 10_000.0), // const-ok: quantisation step of the file format
                q(f64::from(c.slip), 1000.0),        // const-ok: quantisation step of the file format
                i64::from(c.material),
            ]);
        }
        o.extend(v.ledger_n.iter().map(|x| q(f64::from(*x), 1.0)));
        for w in &v.weapons {
            o.extend([
                i64::from(w.ready),
                q(f64::from(w.reload_s), 1000.0), // const-ok: quantisation step of the file format
                i64::from(w.rounds),
                q(f64::from(w.aim_error_rad), 100_000.0), // const-ok: quantisation step of the file format
            ]);
        }
    }
    o
}

fn unflatten(ints: &[i64], joint_names: &[Vec<String>]) -> Result<Frame, String> {
    let mut it = ints.iter().copied();
    let t_s = next(&mut it)? as f64 / 1e6; // const-ok: quantisation step of the file format
    let n = next(&mut it)? as usize;
    let mut vehicles = Vec::with_capacity(n);
    for _ in 0..n {
        let vehicle = next(&mut it)? as u32;
        let (nj, nc, nl, nw) =
            (next(&mut it)? as usize, next(&mut it)? as usize, next(&mut it)? as usize, next(&mut it)? as usize);
        let pos_m = get_vec(&mut it, MM)?;
        let rot = get_rot(&mut it)?;
        let lin_vel_m_s = get_vec(&mut it, MM)?;
        let ang_vel_rad_s = get_vec(&mut it, MM)?;
        let names = joint_names.get(vehicle as usize).ok_or("vehicle index outside the header")?;
        let mut joints = Vec::with_capacity(nj);
        for k in 0..nj {
            joints.push((next(&mut it)? as f64 / names.get(k).map_or(ANGLE_STEPS_PER_RAD, |n| joint_steps(n))) as f32);
        }
        let engine_rpm = next(&mut it)? as f32;
        let gear = next(&mut it)? as i8;
        let limiting = LIMITING.get(next(&mut it)? as usize).copied().unwrap_or_default();
        let mut contacts = Vec::with_capacity(nc);
        for _ in 0..nc {
            contacts.push(ContactFrame {
                flags: next(&mut it)? as u8,
                normal_force_n: (next(&mut it)? as f64 / 0.25) as f32,
                sinkage_m: (next(&mut it)? as f64 / 10_000.0) as f32, // const-ok: quantisation step of the file format
                slip: (next(&mut it)? as f64 / 1000.0) as f32,        // const-ok: quantisation step of the file format
                material: next(&mut it)? as u16,
            });
        }
        let mut ledger_n = Vec::with_capacity(nl);
        for _ in 0..nl {
            ledger_n.push(next(&mut it)? as f32);
        }
        let mut weapons = Vec::with_capacity(nw);
        for _ in 0..nw {
            weapons.push(WeaponFrame {
                ready: next(&mut it)? != 0,
                reload_s: (next(&mut it)? as f64 / 1000.0) as f32, // const-ok: quantisation step of the file format
                rounds: next(&mut it)? as u16,
                aim_error_rad: (next(&mut it)? as f64 / 100_000.0) as f32, // const-ok: quantisation step of the file format
            });
        }
        vehicles.push(VehicleFrame {
            vehicle,
            pos_m,
            rot,
            lin_vel_m_s,
            ang_vel_rad_s,
            joints,
            engine_rpm,
            gear,
            contacts,
            ledger_n,
            limiting,
            weapons,
        });
    }
    Ok(Frame { t_s, vehicles, events: Vec::new(), projectiles: Vec::new() })
}

/// Zero-run packing of a group payload: a zero byte (an unchanged field) is followed by the number of further zeros it stands for.
fn rle(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        out.push(raw[i]);
        if raw[i] == 0 {
            let run = raw[i + 1..].iter().take(255).take_while(|b| **b == 0).count();
            out.push(run as u8);
            i += run;
        }
        i += 1;
    }
    out
}

fn unrle(packed: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(packed.len() * 2);
    let mut it = packed.iter();
    while let Some(&b) = it.next() {
        out.push(b);
        if b == 0 {
            let run = *it.next().ok_or("a zero run is cut short")?;
            out.resize(out.len() + run as usize, 0);
        }
    }
    Ok(out)
}

fn put_varint(out: &mut Vec<u8>, v: i64) {
    let mut z = ((v << 1) ^ (v >> 63)) as u64;
    while z >= 0x80 {
        out.push((z & 0x7f) as u8 | 0x80);
        z >>= 7;
    }
    out.push(z as u8);
}

fn get_varint(b: &[u8], pos: &mut usize) -> Result<i64, String> {
    let (mut z, mut shift) = (0u64, 0);
    loop {
        let byte = *b.get(*pos).ok_or("the replay ends inside a number")?;
        *pos += 1;
        z |= u64::from(byte & 0x7f) << shift;
        if byte < 0x80 {
            return Ok(((z >> 1) as i64) ^ -((z & 1) as i64));
        }
        shift += 7;
        if shift > 63 {
            return Err("number too long".to_string());
        }
    }
}

/// Is slot `i` of a frame's integer list smooth in time (pose, velocities, joints)? Those are predicted from the two previous
/// frames (second differences are tiny, wheel spin costs one byte instead of two); contacts and forces are noisy and use the
/// previous frame alone. Decoder and encoder call this with the slots decoded so far, so it only looks at earlier counts.
fn smooth(i: usize, ints: &[i64]) -> bool {
    let mut base = 2;
    while i >= base {
        if i < base + 5 {
            return false;
        }
        let nj = ints[base + 1] as usize;
        let len = VEHICLE_FIXED
            + nj
            + 3
            + 5 * ints[base + 2] as usize
            + ints[base + 3] as usize
            + 4 * ints[base + 4] as usize;
        if i < base + VEHICLE_FIXED + nj {
            return true;
        }
        base += len;
    }
    false
}

/// Integers before the joints in a vehicle block: five ids and counts, position, rotation, two velocities.
const VEHICLE_FIXED: usize = 5 + 3 + 4 + 3 + 3;

fn predict(i: usize, so_far: &[i64], p1: &[i64], p2: Option<&[i64]>) -> i64 {
    let a = p1.get(i).copied().unwrap_or(0);
    match p2 {
        Some(p2) if smooth(i, so_far) => 2 * a - p2.get(i).copied().unwrap_or(0),
        _ => a,
    }
}

/// Extension block of a frame: events and shells in flight as JSON (empty when there are none).
fn ext_bytes(f: &Frame) -> Result<Vec<u8>, String> {
    if f.events.is_empty() && f.projectiles.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::to_vec(&(&f.events, &f.projectiles)).map_err(|e| format!("cannot encode events: {e}"))
}

pub fn encode(r: &ReplayFile) -> Result<Vec<u8>, String> {
    let header = serde_json::to_vec(&r.header).map_err(|e| format!("cannot encode the header: {e}"))?;
    let names: Vec<Vec<String>> = r.header.vehicles.iter().map(|v| v.joint_names.clone()).collect();
    if names.is_empty() {
        return Err("the replay has no vehicles".to_string());
    }
    let mut out = MAGIC.to_vec();
    out.extend((r.header.version).to_le_bytes());
    out.extend((header.len() as u32).to_le_bytes());
    out.extend(header);
    out.extend((r.frames.len() as u32).to_le_bytes());
    for group in r.frames.chunks(KEYFRAME_INTERVAL) {
        let mut payload = Vec::new();
        let (mut prev, mut prev2): (Vec<i64>, Option<Vec<i64>>) = (Vec::new(), None);
        for (n, f) in group.iter().enumerate() {
            let ints = flatten(f, &names);
            put_varint(&mut payload, ints.len() as i64);
            for (i, v) in ints.iter().enumerate() {
                put_varint(&mut payload, v - predict(i, &ints[..i], &prev, prev2.as_deref()));
            }
            let ext = ext_bytes(f)?;
            put_varint(&mut payload, ext.len() as i64);
            payload.extend(ext);
            let old = std::mem::replace(&mut prev, ints);
            prev2 = (n > 0).then_some(old);
        }
        let payload = rle(&payload);
        out.extend((payload.len() as u32).to_le_bytes());
        out.extend(payload);
    }
    Ok(out)
}

fn read_u32(b: &[u8], pos: &mut usize) -> Result<u32, String> {
    let s = b.get(*pos..*pos + 4).ok_or("the replay is truncated")?;
    *pos += 4;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

pub fn decode(b: &[u8]) -> Result<ReplayFile, String> {
    if b.get(..4) != Some(MAGIC) {
        return Err("not a W5KR replay".to_string());
    }
    let mut pos = 4;
    let version = read_u32(b, &mut pos)?;
    if version != 2 {
        return Err(format!("replay version {version} is not supported (this reader reads 2)"));
    }
    let hlen = read_u32(b, &mut pos)? as usize;
    let header: ReplayHeader = serde_json::from_slice(b.get(pos..pos + hlen).ok_or("the header is truncated")?)
        .map_err(|e| format!("bad header: {e}"))?;
    pos += hlen;
    let names: Vec<Vec<String>> = header.vehicles.iter().map(|v| v.joint_names.clone()).collect();
    let count = read_u32(b, &mut pos)? as usize;
    let mut frames = Vec::with_capacity(count);
    while frames.len() < count {
        let glen = read_u32(b, &mut pos)? as usize;
        let g = unrle(b.get(pos..pos + glen).ok_or("a group is truncated")?)?;
        pos += glen;
        let (mut at, end) = (0, g.len());
        let (mut prev, mut prev2): (Vec<i64>, Option<Vec<i64>>) = (Vec::new(), None);
        while at < end {
            let n_ints = usize::try_from(get_varint(&g, &mut at)?).map_err(|_| "bad frame length")?;
            let mut ints = Vec::with_capacity(n_ints);
            for i in 0..n_ints {
                let p = predict(i, &ints, &prev, prev2.as_deref());
                ints.push(p + get_varint(&g, &mut at)?);
            }
            let mut f = unflatten(&ints, &names)?;
            let elen = get_varint(&g, &mut at)? as usize;
            if elen > 0 {
                let (events, projectiles): (Vec<Event>, Vec<ProjectileFrame>) =
                    serde_json::from_slice(g.get(at..at + elen).ok_or("an extension block is truncated")?)
                        .map_err(|e| format!("bad extension: {e}"))?;
                f.events = events;
                f.projectiles = projectiles;
                at += elen;
            }
            let old = std::mem::replace(&mut prev, ints);
            prev2 = (!frames.is_empty() && frames.len() % KEYFRAME_INTERVAL != 0).then_some(old);
            frames.push(f);
        }
    }
    Ok(ReplayFile { header, frames })
}
