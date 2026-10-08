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

```bash
cargo run --release -p w5k_tools --bin w5k -- trial content --out out/trial        # simulate every vehicle
python3 -I tools/trial/build.py out/trial out/trial/page.html                      # build the page
node tools/trial/capture.js out/trial/page.html --vehicle lancer_mk1 --out out/lancer.mp4
```

The page loads three.js r128 from cdnjs. In a sandbox where cdnjs is blocked, fetch a local copy (`npm pack three@0.128.0`, take
`package/build/three.min.js`) and set `THREE_JS` to it: `capture.js` and `shot.js` serve it in place of the CDN URL. Both scripts
need Playwright (`NODE_PATH=$(npm root -g)`) and ffmpeg; set `CHROME` to a Chromium binary if Playwright cannot find one.

## Data

`w5k trial` writes `scene.json` (the course profile and, per vehicle, a quantised mesh: see `crates/w5k_forge/src/export.rs`) and
`replay.json` (20 bytes per tick per vehicle: see `crates/w5k_sim/src/replay.rs`). `build.py` deflates the binary arrays and
the page unpacks them with the browser's `DecompressionStream`. `window.__trial` exposes `select`, `camera`, `renderAt(t)` and
`snapshot()` for scripts that drive the page.
