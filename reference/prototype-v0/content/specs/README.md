# Design specs

Hand-written skeletons for the "fun mix" vehicles in `content/vehicles/`. A spec names a hull, a running gear, weapons and
masts with the sliders the author cares about; it leaves the **running gear sizing and the engine** to the auto-fitter.

```
cargo run --release -p w5k_tools --bin w5k -- fit content --spec content/specs/rail_battery.ron --out content/vehicles/rail_battery.ron
```

`fit` iterates until the gear carries its share of the weight and the engine covers the vehicle's draw, lift and the
spare power per tonne it was asked for, then writes the full design. Two fitter hints are not sliders and may appear in an
attachment's `params`: `margin` on a running gear (size it for that many times the load: wider tracks, bigger feet) and `kw_per_t`
on the engine (spare kilowatts per tonne to aim for). Specs are not loaded by the game; `content/vehicles/` is.
