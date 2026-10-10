import sys, json; sys.path.insert(0, "."); import ref
seed = 42
json.dump({"seed": seed, "cdf": ref.build_cdf(seed), "pal": ref.PAL, "cov": ref.COV, "bare": ref.BARE, "dirt": ref.DIRT, "scale": ref.SCALE_M}, open("params.json", "w"))
