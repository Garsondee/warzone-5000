"""Coverage of colour 0 over many blobs: solid 3D vs triplanar (LUT built for the solid field), random positions and normals."""
import sys, json, random, math; sys.path.insert(0, "."); import ref
import importlib.util, types
src = open("tri.py").read().split("def tone")[0].replace("from PIL import Image", "")
exec(src)
P = json.load(open("params.json")); cdf = P["cdf"]; r = random.Random(5); n = 6000
for name in ("solid", "tri"):
    c = 0
    for _ in range(n):
        p = [r.uniform(-60, 60) for _ in range(3)]; v = [r.gauss(0, 1) for _ in range(3)]; l = math.sqrt(sum(x * x for x in v)); v = [x / l for x in v]
        f = f_solid(p, 42) if name == "solid" else f_tri(p, 42, v)
        c += ref.cdf_lookup(cdf, f) < 0.10
    print(name, "tail 10% coverage", c / n, "(spec 0.10, 1-sigma sampling error about", round(math.sqrt(.10 * .90 / n), 3), ")")
