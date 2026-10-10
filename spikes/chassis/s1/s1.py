"""Spike S1: quarter-car stiffness vs step size. Throwaway; standard library only. Run: python3 -I spikes/chassis/s1/s1.py"""
import math

MS, MU, K, C, KT = 400.0, 50.0, 30e3, 2e3, 250e3
G = 9.81

def deriv_force(zs, zu, vs, vu):
    """Forces on sprung / unsprung (positions up). Tyre cannot pull."""
    fs = K * (zu - zs) + C * (vu - vs)          # strut force on sprung mass (up positive)
    ft = -KT * zu if zu < 0 else 0.0            # tyre pushes unsprung up when zu<0 (road at 0 + r(t))
    return fs, ft

def run(dt, t_end, road=lambda t: 0.0, init=(0, 0, 0, 0), gravity=True, damper=True):
    c = C if damper else 0.0
    zs, zu, vs, vu = init
    t, out, e0 = 0.0, [], None
    n = int(round(t_end / dt))
    for i in range(n):
        r = road(t)
        fs = K * (zu - zs) + c * (vu - vs)
        ft = KT * (r - zu)
        a_s = fs / MS - (G if gravity else 0)
        a_u = (-fs + ft) / MU - (G if gravity else 0)
        vs += a_s * dt; vu += a_u * dt          # semi-implicit: velocity first
        zs += vs * dt;  zu += vu * dt
        t += dt
        out.append((t, zs, zu, vs, vu))
    return out

def energy(zs, zu, vs, vu, r=0.0):
    return 0.5*MS*vs*vs + 0.5*MU*vu*vu + 0.5*K*(zu-zs)**2 + 0.5*KT*(r-zu)**2

# closed form: undamped eigenfrequencies via 2x2 characteristic polynomial
def modes(c):
    # M x'' + C x' + K x = 0, x=(zs,zu); complex roots found numerically via companion eigenvalues (Durand-Kerner)
    a = [MS*MU, c*(MS+MU), K*(MS+MU)+KT*MS, c*KT, K*KT]  # s^4 + ... ; coefficients of det
    a = [x/a[0] for x in a]
    roots = [complex(0.4, 0.9)**k for k in range(4)]
    for _ in range(500):
        new = []
        for i, ri in enumerate(roots):
            p = ri**4 + a[1]*ri**3 + a[2]*ri**2 + a[3]*ri + a[4]
            d = 1
            for j, rj in enumerate(roots):
                if j != i: d *= (ri - rj)
            new.append(ri - p/d)
        roots = new
    return roots

def stats(ns, c, label):
    rs = [r for r in modes(c) if r.imag > 0]
    rs.sort(key=lambda r: abs(r))
    return [(abs(r)/2/math.pi, -r.real/abs(r)) for r in rs]

print("closed-form modes (f_n Hz, zeta):")
for lab, c in (("damped", C), ("undamped", 0.0)):
    print(" ", lab, [(round(f, 3), round(z, 4)) for f, z in stats(4, c, lab)])
f_body, f_hop = stats(4, C, "")[0][0], stats(4, C, "")[1][0]
f_hop_u = stats(4, 0.0, "")[1][0]
f_tyre_only = math.sqrt((KT+K)/MU)/2/math.pi
print(f"  naive wheel-hop (K+KT)/MU: {f_tyre_only:.2f} Hz")

print("\nstability limit (explicit/semi-implicit Euler, damped, 5 s from 1 cm step):")
for hz in (20, 30, 36, 38, 40, 45, 60, 100, 300, 1000):
    dt = 1/hz
    o = run(dt, 5.0, road=lambda t: 0.01, gravity=False)
    m = max(abs(x[2]) for x in o)
    print(f"  {hz:5d} Hz  max|zu|={m:.4g} m  {'UNSTABLE' if (m > 1 or m != m) else 'ok'}")

print("\nundamped frequency + energy drift (zero-road, zs0=5cm, gravity off, 60 s):")
print("  Hz   f_body err%  f_hop err%  max rel energy error  drift/min%")
for hz in (60, 120, 180, 240, 300, 480, 600, 1000):
    dt = 1/hz
    o = run(dt, 60.0, init=(0.05, 0, 0, 0), gravity=False, damper=False)
    e0 = energy(0.05, 0, 0, 0)
    es = [energy(x[1], x[2], x[3], x[4]) for x in o]
    maxdev = max(abs(e-e0) for e in es)/e0
    # drift: slope of window-mean energy between first and last 5 s
    n = len(es); w = int(5/dt)
    drift = (sum(es[-w:])/w - sum(es[:w])/w)/e0/(55/60)
    # body frequency from zero crossings of zs relative to mean
    zs = [x[1] for x in o]
    cr = [i for i in range(1, n) if zs[i-1] < 0 <= zs[i]]
    f_meas = (len(cr)-1)/((cr[-1]-cr[0])*dt) if len(cr) > 2 else float('nan')
    print(f"  {hz:4d}  {100*(f_meas/stats(4,0,'')[0][0]-1):+8.3f}    {'n/a':>8}    {maxdev*100:10.4f}%    {drift*100:+.5f}")

print("\nlog-decrement damping ratio of the body mode (damped, 40 mm step, gravity off):")
zeta_cf = stats(4, C, "")[0][1]
print(f"  closed form zeta_body = {zeta_cf:.4f}")
for hz in (60, 120, 240, 300, 600):
    dt = 1/hz
    o = run(dt, 4.0, init=(0.04, 0, 0, 0), gravity=False)
    zs = [x[1] for x in o]
    pk = [(i, zs[i]) for i in range(1, len(zs)-1) if zs[i-1] < zs[i] >= zs[i+1] and zs[i] > 0]
    if len(pk) >= 3:
        d = math.log(pk[0][1]/pk[2][1])/2
        z = d/math.sqrt(4*math.pi**2 + d*d)
        td = (pk[2][0]-pk[0][0])*dt/2
        print(f"  {hz:4d} Hz  zeta={z:.4f}  err={100*(z/zeta_cf-1):+.2f}%")

print("\nsubstep rule: stiffest-mode frequency vs substeps needed for omega*dt <= 1, and 20 steps/period")
for lab, f in (("body", f_body), ("hop", f_hop), ("hop on bump stop (~2.5x K)", math.sqrt((2.5*K+KT)/MU)/2/math.pi), ("spin/slip oscillator 25 Hz", 25.0)):
    print(f"  {lab:30s} {f:6.2f} Hz  20f/60={math.ceil(20*f/60)}  omega*dt<=1 -> {math.ceil(2*math.pi*f/60/1.0)}  omega*dt<=0.5 -> {math.ceil(2*math.pi*f/60/0.5)}")

print("\nspeed hump response, damped, ~50 mm half-sine 0.3 m long at 20 km/h, error vs 1 kHz reference (max|zs|):")
v = 20/3.6; L = 0.3; H = 0.05
road = lambda t: H*math.sin(math.pi*v*t/L) if 0 <= v*t <= L else 0.0
ref = max(abs(x[1]) for x in run(1/4000, 1.5, road=road, gravity=False))
for hz in (60, 120, 180, 300, 600, 1000):
    m = max(abs(x[1]) for x in run(1/hz, 1.5, road=road, gravity=False))
    print(f"  {hz:5d} Hz  peak sprung excursion {m*1000:.3f} mm  err vs 4 kHz {100*(m/ref-1):+.2f}%")

# --- stopped truck on a 10% grade: relaxation-length friction vs Coulomb switch
print("\nstopped truck on 10% grade (2500 kg, mu=0.8, 4 tyres, 5 s, dt=1/300):")
slope = math.atan(0.10); M = 2500.0; N = M*G*math.cos(slope); mu = 0.8
def grade(model, sigma=0.2, v_eps=0.05, t_end=5.0, dt=1/300):
    v = 0.0; x = 0.0; fx = 0.0; hist = []
    for i in range(int(t_end/dt)):
        drive = M*G*math.sin(slope)             # down-slope pull
        if model == "relax":
            # stretch state: tyre contact point lags; spring-like bristle with stiffness N*mu/sigma, damped
            fx_target = -mu*N*max(-1, min(1, 0))  # not used
        hist.append(v)
        break
    return None
def sim(model, sigma, dt=1/300, t_end=5.0):
    v = 0.0; x = 0.0; s = 0.0; maxv = 0.0; flips = 0; last = 0; x4 = 0.0
    kb = mu*N/sigma                               # bristle stiffness: force reaches mu N after sigma of stretch
    cb = 2*0.7*math.sqrt(kb*M)
    for i in range(int(t_end/dt)):
        pull = M*G*math.sin(slope)
        if model == "relax":
            # bristle anchored to ground; contact patch stretch s = x - x_anchor, anchor slips when |F|>mu N
            f = -(kb*s + cb*v)
            fmax = mu*N
            if abs(f) > fmax:
                f = math.copysign(fmax, f)
            a = (pull + f)/M
            v += a*dt
            # stretch grows with sliding velocity but saturates: anchor drags when saturated
            s += v*dt
            if abs(kb*s) > fmax: s = math.copysign(fmax/kb, s)
        else:                                      # naive Coulomb switch sign(v)
            f = -mu*N*(1 if v > 0 else -1 if v < 0 else 0)
            a = (pull + f)/M
            vn = v + a*dt
            if v != 0 and vn*v < 0: vn = 0.0       # (stick at zero crossing)
            v = vn
        x += v*dt
        if i == int(4.0/dt): x4 = x
        maxv = max(maxv, abs(v))
        sgn = 1 if v > 1e-6 else -1 if v < -1e-6 else 0
        if sgn and last and sgn != last: flips += 1
        if sgn: last = sgn
    return x, maxv, flips, x - x4
for model, sig in (("coulomb", 0), ("relax", 0.05), ("relax", 0.2), ("relax", 0.5)):
    x, mv, fl, late = sim(model, sig or 1)
    print(f"  {model:8s} sigma={sig:4}  displacement 5 s {x*1000:7.3f} mm (static stretch sigma*tan/mu = {sig*math.tan(slope)/mu*1000:6.2f} mm)  moved during last 1 s {late*1e6:8.3f} um")
