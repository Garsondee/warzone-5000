// Checks of the chart models (what is drawn, flagged and counted), no browser needed:  node tools/viewer/test-charts.mjs
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { tornadoModel, ladderModel, niceCeil, ticks } from './src/charts.js';

let failed = 0;
const test = (name, fn) => { try { fn(); console.log('PASS', name); } catch (e) { failed++; console.log('FAIL', name, '\n  ', e.message); } };
const near = (a, b, tol = 1e-9) => assert.ok(Math.abs(a - b) <= tol, `${a} is not within ${tol} of ${b}`);

// An entry as `w5k validation impact` writes it: the benchmark's value at the baseline and with the lever +10%, the relative change, the sign and the verdict.
const entry = (vehicle, lever, bench, base, perturbed, verdict = 'Quiet') => {
  const delta = (perturbed - base) / Math.abs(base), observed = Math.abs(delta) < 0.005 ? 'Zero' : delta > 0 ? 'Plus' : 'Minus';
  return { vehicle, lever, bench, base, perturbed, delta, observed, allowed: [], verdict };
};
const report = (entries, extra = {}) => ({ entries, right: entries.filter((e) => e.verdict === 'Right').length, scored: entries.filter((e) => ['Right', 'Wrong'].includes(e.verdict)).length, dead_levers: [], orphan_benchmarks: [], ...extra });

test('percent_change_is_the_runners_relative_delta_times_one_hundred', () => {
  const m = tornadoModel(report([entry('a_4x4', 'mass', 'B1', 4.8, 5.28, 'Right')]));
  near(m.panels[0].rows[0].bars[0].pct, 10, 1e-9);
});

test('benchmarks_are_ordered_by_their_number_not_by_their_text', () => {
  const m = tornadoModel(report(['B11', 'B4', 'B1', 'B12'].map((b) => entry('a_4x4', 'mass', b, 1, 1.1))));
  assert.deepEqual(m.panels.map((p) => p.bench.id), ['B1', 'B4', 'B11', 'B12']);
});

test('levers_are_sorted_by_the_size_of_their_effect_largest_first_and_ties_keep_the_runners_order', () => {
  const es = [['a', 1.01], ['b', 1.2], ['c', 0.95], ['d', 1.05]].map(([l, f]) => entry('v_4x4', l, 'B1', 10, 10 * f));
  assert.deepEqual(tornadoModel(report(es)).panels[0].rows.map((r) => r.lever), ['b', 'c', 'd', 'a']);
  const ties = [['x', 1.1], ['y', 1.1], ['z', 0.9]].map(([l, f]) => entry('v_4x4', l, 'B1', 10, 10 * f));
  assert.deepEqual(tornadoModel(report(ties)).panels[0].rows.map((r) => r.lever), ['x', 'y', 'z']);
});

test('one_bar_per_vehicle_in_the_order_the_vehicles_first_appear', () => {
  const m = tornadoModel(report([entry('hauler_4x4', 'mass', 'B1', 10, 11), entry('mule_4x4', 'mass', 'B1', 10, 12), entry('scout_4x4', 'mass', 'B1', 10, 13)]));
  assert.deepEqual(m.series.map((s) => s.name), ['Hauler', 'Mule', 'Scout']);
  assert.deepEqual(m.panels[0].rows[0].bars.map((b) => Math.round(b.pct)), [10, 20, 30]);
});

test('a_lever_the_runner_lists_as_dead_is_named_with_its_vehicle', () => {
  const m = tornadoModel(report([entry('mule_4x4', 'mass', 'B1', 10, 11), entry('mule_4x4', 'ground_clearance', 'B1', 10, 10)], { dead_levers: [['mule_4x4', 'ground_clearance']] }));
  assert.deepEqual(m.deadLevers, [{ vehicle: 'Mule', name: 'Ground clearance' }]);
});

test('a_benchmark_the_runner_calls_an_orphan_is_marked_and_without_a_list_it_is_found_by_nothing_moving', () => {
  const es = [entry('v_4x4', 'mass', 'B1', 10, 11), entry('v_4x4', 'mass', 'B4', 10, 10)];
  assert.deepEqual(tornadoModel(report(es, { orphan_benchmarks: ['B4'] })).orphans.map((b) => b.id), ['B4']);
  const bare = report(es); delete bare.orphan_benchmarks;
  assert.deepEqual(tornadoModel(bare).orphans.map((b) => b.id), ['B4']);
});

test('a_failed_check_is_drawn_even_when_its_bar_is_below_the_dead_band', () => {
  const m = tornadoModel(report([entry('v_4x4', 'big', 'B1', 10, 11, 'Right'), entry('v_4x4', 'tiny', 'B1', 10, 10.001, 'Wrong')]));
  assert.deepEqual(m.panels[0].rows.map((r) => [r.lever, r.flagged]), [['big', false], ['tiny', true]]); assert.equal(m.panels[0].hiddenSmall, 0);
});

test('only_the_biggest_rows_are_drawn_and_the_rest_are_counted_not_dropped_silently', () => {
  const es = ['a', 'b', 'c', 'd', 'e'].map((l, i) => entry('v_4x4', l, 'B1', 10, 10 + i + 1)).concat([entry('v_4x4', 'f', 'B1', 10, 10)]);
  const p = tornadoModel(report(es), { maxRows: 3 }).panels[0];
  assert.equal(p.rows.length, 3); assert.equal(p.hiddenBig, 2); assert.equal(p.hiddenSmall, 1);
});

test('the_runners_verdicts_and_counts_are_drawn_as_they_are_and_unlisted_effects_are_counted_not_scored', () => {
  const m = tornadoModel(report([entry('v_4x4', 'a', 'B1', 10, 11, 'Right'), entry('v_4x4', 'b', 'B1', 10, 12, 'Wrong'), entry('v_4x4', 'c', 'B1', 10, 13, 'Unlisted'), entry('v_4x4', 'd', 'B1', 10, 10, 'Quiet')]));
  assert.deepEqual(m.checks, { right: 1, scored: 2 }); assert.equal(m.unlisted, 1);
  assert.deepEqual(m.panels[0].rows.map((r) => [r.lever, r.flagged]), [['c', false], ['b', true], ['a', false]]);
});

test('an_entry_with_a_zero_baseline_has_no_percentage_and_is_counted_not_drawn', () => {
  const e = entry('v_4x4', 'a', 'B1', 10, 11); e.base = 0; e.delta = 1e300;
  const m = tornadoModel(report([e, entry('v_4x4', 'b', 'B1', 10, 12)]));
  assert.equal(m.undefinedPct, 1); assert.equal(m.panels[0].rows.find((r) => r.lever === 'a').bars[0].pct, null);
});

test('benchmarks_with_no_runner_yet_are_listed_not_hidden', () => {
  const m = tornadoModel(report([entry('v_4x4', 'a', 'B1', 10, 11), entry('v_4x4', 'a', 'B4', 10, 11)]));
  assert.equal(m.unrun.length, 14); assert.ok(m.unrun.some((b) => b.id === 'B2' && b.name === 'top speed')); assert.ok(!m.unrun.some((b) => b.id === 'B1'));
});

test('names_written_in_the_file_win_over_the_built_in_ones_and_unknown_ids_are_made_readable', () => {
  const m = tornadoModel(report([entry('v_4x4', 'engine_power', 'B1', 10, 11), entry('v_4x4', 'wing_span', 'B1', 10, 12)], { benchmarks: [{ id: 'B1', name: 'standing start' }], levers: [{ id: 'engine_power', name: 'Motor' }] }));
  assert.equal(m.panels[0].bench.name, 'standing start'); assert.deepEqual(m.panels[0].rows.map((r) => r.name).sort(), ['Motor', 'Wing span']);
});

test('a_file_that_is_not_an_impact_report_or_has_too_many_vehicles_is_refused', () => {
  assert.throws(() => tornadoModel({}), /impact\.json/); assert.throws(() => tornadoModel({ entries: [] }), /impact\.json/);
  const nine = Array.from({ length: 9 }, (_, i) => entry(`v${i}_4x4`, 'a', 'B1', 10, 11));
  assert.throws(() => tornadoModel(report(nine)), /9 vehicles/);
});

const ladder = (rungs, extra = {}) => ({ schema: 'w5k-ladder-1', x: { name: 'load', unit: 't' }, y: { name: 'sinkage', unit: 'm' }, threshold: { name: 'bogged', value: 0.3 }, vehicles: [{ id: 'v', name: 'V', rungs }], ...extra });

test('a_file_of_another_ladder_schema_is_refused', () => {
  assert.throws(() => ladderModel({ schema: 'nope' }), /w5k-ladder-1/);
});

test('a_vehicle_bogs_at_the_first_rung_whose_sinkage_reaches_the_bog_depth', () => {
  const m = ladderModel(ladder([{ x: 0, sim: 0.1, theory: 0.1 }, { x: 1, sim: 0.3, theory: 0.28 }, { x: 2, sim: 0.5, theory: 0.45 }]));
  assert.equal(m.vehicles[0].bogIndex, 1);
});

test('a_vehicle_that_never_reaches_the_bog_depth_never_bogs', () => {
  assert.equal(ladderModel(ladder([{ x: 0, sim: 0.1, theory: 0.1 }, { x: 1, sim: 0.2, theory: 0.2 }])).vehicles[0].bogIndex, -1);
});

test('the_runner_can_say_a_rung_is_bogged_whatever_the_depth', () => {
  assert.equal(ladderModel(ladder([{ x: 0, sim: 0.1, theory: 0.1 }, { x: 1, sim: 0.15, theory: 0.15, bogged: true }])).vehicles[0].bogIndex, 1);
});

test('deviation_from_the_theory_is_measured_up_to_the_bog_point_only', () => {
  const m = ladderModel(ladder([{ x: 0, sim: 0.11, theory: 0.1 }, { x: 1, sim: 0.3, theory: 0.27 }, { x: 2, sim: 0.9, theory: 0.4 }]));
  near(m.vehicles[0].maxDevPct, (100 * 0.03) / 0.27, 1e-9);
});

test('a_rung_with_no_simulated_value_is_skipped_not_counted_as_zero_deviation', () => {
  const m = ladderModel(ladder([{ x: 0, sim: null, theory: 0.1 }, { x: 1, sim: 0.2, theory: 0.2 }]));
  near(m.vehicles[0].maxDevPct, 0);
});

test('a_ladder_with_no_vehicle_no_rungs_or_more_vehicles_than_colours_is_refused', () => {
  assert.throws(() => ladderModel(ladder([])), /no rungs/); assert.throws(() => ladderModel({ ...ladder([{ x: 0, sim: 1, theory: 1 }]), vehicles: [] }), /1 to 8 vehicles/);
  const nine = Array.from({ length: 9 }, (_, i) => ({ id: `v${i}`, name: `V${i}`, rungs: [{ x: 0, sim: 0.1, theory: 0.1 }] }));
  assert.throws(() => ladderModel({ ...ladder([{ x: 0, sim: 1, theory: 1 }]), vehicles: nine }), /1 to 8 vehicles/);
});

test('axis_limits_and_ticks_are_round_numbers', () => {
  assert.equal(niceCeil(0.87), 1); assert.equal(niceCeil(1.01), 1.2); assert.equal(niceCeil(11), 12); assert.equal(niceCeil(41), 50);
  assert.deepEqual(ticks(0, 0.8, 5), [0, 0.2, 0.4, 0.6, 0.8]); assert.deepEqual(ticks(0, 7, 8), [0, 1, 2, 3, 4, 5, 6, 7]);
});

const here = new URL('./samples/', import.meta.url);
test('the_real_impact_sample_draws_six_benchmarks_for_three_vehicles_with_the_counts_its_runner_reported', () => {
  const file = JSON.parse(fs.readFileSync(new URL('impact.json', here), 'utf8')), m = tornadoModel(file);
  assert.deepEqual(m.panels.map((p) => p.bench.id), ['B1', 'B4', 'B6', 'B7', 'B11', 'B12']); assert.equal(m.series.length, 3);
  assert.deepEqual(m.checks, { right: file.entries.filter((e) => e.verdict === 'Right').length, scored: file.entries.filter((e) => ['Right', 'Wrong'].includes(e.verdict)).length });
  assert.equal(m.checks.right, file.right); assert.equal(m.checks.scored, file.scored);
});

test('the_stub_ladder_sample_bogs_the_two_wheeled_trucks_and_not_the_tracked_carrier', () => {
  const m = ladderModel(JSON.parse(fs.readFileSync(new URL('ladder-stub.json', here), 'utf8')));
  assert.deepEqual(m.vehicles.map((v) => v.bogIndex >= 0), [true, true, false]);
});

console.log(failed ? `${failed} FAILED` : 'all passed');
process.exit(failed ? 1 : 0);
