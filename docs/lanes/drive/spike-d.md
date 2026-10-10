# Spike S-D: stiff couplings (lane DRIVE)

Code: `spikes/drive/spike_d.py` (stdlib Python, throwaway). Output: `docs/lanes/drive/spike-d-output.txt`. Re-run: `python3 -I spikes/drive/spike_d.py`.

**Setup.** Engine flywheel `Je = 0.35` kg m^2, output inertia `Jo = 1.2` kg m^2 (a vehicle reflected through first gear; ESTIMATE), clutch capacity 450 N m,
road load 60 N m, flat 300 N m engine cut at the redline. The clutch snaps shut at launch with a 84 rad/s slip (the worst case). Reference: the same run at 20 kHz.

## Result
| Formulation | Stable at | Behaviour |
|---|---|---|
| Explicit regularised sign `T = cap tanh(slip/eps)` | **never**, up to 1 kHz | torque flips sign every step (chatter count grows with rate), slip goes negative (the clutch *creates* energy) |
| Same, linearised implicitly | 1 kHz only (480 Hz still chatters) | fine in principle, but not at the rates we can afford |
| **Implicit stick/slip** (compute the torque that would lock both shafts at the end of the step, clamp to +-capacity; stuck when the clamp is inactive) | **every rate, 60 Hz to 1 kHz** | zero chatter, zero negative slip, locks exactly; final speed within 0.7% of the 20 kHz reference even at 60 Hz |
| Converter, `T_pump = (w/K)^2`, `T_turb = TR T_pump`, explicit | every rate | smooth (the torque *falls* as slip falls, so it is not stiff); 60 Hz error 9%, 240 Hz 0.5% |
| Converter with the pump torque linearised (`dT = 2 T/w dw`, solved implicitly) | every rate | 60 Hz error 1.1%, 240 Hz 0.3% |
| Shift (torque interruption of 0.3 and 0.6 s) | every rate | speed lost during the interruption is exactly `load * t / J` (15.000 and 30.000 rad/s, 120 to 480 Hz) |

## Findings
1. **No microsecond steps, no substepping inside DRIVE.** One implicit stick/slip evaluation per chassis substep (>= 120 Hz) is enough for the clutch, the lock-up and the
   shift engagement. The converter needs its linearised-implicit form; it is cheap (one division).
2. **Why.** A friction clutch is a stiff element: its torque depends on slip with a slope `cap/eps`, so an explicit step is stable only when `dt cap/(eps J_eff) < 2`
   (here `J_eff = Je Jo/(Je+Jo) = 0.27`, `eps = 0.5` gives `dt < 0.3 ms`). The implicit rule removes `eps` altogether: Coulomb friction is solved as a *constraint* (lock if the
   locking torque is within capacity, otherwise slip at capacity). Graphics analogy: it is a contact solver with a friction cone, not a spring.
3. **The shift transient** is the torque interruption itself; the lost speed is `T_load t_shift / J_reflected` (the vehicle coasts). The real shift cost is therefore set by
   `shift_time_s` and the road load, not by the numerics.
4. **Caveat carried into the design.** The spike's load is a constant. With the real chassis the locked driveline sees a speed-dependent load; the lock formula must use
   `ShaftState::load_stiffness_nm_s_rad` (it is in the contract for exactly this) so that the lock-torque estimate includes `dT_load/dw`. Not yet exercised: do it in the lumped bench.
5. Not done in this round: a three-inertia chain (engine, converter turbine, vehicle with a compliant driveline) and a PNG of the traces (no plotter yet; VIEWER's plotter is pending).
