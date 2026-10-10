# Status: CHASSIS

*Update with every PR. Keep it under 40 lines. ARCH reads this file at every check-in instead of your whole session.*

**Last updated:** 2026-10-10 UTC | **Branch:** lane/chassis/build | **Contract pinned:** 80adac2 (0.2.0) | **Phase:** build (M1)

## Done
- S1 + design note (#7); steps 1-4 (#13, #30); wheeled assembly + Ackermann (#42); Mule strip run (#43); modes + ledger (#59);
  tilt-table and skidpad benches + patch-moment fix (#69). Every step with its oracle tests; contract 0.2.0.

## In progress
- #69 merged. ARCH's Scout check (course-compare on slice.ron): **not a chassis bug; a real transient.** 420-460 m is the
  whoops (12 m waves, 0.3 m p-p); at 10.5 m/s that is 0.87 Hz, close to the Scout's 1.15/1.27 Hz ride with light damping
  (zeta 0.19 bump / 0.29 rebound), so it pitches hard: front tyres swing 0.1 to 11.5 kN. The 3.7 m/s2 peak (v x yaw rate) is a yaw
  catch-up: the driver adds steer while the fronts are unloaded, they reload at 57.3 s and the yaw overshoots the kinematic
  v^2 delta / L (1.7) by 2x. The front wheels run 31-46 mm into the bump stops (196/211 of 220 mm travel; stops at 165 mm),
  no hard limit; Mule (102 of 150) and Hauler (63 of 120) never reach theirs. Plausible for a soft jeep near resonance. Would change
  it: Scout damping (zeta 0.25-0.35 bump is typical off-road) and no tyre load sensitivity (yaw jolt reads high). `modes` prints zeta.
- **Known gap: no tyre load sensitivity** (grip and cornering stiffness per unit load are constant, so load transfer never costs grip; the limit
  and the anti-roll front/rear balance read wrong). NOT-MODELLED row requested (`requests/chassis-not-modelled-tyre-load-sensitivity.md`); fix = CCR + one PR.
- **Not trusted yet:** tyre curve linear to the cap (no slide drop; locked braking reads high); roll centre at wheel-centre height (sliding struts);
  single-ray contact; `relaxation_length_m`, `slip_damping_time_s` are estimates.
- **6x6:** expressible today with independent axles; walking beams / inboard leaves (`LinkageDef`) are refused until linkages land (about one PR).

## Blocked
- Nothing.

## Next
- suspension, tyre (rolling-resistance speed scale as a Param, not a const-ok literal), hull, wheeled assembly on the bump strip, ledger, glue API.

## Cards needed / PROVISIONAL decisions in force
- None.

## Evidence
- `python3 -I spikes/chassis/s1/s1.py` output in `spikes/chassis/s1/output.txt`. No Rust changed.

## Owner instructions received
- 2026-10-10 (owner, in chat): "go ahead with the hull".
- ARCH (2026-10-10): CCR-1..3 landed in contract-v0.2 (applied); rolling-resistance fade speed is a `Param` (done).
## Handoff note (fill in when you stop)
- (fill in at M1)
