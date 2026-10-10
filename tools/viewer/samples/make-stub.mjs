// Writes the two STUB sample files for chart.mjs: impact-stub.json and ladder-stub.json. The numbers are invented, shaped by the
// first-order table in docs/validation/IMPACT-MATRIX.md (elasticity = d ln benchmark / d ln lever) and by Bekker's z = (p / (kc/b + kphi))^(1/n);
// they exist to show the layout until VALIDATION and TRACKS produce real ones. Run: node make-stub.mjs [outdir]
import fs from 'node:fs';
import path from 'node:path';

const out = process.argv[2] ?? path.dirname(new URL(import.meta.url).pathname);
const BENCH = [
  ['B1', '0-32 km/h time', 's', 4.8], ['B2', 'top speed', 'km/h', 112], ['B3', 'fuel range at 50 km/h', 'km', 520], ['B4', 'braking distance from 50 km/h', 'm', 17.5],
  ['B5', 'repeated-stop distance, 5th stop', 'm', 19.4], ['B6', 'maximum gradient', '%', 62], ['B7', 'side-slope limit', 'deg', 36],
  ['B8', 'soft-ground mobility (crossing mass)', 't', 4.6], ['B9', 'washboard ride roughness', 'm/s^2', 2.7], ['B10', 'hump settling time', 's', 1.9],
  ['B11', 'obstacle: step height', 'm', 0.42], ['B12', 'cornering limit', 'g', 0.71], ['B13', 'minimum radius at 20 km/h', 'm', 6.4], ['B14', 'sinkage in mud', 'm', 0.17],
];
// lever id, name, { benchmark: [elasticity, signs the table expects for +10% on the lever (- + 0), or undefined when the table is silent] }
const LEVERS = [
  ['engine_power', 'Engine peak power', { B1: [-0.9, '-'], B2: [0.3, '+'], B3: [-0.05, '-0'], B6: [0.06, '+0'] }],
  ['torque_peak', 'Torque peak, lower rpm', { B6: [0.04, '+'], B1: [-0.03] }],
  ['gear1_ratio', 'First-gear ratio', { B6: [0.08, '+'], B2: [0, '0'], B1: [-0.12] }],
  ['final_drive', 'Final-drive ratio', { B1: [-0.35, '-'], B2: [-0.01, '-0'] }],
  ['gear_count', 'Gear count', { B3: [0.02, '+'] }],
  ['brake_torque', 'Brake torque capacity', { B4: [-0.02, '-0'] }],
  ['brake_thermal', 'Brake thermal mass', { B5: [-0.3, '-'], B4: [0, '0'] }],
  ['fuel_tank', 'Fuel tank capacity', { B3: [1.0, '+'], B1: [0.01] }],
  ['mass', 'Vehicle mass', { B1: [0.9, '+'], B4: [0.02, '0+'], B8: [-1.0, '-'], B6: [-0.4, '-'], B14: [1.2], B10: [0.1], B3: [-0.3], B2: [-0.03] }],
  ['com_height', 'Centre-of-mass height', { B7: [-0.9, '-'], B12: [-0.9, '-'], B4: [0.03, '0+'] }],
  ['wheelbase', 'Wheelbase', { B13: [0.9, '+'], B10: [0.05], B9: [-0.03] }],
  ['track_gauge', 'Track gauge', { B7: [0.9, '+'], B12: [0.8, '+'], B13: [0.05] }],
  ['drag_area', 'Frontal area x Cd', { B2: [-0.28, '-'], B3: [-0.25, '-'] }],
  ['clearance', 'Ground clearance', { B11: [0.004, '+'], B7: [-0.2, '-'] }],
  ['ride_freq', 'Ride frequency', { B9: [0.8, '+'], B12: [0.1, '+'], B10: [-0.4] }],
  ['damping', 'Damping ratio', { B10: [-0.45, '-'], B9: [0.25, '+'] }],
  ['susp_travel', 'Suspension travel', { B9: [-0.2, '-'] }],
  ['tyre_pressure', 'Tyre pressure', { B8: [-0.35, '-'], B3: [0.04, '+'], B14: [0.3] }],
  ['tyre_width', 'Tyre width', { B8: [0.5, '+'], B14: [-0.4] }],
  ['tyre_mu', 'Tyre peak friction', { B4: [-0.95, '-'], B12: [0.9, '+'], B6: [0.2] }],
];
const r = (v) => +v.toPrecision(5);
const cells = [];
for (const [lever, , per] of LEVERS) {
  for (const [bench, , , base] of BENCH) {
    const [e, expected] = per[bench] ?? [0];
    cells.push({ lever, benchmark: bench, minus: r(base * 0.9 ** e), plus: r(base * 1.1 ** e), ...(expected ? { expected: [...expected].map((c) => c) } : {}) });
  }
}
const impact = {
  schema: 'w5k-impact-1', stub: true, vehicle: 'utility_4x4 (Mule)', perturb_pct: 10, deadband_pct: 0.5,
  benchmarks: BENCH.map(([id, name, unit, baseline]) => ({ id, name, unit, baseline })), levers: LEVERS.map(([id, name]) => ({ id, name })), cells,
};
fs.writeFileSync(path.join(out, 'impact-stub.json'), JSON.stringify(impact, null, 0).replace(/\{"lever"/g, '\n{"lever"') + '\n');

// Ladder: add a tonne at a time to each vehicle; Bekker with n = 0.8, kc = 5 kN/m^(n+1), kphi = 400 kN/m^(n+2) (invented, "mud").
const bekker = (massKg, areaM2, widthM) => ((massKg * 9.81 / areaM2 / 1000) / (5 / widthM + 400)) ** (1 / 0.8);
const VEH = [['mule', 'Mule', 2600, 0.233, 0.265, 4], ['hauler', 'Hauler', 6500, 0.48, 0.4, 5], ['carrier', 'Carrier', 11000, 1.98, 0.38, 6]];
const vehicles = VEH.map(([id, name, empty, area, width, steps]) => ({
  id, name, rungs: Array.from({ length: steps + 1 }, (_, k) => {
    const theory = bekker(empty + 1000 * k, area, width), sim = theory * (1 + 0.04 + 0.03 * k);
    return { x: k, sim: r(sim), theory: r(theory) };
  }),
}));
const ladder = {
  schema: 'w5k-ladder-1', stub: true, title: 'Sinkage in the mud pit as load climbs the ladder', x: { name: 'added load', unit: 't' }, y: { name: 'sinkage', unit: 'm' },
  threshold: { name: 'bogged', value: 0.35 }, tolerance_pct: 25, vehicles,
};
fs.writeFileSync(path.join(out, 'ladder-stub.json'), JSON.stringify(ladder, null, 0).replace(/\{"id"/g, '\n{"id"') + '\n');
console.log(`wrote impact-stub.json (${cells.length} cells) and ladder-stub.json to ${out}`);
