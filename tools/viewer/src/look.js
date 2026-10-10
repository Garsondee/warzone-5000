// LOOK's hook (docs/lanes/look/design-note.md section 6): injects assets/shaders/glsl/w5k_camo.glsl into the standard material of
// every Paint slot. One shared set of uniforms drives all of them; changing the scheme or the seed is a uniform update, not a rebuild.
import * as THREE from 'three';

// Order of the uW array = the W_* indices in the GLSL file.
const W_KEYS = ['wear_bias', 'dirt_base', 'dirt_freq', 'dirt_max', 'dirt_var', 'dust_freq', 'dust_hi', 'dust_lo', 'dust_max', 'dust_mod_base', 'dust_mod_var', 'splash_freq', 'splash_height_m', 'splash_max', 'splash_noise_m', 'wear_edge_gain', 'wear_noise_amp', 'wear_freq', 'wear_soft'];
const V3 = (a) => new THREE.Vector3(...a);

export function makeLook(data, paintMaterials) {
  const { glsl, schemes, weathering, cdf } = data;
  const u = {
    uCdf: { value: cdf }, uSeed: { value: 1 }, uSalt: { value: 0 }, uN: { value: 2 },
    uCol: { value: [0, 0, 0, 0].map(() => new THREE.Vector3()) }, uCov: { value: [0, 0, 0, 0] }, uScale: { value: 1 },
    uWC: { value: [weathering.bare_linear, weathering.dirt_linear, weathering.splash_linear, weathering.dust_linear].map(V3) },
    uW: { value: W_KEYS.map((k) => weathering[k]) },
    uWearOn: { value: 1 }, uView: { value: 0 }, uTint: { value: new THREE.Vector3() }, uTintMix: { value: 0 },
  };
  const state = { on: true, tint: null, tintMix: 0, scheme: Object.keys(schemes)[0], seed: 1 };
  function set(next = {}) {
    Object.assign(state, next);
    const sc = schemes[state.scheme];
    u.uSeed.value = state.seed >>> 0;
    u.uSalt.value = sc.seed_salt >>> 0;
    u.uN.value = sc.colours.length;
    sc.colours.forEach((c, i) => { u.uCol.value[i].set(...c.linear); u.uCov.value[i] = c.coverage; });
    u.uScale.value = sc.scale_m;
    u.uView.value = state.on ? 0 : 1; // 1 = plain material colour (camo off)
    if (state.tint) u.uTint.value.set(...state.tint); // a vehicle's accent, mixed into its paint to tell trucks apart at a distance
    u.uTintMix.value = state.tintMix ?? 0;
  }
  for (const mat of paintMaterials) {
    mat.onBeforeCompile = (s) => {
      Object.assign(s.uniforms, u);
      s.vertexShader = 'attribute float aEdge; attribute float aCavity; attribute float aHeight;\nvarying vec3 vLP; varying float vE; varying float vC; varying float vH; varying float vUp;\n' +
        s.vertexShader.replace('#include <begin_vertex>', '#include <begin_vertex>\nvLP = position; vE = aEdge; vC = aCavity; vH = aHeight; vUp = normalize(mat3(modelMatrix) * normal).y;');
      s.fragmentShader = 'varying vec3 vLP; varying float vE; varying float vC; varying float vH; varying float vUp;\nuniform int uView; uniform vec3 uTint; uniform float uTintMix;\n' + glsl + '\n' +
        s.fragmentShader.replace('#include <color_fragment>', '#include <color_fragment>\nif (uView == 0) { diffuseColor.rgb = w5k_albedo(vLP, vE, vC, vH, vUp, length(fwidth(vLP))); diffuseColor.rgb = mix(diffuseColor.rgb, uTint, uTintMix); }');
    };
    mat.customProgramCacheKey = () => 'w5k-camo';
  }
  set();
  return { state, set, schemes: Object.keys(schemes) };
}
