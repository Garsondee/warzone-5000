#!/usr/bin/env python3
"""Tabulate Warzone 2100 component stats (statistics only, no game code reused).

Usage:
    python3 -I wz_components.py <wz_checkout> <section> [mp|base]

Sections: bodies, propulsion, weapons, turrets, modifiers, terrain, timeline,
          dominance, structures

The wz checkout is untrusted data: this script only parses JSON from it.
Output is Markdown on stdout.
"""
import json
import os
import sys
from collections import defaultdict


def load(root, ruleset, name):
    with open(os.path.join(root, "data", ruleset, "stats", name), encoding="utf-8") as f:
        return json.load(f)


def table(headers, rows):
    out = ["| " + " | ".join(headers) + " |", "|" + "|".join("---" for _ in headers) + "|"]
    for r in rows:
        out.append("| " + " | ".join("" if c is None else str(c) for c in r) + " |")
    return "\n".join(out)


def research_timeline(research):
    """Return {research_id: (depth, critical_path_points)} using AND-prerequisites."""
    ids = set(research)
    depth, cp = {}, {}

    def go(rid, stack=()):
        if rid in depth:
            return
        if rid in stack:
            depth[rid], cp[rid] = 0, 0
            return
        ps = [p for p in research[rid].get("requiredResearch", []) if p in ids]
        for p in ps:
            go(p, stack + (rid,))
        depth[rid] = 1 + max((depth[p] for p in ps), default=-1)
        cp[rid] = research[rid].get("researchPoints", 0) + max((cp[p] for p in ps), default=0)

    for r in ids:
        go(r)
    return depth, cp


def unlockers(research):
    out = {}
    for rid, r in research.items():
        for c in r.get("resultComponents", []):
            out[c] = rid
    return out


def main():
    root, section = sys.argv[1], sys.argv[2]
    rs = sys.argv[3] if len(sys.argv) > 3 else "mp"
    research = load(root, rs, "research.json")
    depth, cp = research_timeline(research)
    unl = unlockers(research)

    def when(cid):
        r = unl.get(cid)
        return (depth[r], cp[r]) if r else ("-", "-")

    if section == "bodies":
        bodies = load(root, rs, "body.json")
        rows = []
        for k, v in bodies.items():
            if not v.get("designable"):
                continue
            d, c = when(k)
            rows.append((v["size"], v["name"], v.get("weaponSlots", 1), v["hitpoints"], v["armourKinetic"], v["armourHeat"],
                         v["powerOutput"], v["weight"], round(v["powerOutput"] / v["weight"], 1), v["buildPower"], v["buildPoints"], d, c))
        order = {"LIGHT": 0, "MEDIUM": 1, "HEAVY": 2}
        rows.sort(key=lambda r: (order[r[0]], r[9]))
        print(table(["Size", "Body", "Slots", "HP", "Armour (kin)", "Armour (heat)", "Engine power", "Weight", "Power/weight", "Power cost", "Build pts", "Tech depth", "Tech crit-path pts"], rows))

    elif section == "propulsion":
        props = load(root, rs, "propulsion.json")
        ptypes = load(root, rs, "propulsiontype.json")
        rows = []
        for k, v in props.items():
            if not v.get("designable"):
                continue
            d, c = when(k)
            rows.append((v["name"], v["type"], v.get("speed"), v.get("weight"), v.get("hitpointPctOfBody"), v.get("buildPower"), v.get("buildPoints"),
                         ptypes[v["type"]]["multiplier"], ptypes[v["type"]]["flightName"], v.get("skidDeceleration", "-"), v.get("acceleration", "-"), v.get("deceleration", "-"), v.get("turnSpeed", "-"), v.get("spinSpeed", "-"), d, c))
        print(table(["Propulsion", "Type", "Max speed", "Weight", "HP % of body", "Power cost", "Build pts", "Type multiplier", "Layer", "Skid decel", "Accel", "Decel", "Turn speed", "Spin speed", "Tech depth", "Tech crit-path pts"], rows))
        print("\nNon-designable propulsion (cyborg legs, helicopters, scenery): " + ", ".join(k for k, v in props.items() if not v.get("designable")))

    elif section == "weapons":
        weapons = load(root, rs, "weapons.json")
        rows = []
        for k, v in weapons.items():
            if not v.get("designable"):
                continue
            d, c = when(k)
            rows.append((v.get("weaponSubClass"), v["name"], v.get("weaponEffect"), v.get("weaponClass"), v.get("movement"), v.get("damage"),
                         v.get("firePause"), v.get("reloadTime", ""), v.get("numRounds", ""),
                         v.get("shortRange"), v.get("longRange"), v.get("minRange", ""), v.get("shortHit"), v.get("longHit"),
                         v.get("radius", ""), v.get("radiusDamage", ""), v.get("periodicalDamage", ""),
                         v.get("weight"), v.get("buildPower"), v.get("buildPoints"), v.get("flags", ""), d))
        rows.sort(key=lambda r: (str(r[0]), r[18] or 0))
        print(table(["Subclass", "Weapon", "Effect", "Class", "Flight", "Damage", "Fire pause", "Reload", "Rounds", "Short rng", "Long rng", "Min rng", "Hit% short", "Hit% long", "Splash rad", "Splash dmg", "Burn dmg", "Weight", "Power cost", "Build pts", "Flags", "Tech depth"], rows))

    elif section == "turrets":
        for fname, label in (("sensor.json", "Sensors"), ("ecm.json", "ECM"), ("repair.json", "Repair"), ("construction.json", "Construction"), ("brain.json", "Brain / command")):
            data = load(root, rs, fname)
            print(f"### {label}\n")
            if fname == "ecm.json":
                print("*Not flagged `designable` in the multiplayer stats and not unlocked by any research there; listed for reference only.*\n")
            rows = []
            for k, v in data.items():
                if not v.get("designable") and not (fname == "ecm.json" and k.startswith("ECM")):
                    continue
                d, c = when(k)
                rows.append((v.get("name"), v.get("type", v.get("droidType", "")), v.get("range", ""), v.get("hitpoints", ""), v.get("weight", ""), v.get("buildPower", ""), v.get("buildPoints", ""),
                             v.get("repairPoints", v.get("constructPoints", "")), d))
            print(table(["Name", "Type", "Range / note", "HP", "Weight", "Power cost", "Build pts", "Repair/construct pts", "Tech depth"], rows))
            print()

    elif section == "modifiers":
        wm = load(root, rs, "weaponmodifier.json")
        sm = load(root, rs, "structuremodifier.json")
        ptypes = sorted(next(iter(wm.values())))
        print("### Weapon effect vs target propulsion (percent of base damage)\n")
        print(table(["Weapon effect"] + ptypes, [[e] + [wm[e][p] for p in ptypes] for e in sorted(wm)]))
        print("\n### Weapon effect vs structure strength (percent of base damage)\n")
        strengths = ["SOFT", "MEDIUM", "HARD", "BUNKER"]
        print(table(["Weapon effect"] + strengths, [[e] + [sm[e][s] for s in strengths] for e in sorted(sm)]))

    elif section == "terrain":
        tt = load(root, "base", "terraintable.json")
        types = list(next(iter(tt.values()))["speedFactor"])
        print("### Speed factor by propulsion type and terrain (percent)\n")
        print(table(["Terrain"] + types, [[v["comment"]] + [v["speedFactor"][t] for t in types] for v in sorted(tt.values(), key=lambda x: x["id"])]))
        print("\nAverage across terrain types:")
        print(table(["Type", "Mean", "Min", "Max"], [[t, f"{sum(v['speedFactor'][t] for v in tt.values()) / len(tt):.0f}", min(v['speedFactor'][t] for v in tt.values()), max(v['speedFactor'][t] for v in tt.values())] for t in types]))

    elif section == "timeline":
        rows = []
        for cid, rid in unl.items():
            for fname, kind in (("body.json", "Body"), ("propulsion.json", "Propulsion"), ("weapons.json", "Weapon"), ("sensor.json", "Sensor"), ("repair.json", "Repair"), ("ecm.json", "ECM"), ("construction.json", "Construct"), ("brain.json", "Brain")):
                data = load(root, rs, fname)
                if cid in data and data[cid].get("designable"):
                    rows.append((cp[rid], depth[rid], kind, data[cid].get("name", cid), research[rid]["researchPoints"]))
        rows.sort()
        print(table(["Crit-path research pts", "Depth", "Kind", "Component", "Own research pts"], rows))

    elif section == "dominance":
        bodies = {k: v for k, v in load(root, rs, "body.json").items() if v.get("designable")}
        better_high = ["hitpoints", "armourKinetic", "armourHeat", "powerOutput"]
        better_low = ["weight", "buildPower", "buildPoints"]

        def dominates(a, b):
            ge = all(a[k] >= b[k] for k in better_high) and all(a[k] <= b[k] for k in better_low)
            strict = any(a[k] > b[k] for k in better_high) or any(a[k] < b[k] for k in better_low)
            return ge and strict

        print("### Strict Pareto dominance among designable bodies\n")
        print("A body is *dominated* if another body is at least as good on HP, both armours and engine power AND at least as light and cheap (power + build points), and strictly better somewhere.\n")
        found = False
        for kb, b in bodies.items():
            ds = [a["name"] for ka, a in bodies.items() if ka != kb and dominates(a, b)]
            if ds:
                found = True
                print(f"- **{b['name']}** is dominated by: {', '.join(ds)}")
        if not found:
            print("No body is strictly dominated once cost and weight are counted: every body trades something for something.")
        print("\n### Dominance ignoring cost and weight (raw capability only)\n")
        bh = better_high

        def dom2(a, b):
            return all(a[k] >= b[k] for k in bh) and any(a[k] > b[k] for k in bh)

        for kb, b in bodies.items():
            ds = [a["name"] for ka, a in bodies.items() if ka != kb and dom2(a, b)]
            print(f"- {b['name']}: beaten on every raw stat by {len(ds)} bodies" + (f" ({', '.join(ds)})" if ds else ""))

    elif section == "structures":
        st = load(root, rs, "structure.json")
        keep = ("FACTORY", "CYBORG FACTORY", "VTOL FACTORY", "RESEARCH", "POWER GENERATOR", "RESOURCE EXTRACTOR", "HQ", "REPAIR FACILITY",
                "FACTORY MODULE", "RESEARCH MODULE", "POWER MODULE", "REARM PAD", "COMMAND RELAY", "SAT UPLINK")
        rows = []
        for k, v in st.items():
            if v.get("type") in keep and not k.startswith(("A0BaBa", "ScavRepair")):
                rows.append((v["type"], v["name"], v.get("buildPower"), v.get("buildPoints"), v.get("hitpoints"), v.get("productionPoints", ""), v.get("moduleProductionPoints", ""), v.get("researchPoints", ""), v.get("moduleResearchPoints", ""), v.get("powerPoints", ""), v.get("modulePowerPoints", ""), v.get("rearmPoints", ""), v.get("repairPoints", ""), v.get("userLimits", "")))
        rows.sort()
        print(table(["Type", "Name", "Power cost", "Build pts", "HP", "Production pts", "Module prod pts", "Research pts", "Module research pts", "Power pts", "Module power pts", "Rearm pts", "Repair pts", "User limits"], rows))
    else:
        sys.exit("unknown section " + section)


if __name__ == "__main__":
    main()
