# The Upgrade Engine

How research results change the numbers in the game. **Snapshot:** Warzone 2100 `master` @ `d7ce18df8d`. **Evidence:** engine
source read by an analysis agent (paths relative to the Warzone 2100 checkout), with the arithmetic cross-checked against the data
by `tools/research/wz_research_graph.py`.

## 1. Storage: one immutable base, one upgraded copy per player

Every component stat holds an immutable **base** plus an `upgrade[player]` copy for each of the 11 player slots. Loaders copy the
base into every slot. Completing a research topic writes the new values **eagerly** into the owner's copy, through the same setters
that scripts use. Combat, rate of fire, ranges and so on read `upgrade[player]` **at the time of use**, so every existing droid and
structure benefits at once; there is no refit.

**Cached derived values** (hit points, speed) are refreshed through a "dirty" flag: the setters mark droids dirty for body HP, HP
percentage, **Power**, propulsion and turret HP, and sensor/ECM range; on the next update the engine recomputes max HP (keeping the
current HP ratio) and base speed. Structure HP upgrades rescale existing structures.

**Save/load** stores only status, `possible` and points. Upgrades are rebuilt by **replaying every completed topic's results**.

> **Theory note: replayable derived state.** Storing only "which techs are done" and recomputing every modified stat from the
> immutable base is the *event-sourcing* idea: state is a pure function of an ordered history. It makes saves tiny, makes
> network sync cheap (the same inputs produce the same state), and makes balance patches retroactive. It is worth copying.

## 2. The arithmetic

A result entry is `{class, parameter, value, optional filterParameter + filterValue}`.

- **Percent of the base, additive.** For each matching entity, the change is `base x value / 100`, rounded **away from zero** (up if
  positive, down if negative), and added to the *current* value. So five +25 steps give +125% of base, **not** x3.05. Rounding
  happens at each step, so the error accumulates.
- **Skip rule:** entities whose base value is 0 or less are skipped. Unsigned stats clamp at 0.
- **"Compat" vs "improved" modes:** multiplayer and the base campaign use *compat* (above). Two campaign mods use *improved*, which
  keeps the unrounded cumulative sum and rounds once. Worked example: a lab with base 14 and nine +30 steps ends at
  14 + 9 x ceil(4.2) = **59** (compat) versus 14 + ceil(37.8) = **52** (improved).
- **Filter:** one equality test against the entity's **base-stat snapshot**, on any stat key (weapon sub-class, body class, structure
  type, a component id...). No AND, negation or range. A failed filter is silent; a missing key on a matching entity is an error.
- **Arrays** (rank thresholds) scale element-wise, compat mode only.

## 3. What can be upgraded (a closed list)

| Class | Parameters research can change |
|---|---|
| Body | HitPoints, HitPointPct, **Power**, Armour, Thermal, Resistance |
| Weapon | MaxRange, ShortRange, MinRange, HitChance, ShortHitChance, FirePause, ReloadTime, Rounds, Radius, EmpRadius, Damage, MinimumDamage, RadiusDamage, RepeatDamage, RepeatTime, RepeatRadius, HitPoints, HitPointPct |
| Sensor, ECM | Range, HitPoints, HitPointPct |
| Repair / Construct | RepairPoints / ConstructorPoints (plus HitPoints, HitPointPct) |
| Brain | BaseCommandLimit, CommandLimitByLevel, RankThresholds, HitPoints, HitPointPct |
| Propulsion | HitPoints, HitPointPct only (the percent-of-body field is spelled differently in two places, so research cannot reach it) |
| Building | ResearchPoints, RepairPoints, PowerPoints, ProductionPoints, RearmPoints, Armour, Thermal, HitPoints, Resistance (and, **scripts only:** module research/power/production points, limits) |

The shipped multiplayer data uses 419 result entries on 206 topics: **249 weapon, 91 body, 60 building**. It never uses ECM or
propulsion. Its filters are only weapon sub-class (`ImpactClass`), `BodyClass`, structure `Type` and `Id`.

**Never upgradable by research:** weight, build cost and build points, research cost, **propulsion speed, acceleration and turn rate**,
projectile flight speed, and all enumerated fields.

## 4. The engine upgrade in detail

Body `Power` is stored in the body's per-player upgrade (`upgrade[p].power`), initialised from the JSON `powerOutput`. The nine
engine topics are class Body, filter `BodyClass = "Droids"` (so cyborgs and transports are untouched), parameter Power:
+5, +5, +5, +5, +5, +5, +5, +7, +8 (multiplayer: **+50% of base**; campaign: nine steps of +5 = +45%). The result is consumed by the
body-power read inside the base-speed calculation ([../unit-design/04-engine-and-speed.md](../unit-design/04-engine-and-speed.md)),
cached in the droid's base speed, and refreshed by the dirty flag. The weight-bonus *threshold* in that calculation reads the **base**
power, so research never moves it.

## 5. Observations

1. **The shape is good:** immutable base, per-player copy, eager write, use-time read, replay on load. Cheap, deterministic, retroactive.
2. **Percent-of-base stacking makes every step equally valuable**, and "equal steps" is what makes upgrade lines feel like a tax.
   Diminishing returns, compounding, or branching tracks would create *different* values per step.
3. **The upgradable-parameter list is hard-coded** in four places plus a UI wording table. Anything not on the list (speed, weight,
   cost, turn rate) cannot be an upgrade target without engine changes.
4. **One equality filter is a limited query language.** "Heavy bodies AND tracked" or "all weapons except X" cannot be expressed.
5. **Rounding per step is a hidden bias** (about 13% on small bases); a design using integer stats should round once at the end
   (the "improved" mode) or use fixed-point.
6. **Eager write plus replay** means scripts that write absolute values bypass the replay and are not restored on load; a design
   should keep *all* modifiers in one replayable store.
