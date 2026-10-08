# Time-trial viewer

A self-contained page that replays what `w5k trial` simulated: pick a vehicle, watch it drive the course in 3D (chase, side,
orbit and overview cameras), scrub the timeline, click the elevation strip to jump to a place on the course, and read the
results and live telemetry. The same replay file can be played by any other viewer (Godot later).

| File | What it does |
|---|---|
| `template.html` | the page: a three.js scene, the shading of the Rust previews ported to a shader, a canvas HUD, controls |
| `build.py` | `python3 -I build.py DIR page.html` embeds `DIR/scene.json` and `DIR/replay.json` (deflated) into the template |
| `capture.js` | `node capture.js page.html --vehicle ID --out clip.mp4` renders the replay frame by frame and encodes it with ffmpeg |
| `shot.js` | `node shot.js page.html outdir [vehicle]` screenshots desktop and phone widths and reports console errors |
| `frames.js` | `node frames.js page.html outdir --vehicle ID --times 20,21,22 --camera side` renders chosen moments to PNG (to check an animation) |

```bash
cargo run --release -p w5k_tools --bin w5k -- trial content --out out/trial        # simulate every vehicle
python3 -I tools/trial/build.py out/trial out/trial/page.html                      # build the page
node tools/trial/capture.js out/trial/page.html --vehicle lancer_mk1 --out out/lancer.mp4
```

The page loads three.js r128 from cdnjs. In a sandbox where cdnjs is blocked, fetch a local copy (`npm pack three@0.128.0`, take
`package/build/three.min.js`) and set `THREE_JS` to it: `capture.js` and `shot.js` serve it in place of the CDN URL. Both scripts
need Playwright (`NODE_PATH=$(npm root -g)`) and ffmpeg; set `CHROME` to a Chromium binary if Playwright cannot find one.

## Things move

Wheels roll, rotors and fans spin, and legs walk. None of it is simulated: the forge marks the moving shapes of a part as **joints**
(`Node::Joint`: a pivot, an axis and a law of motion: `Roll`, `Spin`, `Hip`, `Knee`), the mesh export tags every vertex with its
joint, and the page hangs each joint's triangles from a pivot object and sets its angle every frame from the replay:

| Joint | Angle |
|---|---|
| `roll` | distance rolled over the radius (the ground distance over `1 - slip`, so a wheel that digs in turns faster than it travels) |
| `spin` | revolutions per second, from the clock |
| `hip`, `knee` | from the gait cycle, which advances with the distance travelled (a stride is 1.6 stances), so a standing vehicle stands and a slow one steps slowly; legs alternate along each side and the two sides are half a cycle apart |

Every axis is exported oriented so that a positive angle is the natural one (wheel rolls forward, leg swings forward, foot lifts); see
`crates/w5k_forge/src/export.rs`. Any other viewer (Godot later) animates a recorded run the same way.

## Data

`w5k trial` writes `scene.json` (the course profile and, per vehicle, a quantised mesh: see `crates/w5k_forge/src/export.rs`) and
`replay.json` (20 bytes per tick per vehicle: see `crates/w5k_sim/src/replay.rs`). `build.py` deflates the binary arrays and
the page unpacks them with the browser's `DecompressionStream`. `window.__trial` exposes `select`, `camera`, `renderAt(t)` and
`snapshot()` for scripts that drive the page.
