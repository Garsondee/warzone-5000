"""Compare GPU outputs (out/gpu.json) with ref.py: hash bytes exactly, tile albedo in 8-bit sRGB. Also coverage on the tile."""
import json, sys; sys.path.insert(0, "."); import ref
g = json.load(open("out/gpu.json")); P = json.load(open("params.json")); W = 256
bad = 0
for y in range(W):
    for x in range(W):
        h = ref.hash3(x - 128 + ref.OFFSET, y - 128 + ref.OFFSET, 7 + ref.OFFSET, P["seed"]); i = (y * W + x) * 4
        if [h >> 24, (h >> 16) & 255, (h >> 8) & 255] != g["hash"][i:i + 3]: bad += 1
print("hash mismatches:", bad, "of", W * W)
for key, fp in (("tile", 0.0), ("tileFar", 4.0 / 256)):
    hist, mx, over = [0] * 41, 0, 0
    cnt = [0, 0, 0]
    for y in range(W):
        for x in range(W):
            u, v = (x + .5) / W, (y + .5) / W
            c = ref.srgb8(ref.albedo((u * 4, 0.3, v * 4), u, v, P["seed"], P["cdf"], fp)); i = (y * W + x) * 4
            d = max(abs(a - b) for a, b in zip(c, g[key][i:i + 3])); mx = max(mx, d); over += d > 2
    print(key, "max |delta| /255:", mx, " pixels over 2:", over, "of", W * W)
