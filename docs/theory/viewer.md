# Theory: the replay as a G-buffer for time

*(Written in stages; this is the first, about how a replay is stored. Later PRs add skeleton, interpolation and the wagon-wheel effect.)*

**A replay is written once and read by many passes**, the way a G-buffer is rendered once and read by lighting, fog and post-effects. The simulation is the expensive
geometry pass; the browser viewer, the Godot player and the validation harness are the cheap passes that only read. That is why the viewer is not allowed to compute physics: anything it invented would disagree with the other readers.

**Quantisation is a precision budget.** Storing every number as a 64-bit float would be wasteful: a position that is wrong by half a millimetre is invisible, so we store whole millimetres.
The budget is set by what you will *do* with the number. Millimetres for position; 0.1 milliradian (2 pi over 65536) for an aiming angle, because a gun-laying error plot has to resolve errors that small.
Rotation uses **smallest-three**: a unit quaternion has four numbers but they must satisfy w^2+x^2+y^2+z^2 = 1, so the largest can be recomputed from the other three; we store which one it was and the three small ones (all within +-0.707).
Resolution: 16 bits over a range of 1.41 is a step of 2.2e-5, which is 0.003 degrees; 10 bits would be a step of 1.4e-3, about 0.14 degrees, visibly wobbly on a long gun barrel. (The test `rotation_round_trips_within_0p01_degrees` guards this.)

**Delta coding is prediction.** Consecutive frames are alike, so store only the *surprise*. If the wheel turned 0.5 rad last frame, it probably turns about 0.5 again: predict
"same step as last time" and store the error, which is near zero. That is the same idea as video P-frames (and PNG's filters). Where motion is smooth we predict by extrapolating the last two frames; where values are noisy (tyre forces) the
extrapolation would amplify noise, so there we predict "same as last frame". Small numbers need fewer bytes in a varint (7 bits per byte), and runs of unchanged values collapse to two bytes. A keyframe every second restarts the prediction from zero so a viewer can jump anywhere.
