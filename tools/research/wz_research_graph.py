#!/usr/bin/env python3
"""Analyse the Warzone 2100 research tree (statistics only, no game code reused).

Usage:
    python3 -I wz_research_graph.py <wz_checkout> [mp|base]

Reads <wz_checkout>/data/<mp|base>/stats/research.json (plus the component stat
files, to know which unlocked items are actually designable) and prints a
Markdown report to stdout.

The wz checkout is untrusted data: this script only parses JSON from it.
"""
import json
import os
import sys
from collections import Counter, defaultdict
from statistics import mean, median


def load(root, ruleset, name):
    path = os.path.join(root, "data", ruleset, "stats", name)
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def pct(n, d):
    return f"{100.0 * n / d:.0f}%" if d else "n/a"


def main():
    root = sys.argv[1]
    ruleset = sys.argv[2] if len(sys.argv) > 2 else "mp"
    research = load(root, ruleset, "research.json")

    # --- component universe -------------------------------------------------
    designable = {}
    kinds = {"body.json": "Body", "propulsion.json": "Propulsion", "weapons.json": "Weapon",
             "sensor.json": "Sensor", "ecm.json": "ECM", "repair.json": "Repair",
             "construction.json": "Construct", "brain.json": "Brain"}
    for fname, kind in kinds.items():
        try:
            for cid, c in load(root, ruleset, fname).items():
                designable[cid] = (kind, bool(c.get("designable")))
        except FileNotFoundError:
            pass
    try:
        structures = load(root, ruleset, "structure.json")
    except FileNotFoundError:
        structures = {}

    ids = set(research)
    # --- graph (requiredResearch is an AND list) ----------------------------
    prereq = {rid: [p for p in r.get("requiredResearch", [])] for rid, r in research.items()}
    missing = {p for ps in prereq.values() for p in ps if p not in ids}
    dependents = defaultdict(list)
    for rid, ps in prereq.items():
        for p in ps:
            dependents[p].append(rid)

    depth = {}

    def get_depth(rid, stack=()):
        if rid in depth:
            return depth[rid]
        if rid in stack:
            return 0  # cycle guard (should not happen)
        ds = [get_depth(p, stack + (rid,)) for p in prereq[rid] if p in ids]
        depth[rid] = 1 + max(ds) if ds else 0
        return depth[rid]

    for rid in ids:
        get_depth(rid)

    # critical path (by research points) to each node
    cp = {}

    def get_cp(rid, stack=()):
        if rid in cp:
            return cp[rid]
        if rid in stack:
            return 0
        best = max((get_cp(p, stack + (rid,)) for p in prereq[rid] if p in ids), default=0)
        cp[rid] = best + research[rid].get("researchPoints", 0)
        return cp[rid]

    for rid in ids:
        get_cp(rid)

    # transitive closure sizes (how much of the tree a node gates)
    desc_cache = {}

    def descendants(rid):
        if rid in desc_cache:
            return desc_cache[rid]
        out = set()
        for d in dependents.get(rid, []):
            out.add(d)
            out |= descendants(d)
        desc_cache[rid] = out
        return out

    # --- classify what each item does ---------------------------------------
    def classify(r):
        comps = r.get("resultComponents", [])
        strs = r.get("resultStructures", [])
        res = r.get("results", [])
        tags = []
        if comps:
            tags.append("component")
        if strs:
            tags.append("structure")
        if res:
            tags.append("upgrade")
        return tags or ["none"]

    kind_counter = Counter()
    for r in research.values():
        kind_counter["+".join(classify(r))] += 1

    out = []
    p = out.append
    p(f"# Research tree analysis ({ruleset})\n")
    p(f"- Research items: **{len(research)}**")
    p(f"- Dangling prerequisite ids (not in file): {sorted(missing) if missing else 'none'}")
    roots = [rid for rid in ids if not [x for x in prereq[rid] if x in ids]]
    leaves = [rid for rid in ids if not dependents.get(rid)]
    p(f"- Root items (no prerequisites): {len(roots)}; leaf items (nothing depends on them): {len(leaves)}")
    maxd = max(depth.values())
    p(f"- Longest prerequisite chain: **{maxd + 1} items** (depth 0..{maxd})")
    indeg = [len([x for x in prereq[r] if x in ids]) for r in ids]
    outdeg = [len(dependents.get(r, [])) for r in ids]
    p(f"- Prerequisites per item: mean {mean(indeg):.2f}, max {max(indeg)}; items with >=2 prereqs: {sum(1 for x in indeg if x >= 2)} ({pct(sum(1 for x in indeg if x >= 2), len(ids))})")
    p(f"- Direct dependents per item: mean {mean(outdeg):.2f}, max {max(outdeg)}; items gating nothing: {sum(1 for x in outdeg if x == 0)}")
    p(f"- Items with `requiredStructures`: {sum(1 for r in research.values() if r.get('requiredStructures'))}; with `disabledWhen`: {sum(1 for r in research.values() if r.get('disabledWhen'))}")
    p(f"- Items that make components redundant (`redComponents`): {sum(1 for r in research.values() if r.get('redComponents'))}; structures redundant: {sum(1 for r in research.values() if r.get('redStructures'))}; `replacedComponents`: {sum(1 for r in research.values() if r.get('replacedComponents'))}")
    pts = [r.get("researchPoints", 0) for r in research.values()]
    pwr = [r.get("researchPower", 0) for r in research.values()]
    p(f"- Research points: total {sum(pts):,}, mean {mean(pts):.0f}, median {median(pts):.0f}, max {max(pts):,}")
    p(f"- Research power cost: total {sum(pwr):,}, mean {mean(pwr):.0f}, max {max(pwr)}")
    p(f"- Most expensive path (sum of researchPoints along the costliest chain): {max(cp.values()):,}\n")

    p("## What each research item does\n")
    p("| Effect | Items | Share |\n|---|---:|---:|")
    for k, v in kind_counter.most_common():
        p(f"| {k} | {v} | {pct(v, len(research))} |")
    p("")

    # --- categories ----------------------------------------------------------
    cat = defaultdict(list)
    for rid, r in research.items():
        cat[r.get("category", "(none)")].append(rid)
    p("## Categories\n")
    p("| Category | Items | Avg points | Avg depth | Deepest | Effect mix |\n|---|---:|---:|---:|---:|---|")
    for c, rids in sorted(cat.items(), key=lambda kv: -len(kv[1])):
        mix = Counter("+".join(classify(research[r])) for r in rids)
        mixs = ", ".join(f"{k}:{v}" for k, v in mix.most_common())
        p(f"| {c} | {len(rids)} | {mean(research[r].get('researchPoints', 0) for r in rids):.0f} | {mean(depth[r] for r in rids):.1f} | {max(depth[r] for r in rids)} | {mixs} |")
    p("")

    # --- depth profile ---------------------------------------------------------
    p("## Depth profile (items per prerequisite-depth)\n")
    byd = defaultdict(list)
    for rid in ids:
        byd[depth[rid]].append(rid)
    p("| Depth | Items | Avg points | Of which unlock components | Of which unlock structures | Of which upgrades |\n|---:|---:|---:|---:|---:|---:|")
    for d in sorted(byd):
        rs = [research[r] for r in byd[d]]
        p(f"| {d} | {len(rs)} | {mean(r.get('researchPoints', 0) for r in rs):.0f} | {sum(1 for r in rs if r.get('resultComponents'))} | {sum(1 for r in rs if r.get('resultStructures'))} | {sum(1 for r in rs if r.get('results'))} |")
    p("")

    # --- unlocked components --------------------------------------------------
    unlocked = defaultdict(list)
    for rid, r in research.items():
        for c in r.get("resultComponents", []):
            unlocked[c].append(rid)
    kc = Counter()
    for c in unlocked:
        kc[designable.get(c, ("Unknown", False))] += 1
    p("## Components unlocked by research\n")
    p("| Kind | Designable? | Count |\n|---|---|---:|")
    for (k, dsg), n in sorted(kc.items()):
        p(f"| {k} | {'yes' if dsg else 'no'} | {n} |")
    multi = {c: v for c, v in unlocked.items() if len(v) > 1}
    p(f"\nComponents unlocked by more than one research item: {len(multi)}\n")

    # --- upgrade engine usage ---------------------------------------------------
    up = Counter()
    for r in research.values():
        for u in r.get("results", []):
            up[(u.get("class"), u.get("parameter"))] += 1
    p("## Upgrade effects (`results`) by class and parameter\n")
    p("| Class | Parameter | Uses |\n|---|---|---:|")
    for (c, par), n in sorted(up.items(), key=lambda kv: (str(kv[0][0]), -kv[1])):
        p(f"| {c} | {par} | {n} |")
    p("")

    filt = Counter()
    for r in research.values():
        for u in r.get("results", []):
            if u.get("filterParameter"):
                filt[(u.get("class"), u.get("filterParameter"))] += 1
    p("Filter parameters used: " + ", ".join(f"{c}.{f} ({n})" for (c, f), n in filt.most_common()) + "\n")

    # --- upgrade chains -------------------------------------------------------
    chains = defaultdict(list)
    for rid, r in research.items():
        for u in r.get("results", []):
            key = (u.get("class"), u.get("parameter"), u.get("filterParameter"), str(u.get("filterValue")))
            chains[key].append((depth[rid], rid, u.get("value"), r.get("researchPoints", 0)))
    p("## Longest upgrade chains (same class/parameter/filter)\n")
    p("| Class | Parameter | Filter | Steps | Sum of values | Value range per step | Cumulative research points |\n|---|---|---|---:|---:|---|---:|")
    for key, steps in sorted(chains.items(), key=lambda kv: -len(kv[1]))[:25]:
        vals = [s[2] for s in steps if isinstance(s[2], (int, float))]
        p(f"| {key[0]} | {key[1]} | {key[3] if key[2] else '-'} | {len(steps)} | {sum(vals)} | {min(vals)}..{max(vals)} | {sum(s[3] for s in steps):,} |")
    p("")

    # --- bottlenecks ----------------------------------------------------------
    p("## Biggest gatekeepers (items whose completion unlocks the most of the tree)\n")
    p("| Item | Name | Transitive dependents | Depth |\n|---|---|---:|---:|")
    gk = sorted(ids, key=lambda r: -len(descendants(r)))[:15]
    for rid in gk:
        p(f"| {rid} | {research[rid].get('name', '')} | {len(descendants(rid))} | {depth[rid]} |")
    p("")

    # --- structures unlocked ----------------------------------------------------
    sk = Counter()
    for rid, r in research.items():
        for s in r.get("resultStructures", []):
            sk[structures.get(s, {}).get("type", "?")] += 1
    p("## Structures unlocked by research (by structure type)\n")
    p("| Type | Count |\n|---|---:|")
    for k, n in sk.most_common():
        p(f"| {k} | {n} |")
    p("")

    # --- engine-related items ---------------------------------------------------
    p("## Items whose results touch body `Power` (engine output)\n")
    p("| Item | Name | Depth | Points | Power cost | Requires | Value(s) |\n|---|---|---:|---:|---:|---|---|")
    for rid, r in sorted(research.items(), key=lambda kv: depth[kv[0]]):
        pw = [u for u in r.get("results", []) if u.get("parameter") == "Power" and u.get("class") == "Body"]
        if pw:
            p(f"| {rid} | {r.get('name')} | {depth[rid]} | {r.get('researchPoints')} | {r.get('researchPower')} | {', '.join(r.get('requiredResearch', []))} | {', '.join(str(u.get('value')) + '% ' + str(u.get('filterValue')) for u in pw)} |")
    p("")

    # --- cross-family coupling ------------------------------------------------
    def family(rid):
        parts = rid.split("-")
        if len(parts) > 2 and parts[1] in ("Vehicle", "Struc", "Sys", "Cyborg"):
            return "-".join(parts[:3]).rstrip("0123456789")
        if len(parts) > 2 and parts[1] == "Wpn":
            return "R-Wpn-" + "".join(ch for ch in parts[2] if not ch.isdigit() and ch != "_")[:12]
        return "-".join(parts[:2])

    edges = Counter()
    for rid, ps in prereq.items():
        for q in ps:
            if q in ids and family(q) != family(rid):
                edges[(family(q), family(rid))] += 1
    p("## Cross-family prerequisite edges (family of prerequisite -> family of dependent)\n")
    p("Shows which research lines gate which other lines. Edges within one family (upgrade chains) are excluded.\n")
    p("| Prerequisite family | Dependent family | Edges |\n|---|---|---:|")
    for (a, b), n in edges.most_common(30):
        p(f"| {a} | {b} | {n} |")
    p("")
    body_unlocks = [(rid, r) for rid, r in research.items() if any(designable.get(c, ("", False))[0] == "Body" and designable.get(c, ("", False))[1] for c in r.get("resultComponents", []))]
    p("## Which research lines gate each designable body\n")
    p("| Body research | Requires |\n|---|---|")
    for rid, r in sorted(body_unlocks, key=lambda kv: depth[kv[0]]):
        p(f"| {r.get('name')} | {', '.join(research[x].get('name', x) for x in r.get('requiredResearch', []))} |")
    p("")

    # --- dump of tree in tier order, compact ---------------------------------
    if "--list" in sys.argv:
        p("## Full list in depth order\n")
        p("| Depth | Id | Name | Category | Points | Power | Requires | Effect |\n|---:|---|---|---|---:|---:|---|---|")
        for rid in sorted(ids, key=lambda r: (depth[r], r)):
            r = research[rid]
            p(f"| {depth[rid]} | {rid} | {r.get('name')} | {r.get('category', '')} | {r.get('researchPoints')} | {r.get('researchPower')} | {', '.join(r.get('requiredResearch', []))} | {'+'.join(classify(r))} |")

    print("\n".join(out))


if __name__ == "__main__":
    main()
