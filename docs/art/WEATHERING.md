# Weathering (specification; reference `weathered()` in `assets/reference/w5k_look.py`, shader `assets/shaders/glsl/w5k_camo.glsl`)

Four terms applied in this order to the camo colour, each a mix towards a colour by an amount; parameters in `assets/materials/weathering.ron` (TUNED by eye)
and baked to `assets/materials/baked/weathering.json`:
1. **Wear (chips to bare metal):** `smoothstep(-w, w, edge * edge_gain + noise_amp * (n - 0.5) - bias)`; rises with `edge`. `bias > noise_amp / 2` guarantees a face with `edge = 0` never chips (the bake checks it).
2. **Dirt in creases:** `clamp(cavity * (base + var * n), 0, dirt_max)`; rises with `cavity`.
3. **Splash line:** `splash_max * (1 - smoothstep(0, splash_height_m, height + (n - 0.5) * splash_noise_m))`; falls with height above ground. `height` is measured in the *design pose* (node rest offset plus the ground height, passed as uniforms), so a vehicle that pitches does not change its splash line.
4. **Dust:** `dust_max * smoothstep(dust_lo, dust_hi, up_y) * (mod_base + mod_var * n)`; rises with the world-up component of the unit normal, the only term that reads orientation.
Every `n` is solid 3D value noise at the term's own frequency on the node-local position, seeded by the vehicle seed XOR a fixed constant per term; footprint widens the wear smoothstep so distant vehicles do not shimmer.
Tests: L5 in `assets/tests/test_shader_albedo.py` (monotone per input, clean flat face, triplanar weights), L7 (shader against reference, headless Chromium). Image: `docs/lanes/look/media/weathering_ramps.png` (edge, cavity, splash, dust ramps, left to right).
`edge` and `cavity` semantics stay `PROVISIONAL(status:look)` (convex dihedral ramp 20 to 70 degrees; blocked fraction of cosine-weighted rays within 0.5 m) until GEOMETRY's note agrees.
