# Propulsion

**Snapshot:** Warzone 2100 `master` @ `d7ce18df8d`, multiplayer ruleset unless stated. Data:
[`../data/propulsion.md`](../data/propulsion.md), [`../data/terrain.md`](../data/terrain.md),
[`../data/modifiers.md`](../data/modifiers.md). **Evidence:** stat files plus engine source analysis (paths relative to
the Warzone 2100 checkout); community material is flagged as lower confidence.

## 1. The seven propulsion *types*

The engine has a closed list of **7 propulsion types**: Wheeled, Half-Tracked, Tracked, Hover, Legged, Lift (air),
Propellor (water). The loader insists on a row for all seven in `propulsiontype.json`. Each type has a speed
`multiplier`, a movement medium (GROUND or AIR), a row in the weapon-damage modifier table and a column in the terrain
speed table.

| Type | Multiplier (MP) | Medium | Who uses it |
|---|---:|---|---|
| Wheeled | 80 | ground | player designs; scavenger vehicles |
| Half-Tracked | 80 | ground | player designs |
| Tracked | 80 | ground | player designs |
| Hover | 100 | ground (also crosses water) | player designs |
| Legged | 100 | ground | cyborgs and persons only |
| Lift | 140 | air | VTOLs, transporters, helicopters |
| Propellor | 100 | ground (water only) | "Naval": designable but **never unlocked** in MP |

(The campaign data uses 120 for hover and 130 for lift.)

## 2. The designable propulsions

Percent columns are **percentages of the hull** (see [01-design-pipeline.md](01-design-pipeline.md)).

| Propulsion | Max speed (cap) | Weight % | HP % of body | Power cost % | Build pts % | Skid decel | Notes |
|---|---:|---:|---:|---:|---:|---:|---|
| Wheels | 175 | +300 | +100 | +50 | +50 | 350 | cheapest; unlocked at the start |
| Half-tracks | 150 | +400 | +200 | +75 | +75 | 500 | |
| Tracks | 125 | +650 | +300 | +125 | +125 | 600 (default) | spin-in-place threshold 65 degrees |
| Hover | 300 | +200 | +100 | +150 | +100 | 120 | drifts heavily; terrain 150/80/50 |
| VTOL | 700 | +50 | +100 | +150 | +125 | n/a | own flight model; VTOL factory only |
| Naval (unreachable) | 300 | +300 | +300 | +150 | +150 | n/a | water only |

Reading it as a **ladder**: wheels, half-tracks and tracks trade speed for toughness at a steady rate: each step
buys +100% of the body's HP, costs 25-50 points more, and is slower and heavier. Hover and VTOL are the
"speed" propulsions; VTOL is the lightest by far (+50% weight) and the most expensive per hull.

For a Cobra hull with a Medium Cannon (5,000 weight), flat ground, no research:

| Propulsion | Total weight | Speed | HP | Power cost |
|---|---:|---:|---:|---:|
| Wheels | 13,000 | 138 | 610 | 219 |
| Half-tracks | 15,000 | 80 | 740 | 230 |
| Tracks | 20,000 | 60 | 870 | 253 |
| Hover | 11,000 | 204 | 610 | 265 |

Tracks give +43% HP over wheels for +16% price at **-57% speed**. Hover costs 21% more than wheels for the same HP, is 48%
faster, and crosses water.

## 3. Terrain: a table that does little

Twelve terrain types (sand, sandy brush, baked earth, green mud, red brush, pink rock, road, water, cliff face, rubble,
sheet ice, slush) times seven propulsion types give a percentage multiplier on base speed
([`terrain.md`](../data/terrain.md)). Highlights:

- **Road:** wheels 150, half-tracks 135, tracks 120, hover 150, legged 100.
- **Water:** hover 150; every other land type 60, but land units **cannot enter water tiles**, so this value is practically
  unreachable. Likewise cliff faces are blocked for all ground types.
- **Hover** is 150 on sand, mud, ice, road, water; 100 on baked earth; 80 on brush, rubble, slush and cliff; **50 on pink rock**.
- **Lift:** 250 on every terrain (air ignores terrain). **Propellor:** 100 on every terrain.
- **Legged:** 100 except slush 75.
- Averaged over all terrain: wheeled 85, tracked 92, legged 91, hover 108, half-tracked 88.

Two important consequences, both checked in the code:

1. The **speed cap is applied after the terrain factor**, so terrain bonuses above 100% (hover on sand, wheels on road) are
   wasted whenever the cap already binds.
2. **Pathfinding ignores terrain speed completely.** The A* search uses geometric distance (with a x5 penalty only for AI
   "danger" tiles). So hover units do not seek water, wheeled units do not seek roads, and tracks do not avoid rubble. The
   terrain table changes how fast a unit moves along whatever path it was given; it never changes which path it takes.

> **Theory note: terrain as a decision.** For terrain to create tactical or design decisions it must influence *routes*
> (so players and AI plan around it) or *outcomes* (cover, bog-down, ambush). Here it only changes speed along a fixed
> route, and for the cap-bound majority of designs not even that. Making terrain affect pathing cost is a prerequisite
> for propulsion variety to matter on the map.

## 4. Where propulsion types actually behave differently in code

There are about **45 direct `Lift` comparisons** and over **100 uses of `isVtol()` / `isFlying()`** (which test the same
thing), but only about **8** uses of the data-driven ground/air medium field. Behaviour differences by type:

| Type | Path blocking (what it cannot cross) | Movement model | Other special cases |
|---|---|---|---|
| Wheeled / Half-Tracked / Tracked | cliffs, features, structures, **water**; own land "continent" | one shared ground model; differences are data only (skid, spin angle, cap) | Wheeled is the code's proxy for "generic ground" in several places; per-type sounds |
| Legged (cyborgs/persons) | same as ground | separate **person model** (chosen by droid type): no skid, immune to slope, small collision radius | cyborg-only factory and transport pairing; ground droids can squash persons |
| Hover | cliffs and features only: **crosses land and water** (own continent) | ground model with data only (skid 120) | none special |
| Propellor | features and **land**: water only | ground model | a "FIXME submarines" sink-offset hack |
| Lift (VTOL) | only very tall obstacles; ignores campaign scroll limits; straight-line fallback if pathing fails | **VTOL model**: bang-bang height control around 250/300/350 above terrain, own turn parameters (degrees/s), climb/descent speeds | see below |

**VTOL-specific rules** (these are the large block of special cases):
- speed penalty by body size (medium x3/4, heavy /4); built only in a VTOL factory;
- ammo is **attack runs** (times rounds for salvo weapons); the VTOL returns to a rearm pad, which rearms until points
  equal the droid's *weight* (heavier VTOLs rearm slower) and also repairs it;
- attack runs fly 1,000 units past the target before turning;
- counts as an air target only while moving; takes x3 damage when landed; ignores ECM jamming; no self-repair, no
  pickups; cannot be ignited by fire; ground units ignore it unless it is low;
- only enters transports as hover.

**Damage-matrix rows:** the same seven types index the weapon-damage matrix, so propulsion is also the *armour class*
against weapon effects (see [06-damage-and-counters.md](06-damage-and-counters.md)).

## 5. Predefined designs show what the game "wants"

Across the 280 predefined multiplayer templates (used by AIs and scripts), propulsion usage is: **tracks 98**,
**hover 59**, **VTOL 37**, **half-tracks 30**, **wheels 17** (plus 28 cyborg-leg and 10 scenery templates). Bundled AIs have
hover-specific modes (a "hover" personality in NullBot; sea-map detection and a force-hover option in the Cobra and
SemperFi AIs). These counts reflect AI authors' choices, not player statistics, but they agree
with community comments that wheels are only an early-game option.

## 6. The campaign's propulsion *tiers* (a different design answer)

The campaign data defines **Mark I / II / III** variants of each propulsion as separate components:

| Variant | Weight % | HP % of body | Power cost % | Speed cap |
|---|---:|---:|---:|---:|
| Wheels I / II / III | 250 / 200 / 150 | 100 / 200 / 300 | 25 / 75 / 125 | 175 |
| Half-tracks I / II / III | 400 / 350 / 300 | 200 / 350 / 500 | 75 / 100 / 125 | 150 |
| Tracks I / II / III | 650 / 600 / 550 | 400 / 600 / 800 | 125 / 200 / 275 | 125 |
| Hover I / II / III | 200 / 150 / 100 | 150 / 200 / 300 | 100 / 150 / 200 | 200 / 225 / 200 |
| VTOL I / II / III | 50 | 100 / 150 / 300 | 150 / 250 / 300 | 700 / 800 / 800 |
| Cyborg legs I / II / III | 100 | 50 / 100 / 150 | 10 | 400 |

**Tier II** is unlocked by research (campaign Factory Upgrade 4 is the prerequisite; 3,600-14,400 points each). **Tier III has
no unlocking research at all**; it is used by scripted campaign enemies (the campaign template library lists
`wheels3`, `tracks3`, `hover3`, `vtol3`...), so it works as an "enemy elite" tier. Each tier is lighter, tougher and more
expensive. The multiplayer ruleset removed the tiers and replaced them with percentage-upgrade research. The changelog
records a chat command to make produced or spawned units "use type I/II/III propulsions", so the tier concept is still
supported by the engine.

**This is direct evidence that the engine already supports "Mk II / Mk III component generations" as plain data.**

## 7. What the community reports (lower confidence: search excerpts, mostly older versions)

- Forum participants warned in the 2.x/3.x era that unless hover had clear weaknesses, players would use it for
  everything; hover dominance and "Nexus AI fixated on hovers" were recurring themes. Hover's recorded weaknesses are the
  rocky-terrain penalty and artillery (110% damage).
- Wheels were described as lacking a distinct role; one proposal gave them a clearer job. Half-tracks were criticised as
  "unrealistically" outpacing tracks, and a thread questioned why they needed a separate research topic.
- A third-party reference lists max speeds 0.98 (tracks), 1.17 (half-tracks), 1.37 (wheels), 2.34 (hover); these are
  the caps above in tiles per second (125 / 128 = 0.98, and so on).
- The *Battleplan* mod added a weight factor so that "heavy hover tanks are not faster than light wheeled ones". The
  *Contingency* mod gave hover the highest propulsion weight multiplier (12, against 1 for VTOL) while keeping it the
  fastest ground option.
- The GameSpot review of the original judged the practical variety narrower than the "2,000 designs" claim: five bodies,
  each with several tracks and weapons, give "five largely similar tanks".

See [../community-precedents.md](../community-precedents.md).

## 8. How hard is it to add propulsion?

| Change | Effort |
|---|---|
| A new propulsion *variant* of an existing type (e.g. "Tracks IV", "Light hover") | **Data only** (art per body is the main cost) |
| New stats for existing types, new terrain factors, new modifier values | Data only |
| A new propulsion *type* with its own terrain/path/physics rules | **C++**: new enum value, parser, terrain column, modifier row, path-blocking bits, continent flood-fill, movement-model dispatch, about 45 Lift-style checks, and an A* cost term for terrain preference |
| Make terrain affect routes | C++ (A* cost) |

## 9. Findings for our design work

1. **Three of the five designable propulsions (wheels, half-tracks, tracks) are the same machine with different numbers.** The
   only real behaviour differences are: hover (water), VTOL (air), legs (person physics), naval (water-only, unreachable).
   Variety in the original is *quantitative* (a ladder), not *qualitative*.
2. **Propulsion is simultaneously four things**: a hull multiplier (weight/HP/cost), a speed class, a movement domain, and an
   armour class for the damage matrix. A bolder system might separate those (a "locomotion" component with traits; a
   separate armour layer).
3. **Terrain does not steer units**, so terrain-based propulsion differences do not show up as tactics.
4. **A closed enum of seven types** is the biggest structural obstacle to "many more propulsions"; our design should make the
   propulsion *a data-defined trait set* (domain, terrain profile, turning model, signature, load-bearing) from the start.
5. **Component generations (Mk I-III) are a proven, cheap alternative to percentage research upgrades** and could coexist with
   them.
