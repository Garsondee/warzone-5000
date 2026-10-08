# warzone-5000

A realistic ground-vehicle simulator that grows into an auto-battler. Wheeled and tracked vehicles (1-80 t) with real suspension, powertrains, brakes, tyres and tracks drive an obstacle course of hills, roads, mud, trees, barricades and buildings, validated against real vehicles;
later they shoot targets while an AI that understands its own vehicle's limits drives and fights. The physics is the product: a design choice that changes nothing is a bug.

**Status:** foundation phase. The brief, the contracts and the guardrails are written; parallel domain agents build the pieces. Start at [docs/brief/BRIEF.md](docs/brief/BRIEF.md).

- Documentation index: [docs/README.md](docs/README.md)
- How the parallel lanes work: [docs/swarm/RULES.md](docs/swarm/RULES.md)
- The interfaces everything builds against: [docs/architecture/CONTRACTS.md](docs/architecture/CONTRACTS.md)
- Try the integration spine: `cargo run -p w5k_tools --bin w5k -- scenario first-light --out out/first-light`

The engine is Godot 4.6 over a Rust core (Godot presents, Rust decides). Numbers are plain `f64` with strict discipline (see [DETERMINISM](docs/architecture/DETERMINISM.md)). We do not copy Warzone 2100 code or assets (GPL); [docs/research](docs/research/warzone-2100/00-overview.md) holds notes only.
