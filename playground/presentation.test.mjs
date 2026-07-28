// The presentation profile's rules, tested without a browser.
//
// Run with: node --test playground/

import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  VIEW_ORDER,
  analyzeModes,
  applyRun,
  applyRunMobile,
  cycle,
  detectChanges,
  operandCells,
  selectSurface,
} from './presentation.js';

const layout = (left, right) => ({ left, right });

test('choosing Output pulls the right column to Stack', () => {
  assert.deepEqual(selectSurface(layout('input', 'dictionary'), 'output'), {
    left: 'output',
    right: 'stack',
  });
});

test('choosing Dictionary pulls the left column to Input', () => {
  assert.deepEqual(selectSurface(layout('output', 'stack'), 'dictionary'), {
    left: 'input',
    right: 'dictionary',
  });
});

test('Output and Dictionary are never shown together', () => {
  // Every configuration reachable by any sequence of selections.
  const seen = new Set();
  const frontier = [layout('input', 'stack')];
  while (frontier.length) {
    const current = frontier.pop();
    const key = `${current.left}/${current.right}`;
    if (seen.has(key)) continue;
    seen.add(key);
    for (const surface of VIEW_ORDER) frontier.push(selectSurface(current, surface));
  }
  assert.ok(!seen.has('output/dictionary'), [...seen].join(' '));
  assert.ok(seen.size > 1, 'more than one configuration is reachable');
});

test('selection is idempotent', () => {
  for (const surface of VIEW_ORDER) {
    const once = selectSurface(layout('input', 'stack'), surface);
    assert.deepEqual(selectSurface(once, surface), once, surface);
  }
});

test('every surface is reachable by selection', () => {
  for (const surface of VIEW_ORDER) {
    const next = selectSurface(layout('input', 'stack'), surface);
    assert.ok(next.left === surface || next.right === surface, surface);
  }
});

test('a run moves the layout to what it touched', () => {
  const start = layout('input', 'dictionary');
  assert.deepEqual(applyRun(start, { stack: true }), { left: 'input', right: 'stack' });
  assert.deepEqual(applyRun(start, { output: true }), {
    left: 'output',
    right: 'dictionary',
  });
  assert.deepEqual(applyRun(start, { output: true, stack: true }), {
    left: 'output',
    right: 'stack',
  });
  // Dictionary outranks Stack: defining a word is the more notable change.
  assert.deepEqual(applyRun(start, { stack: true, dictionary: true }), {
    left: 'input',
    right: 'dictionary',
  });
});

test('a run that changed nothing moves nothing', () => {
  const start = layout('output', 'stack');
  assert.deepEqual(applyRun(start, {}), start);
  assert.equal(applyRunMobile('input', {}), 'input');
});

test('a single-surface layout shows the most notable change', () => {
  assert.equal(applyRunMobile('input', { stack: true }), 'stack');
  assert.equal(applyRunMobile('input', { output: true, stack: true }), 'output');
  assert.equal(
    applyRunMobile('input', { output: true, stack: true, dictionary: true }),
    'dictionary',
  );
});

test('cycling visits every surface and wraps both ways', () => {
  let at = 'input';
  const visited = [at];
  for (let i = 0; i < 3; i += 1) {
    at = cycle(at, 'left');
    visited.push(at);
  }
  assert.deepEqual(visited, VIEW_ORDER);
  assert.equal(cycle('dictionary', 'left'), 'input');
  assert.equal(cycle('input', 'right'), 'dictionary');
});

test('a failed run always counts as changing Output', () => {
  const state = { stack: [], definitions: [] };
  const changes = detectChanges(state, state, { ok: false, error: 'boom' });
  assert.equal(changes.output, true);
  assert.equal(changes.stack, false);
});

test('a definition is detected and named', () => {
  const before = { stack: [], definitions: [] };
  const after = { stack: [], definitions: [{ name: 'DOUBLE', body: '{ 2 MUL }' }] };
  const changes = detectChanges(before, after, { ok: true });
  assert.equal(changes.dictionary, true);
  assert.equal(changes.reveal, 'DOUBLE');
});

test('the dictionary comparison ignores order', () => {
  const words = [
    { name: 'A', body: '{ 1 }' },
    { name: 'B', body: '{ 2 }' },
  ];
  const changes = detectChanges(
    { stack: [], definitions: words },
    { stack: [], definitions: [...words].reverse() },
    { ok: true },
  );
  assert.equal(changes.dictionary, false, 'reordering is not a change');
});

test('modes are read as whole tokens, so numbers never trigger', () => {
  assert.deepEqual(analyzeModes('1 2 ADD'), { target: 'top', retention: 'eat' });
  assert.deepEqual(analyzeModes('1 2 3 : +'), { target: 'stak', retention: 'eat' });
  assert.deepEqual(analyzeModes('1 2 & +'), { target: 'top', retention: 'keep' });
  assert.deepEqual(analyzeModes('1 2 3 STAK KEEP ADD'), {
    target: 'stak',
    retention: 'keep',
  });
  // The alias and the word are the same thing here, as everywhere.
  assert.deepEqual(analyzeModes('1 2 : & +'), analyzeModes('1 2 STAK KEEP ADD'));
  // Decimals are numbers.
  assert.deepEqual(analyzeModes('.5 5. 0.5 ADD'), { target: 'top', retention: 'eat' });
  // Lower case is the same word.
  assert.deepEqual(analyzeModes('1 2 keep add'), { target: 'top', retention: 'keep' });
});

test('the operand hint marks the surface cell, or all of them', () => {
  assert.deepEqual(operandCells(3, { target: 'top' }), [2]);
  assert.deepEqual(operandCells(3, { target: 'stak' }), [0, 1, 2]);
  assert.deepEqual(operandCells(0, { target: 'stak' }), []);
});
