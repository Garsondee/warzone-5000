// The JS decoder must agree with the Rust decoder: decode the Rust-written binary and compare with the JSON the same Rust
// run wrote (which holds the unquantised original, so the tolerance is the quantum).  node test-decoder.mjs <dir with replay.w5kr and replay.json>
import fs from 'node:fs';
import { decodeReplay } from './src/replay.js';
const dir = process.argv[2];
const bin = decodeReplay(new Uint8Array(fs.readFileSync(`${dir}/replay.w5kr`)));
const ref = JSON.parse(fs.readFileSync(`${dir}/replay.json`, 'utf8'));
let worstPos = 0, worstJoint = 0, worstRot = 0;
if (bin.frames.length !== ref.frames.length) throw new Error('frame count differs');
bin.frames.forEach((f, i) => {
  const a = f.vehicles[0], b = ref.frames[i].vehicles[0];
  worstPos = Math.max(worstPos, Math.hypot(a.pos_m.x - b.pos_m.x, a.pos_m.y - b.pos_m.y, a.pos_m.z - b.pos_m.z));
  const d = Math.abs(a.rot.w * b.rot.w + a.rot.x * b.rot.x + a.rot.y * b.rot.y + a.rot.z * b.rot.z);
  worstRot = Math.max(worstRot, 2 * Math.acos(Math.min(d, 1)) * 180 / Math.PI);
  a.joints.forEach((j, k) => { worstJoint = Math.max(worstJoint, Math.abs(j - b.joints[k]) / Math.max(1, Math.abs(b.joints[k]))); });
  if (a.gear !== b.gear || a.contacts.length !== b.contacts.length) throw new Error(`frame ${i}: gear or contacts differ`);
});
console.log(`frames ${bin.frames.length}; worst position error ${(worstPos * 1000).toFixed(3)} mm, rotation ${worstRot.toFixed(5)} deg, joint (relative) ${worstJoint.toExponential(2)}`);
if (worstPos > 1e-3 || worstRot > 0.01 || worstJoint > 1e-4) throw new Error('decoder disagrees with the Rust codec');
console.log('js_decoder_agrees_with_the_rust_codec: ok');
