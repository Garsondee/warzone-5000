"""PNG of the four weathering ramps on a NATO green panel, standard library only. python3 -I assets/reference/ramps_preview.py OUT.png
Each tile is 0.8 m square at 5 mm per pixel; the ramp runs left to right, noise on (rows differ only by node-local position)."""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import w5k_look as L  # noqa: E402
from preview import png  # noqa: E402

ROOT = os.path.join(HERE, "..", "materials")


def main(out):
    W = json.load(open(os.path.join(ROOT, "baked", "weathering.json")))
    s = L.load_scheme(os.path.join(ROOT, "camo", "baked", "nato_three_tone.json"))
    cdf = L.load_cdf(os.path.join(ROOT, "camo", "cdf.json"), s)
    n, px = 160, 0.005
    rows = [[] for _ in range(n)]
    for k, label in enumerate(("edge", "cavity", "splash", "dust")):
        for j in range(n):
            for i in range(n):
                t = i / (n - 1)
                a = dict(edge=0.0, cavity=0.0, h=10.0, up=0.0)
                if label == "edge": a["edge"] = t
                elif label == "cavity": a["cavity"] = t
                elif label == "splash": a["h"] = 0.6 * (1 - t)
                else: a["up"] = t
                p = (k * 1.0 + i * px, j * px, 0.3)
                c = L.weathered(W, L.colour_linear(s, cdf, p, 3, px), W["bare_linear"], p, a["edge"], a["cavity"], a["h"], a["up"], 3, px)
                rows[j] += list(L.linear_to_srgb8(c))
            rows[j] += [255, 255, 255]
    png(out, 4 * (n + 1), n, rows)


if __name__ == "__main__":
    main(sys.argv[1])
