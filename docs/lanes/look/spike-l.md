# Spike S-L: one recipe, two engines (LOOK)

**Question.** Can one camo + wear + dirt recipe be written once and run in WebGL2 (three.js, `onBeforeCompile`) and in Godot, matching a CPU reference within 2/255?
**Answer.** Yes for WebGL2 (measured). The Godot port is written (`spikes/look/camo.gdshader`) but **untested**: the lane container has no Godot binary. `UNVERIFIED` until GODOT runs it.

Run: `python3 -I spikes/look/prep.py && THREE_DIR=<three@0.160 package dir> node spikes/look/run.mjs spikes/look/out && python3 -I spikes/look/compare.py` (Playwright's Chromium, SwiftShader software GL).

| Check | Result |
|---|---|
| Integer hash, 65,536 lattice points, GLSL ES 3.00 `uint` against Python | **0 mismatches** (exact bits) |
| Albedo-only pass, 256x256 tile, 8-bit sRGB, shader vs CPU reference | **max delta 1/255**, 0 pixels over 2 |
| Same, with a pixel footprint of 4 cm (band-limiting active) | max delta 1/255 |
| ms/frame, 640x360, software GL, 72-per-axis bevelled block (about 28k triangles) | plain standard material 22-27 ms, with the chunk 42-52 ms: the chunk roughly doubles a software-GL frame; absolute numbers say nothing about a GPU |
| Colour-0 coverage, 6,000 random points in a 120 m cube, solid 3D noise | 0.440 for spec 0.45 (1-sigma sampling error 0.006) |
| Tail coverage (10% spec), same sampling | solid 0.098; **triplanar blend 0.074** (a quarter of the tail lost) |

**Decisions made with evidence**
1. **Solid 3D value noise in node-local space, not triplanar 2D noise.** Blending three independent noise fields lowers the variance of the blended field, so a threshold taken from the single-field histogram selects too little of the tails (table: 0.074 for 0.10). Solid noise has no blend zones, no stretching on 45 degree faces, and costs 2 x 3 octaves of 3D noise against 6 x 3 octaves of 2D noise for two fields. The picture `media/spike_l.png` (bottom left: solid, then triplanar) shows the blend zones. Triplanar stays on the list for ground-decal-like detail only if a pattern ever needs it, with the variance-preserving fix of Heitz and Neyret 2018.
2. **Blend the scalar field, threshold afterwards.** Two independent fields u1, u2 (each mapped through the equalising CDF so each is uniform on 0..1): colour 0 where u1 < c0; else colour 1 where u2 < c1 / (1 - c0); else colour 2. Coverage is then c0, c1, c2 by construction.
3. **Band-limit by pixel footprint.** An octave whose wavelength is under two footprints fades to its mean 0.5 (not to zero, which would shift coverage); the wear and dirt thresholds widen with the footprint. Known cost: far away the field regresses toward the mean, so the tails (rare colours) shrink at distance. Accepted for M1; the fix (a footprint-dependent LUT) is an M2 item.
4. **Integer hashing works** in WebGL2 (`uint`, `ivec3`, hex `u` literals) with bit-exact lattice values. One trap found: a negative `int` cast to `uint` is not portable in GLSL ES, so lattice indices get a +32768 offset first (reference does the same; reach limit 32 km at 1 m cells).
5. **Kill criterion** (engines more than 2/255 apart): not triggered. Nothing simplified.

**Open for GODOT / later:** verify the `.gdshader` compiles on 4.6.2 (uniform arrays `float cdf[65]`, `uint` uniforms, `COLOR` carrying edge and cavity); the lit-pass cross-engine grey card (L7 second half); the spike uses `fwidth(local position)` as the footprint, valid because the node-local frame is rigid.
