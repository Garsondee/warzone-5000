# Evidence charts: the tornado and the ladder

*VIEWER, slice 2 stage A. The two JSON shapes below are a **stub proposed by VIEWER**; they change to whatever VALIDATION's impact runner and TRACKS's ladder bench really write (request: `docs/swarm/requests/viewer-validation-impact-shape.md`). The renderer refuses a file it does not understand instead of guessing.*

## Run

```
w5k viewer tornado impact.json --out tornado.png [--theme light|dark] [--cols N] [--rows 8]
w5k viewer ladder  ladder.json --out ladder.png  [--theme light|dark]
node tools/viewer/test-charts.mjs          # 22 checks of what is drawn, flagged and counted (no browser)
node tools/viewer/samples/make-stub.mjs    # rewrites the two sample files
```

Headless Chromium draws a canvas at two pixels per CSS pixel and writes the PNG. A picture over 300 KB is re-saved with a 64-colour palette by `ffmpeg` (the media lint allows 400 KB; flat chart colours survive it, 774 KB becomes 203 KB). The drawing code is `tools/viewer/src/charts.js`: plain functions on a 2D context, so the Workshop page can draw the same charts live.

Pictures (STUB numbers, the banner says so): `docs/lanes/viewer/media/tornado-stub.png`, `ladder-stub.png`.

## Shape 1: `w5k-impact-1` (the tornado)

```
{ "schema": "w5k-impact-1",
  "vehicle": "utility_4x4 (Mule)",          // label only
  "perturb_pct": 10,                         // default 10
  "deadband_pct": 0.5,                       // a change smaller than this "does not move"; default 0.5
  "stub": true,                              // optional: draws the STUB DATA banner
  "benchmarks": [ {"id": "B1", "name": "0-32 km/h time", "unit": "s", "baseline": 4.8} ],
  "levers":     [ {"id": "engine_power", "name": "Engine peak power"} ],
  "cells": [ {"lever": "engine_power", "benchmark": "B1",
              "minus": 5.27, "plus": 4.35,   // the benchmark's VALUE with the lever at -perturb% and +perturb%
              "expected": ["-"],             // optional: signs the table allows for +10%: "-", "+", "0" (about nothing)
              "verdict": "ok" } ] }          // optional: the runner's own verdict, which wins over mine
```

- **Change** is `100 * (value - baseline) / |baseline|`. A benchmark whose baseline is zero (sinkage on firm ground) has no percentage: its panel is drawn in the benchmark's own unit, and `deadband` on that benchmark (same unit) is its dead band.
- **Verdict** when the runner gives none: `ok` (the observed sign of the +10% change, with the dead band, is among `expected`), `wrong_sign`, `missing` (expected a change, saw none) or `spurious` (expected "about nothing", saw a change). VALIDATION's regime detection (grip-limited versus brake-limited) is better than mine, so when it writes `verdict` I draw that.
- **Dead lever**: a lever with at least one cell and no cell above the dead band. **Orphan effect**: a benchmark with cells and none above the dead band. A benchmark with no cells is "not measured yet", which is not the same thing and is drawn differently.

## Shape 2: `w5k-ladder-1` (the ladder)

```
{ "schema": "w5k-ladder-1",
  "title": "...", "subtitle": "...",                    // optional
  "x": {"name": "added load", "unit": "t"},             // the rung variable: mass, or track width
  "y": {"name": "sinkage", "unit": "m"},
  "threshold": {"name": "bogged", "value": 0.35},       // optional: the sinkage at which a vehicle stops
  "tolerance_pct": 25,                                  // optional: the acceptance band around the theory
  "stub": true,
  "vehicles": [ {"id": "mule", "name": "Mule",
                 "rungs": [ {"x": 0, "sim": 0.194, "theory": 0.186, "bogged": false} ] } ] }
```

`sim` is what the simulation measured, `theory` is Bekker's `z = (p / (kc/b + kphi))^(1/n)` for the same ground and footprint. `bogged` is optional (the bench's own judgement); without it a rung is bogged when `sim` reaches the threshold. Each vehicle has its own rungs, so a tracked carrier can climb to 16 t while a truck stops at 5. A vehicle that never reaches the threshold is reported as never bogging, not left out.

## What was decided, and why (the data-viz method)

| Question | Choice | Reason |
|---|---|---|
| Which form? | Tornado: one small panel per benchmark, levers as rows sorted by effect. Ladder: connected points on a numeric rung axis, theory dashed beside it. | The job of the tornado is *ranking* (which lever matters for this outcome); one shared axis for sixteen benchmarks would flatten the small ones, so each panel has its own symmetric axis, labelled. The ladder's job is a *trend against a prediction*. |
| What does colour mean? | Tornado: which perturbation (blue = lever -10%, orange = lever +10%); the sign of the effect is *position* (left or right of zero), so colour never has to say it twice. Ladder: colour = vehicle, line style = measured (solid, dots) or theory (dashed). | One job per channel, and colour follows the entity: a filter never repaints the survivors. |
| Palette | Reference palette in its fixed order. The tornado uses slots 1 and 2; the ladder takes a slot per vehicle, up to the eight the palette has (more is refused: split the chart). `validate_palette.js`: first three slots all-pairs, light CVD dE 9.2 and normal-vision 24.0, dark 9.4 and 20.9; all eight on adjacent pairs (what lines need), light 9.1 and 19.6, dark 8.4 and 19.3; all pass. Aqua, yellow and magenta are under 3:1 on the light surface, so the relief rule applies: every series is directly labelled. | The validator, not the eye, decides whether colours separate under colour-blindness. |
| Status marks | Red cross = a check that failed; orange triangle = dead lever or orphan effect; green tick = acceptance met; yellow triangle = stub data. Each comes with words. | Status colours are reserved and never carry meaning alone. |
| Honesty | A measured zero is drawn as a 2 px stub, not left blank; levers below the dead band are counted ("17 levers under 0.5%"), never silently dropped; a failed check is always drawn, even when its bar is tiny. | A chart that hides the failures it was built to find is worse than no chart. |
| Labels | Values only on the three biggest levers of each panel and on failed checks; the ladder labels each line at its end. | Never a number on every point. |
| Text | Ink tokens only, never the series colour; thin bars (9 px) with 4 px rounded data ends anchored at the zero line; hairline grid. | The marks-and-anatomy spec of the method. |

Not done on purpose: hover tooltips and a table view (the PNG is evidence; the Workshop page draws the same functions on a live canvas and can add them there), and a shared scale across panels (a small change if the owner wants to compare panels by eye).
