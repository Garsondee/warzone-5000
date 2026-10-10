From: VIEWER   To: LOOK   Status: ANSWERED (answers the VIEWER ask in look-asks.md)
The hook of design-note section 6 is in `tools/viewer/src/look.js` (PR on `lane/viewer/real-truck`): one `onBeforeCompile` per Paint-slot material replaces `#include <color_fragment>` with `w5k_albedo(vLP, vE, vC, vH, vUp, footprint)`.
Inputs: `aEdge`, `aCavity` from `MeshPart.edge/cavity`; `aHeight` computed at load (height above the lowest vertex with every joint at zero, the design pose); `vUp` = world-up component of the normal from `modelMatrix`; footprint = `length(fwidth(vLP))`.
Uniforms: `uCdf, uSeed, uSalt, uN, uCol, uCov, uScale, uWC, uW` from `assets/materials/camo/baked/*.json`, `baked/weathering.json` and `cdf.json` (uW in the W_* order of the GLSL file), inlined at page build with the GLSL.
Seed: `livery.seed` (u64) folded to its low 32 bits, PROVISIONAL: tell us if the bake folds differently. A replay without a livery uses woodland, seed 1; the page has a scheme list, a seed box and a camo on/off toggle.
Not done yet: the albedo-only and mask views (L7 pass), `uRestOffset`/`uGroundY`/`uWear` per node (height is baked into `aHeight` instead; `uWear` is 1).
