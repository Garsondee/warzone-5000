"""Summary statistics of a possibility-space sample (python3 -I analyze.py space_sample.csv).

Prints the numbers quoted in docs/design/07-possibility-space.md: valid shares, speed, ground pressure, sight against range,
armour kept, the rotor ceiling and installed power per tonne.
"""
import csv, sys, statistics as st, math, collections
rows = list(csv.DictReader(open(sys.argv[1])))
rows = [r for r in rows if r["gear"] != "none" and float(r["mass_kg"]) > 0]
for r in rows:
    r["v"] = r["valid"] == "true"
    for k in ["mass_kg","top_speed_kmh","power_kw","armour_front_mm","armour_side_mm","firepower_kw","alpha_mj","best_pen_mm","weapon_range_km","sight_km","ground_pressure_kpa","length_m","armour_kept","lift_kw"]:
        r[k] = float(r[k])
valid = [r for r in rows if r["v"]]
print(f"rows {len(rows)} valid {len(valid)} ({100*len(valid)/len(rows):.0f}%)")
def pct(x): return f"{100*x:.0f}%"
print("\nvalid share by gear / hull:")
for key in ["gear","hull"]:
    c = collections.defaultdict(lambda: [0,0])
    for r in rows:
        c[r[key]][0] += 1; c[r[key]][1] += r["v"]
    print("  " + ", ".join(f"{k} {pct(v[1]/v[0])}" for k,v in sorted(c.items())))
print("\nspeed by gear (median / p90 / max km/h) and mass (median):")
for g in ["track","wheel","legs","rail","hover","antigrav","rotor"]:
    s = sorted(r["top_speed_kmh"] for r in valid if r["gear"]==g)
    m = sorted(r["mass_kg"] for r in valid if r["gear"]==g)
    if s: print(f"  {g:9} n={len(s):4} speed {s[len(s)//2]:6.0f} / {s[int(len(s)*0.9)]:6.0f} / {s[-1]:6.0f}   mass median {m[len(m)//2]/1000:8.1f} t, max {m[-1]/1000:9.1f} t")
print("\nshare at the gear speed limit:")
for g, lim in [("track",70.0)]:
    s = [r["top_speed_kmh"] for r in valid if r["gear"]==g]
    print(f"  {g}: {pct(sum(1 for x in s if x>=lim-0.5)/len(s))} of tracked designs at {lim} km/h")
print("\nground pressure by gear (median kPa, p10-p90) for designs with contact:")
for g in ["track","wheel","legs","rail","hover"]:
    s = sorted(r["ground_pressure_kpa"] for r in valid if r["gear"]==g and r["ground_pressure_kpa"]>0)
    if s: print(f"  {g:7} median {s[len(s)//2]:6.1f}  p10 {s[len(s)//10]:6.1f}  p90 {s[int(len(s)*0.9)]:6.1f}")
print("\nweapon range vs sight (armed, valid):")
armed = [r for r in valid if r["weapon_range_km"]>0]
for w in ["turret_gun","turret_missile","turret_beam"]:
    a = [r for r in armed if r["weapon"]==w]
    if a: print(f"  {w:15} n={len(a):4} outrange their own eyes: {pct(sum(1 for r in a if r['weapon_range_km']>r['sight_km'])/len(a))}; median range {st.median(r['weapon_range_km'] for r in a):.1f} km, median sight {st.median(r['sight_km'] for r in a):.1f} km")
print(f"  all armed: {pct(sum(1 for r in armed if r['weapon_range_km']>r['sight_km'])/len(armed))} outrange their eyes")
print("\nsight distribution (valid): p10/median/p90/max km:", ", ".join(f"{x:.1f}" for x in [sorted(r['sight_km'] for r in valid)[int(len(valid)*q)] for q in (0.1,0.5,0.9,0.999)]))
print("share with sight > 4 km:", pct(sum(1 for r in valid if r["sight_km"]>4.05)/len(valid)))
print("\narmour kept (median) by gear:")
for g in ["track","wheel","legs","rail","hover","antigrav","rotor"]:
    s = [r["armour_kept"] for r in valid if r["gear"]==g]
    if s: print(f"  {g:9} {pct(st.median(s))}  (share that had to diet: {pct(sum(1 for x in s if x<0.99)/len(s))})")
print("\nrotor ceiling: valid share by mass bin")
for lo,hi in [(0,2e3),(2e3,1e4),(1e4,3e4),(3e4,1e5),(1e5,3e5),(3e5,1e9)]:
    b=[r for r in rows if r["gear"]=="rotor" and lo<=r["mass_kg"]<hi]
    if b: print(f"  {lo/1000:7.0f}-{hi/1000:9.0f} t: n={len(b):3} valid {pct(sum(1 for r in b if r['v'])/len(b))}")
# power per tonne, engine share
pw = sorted(r["power_kw"]/(r["mass_kg"]/1000) for r in valid)
print(f"\ninstalled power per tonne: median {pw[len(pw)//2]:.0f} kW/t, p10 {pw[len(pw)//10]:.0f}, p90 {pw[int(len(pw)*.9)]:.0f}")
