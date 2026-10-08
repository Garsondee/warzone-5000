# Game Loop: Draft, Design, Deploy, Battle, Shop

*Source: the owner's revised brief (2026-10-07). Status: design. Nothing here is built yet except the parametric
component prototype ([04-parametric-components.md](04-parametric-components.md)).*

## The loop
```
 draft --> design bench --> command-point pricing --> async battle --> rewards --> shop --> design bench ...
 (cards)   (sliders + arena)  (N copies per budget)    (auto-battle)    (gold)    (tech, licences)
```

1. **Draft.** The run opens with drafts of four cards per critical slot: hull, turret, engine, weapon (and
   locomotion). One of each critical component is always guaranteed, so a working unit can always be built. Cards are
   *families* (a "gun turret", "spider legs"), materials ("ceramic armour") or mechanisms ("autoloader"). They are
   not numbered variants.
2. **Design bench.** Players attach components and tune each component's sliders
   ([04](04-parametric-components.md)). A **live mini-arena** beside the bench holds one copy of the design. At any
   time the player can drop in bots and watch the AI fight it out, so the design is evaluated continuously. When
   happy, they name it.
3. **Pricing and deployment.** Each design gets a **command-point (CP) cost**. A battle has a CP budget, and the roster
   fields as many copies as the budget buys: 40 drones, or 1 titan.
4. **Async battle.** The opponent is a snapshot of another player's roster at a similar point in their run. Neither
   side issues commands: units attack-move under the AI. Players may pick routes (lanes) before the battle. The battle
   is deterministic, so it can be replayed from its inputs and verified by a server.
5. **Rewards.** Gold according to the result (and perhaps style: damage dealt, survivors).
6. **Shop.** Gold buys new cards, and **licences**: roster slots for more designs and a bigger CP budget.
7. Back to the bench: refine designs, add new ones, retire failures.

**Run structure (owner, 2026-10-08):** a player has **three lives**. A battle in which all their units are destroyed
costs a life; losing all three ends the run. **Ten victories** against other players make the run golden: a win.

## Command points: pricing a design
The CP price decides whether swarms or titans win, so it must price **combat value**, not just size.

> **Theory: Lanchester's laws.** In 1916 F. W. Lanchester modelled two forces A and B firing at each other: each side
> loses units in proportion to the other side's size, dA/dt = -b B and dB/dt = -a A. It follows that
> a A^2 - b B^2 stays constant. A force's **strength grows with the square of its numbers**, but only linearly with
> each unit's quality. Splitting a budget into twice as many units of half the quality doubles your strength, as long
> as every unit can engage. So naive pricing (CP proportional to mass) makes swarms win. The game's job is to make that
> only *usually* true:
> * **Penetration thresholds** break the square law: if a small gun cannot penetrate, its b is zero and numbers do not
>   help. Armour against penetration is the titan's answer to swarms.
> * **Area weapons** (high explosive, splash) kill several small units per shot, which is the answer to swarms.
> * **Range** lets the bigger gun fire for longer before the swarm can reply.
> * **Overkill** wastes a big shell on a small target, which is the swarm's answer to titans.

Proposed price: **CP = logistics + combat value**.
- Logistics grows with mass and size (transport, fuel, crew), so physically big things are never cheap.
- Combat value comes from automated benchmark fights in the same deterministic simulation: the design is fought against a
  fixed gauntlet of reference rosters at equal CP, and its win share sets the value. This is self-correcting: an
  under-priced design wins too often in the benchmark and its price rises. The breakdown is shown to the player.

## What keeps titans honest: physics
1. **Soft ground.** *Theory, terramechanics:* a vehicle presses on the ground with pressure p = W / A (weight over
   contact area). Soil sinks under pressure following Bekker's relation p = (k_c / b + k_phi) z^n (z is sinkage, b
   contact width, the k and n are soil constants). Sinkage costs motion resistance; the soil's shear strength
   (Mohr-Coulomb: tau = c + sigma tan phi) limits traction. When resistance exceeds traction the vehicle is stuck.
   Because weight grows with the cube of size and contact area only with the square, **pressure grows with size**. So
   giants sink unless they buy wide tracks, many big feet, hover, anti-gravity, or rails. For scale: a tank presses about 90 kPa,
   a person about 50 kPa, and soft soil gives way at around 50-100 kPa.
2. **Being hit.** We already measure each design's silhouette area per direction. The chance of a hit is roughly
   silhouette area over the area of the shot's dispersion ellipse, so big targets are hit more.
3. **Being where the shell lands.** A shell's flight time is range / velocity: 10 km at 800 m/s is 12.5 s, in which a
   15 m/s scout moves 190 m. The shooter must predict, and a target that **jinks** (serpentine movement, irregular
   speed) spreads the prediction. Small, fast, unpredictable units are rarely there when the shell arrives; slow giants
   always are. Evasive movement is a behaviour slider that costs average speed.
4. **Repair.** Repair time grows with damaged mass, so a repair vehicle keeps a squad of small tanks running but
   cannot keep up with a titan everything is shooting at.
5. **Recoil and structure.** A gun's recoil impulse sets the minimum hull mass under it ([04](04-parametric-components.md)).
   Mass grows with size cubed but leg and armour strength only with size squared, so giant walkers need better
   materials (tech gates).
6. **Rail.** Steel wheels on steel rails roll with about 1/20 of a tyre's resistance (C_rr ~ 0.0015 against 0.03)
   and the track spreads the load, so a railway carriage can carry what no tracks can. The price is that it goes only
   where the rails go. A cheap recon drone spotting for a train with four battleship turrets is a legitimate strategy.

## Behaviour is parametric too
In an auto-battler the AI is the player's hands, so behaviour is part of the design. Each design gets a small set of
behaviour sliders:
- preferred engagement range (close in / hold at maximum range);
- aggression (push to objectives, or hold and support);
- target priority (biggest threat, weakest, nearest, by type);
- evasion (jinking amplitude: harder to hit, slower to arrive);
- spacing (spread out against splash, bunch up for mutual support);
- retreat threshold (fall back to repair at X% damage).

Some are taste (free), some cost (evasion costs speed). A strong, legible baseline AI is the project's biggest risk; see
the roadmap.

## Technical consequences
- The simulation must run **in real time** for the design arena (inside Godot) and **fast headless** for pricing
  benchmarks, AI testing and server verification. It is the same deterministic Rust simulation either way.
- Lockstep netcode is no longer central: a battle is a pure function of (roster A, roster B, map, seed), which makes
  replays tiny and lets a server verify results ([decisions](../planning/decisions.md), D5).
- The forge must answer interactively. Slider drags use the fast mass estimate (milliseconds); the full measurement
  (voxels and armour rays, a few hundred ms) runs in the background when the slider is released.
- A design's battle stats must be identical everywhere: the authoritative baker (a server, or the deterministic baker
  later) produces them and they travel with the roster.

## Open questions for the owner
- Does the CP budget grow during a run, or do licences add slots under a fixed budget?
- Is a battle that ends with both sides alive (a timeout) a loss of nothing, or decided on points?
- Route choice: a handful of lanes per map, or free waypoints?
- Can designs be retired or sold back in the shop?
