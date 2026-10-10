"""Spike S-G plot, standard library only: python3 -I spikes/geometry/plot.py spikes/geometry docs/lanes/geometry/media/s-g.png
Left panel: edge flag across the top of the hood (x: 0 to 0.3 m from the chamfer, y: 0 to 1). Right: cavity at the foot of a wall (x: 0 to 0.5 m, y: 0 to 0.5)."""
import csv, struct, sys, zlib

W, H = 900, 380
img = [[(250, 250, 250)] * W for _ in range(H)]


def put(x, y, c, r=1):
    for dy in range(-r, r + 1):
        for dx in range(-r, r + 1):
            if 0 <= x + dx < W and 0 <= y + dy < H:
                img[y + dy][x + dx] = c


def line(a, b, c, r=1):
    n = max(abs(b[0] - a[0]), abs(b[1] - a[1]), 1)
    for i in range(n + 1):
        put(round(a[0] + (b[0] - a[0]) * i / n), round(a[1] + (b[1] - a[1]) * i / n), c, r)


def panel(x0, rows, xmax, ymax, series):
    pw, ph, top = 380, 300, 40
    for gx in range(0, 11):  # grid, 10 divisions
        line((x0 + gx * pw // 10, top), (x0 + gx * pw // 10, top + ph), (225, 225, 225), 0)
    for gy in range(0, 11):
        line((x0, top + gy * ph // 10), (x0 + pw, top + gy * ph // 10), (225, 225, 225), 0)
    line((x0, top + ph), (x0 + pw, top + ph), (40, 40, 40))
    line((x0, top), (x0, top + ph), (40, 40, 40))
    for col, colour, dots in series:
        pts = [(x0 + round(r[0] / xmax * pw), top + ph - round(min(r[col], ymax) / ymax * ph)) for r in rows if r[0] <= xmax]
        if dots:
            for p in pts:
                put(p[0], p[1], colour, 3)
        else:
            for a, b in zip(pts, pts[1:]):
                line(a, b, colour, 1)


def read(path):
    with open(path) as f:
        r = csv.reader(f)
        next(r)
        return [[float(v) for v in row] for row in r]


src, out = sys.argv[1], sys.argv[2]
panel(40, read(src + "/s-g-edge.csv"), 0.3, 1.0, [(1, (150, 150, 150), False), (2, (30, 90, 200), False), (3, (230, 130, 20), False)])
panel(480, read(src + "/s-g-cavity.csv"), 0.5, 0.5, [(2, (20, 20, 20), False), (1, (30, 90, 200), True)])
raw = b"".join(b"\x00" + bytes(v for px in row for v in px) for row in img)


def chunk(t, d):
    return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d))


open(out, "wb").write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))
