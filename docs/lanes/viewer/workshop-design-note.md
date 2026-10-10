# Workshop page: design note (slice 2 stage C, text only, nothing built yet)

*VIEWER, 2026-10-10. Answers `docs/swarm/SLICE-2.md` section 6, acceptance 6: "change any of six levers and the body and scoreboard update in under 10 s, and the changed vehicle can be driven." Written before any code so ARCH, FORGE, GEOMETRY and VALIDATION can object; the interface requests are listed at the end and are filed when building starts.*

## What the owner sees
Sliders on the left (wheelbase, tyre size, spring rate, engine power, mass; track width joins when the carrier lands), the vehicle in the middle reshaping as a slider moves, a scoreboard on the right (the proving-ground rows with their traffic lights), and a big DRIVE button that takes *that exact vehicle* onto the course. After each change one sentence says what moved and why (the principle: **change, number, reason**), taken from FORGE's compile report and the force ledger, not written by hand. Each slider shows its band and its provenance label (`SPEC`, `ESTIMATE`, ...), so a number that is only a guess looks like one.

## What already exists (read from the code, and timed)
| Piece | Today | Source |
|---|---|---|
| Slider limits | every `Param` of a `VehicleDef` carries `lo` and `hi`; the band is the slider | `w5k_contract::def`, `content/vehicles/game/*.ron` |
| Levers to fields | wheelbase = last axle `from_front_m` minus first; tyre size = `tyre.outer_diameter_m` with `section_width_m`; spring rate = FORGE's `ride_frequency_hz` slider (`k = m (2 pi f)^2` is derived and shown); engine power = `peak_power_w`; mass = `hull.mass_kg`; track = axle `track_width_m` | `def.rs`, FORGE brief |
| Body follows the definition | `Skin::from_def(&VehicleDef)`: box, axles, track and tyre are the definition's (wheeled only: it refuses a tracked def) | `w5k_geo::skin` |
| Skin generation | `Skin::parts` (the shape itself) takes **under 10 ms**; the *flag bake* (LOOK's edge and cavity values, ray casting) is what costs: **4.9 s at 256 rays (the default), 1.4 s at 64, 0.4 s at 16** for the hauler on a release build; the cost is proportional to rays times vertices, not to the detail level | timed here: `FlagParams::cavity_rays` |
| Scoreboard | the proving-ground battery for the Mule (braking, 0-48 km/h, gradeability, skidpad, side slope, step) **0.47 s** in total on a release build, loading and compiling the definition included | `w5k scenario proving --test all` |

The long pole is the bake, not the physics and not the geometry. On the repository's dev profile the same skin takes about 15 s (three times slower), so the Workshop must be tried against a release `w5k drive`, which is what the player runs.

## Design: three layers of answer, fastest first
This is the viewport-proxy idea from a render pipeline: show something right immediately, replace it with the real thing when it is ready.
1. **Instant (0 ms, in the browser).** The page re-fits the skin it already has (scale the body to the wheelbase, scale the wheels to the tyre; `fitSkin` in `tools/viewer/src/skin.js` does both already). A stretched box, but it moves under the finger.
2. **Proxy (about 0.5 s, from the server).** A real skin generated from the new definition with a 16-ray bake (0.4 s) replaces the stretch.
3. **Final (about 5 s, from the server).** The 256-ray bake, swapped in without a jump when it arrives.

The numbers beside the sliders use the same rule: the old value stays, greyed, with "updating", until the new one arrives; a stale number is never shown as current. Every request carries a sequence number and a late reply for an older design is dropped.

## Interface (the server is ARCH's; I only draw)
- `GET /api/design/<base>`: the levers of a base vehicle, `[{id, name, unit, value, lo, hi, prov, derived}]`.
- `POST /api/design` `{base, levers: {wheelbase_m: 3.1, ...}}`: clones the definition, sets the parameters (**out of band is an error with a reason, never a silent clamp**), compiles through FORGE and answers within tens of milliseconds with `{id: "design-7", report}`; the proxy skin, the final skin and the scoreboard then run in the background.
- `GET /api/design/<id>/skin?quality=proxy|final`: my compact `.skin` bytes (`w5k_replay::skinpack`).
- `GET /api/design/<id>/score`: the proving rows as they finish, `{id, name, value, unit, light, oracle_or_source}`, the same rows the dashboard shows (one scorer, two views).
- `POST /api/select {vehicle: "design-7"}`: the existing call, accepting design ids, so DRIVE is the real simulation on that exact vehicle (no second model).
Designs live in server memory only; a new session starts from the base. One page, not two: the start screen gets a fourth card, "Build your own", that opens the Workshop panel and shares the camera, skin and drive code of the live page (the package script already ships `index.html` and `skins/`; a second page would need a second entry).

## Budget and the test that enforces it
First useful update about 1 s (compile and scoreboard 0.5 s, proxy skin 0.4 s); final body about 5.5 s; DRIVE about 0.1 s. That meets "under 10 s" with room for a slower PC. The test is `workshop_change_updates_body_and_scoreboard_in_under_10_s_and_the_design_drives`: headless Chromium against a real `w5k drive`, move each of the six sliders in turn, assert that the skin hash and every scoreboard row changed where physics says it must (and did not where it must not), then press DRIVE and assert the vehicle moves. It extends `live-smoke.mjs`.

## Open questions (default taken, `PROVISIONAL`)
1. **Does mass keep ride frequency or spring rate?** Default: frequency (it is the designer's handle), spring rate re-derived and displayed. A heavier truck then rides the same and sags the same fraction.
2. **What moves with the wheelbase?** Default: the rear axle moves, the centre of mass keeps its fractional position along the wheelbase, the hull length grows by the same amount. FORGE owns this; the weight split it implies must show on the scoreboard.
3. **One slider for tyre size** scales diameter and section width together (aspect ratio fixed); the Impact Matrix keeps width as its own lever.
4. **Tracked vehicles** join after stage B: `Skin::from_def` refuses them today, so the carrier needs GEOMETRY's track skin from a definition first. Until then the Workshop base list is the wheeled trucks.
5. **Bake quality knob.** `cavity_rays` is a field of `FlagParams`, which GEOMETRY owns; the proxy needs to set it per call.

## Requests I will file when building starts
ARCH: the endpoints and the in-memory design registry. FORGE: one lever API (id, field, band, derived read-outs, compile report) shared by the Workshop and the impact runner, so a slider and an impact lever are the same object. GEOMETRY: `Skin::from_def` with a `bake quality` argument, and the carrier from a definition. VALIDATION: the scoreboard as a function from a compiled design to rows, not only a dashboard file.

## Theory for the owner (to move into `docs/theory/viewer.md` when built)
A slider is a parameter of a function: `design(sliders) = (body, numbers)`. The page is only a *view* of that function, which is why the body and the numbers must come from the same definition and the same code that drives; two models would drift. The three-layer answer is the same trade a viewport makes between a proxy and a final render: latency is a feature, and the honest way to hide it is to show the cheap answer first and label it as cheap.
