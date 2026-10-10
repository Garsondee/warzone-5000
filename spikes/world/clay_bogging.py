"""Ground pressure and bogging of the stand-in vehicles in the published clay row (Bekker-Wong, arXiv:2603.28965 Table 2, SECONDARY source).

Throwaway arithmetic, standard library only: python3 -I spikes/world/clay_bogging.py
A vehicle bogs when the compaction resistance of its wheels or tracks exceeds the most thrust the soil can give (Coulomb: c A + W tan(phi)).
Vehicle numbers are the design-sheet estimates in content/vehicles/game/*.ron.
"""
import math

G = 9.81
N, KC, KPHI, C, PHI = 0.5, 13.19e3, 692.15e3, 4.14e3, math.radians(13)  # clay row, SI (Pa, m)
# name: (mass kg, wheels, tyre width m, tyre diameter m, inflation Pa)
WHEELED = {"scout": (1150, 4, 0.20, 0.70, 180e3), "mule": (2300, 4, 0.30, 0.82, 220e3), "hauler": (5600, 4, 0.35, 1.05, 400e3)}
CARRIER = (7800, 0.38, 2.9)  # mass kg, track width m, ground contact length m (two tracks)


def compaction_resistance(b, z):
    """Work per metre pressing a plate of width b to depth z into the soil: b (kc/b + kphi) z^(n+1) / (n+1), N."""
    return b * (KC / b + KPHI) * z ** (N + 1) / (N + 1)


def rows(phi=PHI):
    out = []
    for name, (m, nw, b, d, infl) in WHEELED.items():
        w = m * G / nw
        # Rigid wheel (Wong): z = (3 W / (b (3 - n) (kc/b + kphi) sqrt(D)))^(2 / (2 n + 1)). Valid when the tyre is stiffer than the soil.
        z = (3 * w / (b * (3 - N) * (KC / b + KPHI) * math.sqrt(d))) ** (2 / (2 * N + 1))
        length = math.sqrt(d * z - z * z)  # contact length of the front arc
        area = nw * b * length
        res = nw * compaction_resistance(b, z)
        thrust = C * area + m * G * math.tan(phi)
        out.append((name, m * G / area, infl, z, res / (m * G), thrust / (m * G)))
    m, b, length = CARRIER
    p = m * G / (2 * b * length)
    z = (p / (KC / b + KPHI)) ** (1 / N)
    out.append(("carrier", p, 0.0, z, 2 * compaction_resistance(b, z) / (m * G), (C * 2 * b * length + m * G * math.tan(phi)) / (m * G)))
    return out


if __name__ == "__main__":
    print("vehicle  soil p kPa  inflation kPa  sinkage cm  Rc/W   thrust/W  margin/W")
    for name, p, infl, z, rc, h in rows():
        print(f"{name:8s} {p/1e3:9.1f} {infl/1e3:13.0f} {z*100:11.2f} {rc:6.3f} {h:9.3f} {h - rc:9.3f}")
    print("margin/W against the friction angle's band (10 to 16 degrees in materials.ron):")
    for deg in (10, 13, 16):
        print(f"  phi {deg:2d} deg: " + ", ".join(f"{r[0]} {r[5] - r[4]:+.3f}" for r in rows(math.radians(deg))))
    print("A pit wall of grade g costs g (rise over run) of W: a vehicle whose margin/W is below g bogs on the wall.")
