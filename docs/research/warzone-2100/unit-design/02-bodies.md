# Bodies (Chassis)

**Snapshot:** Warzone 2100 `master` @ `d7ce18df8d`, multiplayer ruleset unless stated.
Full table with tech-unlock timing: [`../data/bodies.md`](../data/bodies.md);
dominance analysis: [`../data/dominance.md`](../data/dominance.md).

## 1. What a body is in this game

A body provides: **hit points**, **kinetic armour**, **thermal (heat) armour**, **engine power output**,
**weight**, **power cost**, **build points** (production time), a **size class** (Light / Medium / Heavy), and
a number of **weapon slots** (1, except the Dragon with 2). It also carries a *class*: Droids (player tanks),
Cyborgs, Transports, or "Babas" (scenery/scavenger vehicles).

The body is where the **engine lives**: `powerOutput` is a property of the body, not a separate part, and research
upgrades it (see [04-engine-and-speed.md](04-engine-and-speed.md)).

## 2. The 14 designable bodies (multiplayer)

| Size | Body | Slots | HP | Armour (kin) | Armour (heat) | Engine power | Weight | Power/weight | Power cost | Build pts |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Light | Bug | 1 | 55 | 8 | 8 | 5,000 | 450 | 11.1 | 25 | 100 |
| Light | Viper | 1 | 65 | 10 | 4 | 5,000 | 600 | 8.3 | 30 | 150 |
| Light | Leopard | 1 | 120 | 14 | 6 | 6,500 | 750 | 8.7 | 37 | 170 |
| Light | Retaliation | 1 | 150 | 20 | 15 | 5,000 | 450 | 11.1 | 45 | 170 |
| Medium | Scorpion | 1 | 125 | 12 | 12 | 15,000 | 1,500 | 10.0 | 39 | 250 |
| Medium | Cobra | 1 | 130 | 15 | 6 | 15,000 | 2,000 | 7.5 | 46 | 250 |
| Medium | Panther | 1 | 193 | 18 | 9 | 18,000 | 2,500 | 7.2 | 57 | 300 |
| Medium | Retribution | 1 | 230 | 24 | 20 | 15,000 | 1,500 | 10.0 | 64 | 330 |
| Heavy | Mantis | 1 | 180 | 18 | 18 | 20,000 | 2,100 | 9.5 | 52 | 350 |
| Heavy | Python | 1 | 200 | 20 | 9 | 20,000 | 2,700 | 7.4 | 60 | 350 |
| Heavy | Tiger | 1 | 284 | 22 | 15 | 18,000 | 3,300 | 5.5 | 71 | 420 |
| Heavy | Vengeance | 1 | 300 | 28 | 25 | 23,000 | 2,500 | 9.2 | 80 | 470 |
| Heavy | Wyvern | 1 | 350 | 28 | 28 | 25,000 | 3,500 | 7.1 | 86 | 470 |
| Heavy | Dragon | **2** | 350 | 30 | 30 | 30,000 | 4,500 | 6.7 | 90 | 500 |

"Power/weight" is engine power divided by body weight alone (before propulsion and weapon are added); it is
shown only to compare chassis. Not-designable bodies: scavenger/civilian vehicles (about 25), the two cyborg bodies
(Light 200 HP, Heavy/"super" 300 HP), the Transporter (1,000 HP) and the Super Transporter (2,200 HP).

## 3. Observations from the numbers

### 3.1 Size classes are driven by engine output, not weight

| Class | Engine power | Weight | HP |
|---|---|---|---|
| Light | 5,000-6,500 | 450-750 | 55-150 |
| Medium | 15,000-18,000 | 1,500-2,500 | 125-230 |
| Heavy | 18,000-30,000 | 2,100-4,500 | 180-350 |

Engine output jumps about **3x** between Light and Medium and then grows more slowly into Heavy; weight and HP
grow more smoothly. The class boundary is therefore largely an *engine step*.

### 3.2 It is a ladder with a price tag, not a web of sidegrades

- **No body is strictly dominated** once cost and weight are counted; every body gives something up for what it gains.
- But ignoring cost and weight, **most bodies are strictly worse in raw capability than several others**: Viper is
  beaten on HP, both armours and engine power by 12 of the other 13 bodies; Cobra by 8; Bug by 10; Dragon by none.
- In effect, the body list is one long progression (cost rises with capability), plus a few "weight-efficient" outliers:
  the *armour specialists* **Retaliation, Retribution, Vengeance** (very high armour for their size and low weight,
  so very high power/weight), versus the *HP/size* line **Python, Tiger, Wyvern, Dragon**.
- Bodies with high kinetic but low thermal armour (Viper 10/4, Leopard 14/6, Cobra 15/6, Python 20/9, Panther 18/9)
  are vulnerable to heat weapons; balanced bodies (Scorpion 12/12, Mantis 18/18, Wyvern 28/28, Dragon 30/30) are not.
  This is a real, if mild, source of choice.

### 3.3 Scale of the ladder

| Quantity | Smallest | Largest | Ratio |
|---|---:|---:|---:|
| HP | 55 (Bug) | 350 (Wyvern, Dragon) | 6.4x |
| Armour (kinetic) | 8 | 30 | 3.8x |
| Engine power | 5,000 | 30,000 | 6x |
| Weight | 450 | 4,500 | 10x |
| Power cost | 25 | 90 | 3.6x |
| Build points | 100 | 500 | 5x |

Large bodies buy their HP more cheaply: HP per power-cost point is 2.2 for Bug/Viper and 3.9-4.1 for
Tiger/Wyvern/Dragon (body alone). Once a propulsion and weapon are added, the *weapon's* cost and HP are a fixed
overhead that a larger body dilutes; the propulsion's cost, weight and HP are all **percentages of the body's** (see
[01-design-pipeline.md](01-design-pipeline.md)), so they scale up with the body and do not dilute. Heavier bodies are
slower, so speed is the counterweight, but see [04-engine-and-speed.md](04-engine-and-speed.md): for wheels and
half-tracks the speed cap often hides that cost.

## 4. How bodies are unlocked (tech structure)

Each body is a single research item, and the **engine and armour research lines are prerequisites**:

| Body | Depth in tree | Critical-path research points |
|---|---:|---:|
| Viper | 0 | 600 |
| Cobra | 4 | 7,200 |
| Bug | 5 | 9,000 |
| Scorpion | 6 | 13,800 |
| Python | 6 | 13,800 |
| Mantis | 7 | 17,400 |
| Leopard | 7 | 19,200 |
| Panther | 9 | 29,200 |
| Tiger | 10 | 44,800 |
| Retaliation | 12 | 66,400 |
| Retribution | 13 | 89,200 |
| Vengeance | 14 | 121,200 |
| Wyvern | 16 | 167,000 |
| Dragon | 17 | 210,200 |

The unlock chain for the upper bodies runs through **Superdense Composite Alloys (armour)** and
**Turbo-Charged / Gas Turbine Engine (engine)** research (details in
[../research-system/03-tree-analysis.md](../research-system/03-tree-analysis.md)). The costliest prerequisite chain
leading to the Dragon sums to about **210,000** research points (about 6% of the whole tree's points; the full set of
prerequisites is larger than a single chain).

## 5. Campaign versus multiplayer bodies

The campaign ruleset (`data/base`) has the same 12 designable bodies (no Wyvern/Dragon) but different numbers. Examples
(campaign / multiplayer):

| Body | HP | Power cost | Build pts | Engine power |
|---|---|---|---|---|
| Leopard | 85 / 120 | 40 / 37 | 220 / 170 | 4,000 / 6,500 |
| Retaliation | 100 / 150 | 100 / 45 | 400 / 170 | same |
| Retribution | 200 / 230 | 150 / 64 | 600 / 330 | same |
| Vengeance | same | 200 / 80 | 800 / 470 | same |
| Panther | 145 / 193 | 60 / 57 | same | 13,000 / 18,000 |
| Tiger | 225 / 284 | 80 / 71 | same | same |

Take-away: the same chassis list has been **re-tuned repeatedly**; the changelog even records a change to
"make Leopard and Panther have higher engine power output thus move faster with more weapons". Engine output is
a live balance lever.

## 6. Cyborgs: a closed parallel system

Cyborgs use two special bodies (Light and Heavy "super") and a single legged propulsion, and they appear **only in
predefined templates** (28 of them) using cyborg-only weapons. They cannot be designed by the player. This is a
whole unit category that bypasses the design system.

## 7. Findings for our design work

1. WZ2100's body list is a single quality ladder; a bigger, bolder design could make bodies *role-defined
   sidegrades* (e.g. scout, brawler, carrier, artillery platform, support hull), each with different slot layouts
   and engine/weight curves, so no body is simply "the next one up".
2. Making the chassis the home of slots (weapon, utility, engine, armour) is a natural extension: WZ has 1 slot on 13
   of 14 bodies and 2 on one.
3. The **engine-step between size classes** shows the original designers used engine output as a class
   fence. A real separate engine component would turn that fence into a player decision.
4. Research gating (armour/engine lines unlock chassis) is a cheap and effective technique worth keeping.
