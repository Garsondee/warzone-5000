// The belt maths of the viewer (src/tracks.js) against closed forms and against GEOMETRY's Rust belt.
//   node tools/viewer/test-tracks.mjs [dir with belt.json]     (w5k viewer dump-canned belt --out <dir> writes belt.json; without it the Rust check is skipped and says so)
import fs from 'node:fs';
import { decodeReplay } from './src/replay.js';
import { round, band, linkFrames, linkMatrix, contrast, groundRun, cellCentres } from './src/tracks.js';

let failed = false;
const check = (ok, msg) => { console.log(`${ok ? 'PASS' : 'FAIL'}  ${msg}`); failed ||= !ok; };
const near = (a, b, tol) => Math.abs(a - b) <= tol;
const wheels = (centres, radii) => centres.map((c, i) => ({ z: c[0], y: c[1], r: radii[i] }));
const asRun = (radii, links, direction, sprocket = 0) => ({ wheels: radii.map((r) => ({ radius_m: r })), links, direction, sprocket });
const pts = (cs) => cs.map(([z, y]) => ({ z, y }));

// belt_length_matches_two_circles_closed_form: the band round two discs is two tangents and two arcs, 2 t + R (pi + 2 a) + r (pi - 2 a),
// t = sqrt(d^2 - (R - r)^2), a = asin((R - r) / d); with equal discs, 2 d + 2 pi r
{
  const eq = round(wheels([[0, 0], [2, 0]], [0.3, 0.3]));
  check(near(eq.length, 2 * 2 + 2 * Math.PI * 0.3, 1e-12), `belt_length_matches_two_circles_closed_form: equal discs ${eq.length.toFixed(6)} m = 2d + 2 pi r`);
  const [R, r, d] = [0.5, 0.2, 1.7], a = Math.asin((R - r) / d), un = round(wheels([[0, 0], [d, 0.4]], [R, r]));
  const dd = Math.hypot(d, 0.4), aa = Math.asin((R - r) / dd);
  check(near(un.length, 2 * Math.sqrt(dd * dd - (R - r) ** 2) + R * (Math.PI + 2 * aa) + r * (Math.PI - 2 * aa), 1e-12), `...and unequal discs: ${un.length.toFixed(6)} m (a = ${a.toFixed(3)} rad)`);
}

// no_belt_pattern_with_visible_contrast_ever_moves_the_wrong_way: the light-and-dark pattern repeats every two links (period P). A pattern that moves
// a metres between frames is seen to move by the nearest of a - k P; from a = P / 2 (one pitch) that is the wrong way, or none (the wagon-wheel
// effect). The contrast must be gone wherever that happens, at every frame rate, and be full at rest.
{
  const pitch = 0.149, P = 2 * pitch, apparent = (a) => { const r = a % P; return r > P / 2 ? r - P : r; };
  let raw = 0, shown = 0, falseAt = [];
  for (const fps of [24, 30, 48, 60, 120]) for (let v = 0; v <= 30; v += 0.005) {
    const a = v / fps, wrong = Math.abs(apparent(a) - a) > 1e-9;
    if (wrong) { raw++; shown = Math.max(shown, contrast(a, pitch)); if (fps === 30) falseAt.push(v); }
  }
  check(raw > 0 && shown === 0, `no_belt_pattern_with_visible_contrast_ever_moves_the_wrong_way: without the fade the pattern shows false motion at ${raw} of the sampled speeds (at 30 frames a second from ${(falseAt[0] * 3.6).toFixed(1)} km/h); with it the contrast there is ${shown}`);
  let monotone = true, smooth = 0;
  for (let a = 0; a < 0.3; a += 0.001) { monotone &&= contrast(a + 0.001, pitch) <= contrast(a, pitch) + 1e-12; smooth = Math.max(smooth, Math.abs(contrast(a + 0.001, pitch) - contrast(a, pitch))); }
  check(contrast(0, pitch) === 1 && near(contrast(pitch / 2, pitch), 0.5, 1e-12) && contrast(pitch, pitch) === 0 && contrast(10, pitch) === 0 && monotone && smooth < 0.012,
    `the pattern is full at rest, half at half a pitch a frame, gone at a pitch, never grows with speed and moves by under ${(smooth * 100).toFixed(2)}% per millimetre (no pop)`);
}

// The loops below are the carrier's: GEOMETRY's (rear sprocket, counter-clockwise) and FORGE's (front sprocket, clockwise), as the Rust test writes them.
const dir = process.argv[2];
const vector = dir && fs.existsSync(`${dir}/belt.json`) ? JSON.parse(fs.readFileSync(`${dir}/belt.json`, 'utf8')) : null;
const unmirror = (f) => [-f[0], f[1], -f[2], f[3]];
if (!vector) console.log('SKIP  belt_path_matches_the_rust_belt_to_a_micrometre: no belt.json (run w5k viewer dump-canned belt --out <dir>, then pass <dir>)');
else {
  let worst = 0, worstT = 0, worstL = 0;
  for (const c of vector) {
    const out = linkFrames(c.run, pts(c.centres), c.spin_rad);
    check(!out.error && out.frames.length === 4 * c.run.links, `${c.name} loop, spin ${c.spin_rad}: ${c.run.links} links, ${out.mirrored ? 'clockwise (mirrored)' : 'counter-clockwise'}`);
    worstL = Math.max(worstL, Math.abs(out.length - c.length_m));
    c.frames.forEach((f, k) => {
      const r = c.mirror ? unmirror(f) : f;
      worst = Math.max(worst, Math.hypot(out.frames[4 * k] - r[0], out.frames[4 * k + 1] - r[1]));
      worstT = Math.max(worstT, Math.hypot(out.frames[4 * k + 2] - r[2], out.frames[4 * k + 3] - r[3]));
    });
  }
  check(worst < 1e-6 && worstT < 1e-9 && worstL < 1e-9, `belt_path_matches_the_rust_belt_to_a_micrometre: worst link ${(worst * 1e6).toFixed(4)} um, tangent ${worstT.toExponential(1)}, length ${worstL.toExponential(1)} m over ${vector.length} cases`);

  // the loops used by the tests that follow
  const geo = vector.find((c) => c.name === 'geometry' && c.spin_rad === 0), forge = vector.find((c) => c.name === 'forge' && c.spin_rad === 0);
  for (const [name, c] of [['geometry', geo], ['forge', forge]]) {
    const run = c.run, centres = pts(c.centres), R = run.wheels[run.sprocket].radius_m, n = run.links;
    const base = linkFrames(run, centres, 0), pitch = base.length / n;
    const at = (spin) => linkFrames(run, centres, spin).frames;
    // links_advance_exactly_the_sprocket_rolling_distance: one pitch of rolling moves every link on to the place of the next one, and a lap of the belt brings them home
    const one = at((run.direction * pitch) / R), lap = at((run.direction * base.length) / R);
    let shift = 0, home = 0;
    for (let k = 0; k < n; k++) for (let i = 0; i < 4; i++) {
      shift = Math.max(shift, Math.abs(one[4 * k + i] - base.frames[4 * ((k + 1) % n) + i]));
      home = Math.max(home, Math.abs(lap[4 * k + i] - base.frames[4 * k + i]));
    }
    check(shift < 1e-9, `links_advance_exactly_the_sprocket_rolling_distance (${name}): one pitch of rolling puts link k where link k+1 was (${shift.toExponential(1)} m)`);
    check(home < 1e-9, `the_links_close_the_loop (${name}): a lap of the belt (L / R = ${(base.length / R).toFixed(2)} rad of the sprocket) brings every link home (${home.toExponential(1)} m)`);
    // on a straight run a link moves by exactly R x the spin
    const tiny = 1e-4, moved = at(run.direction * tiny).slice(0, 2), start = base.frames.slice(0, 2);
    check(near(Math.hypot(moved[0] - start[0], moved[1] - start[1]), R * tiny, R * tiny * 0.01), `a link moves R x spin along the belt: ${(Math.hypot(moved[0] - start[0], moved[1] - start[1]) * 1e6).toFixed(2)} um for ${(R * tiny * 1e6).toFixed(2)} um (${name})`);
    // a_link_matrix_is_a_rotation_that_points_out_of_the_belt: +X along the path, +Y away from the loop's middle, never a reflection (the triangle winding survives)
    const m = new Float32Array(16), cz = centres.reduce((a, p) => a + p.z, 0) / centres.length, cy = centres.reduce((a, p) => a + p.y, 0) / centres.length;
    let outward = 0, det = 1;
    for (let k = 0; k < n; k++) {
      linkMatrix(m, 0, base.frames, k, 0, base.mirrored);
      const [x0, x1, x2, , y0, y1, y2, , z0, z1, z2] = m;
      const d = x0 * (y1 * z2 - y2 * z1) - y0 * (x1 * z2 - x2 * z1) + z0 * (x1 * y2 - x2 * y1);
      det = Math.min(det, d);
      outward += (y2 * (base.frames[4 * k] - cz) + y1 * (base.frames[4 * k + 1] - cy)) > 0 ? 1 : 0; // +Y . (point - middle) in (z, y)
    }
    check(near(det, 1, 1e-6) && outward === n, `a_link_matrix_is_a_rotation_that_points_out_of_the_belt (${name}): determinant ${det.toFixed(6)}, ${outward} of ${n} links face out`);
  }

  // the footprint: the viewer derives it from the road wheels (no contact length in the RenderRig), TRACKS spaces its samples over `contact_length_m`
  {
    const road = forge.run.wheels.map((w, i) => ({ z: forge.centres[i][0], y: forge.centres[i][1], r: w.radius_m })).filter((_, i) => forge.roles[i] === 'RoadWheel');
    const g = groundRun(road), { cells, length } = cellCentres(g, forge.track.samples), want = forge.track.contact_length_m / forge.track.samples;
    check(road.length === 5 && near(length, want, want * 0.03), `the_footprint_the_viewer_derives_is_the_contact_length_tracks_uses: ${road.length} road wheels give ${(length * forge.track.samples).toFixed(3)} m, FORGE's contact length is ${forge.track.contact_length_m} m (cells of ${(length * 1000).toFixed(0)} mm for ${(want * 1000).toFixed(0)} mm)`);
    check(cells.every((c, k) => k === 0 || c[0] < cells[k - 1][0]) && cells[0][0] > 0 && cells[cells.length - 1][0] < 0, 'cell_zero_is_the_rearmost_and_the_last_cell_the_front: the cells run from +z (behind) to -z (ahead)');
    check(groundRun(road.slice(0, 1)) === null, 'a track with one road wheel has no footprint to draw');
    // a_track_entering_soft_ground_sinks_its_front_cell_first: a real drive into the slice course's mud crossing at 48 km/h (TRACKS's samples under FORGE's carrier)
    const mud = decodeReplay(new Uint8Array(fs.readFileSync(new URL('./samples/carrier-mud.w5kr', import.meta.url)))), names = mud.header.vehicles[0].contact_names;
    const first = mud.frames.find((f) => f.vehicles[0].contacts.some((c) => c.sinkage_m > 0)), cs = first.vehicles[0].contacts;
    const sunk = (side) => cs.slice(side * 12, side * 12 + 12).map((c, k) => (c.sinkage_m > 0 ? k : -1)).filter((k) => k >= 0);
    const side = sunk(0).length ? 0 : 1, ks = sunk(side), rest = cells.filter((_, j) => !ks.includes(j));
    check(names[0] === 'track_l.0' && names[12] === 'track_r.0' && ks.length >= 1 && ks.every((k) => cells[k][0] < Math.min(...rest.map((c) => c[0]))),
      `a_track_entering_soft_ground_sinks_its_front_cell_first: at t = ${first.t_s.toFixed(2)} s only the ${side ? 'right' : 'left'} track's cell${ks.length > 1 ? 's' : ''} ${ks.join(', ')} ${ks.length > 1 ? 'sink' : 'sinks'} (${(Math.max(...cs.map((c) => c.sinkage_m)) * 1000).toFixed(0)} mm) and ${ks.length > 1 ? 'they are' : 'it is'} ahead of every cell that does not`);
  }

  // a_wheel_lifted_inside_the_band_is_skipped_not_fatal: rubber band round nails
  const run = geo.run, rest = pts(geo.centres), withWheel = (i, dy) => rest.map((p, j) => (j === i ? { z: p.z, y: p.y + dy } : p));
  const lifted = linkFrames(run, withWheel(7, 0.15), 0), without = band(run.wheels.map((w, i) => ({ z: rest[i].z, y: rest[i].y, r: w.radius_m })).filter((_, i) => i !== 7));
  check(!lifted.error && lifted.skipped.join() === '7' && near(lifted.length, without.length, 1e-9) && lifted.frames.every(Number.isFinite),
    `a_wheel_lifted_inside_the_band_is_skipped_not_fatal: road wheel 7 up 15 cm is left out (skipped ${lifted.skipped}), the belt is the band round the other nine (${lifted.length.toFixed(4)} m)`);
  const pushed = linkFrames(run, withWheel(7, -0.04), 0);
  check(!pushed.error && pushed.skipped.join() === '6,8' && pushed.length > linkFrames(run, rest, 0).length, `...and pushed 4 cm down it is the neighbours that fall inside the band (skipped ${pushed.skipped}), the belt grows to ${pushed.length.toFixed(4)} m`);
  check(linkFrames(run, withWheel(7, 0.15), 0.7).frames.every(Number.isFinite) && band([{ z: 0, y: 0, r: 0.3 }]).error !== undefined, 'a belt that cannot be built (one wheel) says so instead of throwing');
}

console.log(failed ? 'FAILED' : 'all passed');
process.exit(failed ? 1 : 0);
