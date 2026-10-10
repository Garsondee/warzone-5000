# Status: COMBAT

*Update with every PR. Keep it under 40 lines. ARCH reads this file at every check-in instead of your whole session.*

**Last updated:** 2026-10-10 UTC | **Branch:** lane/combat/settling | **Contract pinned:** 0.2.0 (`contract-v0.2` merge commit) | **Phase:** slice 2 (spike S6 and the ballistics kernel only)

## Done
- Spike S6 (this PR): `spikes/combat/s6`, finding `docs/lanes/combat/spike-s6.md`, plots in `docs/lanes/combat/media/`, theory `docs/theory/combat.md`. **PASS.**

## In progress
- Design note (`lane/combat/design-note`), then `ballistics` in `w5k_combat` (`lane/combat/ballistics`).

## Blocked
- Nothing.

## Next
- Ballistics kernel with `drag_free_range_equals_v2_sin_2theta_over_g` and `vertical_fall_under_drag_follows_vt_tanh_gt_over_vt`; then the handoff note and idle.

## Cards needed / PROVISIONAL decisions in force
- None. CCRs C1 (the port integrates hull + chain), C2 (pin the servo law in `rig.rs` docs) and C3 (FORGE derives gains and stabiliser bandwidth from the servo) are text in the finding; ARCH settles them into v0.3.

## Evidence
- Momentum of hull + chain + ejecta: 4e-7 of J (dP) and 1.0e-6 (dL) at 2 substeps, 3e-8 at 6 (RK4); Euler 2.6e-3; late-wrench scheme diverges.
- Swivel chair to 1.3e-8 rad; servo overshoot matches `exp(-pi z/sqrt(1-z^2))` to 0.05 points at 0.5x/1x/2x turret mass; stabiliser law to 1% (tuned servo).
- Images: `s6-momentum.png`, `s6-momentum-error.png`, `s6-servo-steps.png`, `s6-swivel.png`.

## Owner instructions received
- None direct (slice 2 brief from ARCH: S6 and ballistics only).
