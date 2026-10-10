# Theory notes: LOOK (paint, wear, light)

**Camouflage as thresholded noise.** A camo pattern is what you get from Photoshop's *Clouds* filter followed by *Threshold*: a smooth random field
cut at a level. Cut at level L and the area below L is whatever fraction of the field lies below L. If you first *equalise the histogram* (map every
value through the field's own cumulative distribution, the same operation as Photoshop's Equalize), the field becomes uniform on 0..1 and the
cut level **is** the coverage: threshold 0.45 paints 45% of the surface. Three colours need two independent fields, because one field gives nested
bands (green always rings brown always rings black), which no real scheme does. We therefore pick colour 1 from field 1 at 45%, and among what
is left pick colour 2 from field 2 at 0.35 / 0.55, and so on.

**Why not blend the colours, and why not triplanar.** Blending colours gives mud between tones; blending the *field* and cutting afterwards gives
crisp edges at any blend amount. Triplanar projection (three flat noise lookups blended by the surface normal, like box mapping in a DCC package)
avoids UVs, but it averages three independent fields, and an average of independent things varies less than any one of them: the tails of the
histogram shrink, so a threshold meant to select 10% selects 7.4% (measured in `docs/lanes/look/spike-l.md`). Using one 3D noise in the object's own
space ("Object" texture coordinates in Blender) needs no blending at all.

**Hashing instead of textures.** The noise is a grid of pseudo-random numbers made by scrambling the integer grid coordinates with a few multiplies
and shifts (an integer hash). Integer arithmetic wraps identically on every CPU and GPU, so the browser, Godot and the Python reference produce the
same bits; `sin()`-style hashes drift between GPU vendors.

**Anchoring.** The pattern is a function of position *in the part's own frame*, so paint is attached to the wheel, not to the world: it rolls with
the wheel and stays put when the vehicle drives away. A function of world position would make paint crawl over the vehicle as it moves.

**Level of detail.** A coarser mesh puts the surface a few millimetres off the fine one, and the pattern follows the surface, so it only changes
where a colour boundary happens to lie in that sliver. The mismatch is about (edge length per square metre) x (sagitta), which is how the L4 bound
was chosen rather than tuned.
