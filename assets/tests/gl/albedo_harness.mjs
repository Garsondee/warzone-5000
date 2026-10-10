// L7 harness: draws a 64x64 grid of test points through assets/shaders/glsl/w5k_camo.glsl in headless Chromium (raw WebGL2, no three.js) and
// prints the 8-bit sRGB result as JSON. Usage: node albedo_harness.mjs <scheme.json> [seed]. Playwright is found via NODE_PATH or /opt/node-tools.
import fs from 'fs'; import path from 'path'; import { createRequire } from 'module';
const here = path.dirname(new URL(import.meta.url).pathname), root = path.join(here, '..', '..');
const require = createRequire((process.env.NODE_PATH || '/opt/node-tools/node_modules') + '/');
const { chromium } = require('playwright');
const scheme = JSON.parse(fs.readFileSync(process.argv[2])), seed = Number(process.argv[3] || 1234);
const W = JSON.parse(fs.readFileSync(path.join(root, 'materials/baked/weathering.json')));
const cdf = Object.values(JSON.parse(fs.readFileSync(path.join(root, 'materials/camo/cdf.json'))))[0];
const glsl = fs.readFileSync(path.join(root, 'shaders/glsl/w5k_camo.glsl'), 'utf8');
const wkeys = ['wear_bias','dirt_base','dirt_freq','dirt_max','dirt_var','dust_freq','dust_hi','dust_lo','dust_max','dust_mod_base','dust_mod_var','splash_freq','splash_height_m','splash_max','splash_noise_m','wear_edge_gain','wear_noise_amp','wear_freq','wear_soft'];
const pad = (a, n) => { const o = a.flat(); while (o.length < n) o.push(0); return o; };
const U = { uCdf: cdf, uW: wkeys.map(k => W[k]), uScale: scheme.scale_m, uCov: pad(scheme.colours.map(c => [c.coverage]), 4), uCol: pad(scheme.colours.map(c => c.linear), 12),
  uWC: [W.bare_linear, W.dirt_linear, W.splash_linear, W.dust_linear].flat() };
const frag = `#version 300 es
precision highp float; precision highp int; out vec4 o;
${glsl}
vec3 srgb(vec3 c){ return mix(c*12.92, 1.055*pow(max(c,vec3(0.0)),vec3(1.0/2.4))-0.055, step(vec3(0.0031308), c)); }
void main(){ int x=int(gl_FragCoord.x), y=int(gl_FragCoord.y);
  vec3 p=vec3(float(x)*0.125, float(y)*0.0625, float((x^y)&63)*0.03125);
  float e=float(x)/64.0, cv=float(y)/64.0, h=float((x*7+y*3)&63)/32.0, up=float(((x*5+y*11)&63)-32)/32.0;
  o=vec4(srgb(w5k_albedo(p,e,cv,h,up,0.0)),1.0); }`;
const html = `<canvas id=c width=64 height=64></canvas>`;
const br = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
const pg = await br.newPage(); await pg.setContent(html);
const res = await pg.evaluate(([frag, U, seed, salt, n]) => {
  const gl = document.getElementById('c').getContext('webgl2'); const mk = (t, s) => { const h = gl.createShader(t); gl.shaderSource(h, s); gl.compileShader(h); if (!gl.getShaderParameter(h, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(h)); return h; };
  const pr = gl.createProgram(); gl.attachShader(pr, mk(gl.VERTEX_SHADER, '#version 300 es\nvoid main(){ vec2 v=vec2((gl_VertexID<<1)&2, gl_VertexID&2); gl_Position=vec4(v*2.0-1.0,0.0,1.0);}'));
  gl.attachShader(pr, mk(gl.FRAGMENT_SHADER, frag)); gl.linkProgram(pr); if (!gl.getProgramParameter(pr, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(pr)); gl.useProgram(pr);
  const L = k => gl.getUniformLocation(pr, k);
  gl.uniform1fv(L('uCdf'), U.uCdf); gl.uniform1fv(L('uW'), U.uW); gl.uniform1f(L('uScale'), U.uScale); gl.uniform1fv(L('uCov'), U.uCov);
  gl.uniform3fv(L('uCol'), U.uCol); gl.uniform3fv(L('uWC'), U.uWC); gl.uniform1ui(L('uSeed'), seed); gl.uniform1ui(L('uSalt'), salt); gl.uniform1i(L('uN'), n);
  gl.viewport(0, 0, 64, 64); gl.drawArrays(gl.TRIANGLES, 0, 3); const b = new Uint8Array(64 * 64 * 4); gl.readPixels(0, 0, 64, 64, gl.RGBA, gl.UNSIGNED_BYTE, b); return Array.from(b);
}, [frag, U, seed, scheme.seed_salt, scheme.colours.length]);
console.log(JSON.stringify(res)); await br.close();
