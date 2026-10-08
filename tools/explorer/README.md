# Possibility-space explorer

An interactive page over the sampled designs (`docs/assets/forge/space_sample.csv`, made by `w5k space`): pick an axis pair,
colour by hull, gear or weapon, filter, and click a dot for the exact command that rebuilds that design. Python standard library
for the build; Node and Playwright only for the optional image and screenshot helpers.

| File | What it does |
|---|---|
| `template.html` | the page: layout, charts (canvas), filters, inspector. Placeholders `__DATA__` and `__IMG_*__` |
| `build.py` | `python3 -I build.py space_sample.csv template.html out.html [images.json]` fills the placeholders |
| `jpeg.js` | `node jpeg.js images.json atlas=atlas.png showcase=showcase.png corners=space_corners.png` converts renders to JPEG data URLs (via the browser canvas) so the page stays small |
| `shot.js` | `node shot.js out.html shots/` screenshots the page at desktop and phone width, exercises the controls and reports console errors |
| `analyze.py` | `python3 -I analyze.py space_sample.csv` prints the numbers quoted in [the possibility-space doc](../../docs/design/07-possibility-space.md) |

Regenerate everything after a model change:

```bash
cargo run --release -p w5k_tools --bin w5k -- space content --out out --seed 7 --count 3500   # about 8 minutes
cargo run --release -p w5k_tools --bin w5k -- atlas content --out out --seed 7
python3 -I tools/explorer/analyze.py out/space_sample.csv
python3 -I tools/explorer/build.py out/space_sample.csv tools/explorer/template.html out/possibility_space.html
```

The same seed gives a byte-identical CSV, so a change in the numbers means a change in the model.
