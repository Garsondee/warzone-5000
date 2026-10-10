# LOOK design note (settling round)

Mission: one specification of paint, wear and light, implemented twice (GLSL for the three.js viewer, `.gdshader` for Godot) and checked against one CPU reference. Evidence: `docs/lanes/look/spike-l.md`. Contract: `contract-v0.1` (commit `f8f5e5d`); `ContactFrame.material` and `VehicleHeader.livery` are already in it, so CCRs (a) and (b) are not needed.

## 1. Camo recipe (as proved in the spike)
Solid 3D value noise (integer lattice hash, quintic fade, 3 octaves, lacunarity 2, gain 0.5) evaluated at `p_local / scale_m`. Each scheme needs one noise field per colour except the last. The field passes through an equalising CDF (a 64-cell table baked by `w5k look bake`, so the table is data, not shader code): afterwards the field is uniform on 0..1 and a threshold **is** a cumulative coverage. Colours are chosen in order (colour 0 where u1 < c0, colour 1 where u2 < c1/(1-c0), the rest colour 2). Triplanar is rejected for the camo (it loses the tails, table in the spike); the "variance-preserving blend" question is therefore moot for M1 and recorded for any future decal-like use.
Anchoring: **node-local position and the per-vehicle seed only**, never world, time or UVs. Wheels therefore carry their pattern round with them (physically right: paint is on the wheel). Weathering is computed in the design pose (the node's local frame at joint zero); the splash gradient takes the node's rest offset and ground height as uniforms.

## 2. RON schema (`assets/materials/camo/<id>.ron`)
```
(id: "nato_three_tone", kind: Blob(octaves: 3, lacunarity: 2.0, gain: 0.5),
 scale_m: Param(0.9, band: (0.5, 1.5), TUNED, "owner look"),
 colours: [ (name: "green", srgb: "#3E4A2A", coverage: 0.45, source: ESTIMATE("FS 34092 family, hand-read", band: 4 dE)), ... ],
 seed_salt: 7)
```
`srgb` hex exists only here (the authoring edge); the bake converts to linear albedo and writes JSON with the CDF table. Coverage sums to 1 (checked). Every colour has a `source` (`SPEC` colour standard chip value, else `ESTIMATE` with a band). Pattern kinds for the six schemes: `Blob` (NATO three-tone, woodland, desert, Soviet-green stripes via a stretched axis), `Fleck` (flecktarn: thresholded high-frequency cells over a blob base), `Pixel` (digital: the same field quantised to a 2D cell grid per face). Names are by look, not by army (card C-002).

## 3. Weathering
Inputs are the contract's `edge` and `cavity` (0..1 per vertex, no UVs) plus normal, height, `up`. Wear = smoothstep(edge x gain + noise - bias) towards primer/bare metal; dirt = cavity x noise towards a soil colour; splash = (height above ground) falloff on lower surfaces; dust = rises with the world-up component of the normal (rotation of the node applied in the shader from the model matrix; dust is the only term that reads orientation, deliberately). All four are monotone in their input (test L5). `edge`/`cavity` semantics stay `PROVISIONAL(status:look)` until GEOMETRY's note agrees. Until their export lands the lane tests use its own bevelled-block mesh with analytic flags (`assets/tests/meshes/`).

## 4. PBR per SlotKind (starting values, reasons; `assets/materials/slots.ron`)
Paint: roughness 0.9, metallic 0 (military paint is flat; chroma bounded by the chips of the cited schemes). Metal: bare steel, roughness 0.45, metallic 1, F0 about 0.55 (worn edges read bright, as in photographs of chipped hulls). Rubber: albedo 0.02-0.04, roughness 0.95. Glass: roughness 0.05, transmission is not used (cost); a dark tinted albedo plus a sky reflection. Canvas: albedo as paint, roughness 1.0. Track: dark Metal with rust-brown wear, roughness 0.7. Optics: albedo near black, roughness 0.05, clearcoat-like specular. A test (L6) parses the `SlotKind` variants from `render.rs` and demands one entry each.

## 5. Light rig and tone mapping
One directional sun (colour about 5800 K, elevation 50 degrees default), a hemisphere sky term (not a bloom, not a neon), fixed exposure per scene preset in `assets/lighting.ron`. Tone mapper both engines ship: **Khronos PBR Neutral** (hue-preserving, no grading look, one short closed formula, so a three.js `NeutralToneMapping` and a hand port in Godot agree; ACES shifts hues of desaturated paint, which is exactly what this art direction forbids). `UNVERIFIED`: that Godot 4.6.2 ships Neutral built in (else the same formula goes in a post step); check at M1 step 5. Cross-engine test: an 18% grey card within 6/255.

## 6. Hook for VIEWER
Per mesh: attributes `aEdge`, `aCavity` (floats). Per node: uniforms `uSeed`, `uRestOffset`, `uGroundY`, `uWear`, scheme uniforms from the baked JSON (the spike's `uPal`, `uCov`, `uCdf[65]`, `uScale`). A single `onBeforeCompile` that replaces `#include <color_fragment>` with `diffuseColor.rgb = w5k_albedo(...)` (done in the spike, works on three r160). View modes: lit, albedo-only (the L7 test pass), mask view (edge, cavity, wear as RGB). VIEWER inlines `assets/shaders/glsl/*.glsl` and the baked JSON at page build. A livery arrives in `VehicleHeader.livery` (scheme id, seed, wear).

## 7. Stateless particles
Dust, mud, muzzle smoke are pure functions of replay time: particle k of an emitter born at t_k has p = p0 + v0 (t - t_k) + g (t - t_k)^2 / 2 with drag by closed form; birth times come from a hash of (emitter, k), emission rate from slip, speed, soft-ground bit and `material` (zero on asphalt, airborne or stopped). Scrubbing is then evaluation, not simulation (L9, L10).

## 8. Expected CCRs and requests
CCR: none (`material`, `livery` are in 0.1.1). Requests (text in `docs/swarm/requests/look-asks.md`): ARCH: emit `Event::Fired` in `tank_slew_and_pitch()`; CI step for `python3 -I assets/tests/*.py` and a `.claude/settings.json` rule for `assets/`; VIEWER: the hook of section 6; GODOT: sync of `assets/shaders/godot`, and a run of `spikes/look/camo.gdshader`.
