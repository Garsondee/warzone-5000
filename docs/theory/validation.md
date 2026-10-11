# Theory: validation (plain language)

**Calibration set versus held-out set.** If you tune a model until it matches the vehicles you check it on, a good match tells you nothing: you taught it the answers. Like machine learning: fit on a training set, report on a test set the model has never seen. Our calibration vehicles (M998, M113A3, M4A3) may inform global constants; the held-out ones (M1A1, Leopard 2A5, T-72B, M35, Tiger II) only score.

**Provenance is asset versioning for numbers.** A texture has a file and an author; a number should have a source. We mark every figure as primary (a document we read), secondary (reported by a page we could not open) or an estimate with a band. A match against a weakly sourced figure is weak evidence, and the dashboard says so.

**Envelopes from uncertain inputs (Monte Carlo).** When inputs are uncertain, one simulated number is misleading. We draw many input sets inside their bands, run each, and look at the spread. The question becomes "is the real value inside what the model can plausibly produce?", similar to sampling many light paths in a path tracer and looking at the noise, not at one sample.

**Sensitivity and the tornado chart.** Change one lever by plus and minus 10% and see how far an outcome moves. Sort by size and you get a tornado: long bars are the levers that matter. A lever with no bar is dead; an outcome no lever moves is an orphan, and both mean the model or the design space is broken.

**Negative controls.** A test that cannot fail proves nothing. For each check we run a deliberately wrong model (double the power, ignore a lever) and require a red.

**The step corner (why a tall step is hard for a wheel).** A wheel of radius `r` meets a step of height `h`. Its centre is `r` above the ground, the corner is `h` up, so the line from the corner to the centre makes an angle `phi` with the vertical where `cos(phi) = (r - h) / r`. The ground can only push through the corner along a direction inside the friction cone (half-angle `atan(mu)`) around that line, so the wheel can roll over only if `tan(phi) <= mu`, which rearranges to `h <= r (1 - 1/sqrt(1 + mu^2))`. For mu = 0.85 that is `0.24 r`. A real truck climbs more than this because of momentum (the hull carries the wheel over), so the scorer treats this number as the *reference for the lower band*, not as a cap: only a result well below it is suspect. The hard cap is `h <= r`, because a rigid wheel's centre cannot rise over a corner taller than its own radius.
