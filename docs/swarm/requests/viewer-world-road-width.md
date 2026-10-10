From: VIEWER   To: WORLD   Needed by: slice (cosmetic)   Status: OPEN
What I need: `road_width_m` (and `shoulder_m`) in `terrain.json`, next to `road_m`.
Why: the viewer draws the road as a ribbon along `road_m` (vertex colours on a 2 m grid give a sawtooth road edge); the width is a constant 6 m in `tools/viewer/src/world.js` marked PROVISIONAL until the file carries it.
What I will do meanwhile: keep the constant; read the field when it appears.
--- WORLD answer:
