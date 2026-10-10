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

## A vehicle is a skeleton
The hull, the turret, the gun and every wheel form a **joint tree**, exactly like a character rig. Each node stores where it sits when its joint is at zero (the rest pose); each frame supplies one number per joint:
an angle for a spinning or steering node, a distance for a sliding one (suspension travel, recoil). Forward kinematics walks the tree from the hull outwards: *travel first, then steer, then spin*.
A wheel's world position is the hull pose, moved down the suspension arm by the travel, turned by the steer angle, and only then spun. Swap the order and the wheel would spin about a steering axis that has not turned yet.
The viewer does nothing else; it never decides where a wheel is, it just evaluates the chain the simulation recorded. `smoke.mjs` proves each joint is wired by moving it alone and watching the pixels (or, for a sliding joint, the exact distance).

## Interpolation: why a viewer can run at any frame rate
The file stores 30 snapshots per second but the screen may refresh at 60 or 144 Hz, and a video may be rendered at 24. The viewer blends the two nearest snapshots: positions and joints linearly, rotations by **slerp** (spherical
interpolation: it moves along the shortest arc of the rotation sphere at constant angular speed, where a plain lerp of quaternion numbers would speed up and slow down). Because wheel spin is stored *unwrapped* (never reset at 2 pi), linear blending is correct; a wrapped angle would sweep the wheel backwards through a whole turn between two frames.

## The wagon-wheel effect is aliasing, not a bug
A wheel turning 11 rad/s advances 0.37 rad (21 degrees) per 30 Hz frame. If a wheel has four identical spokes (symmetry every 90 degrees) and it turns 85 degrees between frames, the eye reads it as 5 degrees *backwards*: with identical spokes, "advanced 85" and "went back 5" look the same. This is the sampling theorem at work, the same reason a texture shimmers when
sampled too coarsely: a signal can only be recovered if you sample at more than twice its highest frequency. The data in the file is right (the spin angle is stored unwrapped); the picture is what any 30 Hz sampling of a symmetric rotating object gives. That is why the box wheels carry one asymmetric lug: it makes direction readable, and the viewer can render extra in-between frames from the interpolated spin if you want a smooth wheel.
