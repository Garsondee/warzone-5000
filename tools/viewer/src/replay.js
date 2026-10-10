// Replay v2 binary decoder: the JS twin of crates/w5k_replay/src/binary.rs (same integer layout, same predictors).
const LIMITING = ['None', 'Power', 'Grip', 'Soil', 'Sinkage', 'Brake', 'Suspension', 'Stuck', 'Overturned'];
const MM = 1000, TRAVEL = 20000, ANGLE = 65536 / (2 * Math.PI), ROT = 32767 * Math.SQRT2, KEYFRAME_INTERVAL = 30, VEHICLE_FIXED = 18;

function varint(b, st) {
  let z = 0, mul = 1;
  for (;;) {
    const byte = b[st.p++];
    if (byte === undefined) throw new Error('the replay ends inside a number');
    z += (byte & 0x7f) * mul;
    if (byte < 0x80) return z % 2 === 0 ? z / 2 : -(z + 1) / 2;
    mul *= 128;
  }
}

function unrle(packed) {
  const out = [];
  for (let i = 0; i < packed.length; i++) {
    out.push(packed[i]);
    if (packed[i] === 0) for (let k = packed[++i]; k > 0; k--) out.push(0);
  }
  return Uint8Array.from(out);
}

// Is slot i smooth in time? Mirrors `smooth` in binary.rs: it only reads counts that are already decoded.
function smooth(i, ints) {
  let base = 2;
  while (i >= base) {
    if (i < base + 5) return false;
    const nj = ints[base + 1];
    if (i < base + VEHICLE_FIXED + nj) return true;
    base += VEHICLE_FIXED + nj + 3 + 5 * ints[base + 2] + ints[base + 3] + 4 * ints[base + 4];
  }
  return false;
}

function unflatten(ints, names) {
  let k = 0;
  const next = () => ints[k++];
  const vec = (s) => ({ x: next() / s, y: next() / s, z: next() / s });
  const f = { t_s: next() / 1e6, vehicles: [], events: [], projectiles: [] };
  const nv = next();
  for (let v = 0; v < nv; v++) {
    const vehicle = next(), nj = next(), nc = next(), nl = next(), nw = next();
    const pos_m = vec(MM);
    const big = next(), c = [0, 0, 0, 0];
    let sum = 0;
    for (let i = 0; i < 4; i++) if (i !== big) { c[i] = next() / ROT; sum += c[i] * c[i]; }
    c[big] = Math.sqrt(Math.max(1 - sum, 0));
    const rot = { w: c[0], x: c[1], y: c[2], z: c[3] };
    const lin_vel_m_s = vec(MM), ang_vel_rad_s = vec(MM);
    const jn = names[vehicle], joints = [];
    for (let j = 0; j < nj; j++) joints.push(next() / (/travel|recoil/.test(jn[j] ?? '') ? TRAVEL : ANGLE));
    const engine_rpm = next(), gear = next(), limiting = LIMITING[next()] ?? 'None';
    const contacts = [];
    for (let i = 0; i < nc; i++) contacts.push({ flags: next(), normal_force_n: next() / 0.25, sinkage_m: next() / 10000, slip: next() / 1000, material: next() });
    const ledger_n = [];
    for (let i = 0; i < nl; i++) ledger_n.push(next());
    const weapons = [];
    for (let i = 0; i < nw; i++) weapons.push({ ready: next() !== 0, reload_s: next() / 1000, rounds: next(), aim_error_rad: next() / 100000 });
    f.vehicles.push({ vehicle, pos_m, rot, lin_vel_m_s, ang_vel_rad_s, joints, engine_rpm, gear, contacts, ledger_n, limiting, weapons });
  }
  return f;
}

export function decodeReplay(bytes) {
  const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (String.fromCharCode(...bytes.subarray(0, 4)) !== 'W5KR') throw new Error('not a W5KR replay');
  if (dv.getUint32(4, true) !== 2) throw new Error('unsupported replay version');
  const hlen = dv.getUint32(8, true);
  const header = JSON.parse(new TextDecoder().decode(bytes.subarray(12, 12 + hlen)));
  const names = header.vehicles.map((v) => v.joint_names);
  let pos = 12 + hlen;
  const count = dv.getUint32(pos, true);
  pos += 4;
  const frames = [];
  while (frames.length < count) {
    const glen = dv.getUint32(pos, true);
    const g = unrle(bytes.subarray(pos + 4, pos + 4 + glen));
    pos += 4 + glen;
    const st = { p: 0 };
    let prev = [], prev2 = null;
    while (st.p < g.length) {
      const n = varint(g, st), ints = [];
      for (let i = 0; i < n; i++) {
        const a = prev[i] ?? 0;
        const p = prev2 && smooth(i, ints) ? 2 * a - (prev2[i] ?? 0) : a;
        ints.push(p + varint(g, st));
      }
      const f = unflatten(ints, names);
      const elen = varint(g, st);
      if (elen > 0) {
        [f.events, f.projectiles] = JSON.parse(new TextDecoder().decode(g.subarray(st.p, st.p + elen)));
        st.p += elen;
      }
      prev2 = frames.length % KEYFRAME_INTERVAL !== 0 ? prev : null;
      prev = ints;
      frames.push(f);
    }
  }
  return { header, frames };
}
