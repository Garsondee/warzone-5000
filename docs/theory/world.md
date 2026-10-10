# How the world is built, in plain language (lane WORLD)

Written for someone who knows computer graphics and animation but is not a programmer. Each section is one idea, the graphics analogy, and where it lives in the code.

## 1. The ground is a displacement map
A course is a grid of heights, one per metre: a 2001 x 2001 image whose pixel value is "how high is the ground here". Asking `height_m(x, z)` is a **bilinear texture fetch**: blend the four nearest pixels. The *normal* is the derivative of that same blend (the gradient of the surface), not of the neighbours, so height and normal can never disagree. A displacement map has one height per (x, z), so **there are no overhangs, no tunnels and nothing under a bridge**; everything below is built within that limit. (`grid.rs`)

## 2. Hills: fractal noise, bent
`fBm` (fractional Brownian motion) is noise summed over octaves, each twice the frequency and half the amplitude: it is the same trick as a procedural cloud or mountain in any DCC tool. **Domain warping** feeds the noise its own noise as an offset, which bends the ridges so they stop looking like a grid. The result is capped by a **grade clamp**: "no slope steeper than 30 per cent" is enforced, exactly, by a distance transform (the same algorithm that makes outlines and glows in image editing), so a stated grade is a promise rather than a hope.

## 3. A road is a shortest path, then a sculpture
A road is found by A* on the grid, where a step costs its length multiplied up by how steep it is: so roads wander to avoid hills, like real ones. That path is staircase-jagged, so it is averaged smooth. Then comes the part that makes it a *road*: the heights along it are **graded** (limited to a stated slope by repeatedly clamping the profile forward and backward, a rate limiter on a curve), and the ground is **cut and filled** to that profile, blended into the hillside over a shoulder. Authored parts (a zig-zag climb, a bridge) are laid exactly and not smoothed: averaging a tight arc would shrink its radius.

## 4. Mud sits where water collects
Every cell passes its water to its lowest neighbour (D8 flow). Counting how many cells drain through each one gives the **drainage area**; where it is large and the ground is flat, water stands: mud. It is the same idea as a "flow map" baked from a height map for water shaders.

## 5. Trees and rocks: blue noise
**Poisson-disc sampling** (Bridson) scatters points no closer than a minimum distance: the even-but-random look of "blue noise" in rendering. Thinning by a slow noise makes stands and clearings. Rock fields use the same sampler, so there is always a lane between the stones, and the survey can say how wide.

## 6. Micro-roughness and washboard are formulas, not stored detail
A wheel must meet the same bump every time it visits, so roughness is a **pure function of position** (hash noise), never a random number drawn when asked. For ripples finer than the grid (a washboard of 0.8 m wavelength) the grid cannot hold them at all: **Nyquist** says a grid with 1 m spacing cannot represent anything shorter than 2 m. So each node stores a *phase* (distance along the road) and a *weight*, and the height is computed: `w A (1 - cos(2 pi phase / lambda)) / 2`. This is a procedural normal map: the texture only steers a formula. Whoops (8 to 15 m) are long enough for the grid and are baked in. (`corrugation.rs`)

## 7. Cliffs, and why a switchback is long
A cliff is a *scarp*: a smoothstep rise. The steepest point of a smoothstep is `1.5 H / depth`, so asking for a face grade `g` fixes the depth `1.5 H / g`. A road may climb only `g_road` (about 8%), so rising `H` needs `H / g_road` of road: 20 m at 8% is 250 m. The generator **folds** that into legs across the face joined by semicircular hairpins whose radius is a stated parameter (it must exceed what the vehicle can turn in). It refuses a layout whose lanes do not fit, and says the numbers. The road on such a face is a **bench**: cut into the slope and filled on the downhill side.

## 8. A river is ground under a water level
The channel is a trapezoid carved into a flat valley; its **bank grade** is what a vehicle must climb to leave the water; the water is a *flat* surface across the channel falling gently along the river. A **ford** is a stretch where the bed rises. There is no flow simulation. A **bridge deck** is ground raised to deck level over the water (nothing drives under it), laid after the grade clamp, which would otherwise build ramps into the river.

## 9. Measuring a terrain: slopes and the roughness spectrum (`w5k world stats`)
Two checks a validator can score.
- **Slope shares**: what fraction of the ground is steeper than 5, 10, 20 and 30 degrees. It tells you whether the landscape is a plain or an Alp.
- **Road roughness as a power spectral density.** Treat the height along a line as a signal in *distance*. Its spectrum `G(n)` says how much roughness lives at each *spatial frequency* `n` (cycles per metre; wavelength `1/n`), in m^3. Real roads follow `G(n) = G(n0) (n/n0)^-2` almost everywhere, so one number, `G(n0)` at `n0 = 0.1` cycles/m (10 m wavelength), places a road in an **ISO 8608 class**, A (a new motorway) to H (a ploughed field). We compute it the audio way: cut the line into overlapping segments, remove the straight-line trend (the *slope* of the ground is not roughness), multiply by a Hann window, FFT, square, average (Welch), and scale so white noise comes out right. The tests check that scale against known signals: white noise gives `2 sigma^2 dx`; a sinusoid integrates to its variance; a road synthesised to class B is measured as class B with an exponent of -2.
- **Material numbers beside published ranges** (`published_ranges.ron`): every range is marked *Unverified* until the VALIDATION lane opens the book (Wong, *Theory of Ground Vehicles*, the soil parameter tables) and fills in the page and table. A value outside its range is a finding for them, not a verdict.

## 10. Determinism
The same seed must give the same world on any machine: all randomness is derived from `(seed, stage, item)`, never from the order of queries; every function comes from the portable maths library; containers are ordered. Each course has a hash of its baked output, checked as a constant on Linux and Windows.
