"""L7 (albedo half) and L5 (weathering): the GLSL chunk against the reference on 4096 points, and the weathering inputs act monotonically.
The Chromium part skips (loudly) when node or Playwright is missing. Run: python3 -B -I -m unittest discover -s assets/tests"""
import json
import os
import shutil
import subprocess
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.join(HERE, "..")
sys.path.insert(0, os.path.join(ROOT, "reference"))
import w5k_look as L  # noqa: E402

W = json.load(open(os.path.join(ROOT, "materials", "baked", "weathering.json")))
CAMO = os.path.join(ROOT, "materials", "camo")
FLAT = dict(edge=0.0, cavity=0.0, height_m=10.0, up_y=0.0)


def grid_point(x, y):
    return ((x * 0.125, y * 0.0625, ((x ^ y) & 63) * 0.03125), x / 64.0, y / 64.0, ((x * 7 + y * 3) & 63) / 32.0, (((x * 5 + y * 11) & 63) - 32) / 32.0)


class TestWeathering(unittest.TestCase):
    def w(self, **kw):
        a = dict(FLAT); a.update(kw)
        return L.weathered(W, (0.05, 0.07, 0.03), W["bare_linear"], (0.3, 0.2, 0.1), a["edge"], a["cavity"], a["height_m"], a["up_y"], 9)

    def test_weathering_inputs_act_monotonically(self):  # L5
        lum = lambda c: sum(c) / 3.0
        paint = (0.05, 0.07, 0.03)
        bare, dirt, splash, dust = (W[k + "_linear"] for k in ("bare", "dirt", "splash", "dust"))
        pairs = [("edge", 0.0, 1.0, bare), ("cavity", 0.0, 1.0, dirt), ("height_m", 0.6, 0.0, splash), ("up_y", 0.0, 1.0, dust)]
        for key, lo, hi, target in pairs:  # more input -> closer to that term's colour
            d = [L.delta_e76(self.w(**{key: lo + (hi - lo) * k / 20.0}), target) for k in range(21)]
            self.assertTrue(all(b <= a + 1e-9 for a, b in zip(d, d[1:])), key)
            self.assertLess(d[-1], d[0], key)

    def test_zero_flags_give_a_clean_flat_face(self):  # L5
        for seed in range(50):
            c = L.weathered(W, (0.05, 0.07, 0.03), W["bare_linear"], (seed * 0.37, seed * 0.11, 1.3), 0.0, 0.0, 10.0, 0.0, seed)
            self.assertEqual(c, (0.05, 0.07, 0.03))

    def test_triplanar_weights_sum_to_one_and_split_evenly_at_45_degrees(self):  # L5
        for n in ((0.3, 0.5, 0.8), (1, 0, 0), (0.6, -0.6, 0.5)):
            self.assertAlmostEqual(sum(L.triplanar_weights(n)), 1.0, 12)
        w = L.triplanar_weights((2 ** -0.5, 0.0, 2 ** -0.5))
        self.assertAlmostEqual(w[0], 0.5, 12)
        self.assertAlmostEqual(w[2], 0.5, 12)


@unittest.skipUnless(shutil.which("node") and os.path.isdir(os.environ.get("NODE_PATH", "/opt/node-tools/node_modules") + "/playwright"), "needs node and Playwright with Chromium")
class TestShader(unittest.TestCase):
    def test_shader_albedo_matches_the_reference_within_2_of_255(self):  # L7
        cdf_all = os.path.join(CAMO, "cdf.json")
        for name in ("nato_three_tone", "woodland", "desert_three"):
            path = os.path.join(CAMO, "baked", name + ".json")
            s = L.load_scheme(path)
            cdf = L.load_cdf(cdf_all, s)
            out = subprocess.run(["node", os.path.join(HERE, "gl", "albedo_harness.mjs"), path, "1234"], capture_output=True, text=True, timeout=240, check=True).stdout
            gpu = json.loads(out)
            worst, over = 0, 0
            for y in range(64):
                for x in range(64):
                    p, e, cv, h, up = grid_point(x, y)
                    ref = L.linear_to_srgb8(L.weathered(W, L.colour_linear(s, cdf, p, 1234), W["bare_linear"], p, e, cv, h, up, 1234))
                    d = max(abs(a - b) for a, b in zip(ref, gpu[(y * 64 + x) * 4:(y * 64 + x) * 4 + 3]))
                    worst, over = max(worst, d), over + (d > 2)
            print("L7 %s: worst delta %d/255, points over 2: %d of 4096" % (name, worst, over))
            self.assertEqual(over, 0, name)


if __name__ == "__main__":
    unittest.main()
