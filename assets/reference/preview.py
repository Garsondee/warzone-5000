"""PNG previews of the camo schemes at true scale, standard library only (zlib + struct).
python3 -I assets/reference/preview.py OUT.png : per scheme a flat panel 2.4 m x 1.2 m (6 mm per pixel) and a sphere of 0.9 m radius."""
import json
import math
import os
import struct
import sys
import zlib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import w5k_look as L  # noqa: E402

CAMO = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "materials", "camo")


def png(path, w, h, rows):
    raw = b"".join(b"\x00" + bytes(r) for r in rows)
    def chunk(t, d):
        c = struct.pack(">I", len(d)) + t + d
        return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    open(path, "wb").write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


def main(out):
    names = ["nato_three_tone", "woodland", "desert_three"]
    pw, ph, sd = 400, 200, 200
    cdf_all = os.path.join(CAMO, "cdf.json")
    rows = []
    for name in names:
        s = L.load_scheme(os.path.join(CAMO, "baked", name + ".json"))
        cdf = L.load_cdf(cdf_all, s)
        block = [[] for _ in range(ph)]
        px = 0.006  # 6 mm per pixel
        for j in range(ph):
            for i in range(pw):
                block[j] += L.linear_to_srgb8(L.colour_linear(s, cdf, (i * px, 0.3, j * px), 21, px))
            for i in range(sd):
                x, y = (i + 0.5) / sd * 2 - 1, 1 - (j + 0.5) / ph * 2
                r2 = x * x + y * y
                if r2 > 1:
                    block[j] += [24, 24, 24]
                else:
                    p = (0.9 * x, 0.9 * y, 0.9 * math.sqrt(1 - r2))
                    block[j] += L.linear_to_srgb8(L.colour_linear(s, cdf, p, 21, 0.9 * 2 / sd))
        rows += block + [[255, 255, 255] * (pw + sd)] * 2
    png(out, pw + sd, len(rows), rows)


if __name__ == "__main__":
    main(sys.argv[1])
