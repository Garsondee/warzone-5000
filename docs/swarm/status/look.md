# Status: LOOK

**Last updated:** 2026-10-10 UTC | **Branch:** lane/look/build | **Contract pinned:** contract-v0.1 (f8f5e5d) | **Phase:** building

## Done
- Spike S-L (`spikes/look/`, `docs/lanes/look/spike-l.md`): hash bit-exact in WebGL2, albedo within 1/255 of the CPU reference, solid 3D noise chosen over triplanar with numbers.
- Design note `docs/lanes/look/design-note.md`; requests `docs/swarm/requests/look-asks.md`.

## In progress
- Build step 1 PR (reference noise, three schemes, `w5k look bake`, L1-L4); the settling PR is #19.

## Blocked
- Nothing. Godot half of the spike is untested (no Godot binary here): request to GODOT.

## Next
- Step 2: ART-DIRECTION.md and slots.ron (L6); step 3: weathering (L5).

## Cards needed / PROVISIONAL decisions in force
- C-004: procedural textures (applied). Palette colours are `ESTIMATE` until checked against FS 595 / RAL chips.
- `edge`/`cavity` semantics PROVISIONAL(status:look) until GEOMETRY agrees.

## Evidence
- `python3 -B -I -m unittest discover -s assets/tests`: L1 L2 L3(numeric) L4 + CDF reproducibility pass (CI runs it); `cargo test -p w5k_tools look`; `docs/lanes/look/media/camo_panel_sphere.png`.
- L4 note: at 16 segments 1.1% of points over dE 3 (bound 1%); asserted at 24 vs 64 segments, 16 printed.
- `docs/lanes/look/media/spike_l.png`; compare.py output: 0 hash mismatches, max delta 1/255.

## Owner instructions received
- 2026-10-10: run without checking in (STATE.md).
