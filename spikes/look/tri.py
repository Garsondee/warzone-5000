"""Solid 3D noise vs triplanar blend of 2D noise on a sphere: picture + coverage of colour 0 (spec 0.45)."""
import sys, json, math; sys.path.insert(0, "."); import ref
from PIL import Image
P = json.load(open("params.json")); cdf = P["cdf"]; S = ref.SCALE_M; N = 150; R = 1.5
def f_solid(p, seed): return ref.fbm(p[0] / S, p[1] / S, p[2] / S, seed)
def f_tri(p, seed, n):
    w = [abs(c) ** 4 for c in n]; t = sum(w); w = [x / t for x in w]
    return (w[0] * ref.fbm(p[1] / S, p[2] / S, 0.0, seed) + w[1] * ref.fbm(p[0] / S, p[2] / S, 1.7, seed) + w[2] * ref.fbm(p[0] / S, p[1] / S, 3.3, seed))
def tone(u1, u2):
    return 0 if u1 < ref.COV[0] else (1 if u2 < ref.COV[1] / (1 - ref.COV[0]) else 2)
if __name__ != "__main__": raise SystemExit
res = {}
for name in ("solid", "tri"):
    img = Image.new("RGB", (N, N), (30, 30, 30)); cnt = [0, 0, 0]; tot = 0
    for j in range(N):
        for i in range(N):
            x, y = (i + .5) / N * 2 - 1, 1 - (j + .5) / N * 2; r2 = x * x + y * y
            if r2 > 1: continue
            n = (x, y, math.sqrt(1 - r2)); p = tuple(c * R for c in n)
            if name == "solid": a, b = f_solid(p, 42), f_solid(p, 42 ^ 0x5BD1E995)
            else: a, b = f_tri(p, 42, n), f_tri(p, 42 ^ 0x5BD1E995, n)
            k = tone(ref.cdf_lookup(cdf, a), ref.cdf_lookup(cdf, b)); cnt[k] += 1; tot += 1
            img.putpixel((i, j), ref.srgb8(ref.PAL[k]))
    res[name] = [c / tot for c in cnt]; img.save(f"out/sphere_{name}.png")
print(json.dumps(res))
