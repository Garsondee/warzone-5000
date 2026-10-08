"""Draw the first-light replay as one picture: where the truck went (coloured by speed), and speed/rpm/pitch/travel over time.

Usage: python3 -I tools/control_room/plot_first_light.py out/first-light/replay.json docs/swarm/media/first-light.png
(ARCH digest tool; presentation only, so it may use float maths freely.)"""
import json, math, sys
from PIL import Image, ImageDraw, ImageFont

src, out = sys.argv[1], sys.argv[2]
d = json.load(open(src))
dt = d["header"]["frame_dt_s"]
fr = d["frames"]
names = d["header"]["vehicles"][0]["joint_names"]
ti = [i for i, n in enumerate(names) if n.endswith(".travel")]

t, x, z, spd, rpm, gear, pitch, travel = [], [], [], [], [], [], [], []
for f in fr:
    v = f["vehicles"][0]
    t.append(f["t_s"])
    x.append(v["pos_m"]["x"]); z.append(v["pos_m"]["z"])
    lv = v["lin_vel_m_s"]; spd.append(math.sqrt(lv["x"] ** 2 + lv["y"] ** 2 + lv["z"] ** 2))
    rpm.append(v["engine_rpm"]); gear.append(v["gear"])
    q = v["rot"]; pitch.append(math.degrees(math.asin(max(-1, min(1, 2 * (q["w"] * q["x"] - q["y"] * q["z"]))))))
    travel.append(max(v["joints"][i] for i in ti))

W, H = 1500, 860
BG, FG, MUT, GRID = (244, 244, 236), (34, 40, 28), (110, 118, 96), (214, 216, 200)
ACC, OLIVE, SAGE, RUST = (222, 106, 28), (92, 110, 60), (150, 168, 120), (170, 62, 40)
img = Image.new("RGB", (W, H), BG)
dr = ImageDraw.Draw(img)

def font(size, bold=False):
    path = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf" if bold else "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
    return ImageFont.truetype(path, size)

f_title, f_sub, f_lab, f_small = font(30, True), font(15), font(14, True), font(12)
dr.text((40, 26), "FIRST LIGHT  |  the spine runs end to end on stand-ins", font=f_title, fill=FG)
dr.text((40, 66), "box truck on a bump strip, scripted drive: launch, steer right, steer left, brake. Same code path the real vehicle will use; "
        "hash chain identical on every run.", font=f_sub, fill=MUT)

def lerp(a, b, u): return a + (b - a) * u
SLOW = (62, 96, 140)
def ramp(u):  # slate blue (slow) -> orange (fast)
    u = max(0.0, min(1.0, u))
    return tuple(int(lerp(SLOW[i], ACC[i], u)) for i in range(3))

# ---- left: top-down path -------------------------------------------------------------------------------------------------
px0, py0, px1, py1 = 40, 110, 560, 820
dr.rectangle([px0, py0, px1, py1], outline=GRID, width=2)
dr.text((px0 + 10, py0 + 8), "TOP-DOWN PATH  (colour = speed)", font=f_lab, fill=FG)
xmin, xmax, zmin, zmax = min(x), max(x), min(z), max(z)
span = max(xmax - xmin, zmax - zmin, 1.0) * 1.12
cx, cz = (xmin + xmax) / 2, (zmin + zmax) / 2
def P(xx, zz):  # +x right, -z forward drawn upwards
    s = (min(px1 - px0, py1 - py0) - 70) / span
    return ((px0 + px1) / 2 + (xx - cx) * s, (py0 + py1) / 2 + 12 + (zz - cz) * s)
vmax = max(spd)
for k in range(len(t) - 1):
    dr.line([P(x[k], z[k]), P(x[k + 1], z[k + 1])], fill=ramp(spd[k] / vmax), width=5)
sx, sy = P(x[0], z[0]); ex, ey = P(x[-1], z[-1])
dr.ellipse([sx - 7, sy - 7, sx + 7, sy + 7], fill=FG); dr.text((sx + 12, sy - 8), "start", font=f_small, fill=FG)
dr.rectangle([ex - 7, ey - 7, ex + 7, ey + 7], fill=RUST); dr.text((ex + 12, ey - 8), "stopped", font=f_small, fill=FG)
# 50 m scale bar
s = (min(px1 - px0, py1 - py0) - 70) / span
bx, by = px0 + 24, py1 - 30
dr.line([bx, by, bx + 50 * s, by], fill=FG, width=3); dr.text((bx, by - 22), "50 m", font=f_small, fill=FG)
dr.text((px0 + 10, py1 - 70), f"{sum(math.hypot(x[k+1]-x[k], z[k+1]-z[k]) for k in range(len(t)-1)):.0f} m driven, top speed {vmax:.1f} m/s ({vmax*3.6:.0f} km/h)", font=f_small, fill=MUT)

lx0, ly0 = px1 - 190, py1 - 34
for k in range(150):
    dr.line([lx0 + k, ly0, lx0 + k, ly0 + 12], fill=ramp(k / 149))
dr.text((lx0, ly0 + 15), "0", font=f_small, fill=MUT); dr.text((lx0 + 112, ly0 + 15), f"{vmax:.0f} m/s", font=f_small, fill=MUT)

# ---- right: four strips over time ----------------------------------------------------------------------------------------
def strip(top, height, series, color, title, unit, lo=None, hi=None, extra=None):
    x0, x1 = 620, 1460
    dr.rectangle([x0, top, x1, top + height], outline=GRID, width=2)
    lo = min(series) if lo is None else lo
    hi = max(series) if hi is None else hi
    if hi - lo < 1e-9: hi = lo + 1
    pad = (hi - lo) * 0.08
    lo -= pad; hi += pad
    def X(tt): return x0 + (tt - t[0]) / (t[-1] - t[0]) * (x1 - x0)
    def Y(vv): return top + height - (vv - lo) / (hi - lo) * height
    for g in range(1, 4):  # horizontal guide lines
        yy = top + height * g / 4
        dr.line([x0, yy, x1, yy], fill=GRID, width=1)
    if extra: extra(X, Y)
    dr.line([(X(t[k]), Y(series[k])) for k in range(len(t))], fill=color, width=3)
    dr.text((x0 + 10, top + 6), title, font=f_lab, fill=FG)
    dr.text((x1 - 140, top + 6), f"max {max(series):.2f} {unit}", font=f_small, fill=MUT)
    dr.text((x0 - 52, Y(hi - pad) - 8), f"{hi - pad:.0f}" if hi - pad > 20 else f"{hi - pad:.1f}", font=f_small, fill=MUT)
    dr.text((x0 - 52, Y(lo + pad) - 8), f"{lo + pad:.0f}" if hi - pad > 20 else f"{lo + pad:.1f}", font=f_small, fill=MUT)

def gears(X, Y):
    last = None
    for k in range(len(t)):
        if gear[k] != last:
            xx = X(t[k])
            dr.line([xx, Y(0) , xx, Y(0) - 14], fill=FG, width=2)
            dr.text((xx + 4, Y(0) - 28), str(gear[k]), font=f_lab, fill=FG)
            last = gear[k]
sh = 165
strip(110, sh, spd, OLIVE, "SPEED", "m/s", lo=0)
strip(110 + (sh + 20), sh, rpm, ACC, "ENGINE RPM  (stand-in torque curve; numbers = gear)", "rpm", lo=0, extra=gears)
strip(110 + 2 * (sh + 20), sh, pitch, (62, 96, 140), "BODY PITCH  (+ = nose up)", "deg")
strip(110 + 3 * (sh + 20), sh, travel, RUST, "WORST SUSPENSION TRAVEL  (+ = compressed)", "m")
x0, x1 = 620, 1460
for k in range(0, 21, 4):
    tt = t[0] + (t[-1] - t[0]) * k / 20
    xx = x0 + (tt - t[0]) / (t[-1] - t[0]) * (x1 - x0)
    dr.text((xx - 8, 110 + 4 * (sh + 20) - 14), f"{tt:.0f} s", font=f_small, fill=MUT)
img.save(out, optimize=True)
print("wrote", out, img.size)
