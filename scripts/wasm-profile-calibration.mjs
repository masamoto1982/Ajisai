#!/usr/bin/env node
// What the meters charge per millisecond *under WebAssembly* — the engine the
// browser playground actually runs on.
//
// Run with:  node scripts/wasm-profile-calibration.mjs
//
// `rust/examples/work_meter_calibration.rs` and
// `rust/examples/collection_word_calibration.rs` measure the same meters on
// native. This exists because those two are not a proxy for this one, which is
// not obvious and was assumed once: measured back to back on one container,
// the native->WASM rate ratio is 2.1x for `numericWork`, 4.0x for
// `collectionWork` and 0.66x for `executionSteps`. A ratio that swings from
// "twice as fast" to "a third slower" depending on the meter means a single
// native calibration cannot make three ceilings bound equal time on WASM —
// which is the entire job of a derived host profile.
//
// Why the Node bundle rather than the browser one: `src/wasm/generated` is a
// `--target web` build that needs `fetch`, while `tools/mcp-server/wasm/
// generated` is `--target nodejs` and loads synchronously. Both come from the
// same Rust through the same wasm-pack profile in the same rebuild scripts, so
// the compiled module being measured is the module the playground ships. What
// differs is the embedding engine, and Node and Chrome are both V8 — so this
// measures a desktop-class V8, which is what the playground profile is
// derived for. A mobile browser is slower; see the profile's own doc comment.
//
// KNOWN FINDING, 2026-08-14 — read the collectionWork floor with this in mind.
// `UNIQUE 4k x 4096-digit` comes back roughly 300x below every other
// collection path (248 units/ms against 73,491 for `REVERSE 100k` here; 780
// against 45,025 on native release). That is not a property of WASM: it is
// `Fraction::hash` on the `Big` representation, which clones through
// `to_bigint_pair()` and runs a full `BigInt::gcd` once per element per scan.
// Measured in isolation, hashing costs 2.1 us at 64 digits, 21.8 us at 512 and
// 602 us at 4096 — superlinear in width — while the scan charges
// `work_limbs(bits)`, which is linear (4, 27 and 213 units). So the charge and
// the cost diverge with width, reaching ~470x at 4096 digits.
//
// Until that is resolved, the collectionWork floor printed below is the floor
// of a meter with a known hole in it, and a budget derived from it is not
// meaningful. The de-quadraticization follow-up fixed exactly this shape of
// defect for the `Small` representation and left `Big` on the slow path.
//
// Method, matching the native harnesses after their 2026-08-14 fix:
//   - operand construction is executed first and subtracted, in both time and
//     charged units, so a rate is charged-work over the time that charged it;
//   - charged units are read back from the engine's own `resourceUsage`, never
//     derived from the pricing formula by hand;
//   - several repetitions, reporting the *median* — a minimum over a noisy
//     shared machine measures scheduling luck, not the path.

import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const bundleDir = resolve(here, '../tools/mcp-server/wasm/generated');
const require = createRequire(`${bundleDir}/`);
const wasm = require('./ajisai_core.js');

const STEP_LIMIT = 4_000_000_000;
const REPS = 3;

// `agent_compute` applies LOCAL_AGENT_RUNTIME_LIMITS (the MCP profile). Limits
// bound when work stops, not how fast it runs, so rates measured under them
// are the engine's rates — but every case below has to *fit*: 10,000,000
// numericWork, 20,000,000 collectionWork, 100,000 materializedElements.
async function once(source) {
    const started = process.hrtime.bigint();
    const json = await wasm.agent_compute(source, STEP_LIMIT);
    const millis = Number(process.hrtime.bigint() - started) / 1e6;
    const report = JSON.parse(json);
    if (report.status !== 'ok') {
        throw new Error(`case must complete to be measurable, got: ${report.message ?? report.status}`);
    }
    return { millis, usage: report.resourceUsage };
}

const median = (xs) => [...xs].sort((a, b) => a - b)[Math.floor(xs.length / 2)];

/**
 * Rate for `meter` over `ops`, with `setup`'s time and units subtracted.
 *
 * Setup and ops run in one `execute`, and the subtrahend is `setup` alone in
 * one `execute`, because that is the interval a ceiling actually governs: a
 * runaway computation is a loop *inside* one execute. Crossing an execute
 * boundary costs a large fixed amount on a big operand — ~380 ms to first
 * touch a 100,000-element vector carried over from a previous execute, against
 * ~11 ms for the same Word repeated inside one — since the epoch snapshot
 * shares the stack and the first mutation has to unshare it. That constant is
 * paid once per run, not once per iteration, so folding it into a per-unit
 * rate would misprice every budget derived from it.
 *
 * Returns `null` when the ops are too cheap to separate from setup jitter.
 * That is a real answer, not a failure: a path that cannot be timed against
 * its own setup is nowhere near the floor, which is the only thing this
 * harness is looking for.
 */
async function rate(meter, setup, ops) {
    const samples = [];
    let units = 0;
    for (let i = 0; i < REPS; i++) {
        const base = setup ? await once(setup) : { millis: 0, usage: { [meter]: 0 } };
        const full = await once(`${setup}${ops}`);
        units = full.usage[meter] - (base.usage[meter] ?? 0);
        const deltaMs = full.millis - base.millis;
        // Require the ops to be both positive and clear of setup jitter.
        if (deltaMs > 1 && deltaMs > base.millis * 0.02) samples.push(units / deltaMs);
    }
    if (!samples.length) return { units, rate: null };
    return { units, rate: median(samples) };
}

const wide = '9'.repeat(4096);
const PAIRS = [[2, 3], [5, 7], [11, 13], [17, 19], [23, 29], [31, 37], [41, 43], [47, 53]];
const cascade = (f) => PAIRS.slice(0, f).map(([l, r], i) => `${l} SQRT ${r} SQRT +${i > 0 ? ' *' : ''}`).join(' ');

const NUMERIC = [
    // The floor candidate: one charged unit per lane, a machine-word add
    // inside a tensor kernel, with nothing else to amortize it.
    ['dense tensor lanes (100k) x80', '[ 0 99999 ] RANGE', ' 1 +'.repeat(80)],
    // 19, not 20: the twentieth multiplication crosses the MCP profile's
    // `bigintBits` (272,133 against 262,144), which is the very boundary
    // `profile_liveness_tests` pins. Measuring under a real profile means
    // living inside every one of its ceilings, not just the one being timed.
    ['scalar wide MUL x19 (4096-digit)', `{ ${wide} * } 'M' DEF 1`, ' M'.repeat(19)],
    ['scalar wide ADD x200 (4096-digit)', `{ ${wide} + } 'W' DEF 1`, ' W'.repeat(200)],
    ['algebraic cascade(8) x18', `{ ${cascade(8)} } 'C' DEF`, ' C'.repeat(18)],
];

const COLLECTION = [
    // Repetition counts are capped by the MCP profile's 20,000,000
    // `collectionWork`, which the *setup* also spends: `[ 0 99999 ] RANGE`
    // charges 1,700,000 before a single measured Word runs.
    ['REVERSE 100k x10', '[ 0 99999 ] RANGE', ' REVERSE'.repeat(10)],
    ['UNIQUE 100k all-distinct x5', '[ 0 99999 ] RANGE', ' UNIQUE'.repeat(5)],
    // Built with FILL, not `RANGE { 1 MOD } MAP`: the MAP costs 100,000
    // execution steps, and a setup that large and that variable swamps the
    // few milliseconds nine hash-hitting UNIQUE passes actually take.
    ['UNIQUE 100k d=1 x9', '[ 100000 7 ] FILL', ' UNIQUE'.repeat(9)],
    ['SORT 100k x5', '[ 0 99999 ] RANGE', ' SORT'.repeat(5)],
    ['TALLY 100k all-distinct x5', '[ 0 99999 ] RANGE', ' TALLY'.repeat(5)],
    // Wide elements: an equality probe walks limbs, and this was by far the
    // dearest per-element shape on native. If any collection path is the WASM
    // floor, it is a candidate.
    ['UNIQUE 4k x 4096-digit x2', `[ 1 4000 ] RANGE { ${wide} * } MAP`, ' UNIQUE'.repeat(2)],
];

const STEPS = [
    ['add loop x200k', '', `1${' 7 +'.repeat(200_000)}`],
    // Every iteration pays a dictionary lookup and a frame push, not just an
    // arithmetic op. This was the native floor and is the shape
    // `cli::step_limit_tests::DOWN_PROBE` uses.
    ['DOWN trampoline x200k', '', `{\n{ [ 0 ] > | [ 1 ] - DOWN }\n{ IDLE | [ 'done' ] } COND\n} 'DOWN' DEF\n200000 DOWN`],
];

async function section(title, meter, cases) {
    console.log(`\n── ${title} ${'─'.repeat(Math.max(0, 52 - title.length))}\n`);
    console.log(`${'path'.padEnd(36)}${'units'.padStart(14)}${'units/ms'.padStart(12)}`);
    let floor = Infinity;
    let floorName = '';
    for (const [name, setup, ops] of cases) {
        const r = await rate(meter, setup, ops);
        if (r.rate === null) {
            console.log(`${name.padEnd(36)}${String(r.units).padStart(14)}${'too fast'.padStart(12)}`);
            continue;
        }
        if (r.rate < floor) { floor = r.rate; floorName = name; }
        console.log(`${name.padEnd(36)}${String(r.units).padStart(14)}${r.rate.toFixed(0).padStart(12)}`);
    }
    console.log(`\nfloor: ${floor.toFixed(0)} ${meter === 'executionSteps' ? 'steps' : 'units'}/ms  (${floorName})`);
    return floor;
}

const numeric = await section('numericWork, under WASM', 'numericWork', NUMERIC);
const collection = await section('collectionWork, under WASM', 'collectionWork', COLLECTION);
const steps = await section('executionSteps, under WASM', 'executionSteps', STEPS);

console.log('\n── derived budgets at a 30s host time budget ────────────\n');
for (const [name, floor] of [['numericWork', numeric], ['collectionWork', collection], ['executionSteps', steps]]) {
    console.log(`${name.padEnd(16)} 30000 ms x ${floor.toFixed(0).padStart(8)} = ${Math.round(30_000 * floor).toLocaleString()}`);
}
console.log(
    '\nA budget here is only as good as the floor above it. Check that the\n' +
    'floor path is one the meter prices correctly before deriving anything\n' +
    'from it — see the KNOWN FINDING at the top of this file, which is why\n' +
    'the collectionWork row currently is not.'
);
