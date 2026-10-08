# warzone-5000
An attempt to create a game in the style of Warzone 2100

Documentation lives in [`docs/`](docs/README.md), starting with the [Warzone 2100 research dossier](docs/research/warzone-2100/00-overview.md).

**Time trials.** Vehicles drive a hill-and-valley course in a deterministic simulation (real physics, soft earth that sinks and bogs them,
wheels that roll and legs that walk) and every result says why: see [docs/design/08-time-trial.md](docs/design/08-time-trial.md). Run
`cargo run --release -p w5k_tools --bin w5k -- trial content --out out/trial`, then
`python tools/trial/build.py out/trial out/trial/page.html` and open the page.
