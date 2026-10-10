// Spike S-L: the shared recipe as GLSL ES 3.00 (WebGL2). Same maths as ref.py. Inputs: node-local position, edge, cavity, seed, CDF LUT.
uniform float uCdf[65];
uniform uint uSeed;
uniform vec3 uPal[3]; uniform vec3 uCov; uniform vec3 uBare; uniform vec3 uDirt; uniform float uScale;
uint w5k_hash3(ivec3 v, uint seed) {
  uvec3 u = uvec3(v + ivec3(32768));
  uint h = seed ^ (u.x * 0x8DA6B343u) ^ (u.y * 0xD8163841u) ^ (u.z * 0xCB1AB31Fu);
  h ^= h >> 16; h *= 0x7FEB352Du; h ^= h >> 15; h *= 0x846CA68Bu; h ^= h >> 16; return h;
}
float w5k_lat(ivec3 i, uint s) { return float(w5k_hash3(i, s) >> 8) * (1.0 / 16777216.0); }
float w5k_vnoise(vec3 p, uint s) {
  vec3 fl = floor(p); ivec3 i = ivec3(fl); vec3 f = p - fl; f = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
  float a = mix(mix(w5k_lat(i, s), w5k_lat(i + ivec3(1,0,0), s), f.x), mix(w5k_lat(i + ivec3(0,1,0), s), w5k_lat(i + ivec3(1,1,0), s), f.x), f.y);
  float b = mix(mix(w5k_lat(i + ivec3(0,0,1), s), w5k_lat(i + ivec3(1,0,1), s), f.x), mix(w5k_lat(i + ivec3(0,1,1), s), w5k_lat(i + ivec3(1,1,1), s), f.x), f.y);
  return mix(a, b, f.z);
}
float w5k_fbm(vec3 p, uint s, float fp) {
  float f = 1.0, a = 1.0, sum = 0.0, tot = 0.0;
  for (int o = 0; o < 3; o++) {
    float w = 1.0 - clamp((fp * f * 2.0 - 0.5) / 0.5, 0.0, 1.0);
    float fo = float(o);
    float n = w5k_vnoise(p * f + vec3(17.3 * fo, -9.1 * fo, 4.7 * fo), s + uint(o) * 0x9E3779B9u);
    sum += a * (0.5 + w * (n - 0.5)); tot += a; f *= 2.0; a *= 0.5;
  }
  return sum / tot;
}
float w5k_cdf(float f) {
  float x = clamp(f, 0.0, 1.0) * 64.0; int i = min(int(x), 63);
  return mix(uCdf[i], uCdf[i + 1], x - float(i));
}
vec3 w5k_albedo(vec3 p, float edge, float cavity, float footprint) {
  vec3 q = p / uScale; float fp = footprint / uScale;
  float u1 = w5k_cdf(w5k_fbm(q, uSeed, fp));
  float u2 = w5k_cdf(w5k_fbm(q, uSeed ^ 0x5BD1E995u, fp));
  vec3 c = uPal[2];
  if (u1 < uCov.x) c = uPal[0]; else if (u2 < uCov.y / (1.0 - uCov.x)) c = uPal[1];
  float nz = w5k_vnoise(p * 14.0, uSeed ^ 0xA511E9B3u);
  float wd = max(0.04, footprint * 14.0);
  c = mix(c, uBare, smoothstep(-wd, wd, edge * 1.2 + 0.6 * (nz - 0.5) - 0.45));
  float nz2 = w5k_vnoise(p * 5.0 + vec3(3.1, 0.0, 0.0), uSeed ^ 0x68E31DA4u);
  return mix(c, uDirt, clamp(cavity * (0.6 + 0.8 * nz2), 0.0, 0.85));
}
