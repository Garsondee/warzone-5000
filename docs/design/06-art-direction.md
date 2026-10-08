# Art Direction

*Status: in use by the preview renderer (`crates/w5k_forge/src/preview.rs`), the palettes (`content/materials.ron`) and
the shared design language (`crates/w5k_forge/src/family/style.rs`). Renders:
[army lineup](../assets/forge/lineup_vanguard.png), [factions](../assets/forge/factions_bastion_twin.png).*

## Goal
Vibrant, striking and stylised rather than realistic: a sci-fi army whose units are readable at RTS distance and
instantly recognisable as one faction, whatever their size or shape. Every component is generated, so the look is
encoded in rules that every family follows.

## Colour: palettes built on theory
Each faction palette has eight slots. Geometry names a slot and the palette decides the colour, so team colours are free.

| Slot | Role | Rule |
|---|---|---|
| Primary | dominant body colour (about 60%) | mid value, strong saturation |
| Secondary | supporting colour (about 30%) | a neighbouring hue, much darker |
| Trim | accent (about 10%) | the **complement** of Primary, high saturation and value |
| Glow | emissive energy | a high-energy hue, distinct from Trim |
| Dark | recesses, vents, running gear | near-black **tinted** toward the faction hue, never neutral grey |
| Metal, Rubber, Glass | materials | low saturation, cool |

> **Theory.**
> * **60-30-10** is an old interior-design and illustration rule: one dominant colour, one supporting, one accent. It
>   gives a clear hierarchy, so the eye knows where to look.
> * **Complementary accents** (opposite on the colour wheel: blue and orange, red and green, violet and yellow) produce
>   the strongest hue contrast. Used sparingly, they mark focal points: the chevron on the glacis, the trim around a
>   turret.
> * **Value before hue.** At a distance, and for colour-blind players, units are read by lightness, not hue. So each
>   palette spaces its values widely: dark Secondary and Dark, mid Primary, light Trim, brightest Glow.
> * **Tinted darks.** Shadows and recesses tinted toward the faction hue keep the image in one colour world; neutral
>   grey looks dead next to saturated colour.

The four factions:

| Faction | Primary | Secondary | Trim | Glow |
|---|---|---|---|---|
| Vanguard | cobalt | navy | orange | electric cyan |
| Crimson | crimson | maroon | gold | acid lime |
| Verdant | teal | deep sea green | magenta | lemon |
| Ultraviolet | violet | indigo | yellow | hot pink |

## Light: stylised, not photographic
- **Warm key, cool fill.** Warm against cool reads as depth (warm advances, cool recedes). The sky ambient is cool
  blue from above and a warm bounce from the ground.
- **A rim light in the faction's glow colour** separates silhouettes from the background and ties the lighting to the
  faction.
- **Bloom.** Emissive surfaces are blurred and added back (a tight halo plus a wide glow), so glow strips read as light
  sources rather than paint.
- **Filmic tone mapping (ACES).** Highlights roll off smoothly instead of clipping to flat white.
- **Neutral background.** Saturated units stand out best against a background with little colour.
- **Ink outlines** at silhouettes and depth breaks, in a tinted deep blue rather than black.

The same rules move to the Godot shader in M2: slot colours from a palette texture, an edge flag and ambient occlusion
from the mesh, rim light, emission and bloom from the post-process.

## Shape and design language
Every family decorates its surfaces through `family/style.rs`, so components from different families match:
- **Value split, designed for the top-down view.** The RTS camera mostly sees *top* surfaces, so the two-tone lives
  there: Secondary strips along deck edges, a Secondary rear band on turret roofs, dark running gear underneath.
- **One bold accent per component:** a Trim chevron across a glacis or bow, trim bands on a turret's cheeks, corners
  on a sponson. Never more than about a tenth of the surface.
- **Energy, sparingly:** thin glow lines along a flank or deck edge, glowing sensor eyes, heat glow under dark vents,
  glowing caps on the end road wheels only.
- **Layered plates:** armour plates laid proud of the surface; their chamfers catch the light.
- **Chamfers scale with armour**, so heavy units look heavy.
- **Human-scale fittings** (hatches, cupolas) stay the same size on every unit, so a titan's tiny hatches tell you how
  big it is.

## Scenery: making physics visible
Some quantities have no solid shape, so families draw them as **scenery** (the `scenery` material has no mass, no
armour and no bounds) in the faction's Glow colour:
- rails, ballast and sleepers under a rail bogie, running on past the ends of the vehicle (the vehicle can only go there);
- the field of an anti-gravity pod: a thin beam to the ground and two rings where it lands (the ride height at a glance);
- a rotor's tip-path ring (the disc it sweeps) and, for fliers, a plumb line and downwash ring down to the ground (altitude);
- the glowing seam at the bottom of an air-cushion skirt.
Scenery is capped at one hue (Glow) and kept thin, so the army still reads by its Primary and Trim, and a flier's
altitude or a field's reach is visible without a label.

## Checklist for a new family
1. Body in Primary, lower or supporting surfaces in Secondary; check the top view first.
2. One Trim accent, placed where it marks the front or a focal point.
3. Glow: at most a line and a pair of eyes per component.
4. Dark vents or slats where heat or machinery would be.
5. Render it in all four factions (`w5k factions`) and in the army lineup (`w5k lineup`) next to the others.
