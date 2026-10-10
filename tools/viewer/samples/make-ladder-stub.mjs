// Writes ladder-stub.json, the STUB sample for `chart.mjs ladder`: invented numbers from Bekker's z = (p / (kc/b + kphi))^(1/n) with
// n = 0.8, kc = 5 kN/m^(n+1), kphi = 400 kN/m^(n+2) ("mud"), a simulation that runs a little deeper than the theory as the load grows, and a
// footprint per vehicle. It shows the layout until TRACKS's `w5k tracks bench ladder` writes real rungs. Run: node make-ladder-stub.mjs [outdir]
import fs from 'node:fs';
import path from 'node:path';

const out = process.argv[2] ?? path.dirname(new URL(import.meta.url).pathname);
const r = (v) => +v.toPrecision(5);
const bekker = (massKg, areaM2, widthM) => ((massKg * 9.81 / areaM2 / 1000) / (5 / widthM + 400)) ** (1 / 0.8);
// id, name, empty mass (kg), contact area (m^2), tyre or track width (m), rungs above empty (one tonne each)
const VEHICLES = [['mule', 'Mule', 2600, 0.233, 0.265, 3], ['hauler', 'Hauler', 6500, 0.48, 0.4, 4], ['carrier', 'Carrier', 11000, 1.98, 0.38, 6]];
const vehicles = VEHICLES.map(([id, name, empty, area, width, steps]) => ({
  id, name, rungs: Array.from({ length: steps + 1 }, (_, k) => {
    const theory = bekker(empty + 1000 * k, area, width);
    return { x: k, sim: r(theory * (1 + 0.04 + 0.03 * k)), theory: r(theory) };
  }),
}));
const ladder = {
  schema: 'w5k-ladder-1', stub: true, title: 'Sinkage in the mud pit as load climbs the ladder', x: { name: 'added load', unit: 't' }, y: { name: 'sinkage', unit: 'm' },
  threshold: { name: 'bogged', value: 0.35 }, tolerance_pct: 25, vehicles,
};
fs.writeFileSync(path.join(out, 'ladder-stub.json'), JSON.stringify(ladder).replace(/\{"id"/g, '\n{"id"') + '\n');
console.log(`wrote ladder-stub.json to ${out}`);
