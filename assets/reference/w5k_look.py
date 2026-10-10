"""LOOK reference implementation (the one specification both engines are checked against). Standard library only.

Pattern code uses only + - * / floor and 32-bit integer ops: no libm, so the bits are portable (test L1). sRGB conversion
(`linear_to_srgb8`, `delta_e76`) lives at the edge and may use pow/cbrt-like maths; it never feeds back into the pattern.
Spec: docs/art/CAMO-ALGORITHM.md.
"""
import json
import random

M = 0xFFFFFFFF
OFFSET = 32768  # lattice ints are shifted positive before the unsigned cast (a negative int to uint cast is not portable in GLSL ES)
CDF_CELLS = 64  # const-ok: LUT resolution, mirrored in the shaders (uCdf[65])


def floor(x):
    i = int(x)
    return i - 1 if x < i else i


def hash3(ix, iy, iz, seed):
    h = (seed ^ ((ix * 0x8DA6B343) & M) ^ ((iy * 0xD8163841) & M) ^ ((iz * 0xCB1AB31F) & M)) & M
    h ^= h >> 16
    h = (h * 0x7FEB352D) & M
    h ^= h >> 15
    h = (h * 0x846CA68B) & M
    h ^= h >> 16
    return h


def lattice(ix, iy, iz, seed):
    return (hash3(ix + OFFSET, iy + OFFSET, iz + OFFSET, seed) >> 8) * (1.0 / 16777216.0)


def vnoise(x, y, z, seed):
    """3D value noise in 0..1, quintic fade."""
    ix, iy, iz = floor(x), floor(y), floor(z)
    fx, fy, fz = x - ix, y - iy, z - iz
    fx, fy, fz = (t * t * t * (t * (t * 6.0 - 15.0) + 10.0) for t in (fx, fy, fz))

    def corner(a, b, c):
        return lattice(ix + a, iy + b, iz + c, seed)

    def mix(a, b, t):
        return a + (b - a) * t
    return mix(mix(mix(corner(0, 0, 0), corner(1, 0, 0), fx), mix(corner(0, 1, 0), corner(1, 1, 0), fx), fy),
               mix(mix(corner(0, 0, 1), corner(1, 0, 1), fx), mix(corner(0, 1, 1), corner(1, 1, 1), fx), fy), fz)


def fbm(x, y, z, seed, octaves=3, lacunarity=2.0, gain=0.5, footprint=0.0):
    """Band-limited fractal noise in 0..1: an octave whose wavelength is under 2 footprints (lattice units) fades to its mean 0.5."""
    f, a, s, tot = 1.0, 1.0, 0.0, 0.0
    for o in range(octaves):
        w = 1.0 - min(max((footprint * f * 2.0 - 0.5) / 0.5, 0.0), 1.0)
        n = vnoise(x * f + 17.3 * o, y * f - 9.1 * o, z * f + 4.7 * o, (seed + o * 0x9E3779B9) & M)
        s += a * (0.5 + w * (n - 0.5))
        tot += a
        f *= lacunarity
        a *= gain
    return s / tot


def cdf_key(p):
    return "o%d_l%g_g%g" % (p["octaves"], p["lacunarity"], p["gain"])


def build_cdf(p, samples=100000):
    """Equalising table: CDF of the fbm field over a large volume, CDF_CELLS cells over 0..1 (65 knots). Seeded: deterministic."""
    r = random.Random(1234)
    hist = [0] * CDF_CELLS
    for _ in range(samples):
        v = fbm(r.uniform(-50, 50), r.uniform(-50, 50), r.uniform(-50, 50), 42, p["octaves"], p["lacunarity"], p["gain"])
        hist[min(int(v * CDF_CELLS), CDF_CELLS - 1)] += 1
    c, out = 0, [0.0]
    for h in hist:
        c += h
        out.append(c / samples)
    return out


def cdf_lookup(cdf, f):
    x = min(max(f, 0.0), 1.0) * (len(cdf) - 1)
    i = min(int(x), len(cdf) - 2)
    return cdf[i] + (cdf[i + 1] - cdf[i]) * (x - i)


def tone_index(scheme, cdf, p, seed, footprint=0.0):
    """Which colour of the scheme sits at node-local position p (metres). Depends on p, the seed and the scheme only."""
    pat, s = scheme["pattern"], scheme["scale_m"]
    q = (p[0] / s, p[1] / s, p[2] / s)
    base = (seed ^ scheme["seed_salt"]) & M
    left = 1.0
    cols = scheme["colours"]
    for i, c in enumerate(cols[:-1]):
        u = cdf_lookup(cdf, fbm(q[0], q[1], q[2], hash3(i + 1, 0, 0, base), pat["octaves"], pat["lacunarity"], pat["gain"], footprint / s))
        if u < c["coverage"] / left:
            return i
        left -= c["coverage"]
    return len(cols) - 1


def colour_linear(scheme, cdf, p, seed, footprint=0.0):
    return tuple(scheme["colours"][tone_index(scheme, cdf, p, seed, footprint)]["linear"])


def linear_to_srgb8(c):
    out = []
    for v in c:
        v = min(max(v, 0.0), 1.0)
        v = 12.92 * v if v <= 0.0031308 else 1.055 * v ** (1 / 2.4) - 0.055  # const-ok: sRGB OETF (IEC 61966-2-1)
        out.append(int(v * 255.0 + 0.5))
    return tuple(out)


def srgb_hex_to_linear(h):
    out = []
    for k in (1, 3, 5):
        v = int(h[k:k + 2], 16) / 255.0
        out.append(v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4)  # const-ok: sRGB EOTF (IEC 61966-2-1)
    return tuple(out)


def lab(c):
    """Linear sRGB -> CIELAB (D65). Edge maths for tests only."""
    r, g, b = c
    x, y, z = (0.4124564 * r + 0.3575761 * g + 0.1804375 * b) / 0.95047, 0.2126729 * r + 0.7151522 * g + 0.0721750 * b, (0.0193339 * r + 0.1191920 * g + 0.9503041 * b) / 1.08883

    def f(t):
        return t ** (1 / 3) if t > 216 / 24389 else (24389 / 27 * t + 16) / 116  # const-ok: CIE constants
    fx, fy, fz = f(x), f(y), f(z)
    return 116 * fy - 16, 500 * (fx - fy), 200 * (fy - fz)  # const-ok: CIE constants


def delta_e76(c1, c2):
    a, b = lab(c1), lab(c2)
    return sum((p - q) ** 2 for p, q in zip(a, b)) ** 0.5


def load_scheme(path):
    """A baked scheme (output of `w5k look bake`)."""
    with open(path) as fh:
        return json.load(fh)


def load_cdf(path, scheme):
    with open(path) as fh:
        return json.load(fh)[cdf_key(scheme["pattern"])]


if __name__ == "__main__":
    import sys
    if sys.argv[1:2] == ["cdf"]:  # python3 -I assets/reference/w5k_look.py cdf > assets/materials/camo/cdf.json
        pat = {"octaves": 3, "lacunarity": 2.0, "gain": 0.5}
        print(json.dumps({cdf_key(pat): build_cdf(pat)}, indent=1))
