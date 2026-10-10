# VIEWER design note: replay v2 and the viewer

## 1. Replay v2 binary layout
A file is `magic "W5KR" | u32 version | u32 header_len | header JSON (ReplayHeader + extension table) | frame blocks`.
Everything in the header (vehicle names, joint names, contact names, dt) is the contract's `ReplayHeader`, so a human can read it with `head -c`.
Frame blocks are **keyframe every 30 frames (1 s), deltas between**, each block `u32 len | payload`, so a viewer can seek to any second.

| Field | Encoding | Error budget |
|---|---|---|
| position | i32 mm in keyframes; deltas as zig-zag varint mm | 0.5 mm |
| rotation | smallest-three, 3 x 10 bits + 2-bit index = 32 bits | < 0.01 deg |
| velocities | not stored; a viewer differences positions (the contract fields are written but only in the debug JSON) | |
| joints | per-kind scale, i16 steps: travel 0.05 mm, spin/steer/aim 2pi/65536 rad = 0.096 mrad, recoil 0.05 mm; **delta from the previous frame, varint** (wheel spin never wraps, deltas stay small) | at or below the 0.1 mrad aim requirement |
| rpm, gear | rpm u16 at 1 rpm steps; gear i8 | |
| contacts | per contact: flags u8, normal force u16 (N/4, saturating at 262 kN), sinkage u8 mm, slip i8 (1/127), material u8 | |
| ledger | one f16 per `ForceTerm` | 0.1 % |
| limiting, weapons | u8; per weapon ready/reload/rounds/aim error (i16 in 0.01 mrad steps) | |
| events, projectiles | **extension blocks** (id + length), version bit in the block flags, so GODOT/VALIDATION can skip what they do not know | |

**Budget: <= 100 bytes per vehicle-frame at 30 Hz.** Truck (4 wheels: 4 spin + 4 travel + 2 steer = 10 joints, 4 contacts): pose 6+4 B, joints ~20 B as deltas,
contacts 4x5 = 20 B, ledger ~16 B, rpm/gear/limiting 4 B = about 70 B. The 14-station tank (31 joints, 14 contacts) is the stress case: ~190 B raw, ~100 B after
varint deltas; if a rig exceeds the budget the named test fails and tells which field is big. 20 vehicles x 300 s x 30 Hz x 100 B = 18 MB raw; an optional deflate pass
(`flate2` needs an approved card; the first version uses a built-in zero-run packer instead, see the codec's module comment) typically halves that again. Replays are CI artifacts, not committed.

## 2. How a page receives a replay
Default **embedded**: the build script inlines rig + replay (base64 of the binary) into the single HTML file, which therefore works from `file://` in a cloud
session and can be attached to a message. Large replays (> 30 MB) use a sibling file loaded with `fetch` from the same directory; that is local, not a network fetch.
The page is a **pure function of the replay**: it holds no physics and never reads the clock for anything but playback speed.

## 3. Module structure (`tools/viewer/src`)
`replay.js` (decode, `sample(t)`: linear for positions and joints, slerp for rotation) · `rig.js` (node tree, joint bindings, forward kinematics: travel, then steer, then spin) ·
`scene.js` (renderer, ground from the terrain export, lights) · `debugdraw.js` (contact patches, force bars, CoM, friction-circle glyphs, per-`ForceTerm` arrows) ·
`hud.js` and `scope.js` (HUD and plots, cursor synced to playback) · `cameras.js` (orbit, chase) · `main.js` (UI, scrub, speed). `build.mjs` inlines it all; `capture.mjs` drives
headless Chromium frame by frame (`window.__v.renderAt(t)`), ffmpeg encodes. The Rust side is `w5k_replay` (encode/decode, JSON kept for debugging; a JS decoder mirrors it and a
test decodes a Rust-written file in node) and `w5k viewer render|plot`.

## 4. Expected CCRs (text only; none filed yet)
1. `Frame` has no **camera hints** (a target vehicle id or a framing box per scenario); the viewer defaults to the first vehicle. Low priority.
2. Per-contact **world position and normal** are not in `ContactFrame`; the viewer reconstructs the patch position from the rig and the pose (wheel node origin projected on the
   terrain). Fine for wheels; for track samples we need the sample positions, which FORGE should expose in the `RenderRig` (a `contact_nodes` list mapping `contact_names` to nodes). **Asking FORGE via an interface request when the debug-draw step starts.**
3. `ForceTerm` names for the arrow legend: read from `ledger.rs`; if the ledger summary has no per-term application point, vectors are drawn from the CoM (acceptable for M1).

## 5. As built (PR `lane/viewer/build`, step 1)
The layout above was simplified while writing it: every field of a frame becomes an integer (quantised), the frame is a flat list of integers, and the list is stored as zig-zag
varints of the **difference from a prediction**: smooth fields (pose, velocities, joints) predict by linear extrapolation from the two previous frames, noisy ones (contacts, ledger) from the previous frame; zero bytes are run-length packed.
Rotation is smallest-three at 16 bits per component (a 10-bit version, as first sketched, gives 0.14 degrees, 14 times too coarse). Measured on the canned stand-ins: **23 B per vehicle-frame (truck), 36 B (tank)**;
the stand-ins are smooth, so real simulation output will cost more; the test keeps the 100 B guard. Groups of 30 frames start from zeros (seekable).
