// Spike S-L: Godot port of camo.glsl. UNTESTED: no Godot binary in the lane container (verify uint ops, uniform arrays and hex literals on 4.6.2).
shader_type spatial;
uniform float cdf[65];
uniform uint seed = 42u;
uniform vec3 pal[3];
uniform vec3 cov;
uniform vec3 bare;
uniform vec3 dirt;
uniform float scale_m = 0.9;
varying vec3 lp; // node-local position (set in vertex() from VERTEX)
varying float v_edge; // from COLOR.r
varying float v_cavity; // from COLOR.g

uint hash3(ivec3 v, uint s) {
	uvec3 u = uvec3(v + ivec3(32768));
	uint h = s ^ (u.x * 0x8DA6B343u) ^ (u.y * 0xD8163841u) ^ (u.z * 0xCB1AB31Fu);
	h ^= h >> 16u; h *= 0x7FEB352Du; h ^= h >> 15u; h *= 0x846CA68Bu; h ^= h >> 16u; return h;
}
float lat(ivec3 i, uint s) { return float(hash3(i, s) >> 8u) * (1.0 / 16777216.0); }
float vnoise(vec3 p, uint s) {
	vec3 fl = floor(p); ivec3 i = ivec3(fl); vec3 f = p - fl; f = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
	float a = mix(mix(lat(i, s), lat(i + ivec3(1,0,0), s), f.x), mix(lat(i + ivec3(0,1,0), s), lat(i + ivec3(1,1,0), s), f.x), f.y);
	float b = mix(mix(lat(i + ivec3(0,0,1), s), lat(i + ivec3(1,0,1), s), f.x), mix(lat(i + ivec3(0,1,1), s), lat(i + ivec3(1,1,1), s), f.x), f.y);
	return mix(a, b, f.z);
}
float fbm(vec3 p, uint s, float fp) {
	float f = 1.0, a = 1.0, sum = 0.0, tot = 0.0;
	for (int o = 0; o < 3; o++) {
		float w = 1.0 - clamp((fp * f * 2.0 - 0.5) / 0.5, 0.0, 1.0);
		float fo = float(o);
		float n = vnoise(p * f + vec3(17.3 * fo, -9.1 * fo, 4.7 * fo), s + uint(o) * 0x9E3779B9u);
		sum += a * (0.5 + w * (n - 0.5)); tot += a; f *= 2.0; a *= 0.5;
	}
	return sum / tot;
}
float cdf_at(float f) {
	float x = clamp(f, 0.0, 1.0) * 64.0; int i = min(int(x), 63);
	return mix(cdf[i], cdf[i + 1], x - float(i));
}
vec3 albedo_at(vec3 p, float edge, float cavity, float footprint) {
	vec3 q = p / scale_m; float fp = footprint / scale_m;
	float u1 = cdf_at(fbm(q, seed, fp));
	float u2 = cdf_at(fbm(q, seed ^ 0x5BD1E995u, fp));
	vec3 c = pal[2];
	if (u1 < cov.x) c = pal[0]; else if (u2 < cov.y / (1.0 - cov.x)) c = pal[1];
	float nz = vnoise(p * 14.0, seed ^ 0xA511E9B3u);
	float wd = max(0.04, footprint * 14.0);
	c = mix(c, bare, smoothstep(-wd, wd, edge * 1.2 + 0.6 * (nz - 0.5) - 0.45));
	float nz2 = vnoise(p * 5.0 + vec3(3.1, 0.0, 0.0), seed ^ 0x68E31DA4u);
	return mix(c, dirt, clamp(cavity * (0.6 + 0.8 * nz2), 0.0, 0.85));
}
void vertex() { lp = VERTEX; v_edge = COLOR.r; v_cavity = COLOR.g; }
void fragment() {
	ALBEDO = albedo_at(lp, v_edge, v_cavity, length(fwidth(lp)));
	ROUGHNESS = 0.9;
}
