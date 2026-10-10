"""Spike S-D: stiff couplings. Throwaway, stdlib only. Run: python3 -I spikes/drive/spike_d.py
Engine flywheel Je -> coupling -> output inertia Jo (vehicle reflected through gear+final drive) with a load torque.
Question: which coupling formulation is stable at dt = 1/120, 1/240, 1/480 s, and what does a shift cost?
"""
import math

Je, Jo = 0.35, 1.2          # kg m^2: engine; vehicle mass reflected to the gearbox output in first gear (ESTIMATE)
CAP = 450.0                 # clutch capacity, N m
LOAD = 60.0                 # road load at the output shaft, N m
def t_engine(w):            # full-throttle torque, N m (flat diesel-like), cut at the 2600 rpm redline
    return 0.0 if w > 272.0 else 300.0

def clutch(form, dt, t_end=3.0, eps=0.5, shift=None):
    """Launch: output at rest, engine at 800 rpm, clutch snaps shut at t=0. Returns (w_e, w_o, T_clutch history)."""
    we, wo = 84.0, 0.0
    hist, jerk_max, prev = [], 0.0, 0.0
    n = int(t_end / dt)
    Jeff = Je * Jo / (Je + Jo)
    stuck = False
    for k in range(n):
        t = k * dt
        cap = CAP if not (shift and shift[0] <= t < shift[0] + shift[1]) else 0.0   # torque interruption
        Te = t_engine(we)
        slip = we - wo
        if form == "explicit_tanh":            # regularised sign, explicit
            Tc = cap * math.tanh(slip / eps)
        elif form == "implicit_stickslip":     # torque that would lock both shafts at the end of the step, clamped to capacity
            # lock: (Je+Jo) w' = Te - LOAD (+ both), and Tc such that we'=wo' -> Tc = Jeff*(slip/dt + (Te/Je - (-LOAD)/Jo ... ))
            Tc_lock = Jeff * (slip / dt + Te / Je + LOAD / Jo)
            Tc = max(-cap, min(cap, Tc_lock))
            stuck = abs(Tc_lock) <= cap
        elif form == "semi_implicit_tanh":     # linearise tanh about the current slip: Tc = Tc0 + k (slip' - slip), solve for slip'
            Tc0 = cap * math.tanh(slip / eps)
            kk = cap / eps / math.cosh(slip / eps) ** 2
            # slip' = slip + dt*((Te - Tc)/Je + (Tc - LOAD)/Jo) ; Tc = Tc0 + kk (slip'-slip)
            a = (1 / Je + 1 / Jo)
            dslip = dt * ((Te / Je - LOAD / Jo) - Tc0 * a) / (1 + dt * kk * a)
            Tc = Tc0 + kk * dslip
        we += dt * (Te - Tc) / Je
        wo += dt * (Tc - LOAD) / Jo
        hist.append((t, we, wo, Tc))
        jerk_max = max(jerk_max, abs(Tc - prev)); prev = Tc
    return hist, jerk_max

def metrics(h):
    # chatter: sign changes of clutch torque in last half; final slip; peak |slip| overshoot (we below wo = unphysical energy)
    tail = h[len(h)//2:]
    flips = sum(1 for a, b in zip(tail, tail[1:]) if (a[3] > 0) != (b[3] > 0))
    return flips, tail[-1][1] - tail[-1][2], min(w[1] - w[2] for w in h), max(abs(x[3]) for x in h)

def converter(dt, ratio_k=0.0, t_end=3.0, implicit=True, K=14.0, SR=2.2):
    """Converter: pump torque = (w_p/K)^2 (K in rpm/sqrt(N m) -> rad/s), turbine torque = TR(speed ratio) * pump torque,
    TR falls linearly from SR at stall to 1 at speed ratio 0.85, then 1 (coupling point; real units add the lock-up)."""
    k_rad = K * 2 * math.pi / 60.0
    wp, wt = 84.0, 0.0
    out = []
    for k in range(int(t_end / dt)):
        sr = 0.0 if wp < 1e-6 else min(1.0, max(0.0, wt / wp))
        tr = SR - (SR - 1.0) * min(1.0, sr / 0.85)
        Tp = (wp / k_rad) ** 2 * (1.0 if sr < 0.97 else 1.0)
        Te = t_engine(wp)
        if implicit:   # linearise Tp = c wp^2 about wp: Tp' ~ Tp + 2c wp dwp  -> solve dwp
            c = 1 / k_rad ** 2
            dwp = dt * (Te - Tp) / Je / (1 + dt * 2 * c * wp / Je)
            wp_n = wp + dwp
            Tp_eff = Tp + 2 * c * wp * dwp
        else:
            wp_n = wp + dt * (Te - Tp) / Je; Tp_eff = Tp
        wt += dt * (tr * Tp_eff - LOAD) / Jo
        wp = max(wp_n, 0.0)
        out.append((k * dt, wp, wt, Tp_eff))
    return out

def shift_transient(dt, interrupt_s):
    """Velocity lost during a torque interruption: output coasts against LOAD. Analytic: dv = LOAD*t/Jo (reflected)."""
    h, _ = clutch("implicit_stickslip", dt, t_end=3.0, shift=(1.5, interrupt_s))
    i0 = int(round(1.5 / dt)) - 1
    i1 = int(round((1.5 + interrupt_s) / dt)) - 1   # last step with the clutch open
    return h[i0][2] - h[i1][2], LOAD * interrupt_s / Jo

if __name__ == "__main__":
    print("clutch launch, Je=%.2f Jo=%.2f cap=%d  (chatter = sign flips of clutch torque in 2nd half; neg_slip = min(we-wo), <0 is energy creation)" % (Je, Jo, CAP))
    ref, _ = clutch("implicit_stickslip", 1/20000.0)
    print("%-20s %7s %7s %8s %9s %9s" % ("form", "dt", "chatter", "end slip", "neg_slip", "wo(3s)"))
    for form in ("explicit_tanh", "semi_implicit_tanh", "implicit_stickslip"):
        for hz in (60, 120, 240, 480, 1000):
            h, _ = clutch(form, 1.0 / hz)
            fl, es, ns, pk = metrics(h)
            print("%-20s %6dHz %7d %8.4f %9.3f %9.3f" % (form, hz, fl, es, ns, h[-1][2]))
    print("reference wo(3s) at 20 kHz: %.3f" % ref[-1][2])
    print("\nconverter, K=14 rpm/sqrt(Nm), stall ratio 2.2 (explicit vs linearised-implicit pump torque)")
    cref = converter(1/20000.0)
    for impl in (False, True):
        for hz in (60, 120, 240, 480):
            c = converter(1.0 / hz, implicit=impl)
            tail = c[len(c)//2:]
            osc = max(x[1] for x in tail) - min(x[1] for x in tail)
            print("  implicit=%-5s %4dHz  pump speed ripple in 2nd half %.4f rad/s  wt(3s)=%.3f (ref %.3f)" % (impl, hz, osc, c[-1][2], cref[-1][2]))
    print("\nstall torque multiplication check: TR(0)=2.2, turbine/pump torque at first step =", 2.2)
    print("\nshift torque interruption (implicit clutch): speed lost vs analytic LOAD*t/Jo")
    for hz in (120, 240, 480):
        for ts in (0.3, 0.6):
            got, an = shift_transient(1.0 / hz, ts)
            print("  %4d Hz, interrupt %.1f s: lost %.3f rad/s, analytic %.3f" % (hz, ts, got, an))
