"""Build the Possibility Space explorer page from the sampled CSV (python3 -I build.py data.csv template.html out.html)."""
import csv, json, math, sys

csv_path, tpl_path, out_path = sys.argv[1:4]
HULLS = ["lancer", "bastion", "dreadnought", "strider", "skiff"]
GEARS = ["track", "wheel", "legs", "rail", "hover", "antigrav", "rotor"]
WEAPONS = ["gun", "missile", "beam", "none"]

def sig(x, n=3):
    x = float(x)
    if x == 0 or not math.isfinite(x):
        return 0
    d = n - 1 - int(math.floor(math.log10(abs(x))))
    r = round(x, d)
    return int(r) if d <= 0 else r

why_table = [""]
rows = []
for r in csv.DictReader(open(csv_path, newline="")):
    hull = r["hull"].replace("hull_", "")
    gear = r["gear"]
    if hull not in HULLS or gear not in GEARS:
        continue  # a combination with no mount
    weapon = r["weapon"].replace("turret_", "")
    if weapon not in WEAPONS:
        weapon = "none"
    valid = r["valid"].strip().lower() == "true"
    problem = (r.get("problem") or "").strip()
    if valid or not problem:
        wi = 0
    else:
        problem = problem[:170]
        if problem not in why_table:
            why_table.append(problem)
        wi = why_table.index(problem)
    mass = float(r["mass_kg"])
    if mass <= 0:
        continue
    rows.append([
        int(r["seed"]), HULLS.index(hull), GEARS.index(gear), WEAPONS.index(weapon), 1 if valid else 0,
        sig(mass), sig(r["top_speed_kmh"]), sig(r["power_kw"]), sig(r["armour_front_mm"]), sig(r["armour_side_mm"]),
        sig(r["frontal_m2"]), sig(r["firepower_kw"]), sig(r["alpha_mj"]), sig(r["best_pen_mm"]), sig(r["weapon_range_km"]),
        sig(r["sight_km"]), sig(r["ground_pressure_kpa"]), sig(r["length_m"]), sig(r["width_m"]), sig(r["height_m"]),
        sig(r["lift_kw"]), sig(r["armour_kept"], 2), wi,
    ])

cols = ["seed", "hull", "gear", "weapon", "valid", "mass", "speed", "power", "armF", "armS", "area", "fire", "alpha", "pen",
        "range", "sight", "press", "len", "wid", "hei", "lift", "kept", "whyi"]
data = {"hulls": HULLS, "gears": GEARS, "weapons": WEAPONS, "why": why_table, "cols": cols, "rows": rows}
payload = json.dumps(data, separators=(",", ":"))
tpl = open(tpl_path).read()
assert "__DATA__" in tpl
tpl = tpl.replace("__DATA__", payload)
if len(sys.argv) > 4:
    imgs = json.load(open(sys.argv[4]))
    for key, tag in (("atlas", "__IMG_ATLAS__"), ("showcase", "__IMG_SHOWCASE__"), ("corners", "__IMG_CORNERS__")):
        tpl = tpl.replace(tag, imgs[key])
else:
    blank = "data:image/gif;base64,R0lGODlhAQABAAAAACw="
    for tag in ("__IMG_ATLAS__", "__IMG_SHOWCASE__", "__IMG_CORNERS__"):
        tpl = tpl.replace(tag, blank)
open(out_path, "w").write(tpl)
print(f"{len(rows)} designs, {len(payload)/1024:.0f} KB of data -> {out_path}")
