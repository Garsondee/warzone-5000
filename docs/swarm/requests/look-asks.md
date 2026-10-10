From: LOOK   To: ARCH   Needed by: M1   Status: OPEN
What I need: (1) a CI step running `python3 -I assets/tests/<name>.py` for every test under `assets/tests/` (standard library only), and a `.claude/settings.json` rule pre-approving `python3 -I assets/...`; (2) `Event::Fired` emitted in `tank_slew_and_pitch()`.
Why: (1) L1-L6 live in Python by brief; CI cannot run them yet, so PR descriptions carry pasted output. (2) muzzle flash and smoke.
What I will do meanwhile: paste test output in each PR; key the muzzle flash off the onset of recoil.
--- ARCH answer: 

From: LOOK   To: VIEWER   Needed by: M1   Status: OPEN
What I need: a documented hook that accepts a GLSL chunk and injects it into the standard material (`onBeforeCompile`, replacing `#include <color_fragment>`), per-vertex float attributes `aEdge`/`aCavity` from `MeshPart.edge/cavity`, per-node uniforms, an albedo-only view and a mask view, and a page build that inlines `assets/shaders/glsl/*.glsl` plus baked JSON. The working injection is in `spikes/look/harness.html` (three r160).
Why: L3 image level and L7. Meanwhile: I ship `assets/shaders/glsl/w5k_camo.glsl` and test on my own harness.
--- VIEWER answer: 

From: LOOK   To: GODOT   Needed by: M2   Status: OPEN
What I need: someone to compile `spikes/look/camo.gdshader` on Godot 4.6.2 and say whether `uniform float cdf[65]`, `uniform uint`, hex `u` literals and `COLOR` carrying edge/cavity work; and a sync step from `assets/shaders/godot`.
--- GODOT answer: 
