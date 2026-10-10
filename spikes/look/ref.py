"""Spike S-L CPU reference: integer hash, value noise, fbm, quantile-equalised three-tone camo, edge wear and cavity dirt.
Only + - * / floor and 32-bit integer ops (no libm), so the bits are portable. Standard library only."""
M = 0xFFFFFFFF
OFFSET = 32768  # keeps lattice ints positive before the uint cast (a negative int to uint cast is not portable in GLSL ES)
OCT = 3
LAC = 2.0
GAIN = 0.5
# NATO three-tone, linear albedo (ESTIMATE, PROVISIONAL(C-004)): green, brown, black
PAL = [(0.055, 0.075, 0.030), (0.090, 0.058, 0.032), (0.012, 0.012, 0.011)]
COV = [0.45, 0.35, 0.20]
BARE = (0.20, 0.19, 0.18)   # primer / bare metal under chipped paint
DIRT = (0.060, 0.045, 0.030)
SCALE_M = 0.9


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


def fade(t):
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0)


def vnoise(x, y, z, seed):
    ix, iy, iz = floor(x), floor(y), floor(z)
    fx, fy, fz = fade(x - ix), fade(y - iy), fade(z - iz)
    def L(a, b, c): return lattice(ix + a, iy + b, iz + c, seed)
    def mix(a, b, t): return a + (b - a) * t
    return mix(mix(mix(L(0,0,0), L(1,0,0), fx), mix(L(0,1,0), L(1,1,0), fx), fy),
               mix(mix(L(0,0,1), L(1,0,1), fx), mix(L(0,1,1), L(1,1,1), fx), fy), fz)


def fbm(x, y, z, seed, footprint=0.0):
    """Band-limited: an octave whose wavelength is under 2 footprints fades to its mean 0.5. Result is in 0..1."""
    f, a, s, tot = 1.0, 1.0, 0.0, 0.0
    for o in range(OCT):
        w = 1.0 - min(max((footprint * f * 2.0 - 0.5) / 0.5, 0.0), 1.0)  # footprint in noise-lattice units
        n = vnoise(x * f + 17.3 * o, y * f - 9.1 * o, z * f + 4.7 * o, (seed + o * 0x9E3779B9) & M)
        s += a * (0.5 + w * (n - 0.5)); tot += a
        f *= LAC; a *= GAIN
    return s / tot


def build_cdf(seed, n=200000, cells=64):
    import random
    r = random.Random(1234)
    hist = [0] * cells
    for _ in range(n):
        v = fbm(r.uniform(-50, 50), r.uniform(-50, 50), r.uniform(-50, 50), seed)
        hist[min(int(v * cells), cells - 1)] += 1
    c, out = 0, [0.0]
    for h in hist:
        c += h; out.append(c / n)
    return out


def cdf_lookup(cdf, f):
    x = min(max(f, 0.0), 1.0) * (len(cdf) - 1)
    i = min(int(x), len(cdf) - 2)
    return cdf[i] + (cdf[i + 1] - cdf[i]) * (x - i)


def smoothstep(a, b, x):
    t = min(max((x - a) / (b - a), 0.0), 1.0)
    return t * t * (3.0 - 2.0 * t)


def albedo(p, edge, cavity, seed, cdf, footprint=0.0):
    q = [c / SCALE_M for c in p]; fp = footprint / SCALE_M
    u1 = cdf_lookup(cdf, fbm(q[0], q[1], q[2], seed, fp))
    u2 = cdf_lookup(cdf, fbm(q[0], q[1], q[2], (seed ^ 0x5BD1E995) & M, fp))
    c = PAL[2]
    if u1 < COV[0]: c = PAL[0]
    elif u2 < COV[1] / (1.0 - COV[0]): c = PAL[1]
    nz = vnoise(p[0] * 14.0, p[1] * 14.0, p[2] * 14.0, (seed ^ 0xA511E9B3) & M)
    wd = max(0.04, footprint * 14.0)
    chip = smoothstep(0.0 - wd, 0.0 + wd, edge * 1.2 + 0.6 * (nz - 0.5) - 0.45)
    c = tuple(a + (b - a) * chip for a, b in zip(c, BARE))
    nz2 = vnoise(p[0] * 5.0 + 3.1, p[1] * 5.0, p[2] * 5.0, (seed ^ 0x68E31DA4) & M)
    d = min(max(cavity * (0.6 + 0.8 * nz2), 0.0), 0.85)
    return tuple(a + (b - a) * d for a, b in zip(c, DIRT))


def srgb8(c):
    def enc(v):
        v = min(max(v, 0.0), 1.0)
        v = 12.92 * v if v <= 0.0031308 else 1.055 * v ** (1 / 2.4) - 0.055  # const-ok: sRGB OETF (spike only, edge conversion)
        return int(v * 255.0 + 0.5)
    return tuple(enc(v) for v in c)
