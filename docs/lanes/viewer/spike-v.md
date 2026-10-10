# Spike S-V: can a cloud session see it?

**Answer: yes.** A self-contained HTML page plays `tank_slew_and_pitch()` in headless Chromium with software GL, and ffmpeg encodes it.

| Question | Result |
|---|---|
| WebGL | WebGL 2.0 (ANGLE + SwiftShader; flags `--use-angle=swiftshader --enable-unsafe-swiftshader`) |
| three.js | `three@0.170.0` from npm, pinned in `tools/viewer/package-lock.json`, inlined into the page as a blob module (no CDN, no run-time fetch) |
| Page size | 2.6 MB (three 0.69 MB + the replay as JSON 1.8 MB + the rig 0.18 MB); the binary replay will shrink the data part about 20x |
| Triangles | 2308 drawn = 2308 in `RenderRig::triangle_count()` |
| Time per frame | 46 ms at 960x540 (render + JPEG read-back + pipe), 300 frames in 13.7 s |
| 10 s at 30 fps | 72 KB MP4 (flat shading compresses well; expect 1-3 MB with terrain and plots) |
| Console errors | none |

Frame (turret slewed, gun raised): `docs/lanes/viewer/media/spike-v-tank.png`.

Recipe: `w5k viewer dump-canned tank --out out/v/tank` (writes `replay.json`, `rig.json`), then
`node tools/viewer/build.mjs --rig .. --replay .. --out page.html`, then `node tools/viewer/capture.mjs page.html --out clip.mp4`.
Playwright comes from the container's global install (`PLAYWRIGHT_DIR`); the browser is `CHROME=/opt/pw-browsers/chromium-1194/chrome-linux/chrome`.

Lessons: (1) a page rendered frame-by-frame at exact times (`window.__v.renderAt(t)`) is deterministic and independent of machine speed;
(2) a blob-module import is how to inline an ES-module library without a bundler; (3) `preserveDrawingBuffer` is needed for read-back.
Not yet checked (build step 2): the pixel-difference test proving that wheels, turret, gun and recoil move.
