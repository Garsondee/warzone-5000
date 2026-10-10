# Evidence charts: the tornado and the ladder

*VIEWER, slice 2 stage A. The **tornado reads VALIDATION's `impact.json` exactly as `w5k validation impact` writes it** (no conversion step). The **ladder's JSON is a stub of VIEWER's**, drawn from invented numbers (the picture says so) until TRACKS's `w5k tracks bench ladder` exists; tell me its columns or write this shape.*

## Run

```
w5k validation impact --out DIR                     # about 16 s on a release build: writes DIR/impact.json
w5k viewer tornado DIR/impact.json --out tornado.png [--theme light|dark] [--cols N] [--rows 8]
w5k viewer ladder  ladder.json --out ladder.png      [--theme light|dark]
node tools/viewer/test-charts.mjs                    # 23 checks of what is drawn, flagged and counted (no browser)
```

Headless Chromium draws a canvas at two pixels per CSS pixel and writes the PNG. A picture over 300 KB is re-saved with a 64-colour palette by `ffmpeg` (the media lint allows 400 KB; flat chart colours survive it: 774 KB became 203 KB). The drawing code is `tools/viewer/src/charts.js`, plain functions on a 2D context, so the Workshop page can draw the same charts live.

Pictures: `docs/lanes/viewer/media/tornado-impact.png` is **real**: the output of `w5k validation impact` on `integration` after TRACKS and CHASSIS merged (2026-10-10): 31 of 54 signs right (57%, under the 80% bar; the findings are VALIDATION's in `docs/lanes/validation/impact-v0.md`). `ladder-stub.png` is **invented** and carries a STUB DATA banner.

## The tornado reads `impact.json` (VALIDATION's file)

```
{ "entries": [ {"vehicle": "mule_4x4", "lever": "engine_power", "bench": "B1",
                "base": 11.09, "perturbed": 10.07,           // the benchmark's value at the baseline and with the lever +10%
                "delta": -0.092,                              // (perturbed - base) / |base|: the bar is 100 times this
                "observed": "Minus",                          // Minus | Zero | Plus  (Zero: under the 0.5% dead band)
                "allowed": ["Minus"],                         // what the table allows (the regime already applied)
                "verdict": "Right" } ],                       // Right | Wrong | Unlisted (moved, no table row) | Quiet
  "right": 31, "scored": 54,                                   // scored = Right + Wrong
  "dead_levers": [["hauler_4x4", "first_gear"]],               // per vehicle: a lever that moved nothing
  "orphan_benchmarks": [] }                                    // benchmarks no lever moved
```

One panel per benchmark (ordered by number), one row per lever (biggest effect first), one bar per vehicle. The verdicts are the runner's own, because it knows the regimes; the chart only draws them. A failed check (`Wrong`) is always drawn, even when its bar is tiny; a lever that moves a benchmark by under 0.5% on every vehicle is counted in a footnote ("5 levers under 0.5%, not drawn"), never silently dropped; a measured zero is a 2 px stub. Benchmarks the table lists but the runner has no test for are named in the footer ("No runner yet: B2 top speed, ..."), so a gap in coverage is visible rather than looking like good news.

**Optional fields, all with defaults, that would let me delete my built-in tables** (asked in `docs/swarm/requests/viewer-validation-impact-shape.md`): `benchmarks: [{id, name}]` and `levers: [{id, name}]` (until then the names of B1 to B16 and of the ten levers are looked up in `charts.js`, marked provisional, and an unknown id is made readable: `wing_span` becomes "Wing span"), `perturb_pct` (default 10), `deadband_pct` (default 0.5, the runner's `EPS`), `acceptance_pct` (default 80, SLICE-2 acceptance 3). An entry whose baseline is zero has no percentage; it is counted and not drawn.

## The ladder: `w5k-ladder-1` (VIEWER's stub)

```
{ "schema": "w5k-ladder-1", "title": "...", "subtitle": "...",    // title and subtitle optional
  "x": {"name": "added load", "unit": "t"},                        // the rung variable: mass, or track width
  "y": {"name": "sinkage", "unit": "m"},
  "threshold": {"name": "bogged", "value": 0.35},                  // optional: the sinkage at which a vehicle stops
  "tolerance_pct": 25,                                             // optional: the acceptance band around the theory
  "stub": true,                                                    // optional: draws the STUB DATA banner
  "vehicles": [ {"id": "mule", "name": "Mule",
                 "rungs": [ {"x": 0, "sim": 0.194, "theory": 0.186, "bogged": false} ] } ] }
```

`sim` is what the simulation measured; `theory` is Bekker's `z = (p / (kc/b + kphi))^(1/n)` for the same ground and footprint; `bogged` is optional (the bench's own judgement; without it a rung is bogged when `sim` reaches the threshold). Each vehicle has its own rungs, so a tracked carrier can climb to 16 t while a truck stops at 5; a vehicle that never reaches the threshold is reported as never bogging, not left out. One to eight vehicles (the palette's validated colours); more is refused.

## What was decided, and why (the data-viz method)

| Question | Choice | Reason |
|---|---|---|
| Which form? | Tornado: one small panel per benchmark, levers as rows sorted by effect, one bar per vehicle. Ladder: connected points on a numeric rung axis, theory dashed beside it. | The tornado's job is *ranking* (which lever matters for this outcome); one shared axis across benchmarks would flatten the small ones, so each panel has its own symmetric axis, labelled. The ladder's job is a *trend against a prediction*. |
| What does colour mean? | Tornado: the vehicle; the sign of the effect is *position* (left or right of zero), so colour never has to say it twice. Ladder: colour = vehicle, line style = measured (solid, dots) or theory (dashed). | One job per channel, and colour follows the entity: a vehicle keeps its colour in every panel and chart. |
| Palette | Reference palette in its fixed order, a slot per vehicle, up to the eight it has (more is refused: split the chart). `validate_palette.js`: first three slots all-pairs, light CVD dE 9.2 and normal-vision 24.0, dark 9.4 and 20.9; all eight on adjacent pairs (what lines need), light 9.1 and 19.6, dark 8.4 and 19.3; all pass. Aqua, yellow and magenta are under 3:1 on the light surface, so the relief rule applies: every series is directly labelled. | The validator, not the eye, decides whether colours separate under colour-blindness. |
| Status marks | Red cross = a check that failed; orange triangle = dead lever or orphan benchmark; green tick = acceptance met; yellow triangle = stub data. Each comes with words. | Status colours are reserved and never carry meaning alone. |
| Honesty | Failed checks always drawn; below-threshold levers counted; measured zero drawn; missing runners listed; the acceptance line says "57%, the bar is 80%" with a red cross rather than hiding behind a pass-coloured number. | A chart that hides the failures it was built to find is worse than no chart. |
| Labels | Values only on the biggest bar of the three biggest levers of each panel and on failed checks; the ladder labels each line at its end. | Never a number on every bar. |
| Text | Ink tokens only, never the series colour; thin bars (9 px) with 4 px rounded data ends anchored at the zero line; hairline grid. | The marks-and-anatomy spec of the method. |

Not done on purpose: hover tooltips and a table view (the PNG is evidence; the Workshop page draws the same functions on a live canvas and can add them there), and a shared scale across panels (a small change if the owner wants to compare panels by eye). A two-sided run (-10% and +10%) would need one more bar per vehicle and a naming change in the legend; VALIDATION's runner is one-sided today, so I did not build it.
