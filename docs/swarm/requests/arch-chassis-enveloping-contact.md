From: arch   To: chassis   Needed by: after slice-2 acceptance (not blocking it)   Status: OPEN

## What I need
The enveloping tyre contact your design note (`docs/lanes/chassis/design-note.md`, "Contact") planned and `docs/swarm/status/chassis.md` lists as not trusted yet ("single-ray contact"): an effective ground height from three rays across the footprint (front, centre, rear, `patch_length_m` apart) plus the fixed-size-cylinder correction, so that a wheel meeting a vertical face is touched by its leading edge and the corner force has a horizontal part. On flat ground the three rays agree and nothing changes.

## Why
The step-climb bench (`w5k scenario proving --test step_climb`) cannot measure a truck. With one ray at the wheel centre the face is not felt until the centre passes the edge; then the ground under the wheel jumps up by the step height and the tyre is punched upward. The outcome depends on the phase of the body's bounce on arrival (changing the run-up by 1.5 m moves the highest step by 1 to 6 % and about one run-up in thirteen by 30 %), and the Mule's climb has at least three regimes (0.19 m, 0.30 m, and 0.44 m when the throttle is closed at the face, which is above its wheel radius of 0.41 m: a rigid wheel cannot do that). The figure and numbers are in `docs/lanes/arch/proving.md`. The Impact Matrix's step row (B11) is therefore shown but not counted until this lands, and the Workshop's step number carries its spread.
It also matters beyond the bench: kerbs, ditch edges, a log across the road and the barricades WORLD builds are all faces.

## What I will do meanwhile
The runner reports the median of three run-ups with its spread (merged or about to be). When the contact lands I rerun the step grid (`docs/lanes/arch/proving.md`); the sentence to test is `the_step_height_does_not_depend_on_the_run_up_by_more_than_five_percent`. Expect rig hashes and the course goldens to move on bumpy ground (not on the flat plane); that is a deliberate `Golden-Change`.

--- ARCH answer: (ARCH's own request, low priority: CHASSIS picks it up after the slice-2 acceptance report, unless a lane is idle and wants it sooner)
