# Possibility Space

*Status: built and sampled (2026-10-08). Tools: `w5k atlas`, `roll`, `ladder`, `space`, `gallery`, `fit`; the interactive explorer
(`tools/explorer/`) is generated from [`space_sample.csv`](../assets/forge/space_sample.csv). Everything below comes from
`crates/w5k_forge/src/explore.rs`.*

With eighteen families and continuous sliders nobody can reason about the whole design space, so we **sample it**: roll a
vehicle from a seed, let the auto-fitter make it physically coherent, evaluate it, and do that thousands of times. The result
is a map of which mixes exist, where they are strong, where they fail and why, and where the model itself is wrong.

## How a design is rolled and fitted
1. **Roll** (`Explorer::roll`): pick a hull and a relative size, then sliders (budgeted metre sliders follow the size; armour
   stays on the thin side; the rest are uniform). Pick running gear that has a socket on this hull *and* a family that fits that
   socket kind. Fill the turret rings with guns, missile racks and beam turrets that are shrunk until they fit the ring, put a
   sensor or repair rig on the masts, and add an engine. The roll also draws two fitter hints: `margin` (how generously to size the
   gear) and `kw_per_t` (spare power per tonne).
2. **Fit** (`Explorer::fit`): size each running gear to carry its share of the weight (`fit_to_load`), size the engine to
   draw + lift + `kw_per_t` x mass, bump the engine technology if it does not fit its bay, and repeat until the mass settles.
   If the gear still cannot carry the vehicle, thin the armour by 30 % steps (the *diet*) and fit again. See
   [the theory note](../notes/m1b-movement-and-mixing-theory.md#11-auto-fit-is-a-fixed-point-iteration).
3. **Evaluate** with the quick path (exact-volume mass, four-direction armour): about 100 times cheaper than a full build and within
   roughly 10-20 % of it (a test compares them).
4. **Stratify**: `w5k space` gives every hull x gear cell the same number of attempts (100 each for 3,500 designs), so the
   charts show the space, not the odds of a random roll. Cells with no mount (a track on a walker's hip) are skipped.

Seeds are `Pcg32::derive` streams: the same seed gives the same design, and any dot in the explorer can be re-rendered with
`w5k roll content --out out --seed N --count 1 --hull H --gear G`.

## The atlas: every hull with every kind of running gear
![atlas](../assets/forge/atlas.png)

Five hulls across seven kinds of running gear, each sized by the fitter. A Dreadnought on legs, a Skiff on rails, a Lancer on
anti-gravity pods are all legal; the Strider's hips take legs, pods, cushions and rotors but not tracks, wheels or rail.

## Fun mixes
![fun mixes](../assets/forge/showcase.png)

Thirteen deliberate combinations, written as short specs (`content/specs/`) and fitted by `w5k fit` into `content/vehicles/`:

| Design | Mix | What it is for |
|---|---|---|
| Rail Battery | Dreadnought + rail bogies + two 250 mm guns + radar mast + repair crane | artillery that only goes where the track goes |
| Recon Drone | Skiff + one big rotor at 28 m + radar | eyes for the battery: altitude buys horizon |
| Walking Cathedral | Strider (22 m) + ten legs + 300 mm gun + radar spire + repair crane | the square-cube law made visible |
| Hover Sniper | Skiff + air cushion + very long 90 mm gun + radar | crosses swamp and water, hits at range |
| Spider Artillery | Strider + four leg pairs + 16-tube missile battery | lobs a barrage from behind cover |
| Sky Battery | Dreadnought (40 m) + anti-gravity pods at 14 m + two guns, a beam and a missile rack | a flying battleship; anti-gravity's cost is linear, so titans float |
| Mech Bastion | Bastion + six legs + gun + missile rack | a tank on stilts |
| Rotor Gunship | Lancer + big mast rotor + missile rack | rides over ridges the tracks cannot cross |
| Swarm Hover | tiny Skiff + cushion + rapid-fire cannon | nothing alone; cheap enough to field in dozens |
| Beam Titan | Strider (14 m) + legs + 60 MW beam + repair crane | capacitors and a long mirror |
| Wheel Scout | Lancer + six wheels + tall optical mast | sees far, runs from everything |
| Maglev Lancer | Lancer + four anti-gravity pods at 2 m + beam | trenches and mud pass underneath |
| Quad Skiff | Skiff + four rotor arms + light gun | a quadcopter; many small discs cost far more power than one big one |

Close-ups of six of them (`w5k view content --design ID --out FILE --zoom 0.7`):

| | |
|---|---|
| ![Sky Battery](../assets/forge/hero_sky_battery.png) | ![Rail Battery](../assets/forge/hero_rail_battery.png) |
| ![Walking Cathedral](../assets/forge/hero_walking_cathedral.png) | ![Rotor Gunship](../assets/forge/hero_rotor_gunship.png) |
| ![Quad Skiff](../assets/forge/hero_quad_skiff.png) | ![Maglev Lancer](../assets/forge/hero_maglev_lancer.png) |

## Scale has physics
![walker ladder](../assets/forge/ladder_strider_legs.png)

The same walker at six sizes (322 kg to 493 t, 2 m to 34 m long), fitted at each. The feet grow from 0.1 m to 3.3 m in radius,
the hips rise from 0.5 m to 4.4 m, and the leg tube thickens from 3 % of its segment length to 14-18 %: by the buckling law a
tube's strength grows with the fourth power of its section while the load it carries grows with the cube of the size. A
scaled-up small walker would have legs that buckle or sink.

The same Skiff hull with one big rotor, fitted at three sizes:

![rotor ladder](../assets/forge/ladder_skiff_rotor.png)

The rotor ladder stops early. The 4 m drone (0.7 t) and the 21 m and 28 m craft (45 t and 85 t) are valid; the next three sizes
are not, because one disc on one mast cannot lift the mass (the power needed grows as W^1.5 and the disc cannot outgrow its mast).
A heavier craft needs more discs, which is what the Quad Skiff above does, at a price in power: see the "rotor ceiling" below.

## Rolling the dice
![random designs](../assets/forge/roll_11.png)

Twenty-four designs from seed 11: random hull, running gear, weapon and size, each one fitted. The shared palettes, trim and glow
keep them one army; the surprises are real mixes that nobody drew: a Skiff floating on anti-gravity pods under a beam turret, a
301 t Dreadnought on rails firing missiles, a Strider saucer riding an anti-gravity ring, a Lancer on six wheels with a missile
rack. Any of them can be rebuilt from its seed (`w5k roll content --out out --seed 11 --count 24`).

## What the sample says
3,500 designs were rolled (100 for each of the 35 hull x gear cells); 3,200 had a socket for their gear and **2,782 (87 %) came
out valid**. The same command with the same seed gives a byte-identical `space_sample.csv`: the whole pipeline is deterministic.

### Which mixes survive
![viability](../assets/forge/space_viability.png)

| Running gear | Valid | Why |
|---|---|---|
| tracks, legs, rail | 100 % | the unit grows with the load (wider track, thicker legs and bigger feet, longer bogie), so there is always a size that works |
| anti-gravity | 98 % | field power is linear in the weight, so only the engine bay can object |
| wheels | 90 % | a tyre carries its pressure x width x a contact patch about 0.35 D long; a heavy vehicle runs out of room for tyres under the hull |
| air cushion | 84 % | leakage power grows as W^1.5 at a fixed footprint, and the hull must stay light |
| rotors | 43 % | the actuator disc: hover power grows as W^1.5/sqrt(A), and one mast limits A |

The Strider is the pickiest hull (66 %): it is small and round, with few sockets. Of the designs that fail, 85 % fail because
the running gear cannot carry the weight, 13 % because the engine does not fit its bay and 1.4 % because the craft cannot lift itself.

### Speed
![speed](../assets/forge/space_speed.png)

Legs are slow at any size (median 12 km/h, none above 31); tracks stop at their rated 70 km/h (60 % of tracked designs sit at the
rating); wheels at 140; rail reaches 250 and anti-gravity 240; rotors are the only gear above 250 km/h. The plateaus are the
gear's *ratings*, reached because engines are cheap (see below), not a power curve.

### Ground pressure
![pressure](../assets/forge/space_pressure.png)

Soft ground gives way somewhere between 50 and 100 kPa. Median pressure: air cushion 7 kPa, rail 42, legs 69 (their feet are
sized for about 100), tracks 81, wheels 138 (the 90th percentile is 329 kPa). When the terrain model arrives, tracks, legs and
wheels will bog down together and only cushions, anti-gravity and rotors will not, which is a role split for free.

### Sight and range
![sight and range](../assets/forge/space_sight_range.png)

The vision rule (height buys horizon) plus weapon ranges makes **spotting emerge**: 9 % of guns outrange their own eyes, 39 % of
beams and 49 % of missiles (27 % of all armed designs). A gun's median range is 0.7 km against 3.1 km of sight; a missile's is
3.2 km against 3.0 km. Long weapons need an allied spotter without a rule saying so. The chart also shows a flaw: sight is
nearly flat. Median 3.0 km, 90th percentile 4.0 km, and only 7 % of designs see beyond 4 km: the horizon of a vehicle a few metres
tall is about 3 km, and plain optics stop at 4 km, so only a tall mast with a real sensor sees further.

### Armour, firepower and the diet
![armour and firepower](../assets/forge/space_fire_armour.png)

Nothing reaches the top right. The efficient frontier (the white line: no sampled design has more of both) falls from about
3,500 kW/t at 37 mm of front armour to 1,000 kW/t at 130 mm and 14 kW/t at 480 mm.

![armour kept by scale](../assets/forge/space_scale_armour.png)

Lift is paid for in armour. Floaters and fliers could not keep the plating they were rolled with: 83 % of air cushions and 91 % of
rotors had to diet, keeping a median of 35 % and 25 % of the requested armour. Ground gear keeps everything (tracks 0 % dieted,
legs 4 %, rail 5 %, anti-gravity 6 %, wheels 19 %).

![viability by scale](../assets/forge/space_scale_valid.png)

Scale matters too. Rotors are valid for 70 % of designs between 30 and 100 t and for none above 100 t (0 of 13): the **rotor ceiling**,
which is realistic (the heaviest production helicopter, the Mi-26, takes off at about 56 t) and good for the game, because titans cannot be helicopters.
Air cushions struggle below 2 t (the skirt gap has a fixed minimum, so a small cushion leaks proportionally more) and wheels fall off above 200 t.

### The trade triangle
![trade triangle](../assets/forge/space_triangle.png)

At equal mass (421 armed designs between 58 and 98 t) the three traits are **nearly independent**: rank correlations are
firepower-armour -0.17, armour-speed +0.01 and firepower-speed +0.05. Only 3.6 % of designs are in the top third on all three, which
is what independent traits would give by chance (3.7 %). Speed is the free one: it is a property of the gear, and the engine is a small
part of the mass (installed power is a median 33 kW/t, 11 at the 10th percentile and 115 at the 90th, which at 0.22-2.5 kW/kg
engines is roughly 2-15 % of the vehicle).

Note what this does *not* say. A random roll does not optimise, so its cloud is wide and uncorrelated even where a hard
trade-off exists; the trade lives on the **frontier** (previous chart). The triangle says that mass alone is not the scarce resource
between the three traits.

### The corners
![corners](../assets/forge/space_corners.png)

The extremes among the 2,782 valid designs (the labels are full-build numbers; the sample used the quick path, so a few differ by
up to 15 %):

| Corner | Design | Why it is there |
|---|---|---|
| fastest | Bastion / rotor / beam, 19 t, 260-300 km/h | a rotor has no rated speed cap, only power |
| most armoured | Lancer / wheel / gun, 80-89 t, 480 mm front | wheels carry the weight of a thick hull because tyres are rated by pressure, not by size |
| most firepower, highest alpha strike | Dreadnought / anti-gravity / gun, 503 t, 46 MW at 240 km/h | anti-gravity makes a flying battleship cheap (see below) |
| firepower per tonne | Strider / hover / missile, 2.6 t | a floating missile rack: all weapon, no armour (5 mm) |
| longest range | Skiff / rail / beam, 14 km | a long mirror; it needs a spotter to use it |
| sees farthest | Skiff / anti-gravity / gun, 5.7 km | height buys horizon |
| heaviest, tallest | Dreadnought / legs / beam or missile, 11-14 kt, 1 km/h | the square-cube law: the walker is enormous and barely moves |
| lightest armed | Strider / rotor / beam, 0.37 t | a flying camera with a laser |
| gentlest on the ground | Skiff / air cushion / missile, 2.3 t, 7 kPa | floats over a swamp |

### One budget, six ways to spend it
![one budget](../assets/forge/space_budget.png)

Six designs of about 30 t, picked for what they are best at: the same mass, six different vehicles.

| Pick | Design | Numbers (quick path) |
|---|---|---|
| most armour | Lancer / legs / missile | 350 mm front, 10 km/h |
| most firepower | Lancer / anti-gravity / missile | 29 MW, 170 km/h, 56 mm |
| fastest | Lancer / rail / beam | 250 km/h and still **251 mm** of armour |
| sees farthest | Skiff / rotor / gun | 4.0 km, 147 km/h, 8 mm |
| longest range | Bastion / legs / missile | 9.2 km, 10 km/h |
| best all-rounder | Bastion / anti-gravity / missile | 10.8 MW, 149 mm, 178 km/h |

The fastest 30 t design carries about three quarters of the armour of the most armoured one. Speed is nearly free, which is the first finding below.

## What it says about the model
**What the physics got right without a rule saying so**

- Walkers are slow and heavy at every size, and the biggest (14 kt) crawl at 1 km/h.
- Air cushions are light (median 23 t, largest 480 t) and gentle on the ground; rotors stop at about 100 t.
- Long weapons need spotters; tall vehicles see far and are easy to see.
- Combinations the owner would try for fun (a walker with a missile rack, a flying battleship, a quadcopter) are legal, and the
  ones that cannot work say why (`overloaded`, `cannot hover`, `engine does not fit`).

**What it got wrong, or too easy** (each is a hypothesis to fix, not a bug in the sampler)

| Finding | Evidence | Proposed remedy |
|---|---|---|
| **Speed is almost free.** | the engine is a small share of the mass; 60 % of tracked designs sit at their rating; a 27 t rail Lancer is both the fastest and carries 251 mm | price speed (command points per kW; fuel, heat, signature), let drag bite harder, make ratings depend on the gear's mass |
| **Anti-gravity dominates.** | 98 % valid, median 173 km/h, no ground pressure, linear cost, and it holds the "most firepower" and "best all-rounder" corners | a tech gate (late tier), and real costs: a core that does not scale down, an upkeep draw, a signature |
| **Sight is flat.** | 3.0 km median, 7 % above 4 km | let sensors differentiate: plain optics cap lower, radar costs power and a mast, height matters more |
| **No trade-off in the cloud.** | rank correlations -0.17 / +0.01 / +0.05 | trade-offs must come from **price** and **physics gates**, not from mass alone: this is why M6 (command-point pricing) matters |
| **Fliers have no armour.** | rotors keep 25 %, air cushions 35 % | keep it: lift is expensive, and fragility is the right price of flying |

The theory behind the second-to-last row is in the [theory note](../notes/m1b-movement-and-mixing-theory.md#13-shadow-prices-and-frontiers):
a constraint only shapes decisions where it **binds**. Mass is a budget, but nothing makes spending it on speed hurt, so speed
never competes with armour. A price (a second budget) makes them compete.

## An interactive explorer
The same 3,200 designs as a page you can filter and plot: pick an axis pair, colour by hull, gear or weapon, hide the invalid
ones, and click a dot for the exact command that rebuilds it. It is generated from
[`space_sample.csv`](../assets/forge/space_sample.csv) by `tools/explorer/build.py` (see
[tools/explorer/README.md](../../tools/explorer/README.md)), so a new sample means a new page.

## Reproducing and extending
| Command | What it makes |
|---|---|
| `w5k sweeps content --out DIR` | every family on its host hull, engine sized to the vehicle, red text where the physics objects |
| `w5k atlas content --out DIR` | the hull x gear atlas |
| `w5k roll content --out DIR --seed N --count 24 [--hull H --gear G --weapon W --size 0.5]` | a gallery of random valid designs |
| `w5k ladder content --out DIR --hull strider --gear legs` | one archetype at six sizes, side by side at true scale |
| `w5k space content --out DIR --seed 7 --count 3500` | the charts above and `space_sample.csv` (about 8 minutes; the same seed gives the same file) |
| `w5k gallery content --only a,b,c --out FILE` | labelled renders of designs from `content/vehicles/` |
| `w5k view content --design ID --out FILE [--zoom 0.6]` | one large render |
| `w5k fit content --spec FILE --out FILE` | auto-fit a design spec |
| `w5k rates` / `w5k why` | how often each mix fits, and the problems of the ones that do not (debugging the fitter) |

To add a family to the space: give it `fits()` and `host()`, a `fit_to_load` if it carries weight, and register it in
`family::all()`; the roller needs a line in `Explorer::roll` only if it should be drawn at random.

## Caveats
- **The sample is a map of the model, not of the game.** Every constant (tyre pressure, melt energy, rotor stall, anti-gravity
  cost, safety factors, engine specific power) is a hypothesis; the charts show the structure those hypotheses imply.
- **The fitter decides what "valid" means.** A design the fitter cannot fix is reported invalid; a cleverer fitter (or a player)
  could rescue some. The diet only thins armour; it never shrinks the hull or drops a weapon.
- **Quick evaluation**: exact-volume mass double-counts overlaps and the armour table uses four directions; speed and mass
  agree with the full build to roughly 10-20 %.
- **Sizes and armour are rolled thin and moderate**: nothing in the sample is a 1,000 t tank with 400 mm plates unless the
  roller happened to draw it.
