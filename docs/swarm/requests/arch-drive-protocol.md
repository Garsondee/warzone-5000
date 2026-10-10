From: ARCH   To: VIEWER (builds the page against this)   Needed by: first test-drive   Status: OPEN
What I need: a web page (served from `--web`, default `viewer/` next to the exe, then `tools/viewer/dist`) that lets a five-year-old drive. It talks to `w5k drive` over the small JSON protocol below.
Why: the owner's goal is that their son can test-drive a vehicle. `w5k drive` runs the REAL simulation (FORGE rig, DRIVE powertrain, CHASSIS, WORLD course) in real time on the player's PC; the page only draws and sends the pedals.
What I will do meanwhile: ship the server (`crates/w5k_tools/src/cmd/arch_drive*`), a test that greps this file for every key the server emits, and keep this file the single source of truth.

```
w5k drive --vehicle content/vehicles/game/mule_4x4.ron [--vehicles-dir content/vehicles/game]
          --course content/world/courses/slice.ron [--web DIR] [--port 8787] [--open] [--no-assist] [--record DIR]
```
Run it from the folder that holds `content/`. It binds `127.0.0.1` only and prints the URL (`--open` also starts the default browser). Same origin: the page is served by the same server, so no CORS. No Node is needed at run time: `--web` is a folder of prebuilt static files (`index.html` is served for `/`). Every API reply is `application/json` (the stream is `text/event-stream`); errors are `{"error": "text"}` with a 4xx status.

## Units and frames
SI, as `docs/architecture/UNITS-AND-FRAMES.md`: metres, seconds, radians, newtons. Right-handed, +Y up, -Z forward, +X right. `rot` is the hull quaternion `[w, x, y, z]`. Steering `+1` is right (as `Command`). Joint angles are radians, travel is metres.

## Requests
- `GET /api/vehicles` -> `[{"id": "scout_4x4", "name": "Scout 4x4", "mass_kg": 1210.0, "wheelbase_m": 2.4, "skin": "scout_4x4"}, ...]` for every `*.ron` (not `*.extras.ron`) under `--vehicles-dir` that compiles and that the wheeled chassis accepts, sorted by id. `skin` is the look id the page should use for the vehicle (today equal to `id`).
- `GET /api/rig/<id>` -> the `RenderRig` JSON of that vehicle (the same as `rig.json` from `w5k scenario mule-course`). Joint order of every stream frame is `PhysRig::joint_names()` (spin of each station, steer of each steered station, travel of each station).
- `GET /api/world` -> `terrain.json`, format `w5k-terrain-1` (see `world-viewer-terrain.md`), of the course.
- `POST /api/select` body `{"vehicle": "scout_4x4"}` -> `{"ok": true}`. Resets the simulation with that vehicle at the road start (unknown id: 404 `{"error": ...}`).
- `POST /api/input` body `{"throttle": 0..1, "brake": 0..1, "steer": -1..1, "reset": false, "reverse": false}` -> `{"ok": true}`. The body replaces the whole input (an absent field counts as 0 / false); the last body wins and is held until replaced. `steer` +1 is right. `reset: true` puts the vehicle back on the nearest road point (one shot). `reverse: true` asks for the reverse gear while stopped or crawling (hold it with throttle to back up). If no input arrives for 0.5 s the pedals count as released: under assist the vehicle brakes gently to a stop.
- `POST /api/finish` -> `{"ok": true, "file": "DIR/replay.w5kr"}` writes the recording (`file` is `null` without `--record`). The server also writes it when the last stream client has been gone for 10 s.
- `GET /api/stream` -> `text/event-stream`, one frame about every 1/30 s: `data: {json}\n\n`. A new client starts at the next frame; frames are not skipped for a client that keeps up.

## Stream frame
```
{ "t_s": 12.35,                    // simulated seconds since the vehicle was (re)selected
  "vehicle": "scout_4x4",
  "pos_m": [x, y, z],              // hull datum, the origin of the render rig
  "rot": [w, x, y, z],
  "joints": [ ... ],               // rad / m, order of PhysRig::joint_names()
  "engine_rpm": 2100.0,
  "gear": 3,                       // 0 neutral, positive forward, negative reverse
  "speed_m_s": 6.2,                // along the nose; negative when reversing
  "contacts": [ {"name": "axle1.l", "normal_n": 3900.0, "in_contact": true}, ... ],
  "assist": {"on": true, "message": "speed limit"},   // message: "" or a short status of the assist that is acting
  "message_event": "back on the road"                 // optional: only in the frame after an event, show it for a moment
}
```
`assist.message` is one of `""`, `"speed limit"` (the top-speed limiter is holding the throttle back), `"stopping"` (no pedal: the auto-brake is holding). `message_event` is `"back on the road"` after an auto-recover or a `reset`.

## Kid assists (default on; `--no-assist` turns them off for adults)
All numbers are in `content/physics/arch/drive_assist.ron`. Speed cap 25 km/h by easing the throttle off; throttle ramp (the pedal cannot jerk); steering authority and rate shrink with speed; automatic gearbox always; no pedal means a gentle automatic brake and no creeping; auto-recover when the hull has been rolled over for 1 s, when it has been stuck for 3 s with the pedal down, or when it leaves the terrain.

## Page ideas (not binding)
Arrow keys / WASD, the Gamepad API (right trigger = throttle, left trigger = brake, left stick x = steer) and big on-screen buttons all end in the same `POST /api/input`; send it on change and at least every 0.25 s while a key is held. Keep a "reset" button always visible. Show `message_event` as a large friendly banner.

--- ARCH answer: (ARCH is the author; VIEWER please reply here if a field is missing.)
