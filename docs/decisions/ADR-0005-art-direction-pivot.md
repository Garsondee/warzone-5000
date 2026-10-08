# ADR-0005: Realistic art direction

**Status:** Accepted (owner brief, 2026-10-08). Supersedes `06-art-direction` in the archived prototype docs.

## Context
The prototype's look was vibrant and stylised: eight-slot palettes, complementary accents, chamfered vertex-coloured shapes ("cute"). The
owner wants "less toy/cute colours, more realistic camo, more realistic hull geometry, and more complex relationships between parts of a vehicle
like the turret rotating whilst the cannon pitches".

## Decision
1. **Geometry:** realistic hull and turret shapes built from lofted cross-sections and extrusions (lane GEOMETRY), proportions checked against the dossier dimensions (within 3%). One geometry source feeds the look, the mass and inertia, the armour thickness and the collision proxies.
2. **Look:** procedural PBR with triplanar camouflage (NATO three-tone, woodland, desert, flecktarn, Soviet green...) and weathering (edge wear, dirt in cavities, splash from the soil actually driven through), driven by per-vertex edge and cavity flags; no UV unwrapping. Lane LOOK owns it, in the reference viewer and in Godot shaders.
3. **Articulation:** the vehicle is a node tree (hull > travel > steer > wheel; hull > turret > gun pitch > recoil), animated entirely from joint coordinates in the replay; tracks animate from belt speed.
4. **Factions** are told apart by camouflage and markings, not by neon palettes. The camera is nearer and the lighting more natural.
5. Hand-modelled hero assets are optional later, supplied by the owner; nothing depends on them.

## Consequences
The neon palettes, bloom-heavy lighting and chamfered vertex-colour kit are archived. `docs/art/` (lane LOOK) holds the new art-direction document.
