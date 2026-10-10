# Status: LOOK

**Last updated:** 2026-10-10 UTC | **Branch:** lane/look/settling | **Contract pinned:** contract-v0.1 (f8f5e5d) | **Phase:** settling

## Done
- Spike S-L (`spikes/look/`, `docs/lanes/look/spike-l.md`): hash bit-exact in WebGL2, albedo within 1/255 of the CPU reference, solid 3D noise chosen over triplanar with numbers.
- Design note `docs/lanes/look/design-note.md`; requests `docs/swarm/requests/look-asks.md`.

## In progress
- Build step 1 (reference noise, patterns, RON sets, `w5k look bake`) on `lane/look/build`.

## Blocked
- Nothing. Godot half of the spike is untested (no Godot binary here): request to GODOT.

## Next
- Step 1: `assets/reference/`, three RON schemes, L1 L2 L3-numeric L4; step 2: ART-DIRECTION.md and slots.ron (L6); step 3: weathering (L5).

## Cards needed / PROVISIONAL decisions in force
- C-004: procedural textures (applied). Palette colours are `ESTIMATE` until checked against FS 595 / RAL chips.
- `edge`/`cavity` semantics PROVISIONAL(status:look) until GEOMETRY agrees.

## Evidence
- `docs/lanes/look/media/spike_l.png`; compare.py output: 0 hash mismatches, max delta 1/255.

## Owner instructions received
- 2026-10-10: run without checking in (STATE.md).
