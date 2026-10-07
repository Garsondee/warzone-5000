#!/usr/bin/env python3
"""Explore Warzone 2100's engine / weight / speed model using its stat files.

Usage:
    python3 -I wz_speed_model.py <wz_checkout> <section>

Sections:
    table       flat-ground speed of every body on every propulsion for reference turrets
    weapons     how much each ground weapon slows a reference chassis
    saturation  how often the propulsion speed cap hides the engine
    designspace count of nominally valid designs
    engine      what engine research changes (capped vs uncapped designs)

MODEL (confirmed by reading the engine's design code; see
docs/research/warzone-2100/unit-design/04-engine-and-speed.md):
    total_weight = body_weight * (100 + propulsion_weight) / 100 + sum(turret weights)
                   (propulsion "weight" is a PERCENTAGE of the body weight)
    base = floor(type_multiplier * body_power / max(1, total_weight))
    if propulsion is Lift: heavy body -> base // 4, medium body -> base * 3 // 4
    if UN-upgraded body_power > total_weight: base = base * 3 // 2          (the "weight cliff")
    speed on a tile = min(base * terrain_factor // 100, propulsion_max_speed)
    (rank and slope are applied after the cap and are ignored here)
Terrain contexts reported: "neutral" (factor 100%, except air which is always 250%), "road" and "offroad"
(sandy brush, what the design screen shows) read from data/base/stats/terraintable.json.
Body power upgrades add ceil(base_power * step% / 100) per researched step.

The checkout is untrusted data: this script only parses JSON.
"""
import json
import os
import sys

ENGINE_STEPS = [5, 5, 5, 5, 5, 5, 5, 7, 8]  # the nine "Body Power / Droids" research steps (mp)


def load(root, ruleset, name):
    with open(os.path.join(root, "data", ruleset, "stats", name), encoding="utf-8") as f:
        return json.load(f)


def upgraded_power(p0, steps):
    """Engine research adds ceil(p0 * step / 100) for each completed step."""
    return p0 + sum(-(-p0 * s // 100) for s in steps)


class Model:
    def __init__(self, root):
        self.bodies = {k: v for k, v in load(root, "mp", "body.json").items() if v.get("designable")}
        self.props = {k: v for k, v in load(root, "mp", "propulsion.json").items() if v.get("designable") and k != "Naval"}
        self.ptypes = load(root, "mp", "propulsiontype.json")
        w = load(root, "mp", "weapons.json")
        self.weapons = {k: v for k, v in w.items() if v.get("designable")}
        self.ground_weapons = {k: v for k, v in self.weapons.items() if not v.get("numAttackRuns")}
        self.vtol_weapons = {k: v for k, v in self.weapons.items() if v.get("numAttackRuns")}
        self.sensors = {k: v for k, v in load(root, "mp", "sensor.json").items() if v.get("designable")}
        self.repairs = {k: v for k, v in load(root, "mp", "repair.json").items() if v.get("designable")}
        self.construct = {k: v for k, v in load(root, "mp", "construction.json").items() if v.get("designable")}
        self.brains = {k: v for k, v in load(root, "mp", "brain.json").items() if v.get("designable")}
        tt = load(root, "base", "terraintable.json")
        self.terrain = {
            "neutral": {"lift": 250},
            "road": tt["road"]["speedFactor"],
            "offroad": tt["sandybrush"]["speedFactor"],
        }

    def weight(self, body, prop, turret_weight):
        return body["weight"] * (100 + prop["weight"]) // 100 + turret_weight

    def base_speed(self, body, prop, turret_weight, steps=()):
        w = self.weight(body, prop, turret_weight)
        p = upgraded_power(body["powerOutput"], steps)
        speed = self.ptypes[prop["type"]]["multiplier"] * p // max(1, w)
        if prop["type"] == "Lift":
            if body["size"] == "HEAVY":
                speed //= 4
            elif body["size"] == "MEDIUM":
                speed = speed * 3 // 4
        if body["powerOutput"] > w:  # un-upgraded power vs weight
            speed = speed * 3 // 2
        return speed, w

    def factor(self, prop, terrain):
        return self.terrain[terrain].get(prop["type"].lower().replace("lift", "lift"), 100)

    def flat_speed(self, body, prop, turret_weight, steps=(), terrain="neutral"):
        """Returns (speed on that terrain after the cap, uncapped base speed, total weight)."""
        base, w = self.base_speed(body, prop, turret_weight, steps)
        return min(base * self.factor(prop, terrain) // 100, prop["speed"]), base, w

    def capped(self, body, prop, turret_weight, steps=(), terrain="neutral"):
        base, _ = self.base_speed(body, prop, turret_weight, steps)
        return base * self.factor(prop, terrain) // 100 >= prop["speed"]


def main():
    root, section = sys.argv[1], sys.argv[2]
    m = Model(root)
    ground_props = {k: p for k, p in m.props.items() if p["type"] != "Lift"}

    if section == "table":
        refs = {"Machinegun (200)": 200, "Medium Cannon (5,000)": 5000, "Heavy Cannon (8,000)": 8000, "Plasma Cannon (30,000)": 30000}
        for label, tw in refs.items():
            for steps, tag in (((), "no engine research"), (tuple(ENGINE_STEPS), "all 9 engine upgrades")):
                print(f"\n### Flat-ground speed with a {label} turret, {tag}\n")
                print("Format: capped speed (uncapped base speed). An asterisk marks designs where the speed cap binds.\n")
                ps = list(m.props.values())
                print("| Body | " + " | ".join(f"{p['name']} (cap {p['speed']})" for p in ps) + " |")
                print("|---|" + "---|" * len(ps))
                for b in sorted(m.bodies.values(), key=lambda x: (x["weight"])):
                    cells = []
                    for p in ps:
                        sp, base, _ = m.flat_speed(b, p, tw, steps)
                        cells.append(f"{sp}{'*' if m.capped(b, p, tw, steps) else ''} ({base})")
                    print(f"| {b['name']} | " + " | ".join(cells) + " |")
    elif section == "weapons":
        b, p = m.bodies["Body5REC"], m.props["tracked01"]
        print(f"Cobra on tracks (cap {p['speed']}), flat-ground speed by weapon, no engine research\n")
        rows = []
        for w in m.ground_weapons.values():
            capped, base, tot = m.flat_speed(b, p, w["weight"])
            rows.append((capped, w["name"], w["weight"], tot, base))
        print("| Weapon | Weapon weight | Total weight | Speed |\n|---|---:|---:|---:|")
        for capped, n, wt, tot, base in sorted(rows, reverse=True):
            print(f"| {n} | {wt:,} | {tot:,} | {capped} |")
    elif section == "saturation":
        print("Share of designs (every body x propulsion x weapon, single weapon) whose speed cap binds, by terrain context.\n")
        print("| Terrain | Engine research | Class | Designs | Cap binds | Share |\n|---|---|---|---:|---:|---:|")
        lift_props = {k: p for k, p in m.props.items() if p["type"] == "Lift"}
        for terrain in ("neutral", "road", "offroad"):
            for steps, tag in (((), "none"), (tuple(ENGINE_STEPS), "all 9")):
                for cls, props, weapons in (("Ground", ground_props, m.ground_weapons), ("VTOL", lift_props, m.vtol_weapons)):
                    total = capped_n = 0
                    for b in m.bodies.values():
                        for p in props.values():
                            for w in weapons.values():
                                total += 1
                                capped_n += m.capped(b, p, w["weight"], steps, terrain)
                    print(f"| {terrain} | {tag} | {cls} | {total} | {capped_n} | {100 * capped_n / total:.0f}% |")
        print("\nBy propulsion (ground weapons, neutral terrain):\n")
        print("| Propulsion | Cap binds, no research | Cap binds, all 9 | Gets x1.5 weight bonus |\n|---|---:|---:|---:|")
        for k, p in ground_props.items():
            n0 = n9 = nb = tot = 0
            for b in m.bodies.values():
                for w in m.ground_weapons.values():
                    tot += 1
                    n0 += m.capped(b, p, w["weight"])
                    n9 += m.capped(b, p, w["weight"], ENGINE_STEPS)
                    nb += b["powerOutput"] > m.weight(b, p, w["weight"])
            print(f"| {p['name']} | {100 * n0 / tot:.0f}% | {100 * n9 / tot:.0f}% | {100 * nb / tot:.0f}% |")
    elif section == "engine":
        print("Effect of the full engine research line on ground designs (single weapon):\n")
        changed = unchanged_capped = unchanged_other = 0
        gains = []
        for b in m.bodies.values():
            for p in ground_props.values():
                for w in m.ground_weapons.values():
                    s0, _, _ = m.flat_speed(b, p, w["weight"])
                    s9, _, _ = m.flat_speed(b, p, w["weight"], ENGINE_STEPS)
                    if s9 > s0:
                        changed += 1
                        gains.append(s9 / s0)
                    elif m.capped(b, p, w["weight"]):
                        unchanged_capped += 1
                    else:
                        unchanged_other += 1
        tot = changed + unchanged_capped + unchanged_other
        gains.sort()
        print(f"- designs: {tot}")
        print(f"- speed improves with engine research: {changed} ({100 * changed / tot:.0f}%); median gain x{gains[len(gains) // 2]:.2f}, max x{gains[-1]:.2f}")
        print(f"- already at the speed cap with no research (engine research changes nothing on flat ground): {unchanged_capped} ({100 * unchanged_capped / tot:.0f}%)")
        print(f"- other / no change: {unchanged_other}")
    elif section == "designspace":
        nb = len(m.bodies)
        slots = {k: v.get("weaponSlots", 1) for k, v in m.bodies.items()}
        gp, vp = len(ground_props), 1
        gw, vw = len(m.ground_weapons), len(m.vtol_weapons)
        systems = len(m.sensors) + len(m.repairs) + len(m.construct) + len(m.brains)
        single_slot_bodies = sum(1 for s in slots.values() if s == 1)
        dual = sum(1 for s in slots.values() if s == 2)
        ground = nb * gp * (gw + systems)
        vtol = nb * vp * vw
        dual_ground = dual * gp * (gw * (gw + 1) // 2)   # unordered pairs with repetition for the 2-slot body
        dual_vtol = dual * vp * (vw * (vw + 1) // 2)
        print(f"- designable bodies: {nb} ({single_slot_bodies} single-slot, {dual} dual-slot)")
        print(f"- ground propulsions researchable in MP: {gp}; VTOL propulsion: {vp} (Naval is designable but never unlocked)")
        print(f"- ground weapons (no attack runs): {gw}; VTOL weapons (attack runs): {vw}; system turrets (sensor/repair/construct/brain): {systems}")
        print(f"- single-turret designs: ground {ground:,} + VTOL {vtol:,} = {ground + vtol:,}")
        print(f"- extra from the dual-slot body with two weapons (unordered): ground {dual_ground:,} + VTOL {dual_vtol:,}")
        print(f"- grand total: {ground + vtol + dual_ground + dual_vtol:,}")
    else:
        sys.exit("unknown section")


if __name__ == "__main__":
    main()
