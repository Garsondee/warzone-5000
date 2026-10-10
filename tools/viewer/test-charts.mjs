// Checks of the chart models (what is drawn, flagged and counted), no browser needed:  node tools/viewer/test-charts.mjs
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { tornadoModel, ladderModel, verdictOf, niceCeil, ticks } from './src/charts.js';

let failed = 0;
const test = (name, fn) => { try { fn(); console.log('PASS', name); } catch (e) { failed++; console.log('FAIL', name, '\n  ', e.message); } };
const near = (a, b, tol = 1e-9) => assert.ok(Math.abs(a - b) <= tol, `${a} is not within ${tol} of ${b}`);

const bench = (id, baseline = 10, name = id) => ({ id, name, unit: 'u', baseline });
const spec = (benchmarks, levers, cells, extra = {}) => ({ schema: 'w5k-impact-1', benchmarks, levers: levers.map((id) => ({ id, name: id })), cells, ...extra });
const cell = (lever, benchmark, minus, plus, expected) => ({ lever, benchmark, minus, plus, ...(expected ? { expected } : {}) });

test('percent_change_is_relative_to_the_baseline', () => {
  const m = tornadoModel(spec([bench('B1', 4.8)], ['a'], [cell('a', 'B1', 5.28, 4.32)]));
  near(m.panels[0].rows[0].dm, 10, 1e-9); near(m.panels[0].rows[0].dp, -10, 1e-9);
});

test('a_zero_baseline_is_reported_in_the_benchmarks_own_unit_not_a_percentage', () => {
  const m = tornadoModel(spec([bench('B1', 0)], ['a'], [cell('a', 'B1', -0.02, 0.03)]));
  assert.equal(m.panels[0].abs, true); near(m.panels[0].rows[0].dp, 0.03);
});

test('a_zero_baseline_benchmark_uses_its_own_dead_band_in_its_own_unit', () => {
  const b = { ...bench('B1', 0), deadband: 0.05 };
  const m = tornadoModel(spec([b], ['a', 'b'], [cell('a', 'B1', -0.02, 0.03), cell('b', 'B1', -0.2, 0.3)]));
  assert.deepEqual(m.panels[0].rows.map((r) => r.lever), ['b']); assert.equal(m.panels[0].hiddenSmall, 1);
});

test('levers_are_sorted_by_the_size_of_their_effect_largest_first_and_ties_keep_the_spec_order', () => {
  const m = tornadoModel(spec([bench('B1')], ['a', 'b', 'c', 'd'], [cell('a', 'B1', 9.9, 10.1), cell('b', 'B1', 8, 12), cell('c', 'B1', 10.5, 9.5), cell('d', 'B1', 9.5, 10.5)]));
  assert.deepEqual(m.panels[0].rows.map((r) => r.lever), ['b', 'c', 'd', 'a']);
});

test('a_lever_that_moves_nothing_anywhere_is_listed_as_dead_and_one_that_moves_something_is_not', () => {
  const m = tornadoModel(spec([bench('B1'), bench('B2')], ['live', 'dead'], [cell('live', 'B1', 9, 11), cell('live', 'B2', 10, 10), cell('dead', 'B1', 10, 10.01), cell('dead', 'B2', 10, 10)]));
  assert.deepEqual(m.deadLevers, ['dead']);
});

test('a_benchmark_nothing_moves_is_an_orphan_effect_and_one_with_no_cells_is_only_unmeasured', () => {
  const m = tornadoModel(spec([bench('B1'), bench('B2'), bench('B3')], ['a'], [cell('a', 'B1', 9, 11), cell('a', 'B2', 10, 10)]));
  assert.deepEqual(m.orphans.map((b) => b.id), ['B2']);
  assert.equal(m.panels[2].measured, false);
});

test('a_wrong_sign_a_missing_effect_and_a_spurious_one_are_told_apart', () => {
  assert.equal(verdictOf(['-'], '-'), 'ok'); assert.equal(verdictOf(['-', '0'], '0'), 'ok');
  assert.equal(verdictOf(['-'], '+'), 'wrong_sign'); assert.equal(verdictOf(['+'], '0'), 'missing');
  assert.equal(verdictOf(['0'], '+'), 'spurious'); assert.equal(verdictOf(undefined, '+'), 'unchecked');
});

test('the_expected_signs_are_judged_on_the_plus_side_with_the_dead_band_and_counted', () => {
  const m = tornadoModel(spec([bench('B1')], ['a', 'b', 'c'], [cell('a', 'B1', 11, 9, ['-']), cell('b', 'B1', 9, 11, ['-']), cell('c', 'B1', 10, 10.02, ['+'])]));
  assert.deepEqual(m.panels[0].rows.map((r) => [r.lever, r.verdict]).sort(), [['a', 'ok'], ['b', 'wrong_sign'], ['c', 'missing']]);
  assert.deepEqual(m.checks, { total: 3, right: 1 });
});

test('a_failed_check_is_drawn_even_when_its_effect_is_below_the_dead_band', () => {
  const m = tornadoModel(spec([bench('B1')], ['big', 'tiny'], [cell('big', 'B1', 9, 11, ['+']), cell('tiny', 'B1', 10, 10.01, ['+'])]));
  assert.deepEqual(m.panels[0].rows.map((r) => r.lever), ['big', 'tiny']); assert.equal(m.panels[0].hiddenSmall, 0);
});

test('only_the_biggest_rows_are_drawn_and_the_rest_are_counted_not_dropped_silently', () => {
  const levers = ['a', 'b', 'c', 'd', 'e'];
  const cells = levers.map((l, i) => cell(l, 'B1', 10 - (i + 1), 10 + (i + 1))).concat([cell('f', 'B1', 10, 10)]);
  const m = tornadoModel(spec([bench('B1')], [...levers, 'f'], cells), { maxRows: 3 });
  assert.equal(m.panels[0].rows.length, 3); assert.equal(m.panels[0].hiddenBig, 2); assert.equal(m.panels[0].hiddenSmall, 1);
});

test('a_verdict_from_the_runner_overrides_the_one_derived_here', () => {
  const c = { ...cell('a', 'B1', 9, 11, ['+']), verdict: 'wrong_sign' };
  assert.equal(tornadoModel(spec([bench('B1')], ['a'], [c])).panels[0].rows[0].verdict, 'wrong_sign');
});

test('a_cell_naming_an_unknown_lever_or_benchmark_is_refused_not_dropped', () => {
  assert.throws(() => tornadoModel(spec([bench('B1')], ['a'], [cell('zzz', 'B1', 9, 11)])), /unknown lever or benchmark/);
  assert.throws(() => tornadoModel(spec([bench('B1')], ['a'], [cell('a', 'B9', 9, 11)])), /unknown lever or benchmark/);
});

test('a_file_of_another_schema_is_refused', () => {
  assert.throws(() => tornadoModel({ schema: 'nope' }), /w5k-impact-1/); assert.throws(() => ladderModel({ schema: 'nope' }), /w5k-ladder-1/);
});

const ladder = (rungs, extra = {}) => ({ schema: 'w5k-ladder-1', x: { name: 'load', unit: 't' }, y: { name: 'sinkage', unit: 'm' }, threshold: { name: 'bogged', value: 0.3 }, vehicles: [{ id: 'v', name: 'V', rungs }], ...extra });

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
test('the_stub_impact_sample_has_the_four_failed_checks_three_dead_levers_and_one_orphan_it_was_built_with', () => {
  const m = tornadoModel(JSON.parse(fs.readFileSync(new URL('impact-stub.json', here), 'utf8')));
  const failed = m.panels.flatMap((p) => p.rows.filter((r) => r.verdict !== 'ok' && r.verdict !== 'unchecked')).length;
  assert.equal(failed, 4); assert.equal(m.deadLevers.length, 3); assert.deepEqual(m.orphans.map((b) => b.id), ['B11']);
});

test('the_stub_ladder_sample_bogs_the_two_wheeled_trucks_and_not_the_tracked_carrier', () => {
  const m = ladderModel(JSON.parse(fs.readFileSync(new URL('ladder-stub.json', here), 'utf8')));
  assert.deepEqual(m.vehicles.map((v) => v.bogIndex >= 0), [true, true, false]);
});

console.log(failed ? `${failed} FAILED` : 'all passed');
process.exit(failed ? 1 : 0);
