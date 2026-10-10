# Status: COMBAT

*Update with every PR. Keep it under 40 lines. ARCH reads this file at every check-in instead of your whole session.*

**Last updated:** 2026-10-10 UTC | **Branch:** lane/combat/ballistics | **Contract pinned:** 0.2.0 (`contract-v0.2` merge commit) | **Phase:** slice 2 (spike S6 and the ballistics kernel only)

## Done
- Spike S6 (this PR): `spikes/combat/s6`, finding `docs/lanes/combat/spike-s6.md`, plots in `docs/lanes/combat/media/`, theory `docs/theory/combat.md`. **PASS.**

- Design note: PR #115. Ballistics kernel (`lane/combat/ballistics`, stacked on #114): `w5k_combat::ballistics`, `content/combat/ballistics.ron`, 8 tests, trajectory image.

## In progress
- Nothing: slice-2 scope complete.

## Blocked
- Nothing.

## Next
- Idle until a later slice (armour, damage, articulation step, weapons) is launched.

## Cards needed / PROVISIONAL decisions in force
- None. CCRs C1 (the port integrates hull + chain), C2 (pin the servo law in `rig.rs` docs) and C3 (FORGE derives gains and stabiliser bandwidth from the servo) are text in the finding; ARCH settles them into v0.3.

## Evidence
- Momentum of hull + chain + ejecta: 4e-7 of J (dP) and 1.0e-6 (dL) at 2 substeps, 3e-8 at 6 (RK4); Euler 2.6e-3; late-wrench scheme diverges.
- Swivel chair to 1.3e-8 rad; servo overshoot matches `exp(-pi z/sqrt(1-z^2))` to 0.05 points at 0.5x/1x/2x turret mass; stabiliser law to 1% (tuned servo).
- Images: `s6-momentum.png`, `s6-momentum-error.png`, `s6-servo-steps.png`, `s6-swivel.png`.

- Ballistics: `drag_free_range_equals_v2_sin_2theta_over_g` (1e-6), apex, `vertical_fall_under_drag_follows_vt_tanh_gt_over_vt` (1e-4), RK4 order 4, `Cd(Mach)` from RON. Image: `ballistics-trajectory.png`. `trajectory_matches_a_published_drag_table` NOT done: no table opened; the Cd curve is `UNVALIDATED` (G7-like shape from memory).
- New dependencies for `w5k_combat`: `serde` and `ron`, both already workspace dependencies (RON loading is the brief).

## Owner instructions received
- None direct (slice 2 brief from ARCH: S6 and ballistics only).

## Handoff note (slice 2)
- Changed: S6 spike (#114), design note (#115), `ballistics` kernel (stacked on #114). | Unfinished: published-table oracle; armour, damage, articulation step, weapons (later slice). | Surprised me: the stand-in servo gains give zeta 0.56 to 0.65, not 0.1 to 0.2, but are too soft for the slew-time oracle; a 3 Hz stabiliser with a 50 ms servo delay is unreachable; the one-step-late wrench diverges. | I would do next: the `articulation` step from the S6 scheme once ARCH settles CCR-C1.
