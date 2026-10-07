#!/usr/bin/env python3
"""Explore Warzone 2100's combat numbers from its stat files (statistics only).

Usage:
    python3 -I wz_combat.py <wz_checkout> <section>

Sections:
    dps         nominal damage per second and per power cost for ground weapons
    vs          per-hit damage and effective DPS against representative targets
    floor       which weapons are pinned to the minimum-damage floor by which targets
    race        weapon-damage research versus armour research (per-hit damage over upgrade steps)
    chassis [weapon_id] [--abs-cost]
                body x propulsion efficiency frontier (HP per cost, speed, armour)

MODEL (confirmed by reading the engine's combat code; see
docs/research/warzone-2100/unit-design/06-damage-and-counters.md):
    times in the JSON are in 0.1 s units
    shots per cycle N = numRounds if reloadTime > 0 else 1
    cycle time = (N - 1) * firePause + reloadTime   (salvo)   or   firePause   (single shot)
    droid target:  d = base * (100 + (effectVsPropulsion - 100)) / 100 , at least 1
    then (rank 0):  d = max(d - armour, d * minimumDamage / 100) , at least 1
    armour is the body's kinetic value for KINETIC weapons and its thermal value for HEAT weapons
    total droid HP = body_hp * (100 + propulsion_hp_pct_of_body) / 100 + turret_hp
    research steps add ceil(base * step / 100) per step (percent of base, additive)

The checkout is untrusted data: this script only parses JSON.
"""
import json
import os
import sys


def load(root, name, ruleset="mp"):
    with open(os.path.join(root, "data", ruleset, "stats", name), encoding="utf-8") as f:
        return json.load(f)


def ceil_div(a, b):
    return -(-a // b)


class Data:
    def __init__(self, root):
        self.bodies = load(root, "body.json")
        self.props = load(root, "propulsion.json")
        self.ptypes = load(root, "propulsiontype.json")
        self.weapons = load(root, "weapons.json")
        self.mod = load(root, "weaponmodifier.json")
        self.ground = {k: w for k, w in self.weapons.items() if w.get("designable") and not w.get("numAttackRuns")}
        # weapons that can fire at ground targets (anti-aircraft-only weapons cannot)
        self.vs_ground = {k: w for k, w in self.ground.items() if "AirOnly" not in w.get("flags", "")}

    # ---- weapon helpers -------------------------------------------------
    def cycle_ms(self, w):
        pause = w.get("firePause", 0) * 100
        reload_ = w.get("reloadTime", 0) * 100
        if reload_ > 0 and w.get("numRounds", 0) > 0:
            n = w["numRounds"]
            return (n - 1) * pause + reload_, n
        return max(pause, 100), 1

    def dps(self, w, damage=None):
        cyc, n = self.cycle_ms(w)
        return (damage if damage is not None else w["damage"]) * n * 1000.0 / cyc

    # ---- target helpers -------------------------------------------------
    def target(self, body_id, prop_id, turret_hp=0):
        b, p = self.bodies[body_id], self.props[prop_id]
        hp = b["hitpoints"] * (100 + p.get("hitpointPctOfBody", 0)) // 100 + p.get("hitpoints", 0) + turret_hp
        return {"name": f"{b['name']} / {p['name']}", "hp": hp, "kin": b["armourKinetic"], "heat": b["armourHeat"], "ptype": p["type"]}

    def hit(self, w, t, dmg_override=None, armour_override=None):
        base = dmg_override if dmg_override is not None else w["damage"]
        pm = self.mod[w["weaponEffect"]][t["ptype"]]
        d = max((base * 100 + base * (pm - 100)) // 100, 1)
        armour = armour_override if armour_override is not None else (t["kin"] if w["weaponClass"] == "KINETIC" else t["heat"])
        floor_ = d * w.get("minimumDamage", 33) // 100
        return max(d - armour, floor_, 1), d, armour


def targets(d):
    ts = [
        d.target("Body5REC", "tracked01"),
        d.target("Body5REC", "wheeled01"),
        d.target("Body5REC", "hover01"),
        d.target("Body11ABT", "tracked01"),
        d.target("Body10MBT", "tracked01"),
    ]
    cy = d.bodies["CyborgLightBody"], d.props["CyborgLegs"]
    ts.append({"name": "Light cyborg", "hp": cy[0]["hitpoints"] * (100 + cy[1]["hitpointPctOfBody"]) // 100, "kin": cy[0]["armourKinetic"], "heat": cy[0]["armourHeat"], "ptype": "Legged"})
    return ts


def main():
    root, section = sys.argv[1], sys.argv[2]
    d = Data(root)
    gw = sorted(d.ground.values(), key=lambda w: (w["weaponSubClass"], w["buildPower"]))
    gv = sorted(d.vs_ground.values(), key=lambda w: (w["weaponSubClass"], w["buildPower"]))   # can hit ground units

    if section == "dps":
        print("| Weapon | Sub-class | Damage | Shots/cycle | Cycle (s) | Nominal DPS | Splash dmg | Long range (tiles) | Power cost | DPS per power | Weight |")
        print("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
        for w in gw:
            cyc, n = d.cycle_ms(w)
            dps = d.dps(w)
            print(f"| {w['name']}{' (AA only)' if 'AirOnly' in w.get('flags', '') else ''} | {w['weaponSubClass']} | {w['damage']} | {n} | {cyc / 1000:.1f} | {dps:.1f} | {w.get('radiusDamage', '')} | {w['longRange'] / 128:.1f} | {w['buildPower']} | {dps / w['buildPower']:.2f} | {w['weight']:,} |")

    elif section == "vs":
        ts = targets(d)
        print("Per-hit damage after modifier, armour and floor, rank 0, no research; in brackets effective DPS (nominal DPS x per-hit/base).\n")
        print("| Weapon (base dmg) | " + " | ".join(f"{t['name']} (HP {t['hp']}, arm {t['kin']}/{t['heat']})" for t in ts) + " |")
        print("|---|" + "---|" * len(ts))
        for w in gv:
            if w["weaponSubClass"] in ("ELECTRONIC", "EMP"):
                continue
            cells = []
            for t in ts:
                dmg, pre, arm = d.hit(w, t)
                cells.append(f"{dmg} ({d.dps(w, dmg):.1f})")
            print(f"| {w['name']} ({w['damage']}) | " + " | ".join(cells) + " |")

    elif section == "floor":
        ts = targets(d)
        print("Weapons whose per-hit damage is pinned to the minimum-damage floor (armour would otherwise cancel more than 67% of the hit).\n")
        for t in ts:
            pinned = []
            for w in gv:
                if w["weaponSubClass"] in ("ELECTRONIC", "EMP"):
                    continue
                dmg, pre, arm = d.hit(w, t)
                if pre - arm <= pre * w.get("minimumDamage", 33) // 100:
                    pinned.append(f"{w['name']} ({pre}-{arm} -> {dmg})")
            print(f"- **{t['name']}** (armour {t['kin']}/{t['heat']}): {len(pinned)} of {len(gv) - sum(1 for w in gv if w['weaponSubClass'] in ('ELECTRONIC', 'EMP'))} weapons pinned" + (": " + ", ".join(pinned) if pinned else ""))

    elif section == "race":
        pairs = [("MG1Mk1", "Body5REC", "tracked01"), ("MG3Mk1", "Body5REC", "tracked01"), ("MG4ROTARYMk1", "Body11ABT", "tracked01"),
                 ("Cannon1Mk1", "Body5REC", "tracked01"), ("Cannon375mmMk1", "Body11ABT", "tracked01"), ("Rocket-LtA-T", "Body5REC", "tracked01"),
                 ("Rocket-HvyA-T", "Body11ABT", "tracked01"), ("Mortar1Mk1", "Body5REC", "tracked01")]
        print("Per-hit damage as BOTH sides take the same number of research steps (weapon-damage line +25% of base per step; kinetic armour line +30% of base armour per step, compat rounding).\n")
        print("| Weapon vs target | Step 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | Weapon dmg step | Armour step |")
        print("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
        for wid, bid, pid in pairs:
            w = d.weapons[wid]
            t = d.target(bid, pid)
            base_dmg, base_arm = w["damage"], t["kin"] if w["weaponClass"] == "KINETIC" else t["heat"]
            step_dmg = ceil_div(base_dmg * 25, 100)
            step_arm = ceil_div(base_arm * (30 if w["weaponClass"] == "KINETIC" else 40), 100)
            cells = []
            for k in range(10):
                dmg_k = base_dmg + step_dmg * k
                arm_k = base_arm + step_arm * k
                hit, pre, _ = d.hit(w, t, dmg_override=dmg_k, armour_override=arm_k)
                cells.append(f"{hit}" + ("*" if hit <= pre * w.get('minimumDamage', 33) // 100 else ""))
            print(f"| {w['name']} ({base_dmg}) vs {t['name']} (arm {base_arm}) | " + " | ".join(cells) + f" | +{step_dmg} | +{step_arm} |")
        print("\n`*` = pinned to the minimum-damage floor at that step.")

    elif section == "chassis":
        args = [a for a in sys.argv[3:] if not a.startswith("--")]
        abs_cost = "--abs-cost" in sys.argv
        ref = d.weapons[args[0] if args else "Cannon2A-TMk1"]  # default: Medium Cannon
        ground_props = [k for k, p in d.props.items() if p.get("designable") and k != "Naval" and p["type"] != "Lift"]
        rows = []
        for bid, b in d.bodies.items():
            if not b.get("designable"):
                continue
            for pid in ground_props:
                p = d.props[pid]
                cost = b["buildPower"] * (100 + p["buildPower"]) // 100 + ref["buildPower"]
                hp = b["hitpoints"] * (100 + p["hitpointPctOfBody"]) // 100 + ref["hitpoints"]
                weight = b["weight"] * (100 + p["weight"]) // 100 + ref["weight"]
                base = d.ptypes[p["type"]]["multiplier"] * b["powerOutput"] // max(1, weight)
                if b["powerOutput"] > weight:
                    base = base * 3 // 2
                speed = min(base, p["speed"])
                rows.append({"body": b["name"], "prop": p["name"], "cost": cost, "hp": hp, "speed": speed, "arm": b["armourKinetic"], "hpc": hp / cost})

        def dominated(a, others):
            def vec(r):
                v = [r["hpc"], r["speed"], r["arm"]]
                if abs_cost:
                    v.append(-r["cost"])  # cheaper is better
                return v
            va = vec(a)
            return any(o is not a and all(x >= y for x, y in zip(vec(o), va)) and any(x > y for x, y in zip(vec(o), va)) for o in others)

        front = [r for r in rows if not dominated(r, rows)]
        dims = "HP per power cost, flat-ground speed, kinetic armour" + (", and absolute power cost (cheaper is better)" if abs_cost else "")
        print(f"Reference turret: {ref['name']} (weight {ref['weight']:,}). {len(rows)} body x ground-propulsion combinations; frontier in ({dims}) has {len(front)} members.\n")
        print("| Body | Propulsion | Power cost | HP | HP per cost | Speed | Armour (kin) | On frontier |\n|---|---|---:|---:|---:|---:|---:|---|")
        for r in sorted(rows, key=lambda r: (r["prop"], -r["hpc"])):
            print(f"| {r['body']} | {r['prop']} | {r['cost']} | {r['hp']} | {r['hpc']:.2f} | {r['speed']} | {r['arm']} | {'yes' if r in front else ''} |")
        by_prop = {}
        for r in front:
            by_prop[r["prop"]] = by_prop.get(r["prop"], 0) + 1
        print("\nFrontier membership by propulsion: " + ", ".join(f"{k}: {v}" for k, v in sorted(by_prop.items())))
    else:
        sys.exit("unknown section")


if __name__ == "__main__":
    main()
