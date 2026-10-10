# Camo algorithm (specification; the reference is `assets/reference/w5k_look.py`, the shaders must match it)

Inputs: node-local position `p` (metres), per-vehicle `seed` (u32), a scheme (baked JSON from `w5k look bake`), the equalising table `cdf` (65 knots,
`assets/materials/camo/cdf.json`), optional `footprint` (metres per pixel). Nothing else: no world position, time or UV (test L3).

```
hash3(ix, iy, iz, seed):           # u32 arithmetic wraps; ints are shifted by +32768 first (no negative-int-to-uint cast)
    h = seed ^ ix*0x8DA6B343 ^ iy*0xD8163841 ^ iz*0xCB1AB31F
    h ^= h>>16; h *= 0x7FEB352D; h ^= h>>15; h *= 0x846CA68B; h ^= h>>16
lattice = (hash3 >> 8) / 2^24                      # exact in f32
vnoise  = trilinear blend of the 8 corner lattice values with quintic fade t^3 (6t^2 - 15t + 10)
fbm(q)  = sum over 3 octaves (freq 1,2,4; amp 1, .5, .25) of amp * (0.5 + w_o (vnoise(q*freq + (17.3,-9.1,4.7)*o, seed + o*0x9E3779B9) - 0.5)) / 1.75
          w_o = 1 - clamp((footprint_lattice * freq * 2 - 0.5) / 0.5, 0, 1)      # band limit: a too-fine octave fades to its mean
tone(p):  q = p / scale_m; base = seed ^ salt; left = 1
          for colour i except the last:  u = cdf(fbm(q, hash3(i+1, 0, 0, base))); if u < coverage_i / left: return i; left -= coverage_i
          return last
```
Why it hits the coverage: the table makes `u` uniform on 0..1, the fields for different `i` are independent, and `coverage_i / left` is the
conditional probability that colour `i` is chosen given the earlier ones were not. Tested: L2 (flat tile and sphere, within 0.03; z-scores
printed), L1 (bit-exact golden), L3 (rigid motion), L4 (cylinder at 24 against 64 segments; at 16 segments about 1% of points move across an edge,
physics of a 7.7 mm sagitta, not a bug).
Known limit: far away the band limit pulls the field to its mean, shrinking the rare colours; a footprint-dependent table is an M2 item.
