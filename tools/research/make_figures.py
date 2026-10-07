#!/usr/bin/env python3
"""Draw the research figures as self-contained SVG files (no dependencies).

Usage:
    python3 -I tools/research/make_figures.py <wz_checkout> <output_dir>

Figures (all recomputed from the Warzone 2100 stat files):
    wz-speed-vs-turret-weight.svg   how speed falls with turret weight (cap plateau, 1/x tail, x1.5 cliff)
    wz-cap-binding-dumbbell.svg     share of designs whose speed cap hides the engine, before/after engine research
    wz-research-tree-shape.svg      research items per prerequisite depth, split by what they do

Colour: the validated categorical order (blue, orange, aqua, yellow) and a one-hue two-step ramp, on the light chart
surface. Every figure carries its own surface, so it reads the same on light and dark pages.
The checkout is untrusted data: this script only parses JSON.
"""
import json
import os
import sys
from xml.sax.saxutils import escape

SURF, INK1, INK2, MUTED, GRID, AXIS = "#fcfcfb", "#0b0b0b", "#52514e", "#898781", "#e1e0d9", "#c3c2b7"
SERIES = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100"]          # categorical slots 1-4
RAMP_LIGHT, RAMP_DARK = "#86b6ef", "#184f95"                    # blue ramp steps 250 and 600
FONT = 'system-ui, -apple-system, "Segoe UI", Helvetica, Arial, sans-serif'
ENGINE_STEPS = [5, 5, 5, 5, 5, 5, 5, 7, 8]


def load(root, ruleset, name):
    with open(os.path.join(root, "data", ruleset, "stats", name), encoding="utf-8") as f:
        return json.load(f)


class Svg:
    def __init__(self, w, h, title, desc):
        self.w, self.h = w, h
        self.parts = [
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}" role="img" aria-labelledby="t d" font-family=\'{FONT}\'>',
            f"<title id=\"t\">{escape(title)}</title><desc id=\"d\">{escape(desc)}</desc>",
            f'<rect x="0.5" y="0.5" width="{w - 1}" height="{h - 1}" rx="6" fill="{SURF}" stroke="{GRID}"/>',
        ]

    def add(self, s):
        self.parts.append(s)

    def text(self, x, y, s, size=12, fill=INK2, anchor="start", weight="400", baseline=None):
        b = f' dominant-baseline="{baseline}"' if baseline else ""
        self.add(f'<text x="{x:.1f}" y="{y:.1f}" font-size="{size}" fill="{fill}" text-anchor="{anchor}" font-weight="{weight}"{b}>{escape(str(s))}</text>')

    def line(self, x1, y1, x2, y2, stroke=GRID, width=1, cap="butt"):
        self.add(f'<line x1="{x1:.1f}" y1="{y1:.1f}" x2="{x2:.1f}" y2="{y2:.1f}" stroke="{stroke}" stroke-width="{width}" stroke-linecap="{cap}"/>')

    def dot(self, x, y, color, r=4):
        # 2px surface ring keeps the mark legible where it crosses a line
        self.add(f'<circle cx="{x:.1f}" cy="{y:.1f}" r="{r}" fill="{color}" stroke="{SURF}" stroke-width="2"/>')

    def header(self, title, subtitle):
        self.text(24, 30, title, 16, INK1, weight="600")
        self.text(24, 50, subtitle, 12, INK2)

    def done(self):
        return "\n".join(self.parts + ["</svg>"]) + "\n"


# --------------------------------------------------------------------------------------------------
def figure_speed(root):
    bodies, props = load(root, "mp", "body.json"), load(root, "mp", "propulsion.json")
    ptypes = load(root, "mp", "propulsiontype.json")
    cobra = bodies["Body5REC"]
    rows = [("Wheels", "wheeled01"), ("Half-tracks", "HalfTrack"), ("Tracks", "tracked01"), ("Hover", "hover01")]

    def speed(prop, t):
        w = cobra["weight"] * (100 + prop["weight"]) // 100 + t
        base = ptypes[prop["type"]]["multiplier"] * cobra["powerOutput"] // w
        if cobra["powerOutput"] > w:
            base = base * 3 // 2
        return min(base, prop["speed"])

    W, H = 880, 540
    left, right, top, bottom = 72, W - 30, 118, H - 112
    xmax, ymax = 12000, 320

    def X(t):
        return left + t * (right - left) / xmax

    def Y(v):
        return bottom - v * (bottom - top) / ymax

    s = Svg(W, H, "Speed versus turret weight for a Cobra body on four propulsions",
            "Line chart. Speed is flat at each propulsion's cap for light turrets, then falls as turret weight grows. "
            "Each line also drops by a third at the weight where total weight reaches the body's engine power: "
            "5,000 for half-tracks, 7,000 for wheels and 9,000 for hover. Tracks never get the bonus. "
            "The numbers are in docs/research/warzone-2100/data/speed-table.md.")
    s.header("Speed versus turret weight: a cap, a cliff, then a 1/x tail",
             "Cobra body, no engine research, 100% terrain factor. Tracks never reach their cap of 125. Weapon weights run from 200 to 30,000.")
    # legend (always present for 2+ series)
    lx = 24
    for (name, _), color in zip(rows, SERIES):
        s.line(lx, 76, lx + 20, 76, color, 2, "round")
        s.text(lx + 26, 80, name, 12, INK2)
        lx += 26 + 6.4 * len(name) + 30
    # grid + y axis
    for v in (0, 50, 100, 150, 200, 250, 300):
        y = Y(v)
        s.line(left, y, right, y, GRID if v else AXIS, 1)
        s.text(left - 8, y + 4, f"{v}", 11, MUTED, "end")
    s.text(left - 8, top - 14, "Speed (world units/s)", 11, MUTED, "start")
    for t in range(0, xmax + 1, 2000):
        s.line(X(t), bottom, X(t), bottom + 4, AXIS, 1)
        s.text(X(t), bottom + 18, f"{t:,}", 11, MUTED, "middle")
    s.text((left + right) / 2, bottom + 36, "Turret weight", 12, INK2, "middle")
    # weapon markers (second row under the axis)
    marks = [("Light Cannon", 1000), ("Medium Cannon", 5000), ("Heavy Cannon", 8000), ("Howitzer", 10000)]
    for name, wt in marks:
        s.line(X(wt), bottom, X(wt), bottom + 52, GRID, 1)
        s.text(X(wt) + 4, bottom + 62, f"{name} {wt:,}", 11, INK2)
    # lines
    for (name, pid), color in zip(rows, SERIES):
        p = props[pid]
        cliff = None
        w0 = cobra["weight"] * (100 + p["weight"]) // 100
        if cobra["powerOutput"] > w0:
            cliff = cobra["powerOutput"] - w0       # first turret weight where the x1.5 bonus is lost
        pts = sorted(set(list(range(0, xmax + 1, 25)) + ([cliff - 1, cliff] if cliff and cliff <= xmax else [])))
        d = "M " + " L ".join(f"{X(t):.1f} {Y(speed(p, t)):.1f}" for t in pts)
        s.add(f'<path d="{d}" fill="none" stroke="{color}" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"/>')
    # direct labels on the plateaus (a colour dot carries identity; the text stays in ink)
    labels = [("Hover", 3, 300, "above"), ("Wheels", 0, 175, "above"), ("Half-tracks", 1, 150, "above"), ("Tracks", 2, 80, "above")]
    for name, idx, cap, _ in labels:
        t = 150
        y = Y(speed(props[rows[idx][1]], t))
        s.dot(X(t) + 4, y - 12, SERIES[idx], 4)
        s.text(X(t) + 14, y - 8, f"{name} (cap {props[rows[idx][1]]['speed']})", 11.5, INK1)
    # one cliff annotation
    p = props["HalfTrack"]
    cl = cobra["powerOutput"] - cobra["weight"] * (100 + p["weight"]) // 100
    s.text(X(cl) - 8, Y(100) + 4, f"x1.5 bonus lost: {speed(p, cl - 1)} to {speed(p, cl)}", 11, INK2, "end")
    return s.done()


# --------------------------------------------------------------------------------------------------
def figure_dumbbell(root):
    bodies = {k: v for k, v in load(root, "mp", "body.json").items() if v.get("designable")}
    props = {k: v for k, v in load(root, "mp", "propulsion.json").items() if v.get("designable") and k != "Naval"}
    ptypes = load(root, "mp", "propulsiontype.json")
    weapons = {k: v for k, v in load(root, "mp", "weapons.json").items() if v.get("designable") and not v.get("numAttackRuns")}

    def power(p0, steps):
        return p0 + sum(-(-p0 * s_ // 100) for s_ in steps)

    def capped(b, p, w, steps):
        wt = b["weight"] * (100 + p["weight"]) // 100 + w["weight"]
        base = ptypes[p["type"]]["multiplier"] * power(b["powerOutput"], steps) // wt
        if b["powerOutput"] > wt:
            base = base * 3 // 2
        return base >= p["speed"]

    data = []
    for pid, p in props.items():
        if p["type"] == "Lift":
            continue
        n = len(bodies) * len(weapons)
        a = sum(capped(b, p, w, ()) for b in bodies.values() for w in weapons.values())
        z = sum(capped(b, p, w, ENGINE_STEPS) for b in bodies.values() for w in weapons.values())
        data.append((p["name"], 100.0 * a / n, 100.0 * z / n))
    data.sort(key=lambda r: -r[2])

    W, H = 880, 120 + 60 * len(data) + 70
    left, right, top = 150, W - 60, 118
    s = Svg(W, H, "Share of ground designs whose speed cap hides the engine, by propulsion",
            "Dumbbell chart. For each propulsion, a light dot shows the share of designs already at the speed cap with no engine research "
            "and a dark dot shows the share after all nine engine upgrades. " +
            "; ".join(f"{n}: {a:.0f}% to {z:.0f}%" for n, a, z in data) + ".")
    s.header("Engine research mostly lifts designs into the speed cap",
             "Share of ground designs (every body x weapon) at the propulsion's speed cap on flat ground, before and after all nine engine upgrades.")
    s.dot(24 + 5, 76, RAMP_LIGHT, 5)
    s.text(24 + 18, 80, "No engine research", 12, INK2)
    s.dot(24 + 170, 76, RAMP_DARK, 5)
    s.text(24 + 183, 80, "All nine engine upgrades", 12, INK2)

    def X(v):
        return left + v * (right - left) / 100.0

    bottom = top + 60 * len(data) - 20
    for v in range(0, 101, 20):
        s.line(X(v), top - 12, X(v), bottom + 10, GRID if v else AXIS, 1)
        s.text(X(v), bottom + 28, f"{v}%", 11, MUTED, "middle")
    for i, (name, a, z) in enumerate(data):
        y = top + 60 * i + 12
        s.text(left - 14, y + 4, name, 12.5, INK1, "end")
        s.line(X(a), y, X(z), y, AXIS, 3, "round")
        s.dot(X(a), y, RAMP_LIGHT, 6)
        s.dot(X(z), y, RAMP_DARK, 6)
        s.text(X(a) - 12, y + 4, f"{a:.0f}%", 11.5, INK2, "end")
        s.text(X(z) + 12, y + 4, f"{z:.0f}%", 11.5, INK2, "start")
    return s.done()


# --------------------------------------------------------------------------------------------------
def figure_tree(root):
    research = load(root, "mp", "research.json")
    ids = set(research)
    depth = {}

    def go(r, stack=()):
        if r in depth:
            return depth[r]
        ps = [p for p in research[r].get("requiredResearch", []) if p in ids and p not in stack]
        depth[r] = 1 + max((go(p, stack + (r,)) for p in ps), default=-1)
        return depth[r]

    for r in ids:
        go(r)
    cats = ["Stat upgrade only", "Unlocks a component", "Unlocks structures only"]

    def kind(r):
        if r.get("resultComponents"):
            return 1
        if r.get("resultStructures"):
            return 2
        return 0

    counts = {}
    for rid, r in research.items():
        counts.setdefault(depth[rid], [0, 0, 0])[kind(r)] += 1
    maxd = max(counts)
    totals = {d: sum(v) for d, v in counts.items()}
    peak_d = max(totals, key=totals.get)

    W, H = 880, 460
    left, right, top, bottom = 56, W - 28, 112, H - 70
    ymax = 40
    slot = (right - left) / (maxd + 1)
    bw = min(24, slot - 6)

    def Y(v):
        return bottom - v * (bottom - top) / ymax

    s = Svg(W, H, "Research items per prerequisite depth, by what they do",
            "Stacked column chart of the 390 multiplayer research items by depth in the prerequisite graph. "
            "The tree is narrow at the start, widest at depth 12 with 40 items, and narrow again at the end. "
            "About half of all items are stat upgrades. The table is in docs/research/warzone-2100/research-system/03-tree-analysis.md.")
    s.header("The research tree is spindle-shaped, and half of it is stat upgrades",
             "Warzone 2100 multiplayer tree: 390 items by prerequisite depth (0 = no prerequisites).")
    lx = 24
    for name, color in zip(cats, SERIES):
        s.add(f'<rect x="{lx}" y="70" width="12" height="12" rx="2" fill="{color}"/>')
        s.text(lx + 18, 80, name, 12, INK2)
        lx += 18 + 6.4 * len(name) + 30
    for v in range(0, ymax + 1, 10):
        y = Y(v)
        s.line(left, y, right, y, GRID if v else AXIS, 1)
        s.text(left - 8, y + 4, f"{v}", 11, MUTED, "end")
    s.text(left - 8, top - 8, "Research items", 11, MUTED, "start")
    s.text((left + right) / 2, bottom + 46, "Prerequisite depth (longest chain of required topics before this one)", 12, INK2, "middle")
    for d in range(maxd + 1):
        cx = left + slot * (d + 0.5)
        s.text(cx, bottom + 18, d, 11, MUTED, "middle")
        y_cursor = bottom
        segs = [(i, c) for i, c in enumerate(counts.get(d, [0, 0, 0])) if c]
        for n, (i, c) in enumerate(segs):
            h = c * (bottom - top) / ymax
            y1 = y_cursor - h
            gap = 2 if n else 0                       # 2px surface gap between stacked segments
            y_draw_bottom = y_cursor - gap
            hh = max(y_draw_bottom - y1, 1)
            x0, x1 = cx - bw / 2, cx + bw / 2
            if n == len(segs) - 1:                     # data end: 4px rounded top, square baseline side
                r = min(4, hh / 2)
                path = (f"M {x0:.1f} {y_draw_bottom:.1f} L {x0:.1f} {y1 + r:.1f} Q {x0:.1f} {y1:.1f} {x0 + r:.1f} {y1:.1f} "
                        f"L {x1 - r:.1f} {y1:.1f} Q {x1:.1f} {y1:.1f} {x1:.1f} {y1 + r:.1f} L {x1:.1f} {y_draw_bottom:.1f} Z")
                s.add(f'<path d="{path}" fill="{SERIES[i]}"/>')
            else:
                s.add(f'<rect x="{x0:.1f}" y="{y1:.1f}" width="{bw:.1f}" height="{hh:.1f}" fill="{SERIES[i]}"/>')
            y_cursor = y1
    # one selective label: the peak
    cx = left + slot * (peak_d + 0.5)
    s.text(cx, Y(totals[peak_d]) - 8, f"{totals[peak_d]} items at depth {peak_d}", 11.5, INK1, "middle")
    return s.done()


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    root, out = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    for name, fn in (("wz-speed-vs-turret-weight.svg", figure_speed), ("wz-cap-binding-dumbbell.svg", figure_dumbbell), ("wz-research-tree-shape.svg", figure_tree)):
        with open(os.path.join(out, name), "w", encoding="utf-8", newline="\n") as f:
            f.write(fn(root))
        print("wrote", name)


if __name__ == "__main__":
    main()
