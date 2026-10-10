# ARCH glue gaps (Mule + DRIVE powertrain on the slice course)

Found while building `w5k scenario mule-course` (`crates/w5k_tools/src/cmd/arch_course.rs`, scenario `content/physics/arch/mule_course.ron`).
The glue works with no edits to any lane crate; these are the places where an adapter or a copy stands in for something a lane could offer.
Format follows `docs/swarm/requests/README.md`; ARCH is the author, so each item names the lane that would close it.

## 1. VIEWER / WORLD: the viewer draws flat ground
From: arch   To: viewer, world   Needed by: M1 clip   Status: OPEN
What I need: `w5k world export` (a terrain file) and a viewer that reads `WorldHeader.terrain`. The replay header already has the field; the glue leaves it `None`.
Why: the course has a 14 m hill, so in the clip the truck rises and falls against a flat plane. The physics is right; the picture is misleading.
Meanwhile: the track plot (`track.csv`, column `height_m`) shows the climb.

## 2. CHASSIS: the replay frame builder is private to `chassis.rs`
From: arch   To: chassis   Needed by: next glue change   Status: OPEN
What I need: a public `WheeledChassis::replay_frame(&self, rig, telemetry) -> VehicleFrame` (joints in `PhysRig::joint_names()` order, one contact per station, flags as in `w5k chassis strip`).
Why: ARCH had to copy the 30-line `vehicle_frame` function; two copies will drift when joints or contact flags change.
Meanwhile: the copy in `arch_course.rs` is marked as such.

## 3. DRIVE: fuel is not reported yet
From: arch   To: drive   Needed by: range tests   Status: OPEN
`DriveTelemetry::fuel_used_kg` and `fuel_rate_kg_s` are 0 for the whole run (DRIVE status: fuel map not landed). The scenario prints fuel so it shows up the day DRIVE lands it.

## 4. DRIVE: the automatic box shifts often under a scripted driver
From: arch   To: drive   Status: OPEN (observation, not a bug report)
On the slice road at a 12 m/s cruise (cornering cap 2 m/s^2) the Mule shifts 1<->2 about 20 times in 68 s, mostly down in bends and up on the straights; it never reaches third. Probably the part-throttle shift scaling plus a driver that lifts for every bend. Worth a look at `content/physics/drive/shift_tuning.ron` against the M998-class box once VALIDATION has the dossier numbers.

## 5. WORLD: the slice course has no mud or barricade yet
From: arch   To: world   Status: OPEN (known, in WORLD's next list)
"The whole course" here is the 520 m road from start to finish (hill, 8 % grade limit, bends). The Mule drives all of it. Mud and the barricade will need soft-ground contact (TRACKS/terramech or CHASSIS) before a wheeled truck can be asked to cross them; the glue's stop rules (stall, off-road, cross-track) will say where it stops.

## 6. Glue-owned file location
`cmd/arch_course.rs` is registered by one line in `cmd/mod.rs` and a dispatch arm in `main.rs`' `scenario()` (both ARCH files); `w5k scenario first-light` is untouched.
