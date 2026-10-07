# Limits and Extensibility: What Is Data, What Is Code

**Snapshot:** Warzone 2100 `master` @ `d7ce18df8d`. **Evidence:** engine source read by three analysis agents (design/movement,
combat, research); cited lines are in the Warzone 2100 checkout. "Effort" is my judgement from their findings and is labelled
as opinion.

Why this matters for us: we are *not* building on the Warzone 2100 engine, but the places where its design system is
rigid show which decisions are expensive to reverse. Those are the decisions to make differently in our own architecture.

## 1. What a modder can do with data alone

- Add **any number** of bodies, propulsion variants, weapons, sensors, structures, research topics (with caps below).
- Change **every number**: stats, modifier matrices, terrain speed table, research costs, prerequisites, rank thresholds.
- Define **upgrade effects** with a class, a parameter, a percentage, and one equality filter on any exposed stat.
- Create **component generations** (Mk I / II / III) as separate components (the campaign does this).
- Use **free-form body classes** as tags for research filters.
- Script AI, rules, campaigns and tech grants in JavaScript (see [../research-system/05-scripting-surface.md](../research-system/05-scripting-surface.md)).

Caps even in data: component indices are 8-bit in templates and the network protocol, so at most **256 per non-weapon type**
(weapons use 32-bit); the design screen lists at most **40 bodies**, **40 propulsions** and 80 extra system parts; research
points are 16-bit (max 65,535); research indices are 16-bit.

## 2. What is compiled in

| Fixed in C++ | Value | Where |
|---|---|---|
| Component types | 8 (body, propulsion, brain, sensor, ECM, repair, construct, weapon); one slot per type per droid | `statsdef.h:95-106`, `droiddef.h:44` |
| Weapon slots | max **3** per droid (`MAX_WEAPONS`), 3 UI buttons; per-body `weaponSlots` | `basedef.h:76`, `design.cpp:762-799` |
| Propulsion types | **7**; speed, terrain and modifier tables are sized by them; the loader demands all 7 | `statsdef.h:226-236`, `stats.cpp:1192-1199` |
| Body sizes | 4 (light, medium, heavy, super heavy); drive factory modules, transporter slots, VTOL speed, collision radius | `statsdef.h:133-140` |
| Droid types | 14 + any, derived from components by a fixed priority | `statsdef.h:37-54`, `droid.cpp:1609-1654` |
| Weapon classes / sub-classes / effects / movement models | 2 / 16-17 / 6 / 4 | `statsdef.h:108-198` |
| Structure strengths | 4 ("FIXME - add support for dynamic categories") | `structure.cpp:793-845` |
| Armour channels | 2 (kinetic, thermal) | `statsdef.h:521-527` |
| Terrain types | 12 (also in the map file format) | `terrain_type.h:26-42` |
| Upgrade schema | 9 classes with hard-listed parameters; arithmetic fixed (percent of base, additive, integer, one equality filter) | `wzapi.cpp:3596-4690` |
| Design legality rules | VTOL/weapon rules, slot counts, system exclusivity | `design.cpp:2795-2944` |
| Weapon-versus-sensor pairing for indirect fire | hard-coded | `droid.cpp:3429-3513` |
| Miss distance caps (120/100/150% of range) | hard-coded constants with a "make modifiable one day" comment | `stats.cpp:470-482` |
| Rank bonuses per level (-6% damage, +5% accuracy, +5% speed) | constants | `droid.h:52-54` |

## 3. Effort to extend (opinion, from the findings)

| Goal | What it takes | Effort |
|---|---|---|
| More bodies, weapons, propulsion *variants*, Mk II/III generations | JSON | **Trivial** |
| New damage numbers, new modifier values, new terrain speeds | JSON | **Trivial** |
| Per-faction research trees | hidden "token" topics pre-completed per faction | **Easy** (data/JS) |
| More than 1 weapon slot on a body | `weaponSlots` up to 3 in data; but several code paths read **slot 0 only** (fire-on-move pause, sensor pairing, VTOL validity, template idempotence, `switchComponent`), 3 hard-wired UI buttons, art uses model connectors | **Medium** |
| Speed/turn/acceleration upgradable by research | add the parameters to the upgrade registry | **Small** |
| New armour types (more than kinetic/thermal) | all damage flows through one function (`objDamage`); make armour a vector | **Medium**, well-contained |
| New weapon effects, sub-classes or damage classes | enums, parsers, statically sized tables | **Medium-high** |
| OR-prerequisites or mutually exclusive research choices | the engine has AND only; emulate with scripted zero-prerequisite topics | **Medium** |
| **A separate engine component** | new component type: template array, JSON key, network format, UI, upgrade registry, the `calcSum` pattern (generic over component classes); the speed *physics* needs no change (it consumes only `weight` and `base speed`) | **Medium-high** but mechanical |
| New propulsion *type* with its own rules | new enum value, name parser, terrain-table column, modifier row, path-blocking bits, continent flood-fill, movement-model dispatch, about 45 Lift-style checks | **Hard** |
| Terrain affecting route choice | A* cost term (it currently ignores terrain speed) | **Hard** (new cost model) |
| New terrain types | enum plus map format | **Hard** |
| General status effects (slow, burn, shock...) | EMP, electronic warfare and burn are one-off globals keyed on weapon sub-class | **Hard** |
| Per-component damage | one HP pool per droid; a comment claims per-part damage but it does not exist | **Hard** |

## 4. Latent hooks already in the engine (unused or dormant)

- A **body-size axis** in the damage-modifier table (read by the code; no shipped data defines it).
- A **weapon-size rule** (light weapon only on light body) in the design validator; no data sets `weaponSize`.
- **Per-turret hit points** and per-component `hitpoints` fields.
- A `droidTypeOverride`, `usageClass` tags, component tiers.
- A shield branch in the damage code (dormant).
- ECM as a jammer concept (dormant: no research, no designable ECM turret).
- A latent unsigned-arithmetic edge in `calcDamage` if modifier deficits sum past 100%.

## 5. Quirks worth not repeating

- Derived stats (speed, HP) are **cached** and refreshed only on certain events; `switchComponent` swaps parts without recomputing.
- Weight, cost and research upgrades **cannot be touched by research** (only listed parameters).
- A **second parallel system** for cyborgs (their own bodies, propulsion, weapons, factory, templates).
- The "default location" component trick (Auto-Repair is a repair component swapped into every droid) is clever but needs a
  special case in the validator.
- Weapon-slot visuals depend on **3D model connector points**, so gameplay rules and art are entangled; whether VTOLs can pass
  over a structure depends on the structure model's height.
- Many "multi-weapon" paths use slot 0 only; the code has a "FUTURE" note for multi-weapon bodies.

## 6. Design principles distilled (opinion)

1. **Data-define every enumeration** (propulsion traits, armour types, damage types, terrain, sizes). Closed enums are the
   single biggest source of rigidity found.
2. **Store derived stats as a pure function of the design plus global state**, recomputed on demand; avoid caches that need
   careful invalidation.
3. **Make "component type" a data concept** (slots described by data: name, count, rules), not a fixed array, so engines,
   reactors, armour plating and utility modules are just entries.
4. **Keep damage in one chokepoint** that accepts an armour *vector* and a damage *vector*.
5. **Separate art from rules**: do not let model attachment points determine how many weapons fit.
6. **Keep the upgrade engine generic** (class + parameter + filter + arithmetic mode), as WZ does, but let it reach *every* stat
   including speed, turn and weight.
