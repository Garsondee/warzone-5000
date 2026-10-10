// The track belt (contract 0.3 `TrackRun`): the taut band round a loop of wheel circles, and the links that ride on it. A port of GEOMETRY's
// `w5k_geo::track` (`Belt::round`, `Belt::at`, `link_frames`), which draws the same band in Rust, plus two things a viewer needs and a mesh
// builder does not: either winding of the loop (FORGE's carrier has a front sprocket and runs clockwise in (z, y); GEOMETRY's runs
// counter-clockwise), and a wheel the band does not touch is skipped instead of refused (a bump lifts a road wheel inside the band).
// Pure functions, no three.js: `node tools/viewer/test-tracks.mjs` checks them against closed forms and against the Rust belt.
const TAU = 2 * Math.PI;
const ON_THE_RUN_RAD = 2e-3; // GEOMETRY's tolerance: a wheel on a straight run turns the band by less than 2 mrad
const EDGE_M = 1e-9; // arcs shorter than a nanometre are straight runs

// The band round `circles` ({z, y, r}) in loop order, walked counter-clockwise in (z, y); `{error, wheel}` when a wheel does not touch it.
export function round(circles) {
  const n = circles.length;
  if (n < 2) return { error: 'a belt needs at least two wheels', wheel: 0 };
  // the angle of the outward normal of the tangent that leaves circle i for circle i + 1 (the same normal at both ends)
  const leave = [];
  for (let i = 0; i < n; i++) {
    const a = circles[i], b = circles[(i + 1) % n], d = Math.hypot(b.z - a.z, b.y - a.y);
    if (d <= Math.abs(a.r - b.r)) return { error: `wheel ${i} and wheel ${(i + 1) % n} lie inside one another`, wheel: a.r < b.r ? i : (i + 1) % n };
    leave.push(Math.atan2(b.y - a.y, b.z - a.z) - Math.acos((a.r - b.r) / d));
  }
  // the turn the band makes round each circle, counter-clockwise: from the tangent that arrives to the one that leaves
  const sweeps = leave.map((l, j) => { const s = (((l - leave[(j + n - 1) % n]) % TAU) + TAU) % TAU; return s > TAU - ON_THE_RUN_RAD ? 0 : s; });
  // a closed convex band turns through exactly one full turn; a wheel off the band turns it the other way, and the sum shows it
  if (Math.abs(sweeps.reduce((a, s) => a + s, 0) - TAU) > ON_THE_RUN_RAD * n) {
    return { error: 'a wheel does not touch the belt', wheel: sweeps.indexOf(Math.max(...sweeps)) };
  }
  const pieces = [];
  let s = 0;
  for (let i = 0; i < n; i++) {
    const a = circles[i], b = circles[(i + 1) % n], ang = leave[i], sa = Math.sin(ang), ca = Math.cos(ang);
    const p = [a.z + a.r * ca, a.y + a.r * sa], q = [b.z + b.r * ca, b.y + b.r * sa];
    pieces.push({ s, line: true, p, t: [-sa, ca] });
    s += Math.hypot(q[0] - p[0], q[1] - p[1]);
    const j = (i + 1) % n; // the arc on circle i + 1, from where this tangent arrives to where the next one leaves
    if (sweeps[j] * b.r > EDGE_M) { pieces.push({ s, line: false, c: [b.z, b.y], r: b.r, a0: ang }); s += sweeps[j] * b.r; }
  }
  const length = s;
  // the point at arc length `s` (wrapping round the loop) and the unit direction of travel there
  const at = (arc) => {
    const u = ((arc % length) + length) % length;
    let lo = 0, hi = pieces.length - 1; // the last piece that starts at or before u
    while (lo < hi) { const mid = (lo + hi + 1) >> 1; if (pieces[mid].s <= u) lo = mid; else hi = mid - 1; }
    const k = pieces[lo], d = u - k.s;
    if (k.line) return [k.p[0] + k.t[0] * d, k.p[1] + k.t[1] * d, k.t[0], k.t[1]];
    const sa = Math.sin(k.a0 + d / k.r), ca = Math.cos(k.a0 + d / k.r);
    return [k.c[0] + k.r * ca, k.c[1] + k.r * sa, -sa, ca];
  };
  return { length, at };
}

// The band round `circles` whichever way the loop winds, with every wheel it does not touch left out. `mirrored` is true for a clockwise loop
// (the band is built on the mirror image, z negated, and mirrored back: the order of the wheels, and so the arc length, is unchanged).
export function band(circles) {
  let area = 0; // the polygon of the wheel centres: positive is counter-clockwise in (z, y)
  circles.forEach((a, i) => { const b = circles[(i + 1) % circles.length]; area += a.z * b.y - b.z * a.y; });
  const mirrored = area < 0, work = circles.map((c) => ({ z: mirrored ? -c.z : c.z, y: c.y, r: c.r }));
  const kept = work.map((_, i) => i), skipped = [];
  for (;;) {
    const r = round(kept.map((i) => work[i]));
    if (r.at) return { ...r, mirrored, skipped };
    if (kept.length <= 2) return { error: r.error, mirrored, skipped };
    skipped.push(...kept.splice(r.wheel, 1)); // rubber band round nails: a nail inside the band is not touched
  }
}

// The point (z, y) and unit tangent of every link of `run`, given the wheels' current centres ({z, y}, in the Track node's frame) and the
// sprocket's spin (rad): link k sits at arc length `k L / links + direction R_sprocket spin`, `L` being the length of the band round the wheels.
// Returns {frames: Float64Array [z, y, tz, ty] per link, length, skipped, mirrored}, or {error}.
export function linkFrames(run, centres, spin) {
  const b = band(run.wheels.map((w, i) => ({ z: centres[i].z, y: centres[i].y, r: w.radius_m })));
  if (!b.at) return { error: b.error };
  const shift = run.direction * run.wheels[run.sprocket].radius_m * spin, frames = new Float64Array(4 * run.links);
  for (let k = 0; k < run.links; k++) {
    const [z, y, tz, ty] = b.at((b.length * k) / run.links + shift);
    frames.set(b.mirrored ? [-z, y, -tz, ty] : [z, y, tz, ty], 4 * k);
  }
  return { frames, length: b.length, skipped: b.skipped, mirrored: b.mirrored };
}

// A link's 4x4 matrix (column-major, three.js's order) in the Track node's frame: its origin on the path at (z, y), +X along the tangent, +Y out
// of the belt (right of the travel for a counter-clockwise loop, left for a clockwise one), +Z across the belt; `x` is the belt's centre line.
// A rotation, never a reflection, so the winding of the link's triangles is kept.
export function linkMatrix(out, at, frames, k, x, mirrored) {
  const z = frames[4 * k], y = frames[4 * k + 1], tz = frames[4 * k + 2], ty = frames[4 * k + 3], sigma = mirrored ? -1 : 1;
  out.set([0, ty, tz, 0, 0, -sigma * tz, sigma * ty, 0, sigma, 0, 0, 0, x, y, z, 1], at);
}
