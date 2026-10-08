# Parametric Components

*Status: first family (gun turret) prototyped in `crates/w5k_forge/src/family/`. Renders:
[sweep](../assets/forge/turret_gun_sweep.png), [coupling](../assets/forge/turret_gun_coupling.png).*

## The idea
Instead of a tech tree full of fixed parts ("75 mm turret", "88 mm turret", "heavy 88 mm turret"...), each component is
a **family with sliders**. One "gun turret" card covers everything from a rotary machine gun to a battleship gun; the
player decides where on that range their design sits. This gives:
- **fewer tech items**: cards unlock families, materials and mechanisms rather than numbered variants;
- **far more choices**: the possibility space is continuous;
- **a real design problem**: sliders compete, so every design is a balance the player chose.

## How a family works
A family is a **generator**: slider values in, an ordinary part out (shapes, sockets, function). Everything downstream is
unchanged. The forge meshes the part, measures its mass, inertia and internal volume from voxels, and fires rays for its
armour table ([03-part-forge.md](03-part-forge.md)). So **stats stay honest**: the armour slider makes the plates
thicker and the turret squatter, and the armour rating rises *because the rays see thicker, steeper plates*. No
formula says "armour +10%".

```
sliders --(family generator)--> shape tree --(forge)--> mesh + mass + inertia + armour table --(family)--> performance
```

The family also turns the slider values plus the measured part into **performance figures** (penetration, reload,
traverse time...).

## Three kinds of sliders
| Kind | Behaviour | Examples |
|---|---|---|
| **Free** | A matter of taste or an even trade. The solver never moves it. | slope (protection per mm against interior room), style |
| **Budgeted** | Competes for the part's budget (mass). With *hold mass* on, moving one makes the unlocked others give way. | calibre, barrel length, armour, loader, ammunition |
| **Opposed** | Two quantities linked by physics, so raising one directly costs the other. Not a separate mechanism: it falls out of the generator. | calibre against reload time; barrel length against traverse speed |

### The coupling solver (`family::set_slider`)
When the player drags a budgeted slider with *hold mass* on:
1. The budget is the part's current mass, estimated quickly from exact piece volumes (no voxels).
2. Every **unlocked** budgeted slider shifts by the same amount of slider travel. Each moves in whichever direction
   compensates (lighter if the moved slider made the part heavier, heavier if it freed mass). The amount is found by
   bisection, since mass changes monotonically with the shift.
3. If the others run out of travel, the moved slider **stops where the budget runs out**.
4. Locked sliders never move. If everything else is locked, the part simply gets heavier.

Measured in the prototype: starting from a 4.4 t, 75 mm turret, asking for 120 mm with everything free gives 120 mm
with 5 mm armour and 7 rounds. With armour locked, the calibre stops at 115 mm, the barrel shrinks to 15 calibres
(a stubby howitzer) and penetration falls from 70 to 43 mm. Mass is held within 0.5%.

> **Theory: why equal slider travel?** It is the simplest rule a player can predict. Every free slider gives up
> the same *share of its range*. Proportional-to-cost rules make the cheapest slider collapse first, which feels
> arbitrary. We can revisit this once players use it; the solver is twenty lines.

## Physics inside the gun turret family
The relationships are real scaling laws, so the trade-offs are ones a player can reason about:

| Quantity | Law | Why | Check |
|---|---|---|---|
| Shell mass | m = 7 kg x (d / 75 mm)^3 | geometric similarity: double the bore, eight times the shell | 7.62 mm: 7 g; 406 mm: 1.1 t (both close to real) |
| Muzzle velocity | v = 1200 (1 - e^(-L/35)) m/s, L in calibres | gas does work along the barrel with diminishing returns | L45: 870 m/s |
| Gun mass | 550 kg x (E / 1 MJ)^0.9 | a gun must contain the energy it releases | 75 mm: 1.3 t; 406 mm: 140 t (Iowa: 121 t) |
| Penetration | de Marre: P ~ v^1.43 m^0.71 / d^1.07 | empirical, from naval armour trials | 7.62 mm: 11 mm; 75 mm: 100 mm; 406 mm: about 600 mm |
| Velocity at range | v e^(-x / λ), λ grows with m / d^2 | sectional density: heavy, thin shots keep their speed | a rifle round loses ~90% by 1 km; a 406 mm shell ~5% |
| Reload | action cycle + handling time x m^0.85 | humans handle a few kg/s, machines far more | 75 mm by hand: 4 s; 406 mm by hand: minutes |
| Recoil | J = 1.3 m v | momentum of shot and gases | sets a minimum hull mass (below) |
| Turret size | smallest shape whose interior holds crew, breech recoil space, ammunition and loader | found by bisection, using the geometry kernel to compute exact interior volume | more slope or armour gives less room, so the turret grows |
| Traverse | t180 = 2 sqrt(pi I / tau), tau ~ ring area | constant-torque rotation, accelerating half way and braking half way; I is measured from the geometry | 75 mm: 4 s; 406 mm: about a minute (Iowa: about 45 s) |

### Gates that fall out of the physics
- **Recoil sets the hull you need.** A hull must not be kicked back faster than about 0.5 m/s, so the minimum hull mass is
  J / 0.5. That works out to about 20 kg for a machine gun (a drone can carry one), about 16 t for a 75 mm gun, and
  about 2,600 t for a 406 mm gun. That last figure is why railway guns existed, and why a battleship gun needs a train or
  a titan.
- **Turret ring diameter** grows with what the turret holds. A hull's ring socket limits it, so a big gun needs a big hull.
- **Traverse time** grows with turret inertia. Small, fast units can outrun a heavy turret's tracking by circling it.

## Making size visible
- Crew fittings (cupolas, hatches) are **human-sized** whatever the turret's scale, so a battleship turret's tiny
  hatches tell you how big it is.
- Chamfers grow with plate thickness, and heavy armour also makes the body squatter, so heavy armour *looks* heavy.
- Small rotary guns get a remote station with a sensor head; crewed turrets get cupolas; big guns get muzzle brakes and fume
  extractors.

## Armour semantics (shared with all parts)
- **Hulls and turrets have vital interiors**: a shot that reaches the inside counts as a penetration. Legs, wheels and
  masts are hollow but not vital.
- **Materials can be non-protective** (`armour: false`): guns, engines and electronics. A shot hitting them damages
  that component, so it is not counted as armour. Without this, a ray down the length of a barrel would read as
  20 m of steel.

## Next families
Hull (size, length:width, armour per face, internal volume, ring capacity); engine (power, type: diesel, turbine,
electric; mass, fuel and heat); locomotion (track width, wheel count and diameter, leg count and length, rail bogies); sensor
(mast height, radar against optics). Each gets the same three kinds of sliders and one family test like
`crates/w5k_forge/tests/family.rs`.

## Open questions for the owner
- Should *hold mass* be the default, or should players switch it on per slider group?
- Should slope stay free, or should armour automatically push the shape toward sloped and squat (the first brief
  read both ways)?
- How many sliders per component before it stops being fun? The turret has six; that may be near the limit.
