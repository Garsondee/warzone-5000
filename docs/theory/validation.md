# Theory: validation (plain language)

**Calibration set versus held-out set.** If you tune a model until it matches the vehicles you check it on, a good match tells you nothing: you taught it the answers. Like machine learning: fit on a training set, report on a test set the model has never seen. Our calibration vehicles (M998, M113A3, M4A3) may inform global constants; the held-out ones (M1A1, Leopard 2A5, T-72B, M35, Tiger II) only score.

**Provenance is asset versioning for numbers.** A texture has a file and an author; a number should have a source. We mark every figure as primary (a document we read), secondary (reported by a page we could not open) or an estimate with a band. A match against a weakly sourced figure is weak evidence, and the dashboard says so.

**Envelopes from uncertain inputs (Monte Carlo).** When inputs are uncertain, one simulated number is misleading. We draw many input sets inside their bands, run each, and look at the spread. The question becomes "is the real value inside what the model can plausibly produce?", similar to sampling many light paths in a path tracer and looking at the noise, not at one sample.

**Sensitivity and the tornado chart.** Change one lever by plus and minus 10% and see how far an outcome moves. Sort by size and you get a tornado: long bars are the levers that matter. A lever with no bar is dead; an outcome no lever moves is an orphan, and both mean the model or the design space is broken.

**Negative controls.** A test that cannot fail proves nothing. For each check we run a deliberately wrong model (double the power, ignore a lever) and require a red.
