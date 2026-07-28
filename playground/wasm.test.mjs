// The WebAssembly boundary, tested without a browser.
//
// `ajisai-wasm` has its own Rust tests; these check the other side of the
// contract — that a JavaScript host can actually drive the raw ABI, and that
// what comes back is the JSON the playground expects.
//
// Requires the module to have been built:
//   cargo build -p ajisai-wasm --target wasm32-unknown-unknown --release

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const wasmPath = fileURLToPath(new URL('./ajisai.wasm', import.meta.url));
const { instance } = await WebAssembly.instantiate(readFileSync(wasmPath), {});
const core = instance.exports;

const encoder = new TextEncoder();
const decoder = new TextDecoder();

const takeReply = (length) =>
  JSON.parse(decoder.decode(new Uint8Array(core.memory.buffer, core.ajisai_reply(), length)));

const call = (entry, source) => {
  const bytes = encoder.encode(source ?? '');
  const pointer = core.ajisai_alloc(bytes.length);
  try {
    if (bytes.length) new Uint8Array(core.memory.buffer, pointer, bytes.length).set(bytes);
    return takeReply(entry(pointer, bytes.length));
  } finally {
    core.ajisai_free(pointer, bytes.length);
  }
};

const reset = () => takeReply(core.ajisai_reset());

test('the module exports the whole protocol', () => {
  for (const name of [
    'memory',
    'ajisai_alloc',
    'ajisai_free',
    'ajisai_reply',
    'ajisai_execute',
    'ajisai_lint',
    'ajisai_format',
    'ajisai_steps',
    'ajisai_vocabulary',
    'ajisai_reset',
    'ajisai_snapshot',
  ]) {
    assert.equal(typeof core[name], name === 'memory' ? 'object' : 'function', name);
  }
});

test('a program crosses the boundary and comes back exact', () => {
  reset();
  assert.deepEqual(call(core.ajisai_execute, '1 3 DIV 3 MUL').stack, ['1']);
  reset();
  assert.deepEqual(call(core.ajisai_execute, '0.1 0.2 ADD').stack, ['3/10']);
  reset();
});

test('the session persists across calls', () => {
  reset();
  call(core.ajisai_execute, '1 2');
  assert.deepEqual(call(core.ajisai_execute, 'ADD').stack, ['3']);
  reset();
});

test('an error reports the flow the failing word left', () => {
  reset();
  const reply = call(core.ajisai_execute, '7 1 0 DIV');
  assert.equal(reply.ok, false);
  assert.match(reply.error, /division by zero/);
  assert.deepEqual(reply.stack, ['7', '1', '0']);
  reset();
});

test('definitions come back as source a host can show and re-run', () => {
  reset();
  const reply = call(core.ajisai_execute, '{ 2 MUL } "DOUBLE" DEF');
  assert.deepEqual(reply.definitions, [{ name: 'DOUBLE', body: '{ 2 MUL }' }]);
  // The exported form is a program that recreates the word.
  reset();
  const again = call(core.ajisai_execute, '{ 2 MUL } "DOUBLE" DEF 21 DOUBLE');
  assert.deepEqual(again.stack, ['42']);
  reset();
});

test('a step keeps a vent with its unit', () => {
  const reply = call(core.ajisai_steps, 'TRUE VENT { 1 0 DIV } 7');
  assert.deepEqual(reply.steps, ['TRUE', 'VENT { 1 0 DIV }', '7']);
});

test('a step keeps a mode with the word it governs', () => {
  assert.deepEqual(call(core.ajisai_steps, '1 2 3 STAK ADD').steps, ['1', '2', '3', 'STAK ADD']);
});

test('the formatter normalizes symbols to canonical words', () => {
  assert.equal(call(core.ajisai_format, '1 2 & + : ^ { 3 }').text, '1 2 KEEP ADD STAK VENT { 3 }');
});

test('the lint crosses as findings, not as a refusal to run', () => {
  reset();
  const findings = call(core.ajisai_lint, '[ 1 ] 2 ADD').findings;
  assert.equal(findings.length, 1);
  assert.equal(findings[0].severity, 'error');
  // Linting does not touch the session.
  assert.deepEqual(takeReply(core.ajisai_snapshot()).stack, []);
});

test('the vocabulary crosses with contracts intact', () => {
  const { words } = takeReply(core.ajisai_vocabulary());
  assert.equal(words.length, 54);
  const add = words.find((word) => word.name === 'ADD');
  assert.deepEqual(add.aliases, ['+']);
  assert.equal(add.stack_effect, '( a b -- sum )');
  assert.equal(add.stak, 'fold-left');
  assert.equal(words.find((word) => word.name === 'EQ').stak, 'unsupported');
});

test('an enormous rendering is truncated rather than shipped whole', () => {
  reset();
  const reply = call(core.ajisai_execute, '0 200000 RANGE');
  assert.equal(reply.ok, true);
  assert.ok(reply.stack[0].endsWith('…'), 'should be truncated');
  assert.ok(reply.stack[0].length < 5000, 'should stay small');
  reset();
});

test('text and quotes survive the JSON encoding', () => {
  reset();
  const reply = call(core.ajisai_execute, '"a\\"b\\nc" { 1 ADD }');
  assert.equal(reply.ok, true);
  assert.deepEqual(reply.stack, ['"a\\"b\\nc"', '{ 1 ADD }']);
  reset();
});

test('non-ASCII source crosses intact', () => {
  reset();
  const reply = call(core.ajisai_execute, '{ 2 MUL } "倍" DEF 21 倍');
  assert.deepEqual(reply.stack, ['42']);
  assert.equal(reply.definitions[0].name, '倍');
  reset();
});

test('reset empties the session', () => {
  call(core.ajisai_execute, '{ 1 } "X" DEF 9');
  const reply = reset();
  assert.deepEqual(reply.stack, []);
  assert.deepEqual(reply.definitions, []);
});

test('repeated allocation does not leak into the reply', () => {
  reset();
  for (let i = 0; i < 200; i += 1) {
    const reply = call(core.ajisai_execute, `${i} ${i} ADD DROP`);
    assert.equal(reply.ok, true, `iteration ${i}`);
  }
  assert.deepEqual(takeReply(core.ajisai_snapshot()).stack, []);
  reset();
});
