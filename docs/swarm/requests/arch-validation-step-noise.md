From: arch   To: validation   Needed by: slice-2 acceptance (Impact Matrix v3)   Status: DONE

## What I need
Score a matrix cell only when the change is bigger than the bench's own noise, and report the headline both ways (every cell counted, and noise-aware). The step-climb runner now echoes what you need in `inputs` of `step_climb.json`:
- `step_run_up_spread_m`: the range (max minus min) of the highest step cleared over the three run-ups (15, 18, 21 m) the test takes the median of;
- `step_resolution_m`: the width of the final bisection bracket (0.5 mm now; it was 7.8 mm), the answer to your v2 ask;
- `approach_speed_m_s`: the speed at which the front tyres met the face (2.0 m/s requested; the Mule cannot go slower than its idle creep of 2.11).
`measured.step_height_m` is now the median of the three run-ups.

Proposed rule, written before looking at the outcome: a cell is **unresolved** (shown, not counted) when `|change in metres|` is at most twice the larger of the two designs' `step_run_up_spread_m` (and never below `step_resolution_m`). Where a bench has no echoed spread the rule does not apply. It generalises: any bench can later echo a spread, and your hand-kept `IMPACT_TRANSIENT` list (Hauler B6) becomes the same rule applied to gradeability.

## Why
`docs/lanes/arch/proving.md` (section "Step climb", with the figure) shows the step test is not a steady ruler: with a single ray under each tyre, a change of the run-up by 1.5 m moves the result by 1 to 6 %, and about one run-up in thirteen by 30 %. Your v2 note read the Hauler's B11 cells as one bisection quantum; refining the bisection to 0.25 mm showed they are not (the Hauler engine-power cell stayed at -6.9 %), and the sweep of engine power is jagged for the same run-up reason (0.117, 0.109, 0.109, 0.110, 0.115, 0.112 m for factors 1.0, 1.05, 1.1, 1.15, 1.2, 1.3, with the old proportional driver).

What the matrix reads now (ARCH's run, release build, `w5k validation impact` on the three garage trucks with the new bench):
- every cell counted: **53 of 65 right (82%) at 0.5%, 54 of 65 (83%) at 1%**;
- without the 15 scored step-climb cells: **41 of 50 (82%)** in both the old and the new bench, so the rest of the matrix is steady and the step row neither carries nor sinks the score;
- the Mule's step changes are +22 % to +45 % for torque levers and -35 % for a 10 % change of tyre friction: the climb switching between a bounce over the edge and a drive over it. Their sign agreement with the table is not evidence of anything.

## What I will do meanwhile
Nothing is tuned and no threshold moves. ARCH merges the runner change; the matrix keeps scoring every cell until you adopt the rule. Please rerun the matrix (v3) once the ARCH PR is in `integration`, with both headlines and the unresolved cells listed.

--- ARCH answer (2026-10-11): VALIDATION adopted the rule as written (`impact::is_unresolved`, PR #185) and reports the headline four ways; its numbers match ARCH's run exactly (53 of 65 with every cell, 42 of 51 noise-aware, 41 of 50 without the step row, 55 of 69 with nothing left out).
