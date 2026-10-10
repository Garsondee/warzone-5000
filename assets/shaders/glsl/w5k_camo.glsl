// LOOK: camo + weathering as GLSL ES 3.00 (WebGL2). Mirrors assets/reference/w5k_look.py; spec docs/art/CAMO-ALGORITHM.md and WEATHERING.md.
// The host sets the uniforms (VIEWER's hook, docs/lanes/look/design-note.md section 6) and calls w5k_albedo() with node-local data only.
uniform float uCdf[65];        // equalising table (assets/materials/camo/cdf.json)
uniform uint uSeed;            // per-vehicle seed
uniform uint uSalt;            // scheme seed_salt
uniform int uN;                // colours in the scheme, 2..4
uniform vec3 uCol[4];          // linear albedo
uniform float uCov[4];         // coverage
uniform float uScale;          // blob scale, m
uniform vec3 uWC[4];           // weathering colours: bare, dirt, splash, dust (linear)
uniform float uW[19];          // weathering parameters, W_* indices below (assets/materials/baked/weathering.json)
#define W_BIAS 0
#define W_DIRT_BASE 1
#define W_DIRT_FREQ 2
#define W_DIRT_MAX 3
#define W_DIRT_VAR 4
#define W_DUST_FREQ 5
#define W_DUST_HI 6
#define W_DUST_LO 7
#define W_DUST_MAX 8
#define W_DUST_MOD_BASE 9
#define W_DUST_MOD_VAR 10
#define W_SPLASH_FREQ 11
#define W_SPLASH_H 12
#define W_SPLASH_MAX 13
#define W_SPLASH_NOISE 14
#define W_EDGE_GAIN 15
#define W_NOISE_AMP 16
#define W_WEAR_FREQ 17
#define W_WEAR_SOFT 18

uint w5k_hash_raw(uvec3 u, uint seed) {
  uint h = seed ^ (u.x * 0x8DA6B343u) ^ (u.y * 0xD8163841u) ^ (u.z * 0xCB1AB31Fu);
  h ^= h >> 16; h *= 0x7FEB352Du; h ^= h >> 15; h *= 0x846CA68Bu; h ^= h >> 16; return h;
}
float w5k_lat(ivec3 i, uint s) { return float(w5k_hash_raw(uvec3(i + ivec3(32768)), s) >> 8) * (1.0 / 16777216.0); }
float w5k_vnoise(vec3 p, uint s) {
  vec3 fl = floor(p); ivec3 i = ivec3(fl); vec3 f = p - fl; f = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
  float a = mix(mix(w5k_lat(i, s), w5k_lat(i + ivec3(1,0,0), s), f.x), mix(w5k_lat(i + ivec3(0,1,0), s), w5k_lat(i + ivec3(1,1,0), s), f.x), f.y);
  float b = mix(mix(w5k_lat(i + ivec3(0,0,1), s), w5k_lat(i + ivec3(1,0,1), s), f.x), mix(w5k_lat(i + ivec3(0,1,1), s), w5k_lat(i + ivec3(1,1,1), s), f.x), f.y);
  return mix(a, b, f.z);
}
float w5k_fbm(vec3 p, uint s, float fp) {  // octaves 3, lacunarity 2, gain 0.5 (the only fractal the bake accepts)
  float f = 1.0, a = 1.0, sum = 0.0, tot = 0.0;
  for (int o = 0; o < 3; o++) {
    float w = 1.0 - clamp((fp * f * 2.0 - 0.5) / 0.5, 0.0, 1.0); float fo = float(o);
    float n = w5k_vnoise(p * f + vec3(17.3 * fo, -9.1 * fo, 4.7 * fo), s + uint(o) * 0x9E3779B9u);
    sum += a * (0.5 + w * (n - 0.5)); tot += a; f *= 2.0; a *= 0.5;
  }
  return sum / tot;
}
float w5k_cdf(float f) { float x = clamp(f, 0.0, 1.0) * 64.0; int i = min(int(x), 63); return mix(uCdf[i], uCdf[i + 1], x - float(i)); }
vec3 w5k_camo(vec3 p, float footprint) {
  vec3 q = p / uScale; float fp = footprint / uScale; uint base = uSeed ^ uSalt; float left = 1.0;
  for (int i = 0; i < 3; i++) {
    if (i >= uN - 1) break;
    float u = w5k_cdf(w5k_fbm(q, w5k_hash_raw(uvec3(uint(i + 1), 0u, 0u), base), fp));
    if (u < uCov[i] / left) return uCol[i];
    left -= uCov[i];
  }
  return uCol[uN - 1];
}
// edge, cavity: the contract's per-vertex flags; height_m: above ground in the design pose; up_y: world-up component of the unit normal.
vec3 w5k_albedo(vec3 p, float edge, float cavity, float height_m, float up_y, float footprint) {
  vec3 c = w5k_camo(p, footprint);
  float n0 = w5k_vnoise(p * uW[W_WEAR_FREQ], uSeed ^ 0xA511E9B3u), n1 = w5k_vnoise(p * uW[W_DIRT_FREQ], uSeed ^ 0x68E31DA4u);
  float n2 = w5k_vnoise(p * uW[W_SPLASH_FREQ], uSeed ^ 0x1B873593u), n3 = w5k_vnoise(p * uW[W_DUST_FREQ], uSeed ^ 0xCC9E2D51u);
  float wd = max(uW[W_WEAR_SOFT], footprint * uW[W_WEAR_FREQ]);
  c = mix(c, uWC[0], smoothstep(-wd, wd, edge * uW[W_EDGE_GAIN] + uW[W_NOISE_AMP] * (n0 - 0.5) - uW[W_BIAS]));
  c = mix(c, uWC[1], clamp(cavity * (uW[W_DIRT_BASE] + uW[W_DIRT_VAR] * n1), 0.0, uW[W_DIRT_MAX]));
  c = mix(c, uWC[2], uW[W_SPLASH_MAX] * (1.0 - smoothstep(0.0, uW[W_SPLASH_H], height_m + (n2 - 0.5) * uW[W_SPLASH_NOISE])));
  return mix(c, uWC[3], min(uW[W_DUST_MAX] * smoothstep(uW[W_DUST_LO], uW[W_DUST_HI], up_y) * (uW[W_DUST_MOD_BASE] + uW[W_DUST_MOD_VAR] * n3), uW[W_DUST_MAX]));
}
