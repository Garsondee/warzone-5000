// Debug draw, HUD and the force-ledger panel. Everything here is read from the replay frame: no physics.
import * as THREE from 'three';
import { cellCentres } from './tracks.js';

// Order of `ForceTerm` in crates/w5k_contract/src/ledger.rs (the numbers appear in replays).
export const FORCE_TERMS = ['gravity', 'susp spring', 'susp damper', 'bump stop', 'anti-roll', 'tyre long.', 'tyre lat.', 'tyre normal', 'track shear', 'track normal', 'soil compaction', 'rolling res.', 'aero', 'engine drive', 'engine braking', 'brake', 'servo', 'recoil', 'collision', 'other', 'belly drag'];
const SLIPPING = 2, BOTTOMED = 8;
const BAR_M_PER_KN = 0.06; // bar length per kN of normal force; a 10 kN corner is 0.6 m
const BAR_MAX_M = 2.5; // a track cell that hits the far bank of a pit carries hundreds of kN: the bar stops at 2.5 m (the HUD says how much it is)
// Sinkage (m) to colour: sand to dark soil, one hue getting darker (a sequential scale: magnitude only), pale at 0 and full at 25 cm.
const SINK_STOPS = [[0, new THREE.Color(0xf1e8cf)], [0.1, new THREE.Color(0xc78d3c)], [0.25, new THREE.Color(0x4e2a0e)]];
export function sinkColor(z, out = new THREE.Color()) {
  let i = 1;
  while (i < SINK_STOPS.length - 1 && z > SINK_STOPS[i][0]) i++;
  const [z0, c0] = SINK_STOPS[i - 1], [z1, c1] = SINK_STOPS[i];
  return out.copy(c0).lerp(c1, Math.min(1, Math.max(0, (z - z0) / (z1 - z0))));
}

export function makeDebug(scene, built, header, vi = 0) {
  const group = new THREE.Group();
  scene.add(group);
  const names = header.vehicles[vi].contact_names;
  // A contact "<station>" or "<station>.<k>" is drawn at the wheel whose spin joint is "<station>.spin" (found by joint index, so a
  // skin with other node names still works), at the tyre radius below its centre.
  const jointNames = header.vehicles[vi].joint_names;
  const wheelNode = names.map((n) => {
    const spin = jointNames.indexOf(`${n.replace(/\.\d+$/, '')}.spin`);
    return built.nodes.findIndex((x) => x.def.joint && x.def.joint.kind === 'Revolute' && x.def.joint.index === spin && spin >= 0);
  });
  const radius = wheelNode.map((i) => {
    if (i < 0) return 0;
    let r = 0;
    built.nodes[i].g.children.forEach((c) => { if (c.isMesh) { c.geometry.computeBoundingBox(); r = Math.max(r, c.geometry.boundingBox.max.y); } });
    return r;
  });
  const bars = names.map(() => {
    const m = new THREE.Mesh(new THREE.BoxGeometry(0.12, 1, 0.12), new THREE.MeshBasicMaterial({ color: 0x2ee6a8 }));
    group.add(m);
    return m;
  });
  // A track's contacts are "<track>.<k>", one per sample along its ground run (k = 0 the rearmost). Each gets a ribbon of footprint on the ground just outboard
  // of the belt, coloured by its sinkage, and a force bar further out. The cells sit evenly along the straight ground run the band has under it, whose
  // length is the footprint's (test-tracks.mjs checks it against TRACKS's contact length); the belt's width and that length give the area, so the pressure.
  const tracks = built.belts.map((belt) => {
    const base = built.nodes[belt.run.node].def.name.replace(/\.belt$/, '');
    const ks = names.map((n, k) => (n.startsWith(`${base}.`) && /^\d+$/.test(n.slice(base.length + 1)) ? k : -1)).filter((k) => k >= 0);
    const ribbons = ks.map(() => {
      // drawn over the picture, not tested against it: the sim's ground (a pit's walls, 0.4 m deep) is finer than the terrain mesh drawn here, so a ribbon on the physics surface can lie under the mesh
      const m = new THREE.Mesh(new THREE.PlaneGeometry(1, 1), new THREE.MeshBasicMaterial({ color: 0xf1e8cf, side: THREE.DoubleSide, depthTest: false }));
      m.rotation.x = -Math.PI / 2;
      m.renderOrder = 9;
      built.nodes[belt.run.node].g.add(m);
      return m;
    });
    return { belt, ks, ribbons, name: base };
  }).filter((t) => t.ks.length);
  const cellOf = new Set(tracks.flatMap((t) => t.ks)); // contacts the track code places, not the wheel code
  const com = new THREE.Mesh(new THREE.SphereGeometry(0.18, 16, 12), new THREE.MeshBasicMaterial({ color: 0xff3366, depthTest: false }));
  com.renderOrder = 10;
  group.add(com);
  const vel = new THREE.ArrowHelper(new THREE.Vector3(0, 0, -1), new THREE.Vector3(), 1, 0x2266ff, 0.4, 0.25);
  group.add(vel);
  const wp = new THREE.Vector3(), side = new THREE.Vector3(), right = new THREE.Vector3();
  const flags = { contacts: true, com: true, velocity: true, footprint: true };
  return {
    flags, group,
    // What the HUD says about the tracks' ground contact: the deepest sinkage and the highest pressure on each (force over the cell's footprint).
    summary(frame, index = 0) {
      const v = frame.vehicles[index];
      return tracks.map((t) => {
        const g = t.belt.ground;
        if (!g) return '';
        const area = t.belt.width_m * cellCentres(g, t.ks.length).length, cs = t.ks.map((k) => v.contacts[k]);
        return `   ${t.name}: sinkage max ${(Math.max(...cs.map((c) => c.sinkage_m)) * 1000).toFixed(0)} mm, pressure max ${(Math.max(...cs.map((c) => c.normal_force_n)) / area / 1000).toFixed(0)} kPa`;
      }).join('');
    },
    cells: () => tracks.map((t) => ({ name: t.name, ribbons: t.ribbons.map((r) => ({ visible: r.visible, colour: r.material.color.getHex(), pos: r.position.toArray(), scale: r.scale.toArray() })) })), // the smoke test reads these
    update(frame, index = 0) {
      const v = frame.vehicles[index];
      built.root.updateMatrixWorld(true);
      for (const t of tracks) {
        const { belt } = t, g = belt.ground, node = built.nodes[belt.run.node].g;
        const { cells, length } = g ? cellCentres(g, t.ks.length) : { cells: [], length: 0 };
        const out = Math.sign(belt.x) || 1;
        t.ks.forEach((k, j) => {
          const c = v.contacts[k], ribbon = t.ribbons[j], bar = bars[k];
          const live = g && c && (c.flags & 1) !== 0;
          ribbon.visible = flags.footprint && live;
          bar.visible = flags.contacts && live;
          if (!live) return;
          const [z, y] = cells[j], yGround = y - belt.half_thickness_m + c.sinkage_m; // the surface is `sinkage_m` above the belt's lowest point
          ribbon.position.set(belt.x + out * (belt.width_m / 2 + 0.2), yGround + 0.01, z);
          ribbon.scale.set(0.3, length * 0.94, 1);
          sinkColor(c.sinkage_m, ribbon.material.color);
          const h = Math.min(BAR_MAX_M, Math.max(0.02, (c.normal_force_n / 1000) * BAR_M_PER_KN));
          bar.scale.y = h;
          wp.set(belt.x + out * (belt.width_m / 2 + 0.55), yGround, z);
          node.localToWorld(wp);
          bar.position.set(wp.x, wp.y + h / 2, wp.z);
          bar.material.color.setHex(c.flags & BOTTOMED ? 0xff2222 : c.flags & SLIPPING ? 0xffaa22 : 0x2ee6a8);
        });
      }
      names.forEach((_, k) => {
        if (cellOf.has(k)) return;
        const bar = bars[k], c = v.contacts[k];
        const i = wheelNode[k];
        bar.visible = flags.contacts && i >= 0 && c && (c.flags & 1) !== 0;
        if (!bar.visible) return;
        built.nodes[i].g.getWorldPosition(wp);
        const h = Math.max(0.02, (c.normal_force_n / 1000) * BAR_M_PER_KN);
        bar.scale.y = h;
        // Base at the lowest point of the tyre, standing up in world space, pushed 0.3 m outboard so the hull does not hide it.
        side.copy(wp);
        built.root.worldToLocal(side);
        const out = Math.sign(side.x) * 0.3;
        wp.addScaledVector(right.set(1, 0, 0).applyQuaternion(built.root.quaternion), out);
        bar.position.set(wp.x, wp.y - radius[k] + h / 2, wp.z);
        bar.material.color.setHex(c.flags & BOTTOMED ? 0xff2222 : c.flags & SLIPPING ? 0xffaa22 : 0x2ee6a8);
      });
      com.visible = flags.com;
      com.position.copy(built.root.position);
      const s = Math.hypot(v.lin_vel_m_s.x, v.lin_vel_m_s.y, v.lin_vel_m_s.z);
      vel.visible = flags.velocity && s > 0.1;
      if (vel.visible) {
        vel.position.copy(built.root.position).add({ x: 0, y: 1.0, z: 0 });
        vel.setDirection(new THREE.Vector3(v.lin_vel_m_s.x, v.lin_vel_m_s.y, v.lin_vel_m_s.z).normalize());
        vel.setLength(Math.min(4, 0.2 * s) + 0.5, 0.35, 0.2);
      }
    },
  };
}

export function updateHud(el, ledgerEl, frame, t, steerJoints = [], vi = 0, extra = '') {
  const v = frame.vehicles[vi];
  const speed = Math.hypot(v.lin_vel_m_s.x, v.lin_vel_m_s.y, v.lin_vel_m_s.z);
  const gear = v.gear === 0 ? 'N' : v.gear < 0 ? `R${-v.gear}` : String(v.gear);
  el.textContent = `${(speed * 3.6).toFixed(0)} km/h   ${v.engine_rpm.toFixed(0)} rpm   gear ${gear}   limit: ${v.limiting}   t ${t.toFixed(1)} s`;
  // Steering: a positive joint angle yaws the nose left (UNITS-AND-FRAMES), so right-positive degrees are the negated joint value.
  if (steerJoints.length) el.textContent += '   steer ' + steerJoints.map((j) => `${j.side} ${(-v.joints[j.i] * 180 / Math.PI).toFixed(1)}`).join(' ') + ' deg (+ right)';
  const top = Math.max(1, ...v.ledger_n);
  el.textContent += extra;
  ledgerEl.innerHTML = v.ledger_n.map((n, k) => (n > 0.5 ? `<div class="row"><span>${FORCE_TERMS[k] ?? 'term ' + k}</span><i style="width:${(n / top) * 90}px"></i><b>${n >= 1000 ? (n / 1000).toFixed(1) + ' kN' : n.toFixed(0) + ' N'}</b></div>` : '')).join('');
}

// Fleet HUD (replays with several vehicles): the leader (furthest driven) with its speed, and a small table of name, speed, gear, steer.
export function updateFleet(el, frame, fleet, header, driven, fi) {
  let lead = 0;
  driven.forEach((d, i) => { if (d[fi] > driven[lead][fi]) lead = i; });
  const speed = (v) => Math.hypot(v.lin_vel_m_s.x, v.lin_vel_m_s.y, v.lin_vel_m_s.z) * 3.6;
  const steer = (v, hv) => { const js = hv.joint_names.map((n, i) => ({ n, i })).filter((j) => j.n.endsWith('.steer')); return js.length ? `${(-v.joints[js[0].i] * 180 / Math.PI).toFixed(1)}` : '-'; };
  const rows = frame.vehicles.map((v, i) => {
    const g = v.gear === 0 ? 'N' : v.gear < 0 ? `R${-v.gear}` : String(v.gear);
    return `<tr${i === lead ? ' class="lead"' : ''}><td style="color:${fleet[i].accent}">${i === lead ? '\u25B6 ' : ''}${fleet[i].name}</td><td>${speed(v).toFixed(0)}</td><td>${g}</td><td>${steer(v, header.vehicles[i])}</td></tr>`;
  }).join('');
  el.innerHTML = `<div class="lead">leader ${fleet[lead].name}: ${speed(frame.vehicles[lead]).toFixed(0)} km/h</div><table><tr><th></th><th>km/h</th><th>gear</th><th>steer&deg;</th></tr>${rows}</table>`;
}
