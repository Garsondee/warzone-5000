# Glossary

Plain-language definitions of the terms the briefs and tests use. *Graphics* notes are for the owner's benefit.

## Vehicle dynamics
- **Sprung / unsprung mass**: the hull and everything the springs carry (sprung) vs the wheels, hubs and arms that move with the ground (unsprung). A light wheel follows bumps better. *Graphics: the sprung mass is the camera rig, the unsprung mass is the camera dolly's wheels.*
- **Ride frequency (Hz)**: how fast the body bobs on its springs when pushed. Cars 1-1.5 Hz, trucks 1.5-2.5, tanks 1.2-2. Designers choose it; the spring rate follows from `k = m (2 pi f)^2`.
- **Damping ratio (zeta)**: how quickly the bobbing dies out; 0 = never, 1 = no overshoot (critical). Vehicles sit at 0.2-0.4 so they respond briskly without wallowing.
- **Bump stop**: a stiff rubber block that takes over when the suspension runs out of travel; hitting it hard is "bottoming out".
- **Wheel rate**: the spring force at the wheel per metre of wheel travel, after leverage. A torsion bar acting through an arm has a wheel rate that changes with arm angle.
- **Torsion bar**: a steel bar twisted by the road-wheel arm; the spring of most tanks. **Hydropneumatic**: a gas spring (nitrogen behind oil); it gets stiffer the more it is squeezed (`p V^gamma = const`).
- **Tyre slip ratio**: how much faster or slower the wheel surface moves than the ground: `(wheel speed - ground speed) / ground speed`. **Slip angle**: the angle between where the tyre points and where it travels; it produces cornering force.
- **Friction circle / ellipse**: the total grip of a tyre is limited; braking and cornering share it. Ask for both at once and the tyre gives less of each.
- **Relaxation length**: the distance a tyre must roll before its force builds up. It also stops the numerical jitter of a stopped vehicle.
- **Penalty contact**: a contact modelled as a very stiff spring and damper that pushes back in proportion to how far two things overlap. Simple and visible (you can plot every force) but it needs small time steps. *Graphics: the soft constraints of cloth simulation.*
- **Torque curve**: the engine's full-throttle torque against rpm. Power = torque x angular speed.
- **Gear ratio**: input speed divided by output speed. A low (high-numeric) gear multiplies torque and divides speed.
- **Torque converter**: a fluid coupling between engine and gearbox that slips smoothly and multiplies torque at low speed (the stall ratio).
- **Differential**: lets the two wheels of an axle turn at different speeds in a corner. **Steering unit**: how a tracked vehicle makes its two tracks run at different speeds (clutch-brake, controlled differential, hydrostatic).
- **Skid steering**: turning by driving the two sides at different speeds, so the tracks slide sideways. Costs power; the turning resistance is roughly `mu W L / 4`.
- **Brake fade**: brakes lose grip when they get hot; the thermal model is heat in (braking power) minus heat out (cooling, which grows with speed).
- **Ground pressure**: weight over contact area, kPa. Tracks float on soft ground because the area is large.
- **Bekker pressure-sinkage**: how far a load sinks into soil: `p = (kc/b + kphi) z^n`. **Mohr-Coulomb**: soil gives way when shear stress passes `cohesion + pressure x tan(friction angle)`. **Janosi-Hanamoto**: soil needs to be sheared a certain distance before it delivers its full thrust. Together these decide whether a vehicle gets through mud.
- **Cone index**: the standard field measure of soil strength (a cone pushed into the ground); military mobility tables use it.
- **Pitch / roll / yaw**: nose up-down, lean left-right, turn left-right. See `UNITS-AND-FRAMES.md` for our signs.
- **Stabiliser**: a gun drive that cancels hull motion so the gun stays on target while the vehicle moves. **Recoil**: the barrel slides back against a spring and damper; the reaction acts on the hull.

## Simulation and engineering
- **Fixed step / substep**: the simulation advances in equal time slices (60 Hz outer tick, each vehicle splits it into 4-8 substeps). A stiff spring needs about 20 steps per period of its fastest oscillation.
- **Semi-implicit (symplectic) Euler**: update velocity first, then position using the new velocity. It conserves energy far better than plain Euler at the same cost.
- **Force ledger**: a record of every force term (spring, damper, tyre, engine...) on every body, every tick. *Graphics: render passes (AOVs): the beauty pass is their sum, and you can look at each alone.*
- **Determinism**: the same inputs give bit-identical outputs on every machine. We use plain floats with discipline (`libm`, fixed step, no fast-math). A **golden** is a stored hash of a run that tests compare against.
- **Analytic oracle**: a test that compares the simulation with a closed-form answer (a spring's natural frequency, braking distance `v^2 / 2 mu g`). The strongest kind of test, because the answer does not come from the code under test.
- **Calibration set / held-out set**: the vehicles we tune global constants on, and the vehicles we only measure against. Tuning on the held-out set would be cheating. *Statistics: train and test sets.*
- **Provenance** (`SPEC / MEASURED / ESTIMATE / TUNED`): where a number comes from. **Envelope**: the range of results the simulation gives when its estimated inputs vary within their bands.
- **Design Impact Matrix**: a table of design levers against benchmarks with the expected sign and size of each effect; it is checked by finite-difference runs.
- **Dead lever / orphan effect**: a design choice that changes nothing / an effect that no design choice can influence.

## Project
- **Lane**: a domain agent (a Claude session) that owns a set of paths. **ARCH**: the coordinator. **Squad**: lanes that work closely (dynamics, world, design, visuals, platform, truth, behaviour).
- **Contract**: a versioned interface (`w5k_contract`) that lanes build against. **CCR**: contract change request. **Stand-in**: a simple fake of a neighbour's part (a flat plane, a constant-torque engine, a canned replay) so a lane can start before the real part exists. *Graphics: the checkerboard test cube.*
- **Port**: a contract between two physics lanes (powertrain to running gear; suspension; ground contact).
- **PhysRig / RenderRig / VehicleDef**: the solver-level description of a vehicle / its meshes tagged by articulation node / the designer-level description it is compiled from.
- **Replay**: the recorded frames of a run. Viewers are pure functions of a replay. *Graphics: a G-buffer for time.*
- **First light**: the integration scenario (one real truck, design to physics to replay to viewer to validation) that runs on every merge; also milestone M1.
- **Decision card**: a one-paragraph question with options, a recommendation, the cost of being wrong and a default. **PROVISIONAL(card)**: work proceeding on a default.
- **Design note**: the one-page plan a lane's first PR contains. **Handoff note**: what a lane writes when it stops.
- **Port ledger**: the list of what to re-read from the old prototype and what to drop.
