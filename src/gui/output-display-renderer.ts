import katex from 'katex';
import 'katex/dist/katex.min.css';
import type { Value, ExecuteResult, ExactTerm, Fraction } from '../wasm-interpreter-types';
import { isFailure, readRecordParts, toError } from './interpreter-execution-utils';
import { createEmptyWordsElement } from './vocabulary-state-controller';

export interface DisplayElements {
    outputDisplay: HTMLElement;
    stackDisplay: HTMLElement;
}

export interface DisplayState {
    readonly mainOutput: string;
}

export interface Display {
    readonly init: () => void;
    readonly renderExecutionResult: (result: ExecuteResult) => void;
    readonly renderError: (
        error: Error | { message?: string } | string,
        precedingOutput?: string
    ) => void;
    readonly renderInfo: (text: string, append?: boolean) => void;
    readonly renderFoldedInfo: (label: string, text: string) => void;
    readonly renderDocumentation: (text: string) => void;
    readonly renderStack: (stack: Value[]) => void;
    readonly extractState: () => DisplayState;
}

// A signed decimal string split into its sign and magnitude, so a `-` is
// written once, in front, wherever a number is typeset from its parts.
const splitSign = (numerator: string): { readonly negative: boolean; readonly magnitude: string } => {
    const negative = numerator.startsWith('-');
    return { negative, magnitude: negative ? numerator.slice(1) : numerator };
};

// ── Render budget ───────────────────────────────────────────────────────────
// How much of a stack value the Stack area draws.
//
// The interpreter's materialization ceiling bounds what a generative Word may
// build, not what the host can draw. `1 500000 RANGE` sits well inside that
// ceiling and is an ordinary, correct program — drawn one DOM node per
// element, it would lock the browser tab for tens of seconds with no way to
// abort, clear the editor, or read the result. A safety mechanism that stops
// the interpreter and then hands the host an unbounded drawing job has only
// moved where the program hangs.
//
// So the Stack area draws a bounded prefix of any collection and says, in
// place, how many elements it left out. This is presentation, in the same class
// as the execution step limit: a host control, not a language constraint. The
// value on the stack is whole and unmodified, every Word still sees all of it,
// and `LENGTH` remains the way to ask how long it really is.

/** Elements drawn from any one collection before the rest is summarized. */
export const MAX_RENDERED_ELEMENTS_PER_COLLECTION = 100;

/**
 * Elements drawn across the whole Stack area in one render. A per-collection
 * cap alone still admits a stack of ten thousand short vectors, so the budget
 * is global to the render and every collection draws from it.
 */
export const MAX_RENDERED_ELEMENTS_PER_STACK = 2_000;

export interface RenderBudget {
    remaining: number;
}

export const createRenderBudget = (
    total: number = MAX_RENDERED_ELEMENTS_PER_STACK
): RenderBudget => ({ remaining: Math.max(0, total) });

export interface CollectionRenderPlan {
    /** Leading elements to draw. */
    readonly shown: number;
    /** Elements summarized instead of drawn; 0 means the collection is complete. */
    readonly elided: number;
}

/**
 * Decide how much of a `length`-element collection to draw, and charge the
 * drawn part to the shared budget. A nested collection costs one element in its
 * parent and then draws its own children from the same budget, so the total
 * node count of one render stays bounded whatever the nesting looks like.
 */
export const planCollectionRender = (length: number, budget: RenderBudget): CollectionRenderPlan => {
    const safeLength = Number.isFinite(length) && length > 0 ? Math.floor(length) : 0;
    const shown = Math.min(safeLength, MAX_RENDERED_ELEMENTS_PER_COLLECTION, budget.remaining);
    budget.remaining -= shown;
    return { shown, elided: safeLength - shown };
};

/**
 * The marker that stands in for the undrawn tail. It is deliberately not
 * Ajisai-shaped — a reader must never mistake it for part of the value — and it
 * states the exact count, so the display never implies the collection ends
 * where the drawing does.
 */
export const formatElision = (elided: number): string => `… ${elided} more`;

// ── Canonical text rendering ────────────────────────────────────────────────

// Coloured by nesting depth in CSS (`--bracket-depth-N`).
const createBracketSpan = (bracket: string, depth: number): HTMLSpanElement => {
    const span = document.createElement('span');
    span.className = 'stack-bracket';
    span.dataset.depth = String(depth);
    span.textContent = bracket;
    return span;
};

const checkFractionObject = (value: unknown): Record<string, unknown> | null => {
    if (!value || typeof value !== 'object') return null;
    const candidate = value as Record<string, unknown>;
    if (!('numerator' in candidate) || !('denominator' in candidate)) return null;
    return candidate;
};

// Canonical numeric rendering: every number is a reduced
// numerator/denominator, integers included (`3` -> `3/1`).
const formatNumber = (value: unknown): string => {
    const fraction = checkFractionObject(value);
    if (!fraction) return '?';
    return `${fraction.numerator}/${fraction.denominator}`;
};

// One element of a collection's literal, one level deeper; an element the
// renderer cannot read is drawn as `?` rather than taking the literal down.
const formatElementAt = (item: Value, depth: number): string => {
    try { return formatValue(item, depth); } catch { return '?'; }
};

// A Vector renders as source that rebuilds it — a bracket literal, whatever
// it holds — matching the engine's own renderer
// (`rust/src/types/display_source.rs`). A nested Record is one element,
// because it has a literal of its own (`formatRecord`).
//
// The empty Vector is `[ ]` and not `[]`: a bracket must stand alone
// (`spec/grammar.json`, `bracketMustStandAlone`), so `[]` is a source error
// rather than an empty Vector.
const formatVector = (value: unknown, depth: number): string => {
    if (!Array.isArray(value) || value.length === 0) return '[ ]';
    return `[ ${value.map((v: Value) => formatElementAt(v, depth + 1)).join(' ')} ]`;
};

// The undrawn tail of a collection, stated as a count rather than drawn. See
// the render budget above for why the Stack area is bounded at all.
const createElisionSpan = (elided: number): HTMLSpanElement => {
    const span = document.createElement('span');
    span.className = 'stack-elision';
    span.textContent = formatElision(elided);
    span.title = `${elided} more element(s) are on the stack but not drawn; LENGTH reports the full length.`;
    return span;
};

// An irrational's normal form Σ c·√r written exactly as the engine writes it
// (`rust/src/types/display.rs::render_algebraic_terms`): one spaceless token,
// terms in the normal form's order, `sqrt(r)` for a unit coefficient,
// `n/d*sqrt(r)` otherwise, and the rational term as `n/d` — the canonical
// display the Stack surface is required to show (spec/gui-semantics.md).
const formatNormalForm = (terms: ReadonlyArray<ExactTerm> | undefined): string | null => {
    if (!terms || terms.length === 0) return null;
    let out = '';
    terms.forEach((term, index) => {
        const { negative, magnitude } = splitSign(term.numerator);
        if (index === 0) {
            if (negative) out += '-';
        } else {
            out += negative ? '-' : '+';
        }
        if (term.radicand === '1') {
            out += `${magnitude}/${term.denominator}`;
        } else if (magnitude === '1' && term.denominator === '1') {
            out += `sqrt(${term.radicand})`;
        } else {
            out += `${magnitude}/${term.denominator}*sqrt(${term.radicand})`;
        }
    });
    return out;
};

const NIL: Value = { type: 'nil' } as Value;

// A collection's literal as DOM: `open`, the drawn children each preceded by
// a space, the elision marker for the rest, a space, `close` — the same
// spacing as the canonical text (`[ 1 2 ]`, `{ k v }`), which is source that
// rebuilds the value. Every child is drawn under the one render budget.
const renderCollectionNode = (
    node: HTMLElement,
    open: string,
    close: string,
    children: readonly Value[],
    depth: number,
    budget: RenderBudget
): HTMLElement => {
    const { shown, elided } = planCollectionRender(children.length, budget);
    node.dataset.depth = String(depth);
    node.appendChild(createBracketSpan(open, depth));
    for (let index = 0; index < shown; index++) {
        node.append(' ');
        node.appendChild(renderStackValueNode(children[index]!, depth + 1, budget));
    }
    if (elided > 0) {
        node.append(' ');
        node.appendChild(createElisionSpan(elided));
    }
    node.append(' ');
    node.appendChild(createBracketSpan(close, depth));
    return node;
};

const renderStackValueNode = (item: Value, depth: number, budget: RenderBudget): HTMLElement => {
    const node = document.createElement('span');
    node.className = 'stack-node';

    if (item.type === 'vector' && Array.isArray(item.value)) {
        node.classList.add('stack-node-vector');
        return renderCollectionNode(node, '[', ']', item.value as Value[], depth, budget);
    }

    // A Record's keys and values are two aligned arrays; they are drawn
    // interleaved, a missing value padded with NIL (see `formatRecord`). Each
    // pair counts as two children of the budget.
    if (item.type === 'record') {
        const { keys, values } = readRecordParts(item.value);
        const pairs = keys.flatMap((key, index) => [key, values[index] ?? NIL]);
        node.classList.add('stack-node-record');
        return renderCollectionNode(node, '{', '}', pairs, depth, budget);
    }

    if (depth === 1) {
        node.dataset.depth = String(depth);
    }
    node.textContent = formatValue(item, depth);
    if (item.type === 'nil') annotateNilNode(node, item);
    return node;
};

// A NIL's reason is its observable content (LANG.VALUES.NIL): `1 0 DIV` is a
// NIL whose reason is `divisionByZero`, and a Stack that shows only `NIL`
// leaves the reader to find out why in an Output area that may not be on
// screen. The canonical text stays `NIL` — the display the engine writes — and
// the reason is drawn after it as an annotation. Exported for
// `output-display-renderer.test.ts`.
export const describeNilNode = (item: Value): string => {
    const reason = item.semantics?.absence?.reason;
    return reason ? `NIL · ${reason}` : 'NIL';
};

// The visible text already carries the whole label, so the tooltip repeats it
// and assistive technology reads it from the text itself.
const annotateNilNode = (node: HTMLElement, item: Value): void => {
    const label = describeNilNode(item);
    node.classList.add('stack-node-nil');
    node.title = label;
    if (label === 'NIL') return;
    const reason = document.createElement('span');
    reason.className = 'stack-nil-reason';
    reason.textContent = label.slice('NIL'.length);
    node.appendChild(reason);
};

// A number as the engine displays it. An irrational's `n/d` is only an
// approximation, so its normal form (`semantics.exactTerms`) is written
// instead; a host that sends none gets the approximation marked `≈`, never a
// bare `n/d` that would read as exact.
const formatNumberNode = (item: Value): string => {
    const semantics = item?.semantics;
    const exact = formatNormalForm(semantics?.exactTerms);
    if (exact) return exact;
    const text = formatNumber(item.value);
    return semantics?.approximate === true ? `≈ ${text}` : text;
};

// Exported for `output-display-renderer.test.ts`, which pins these strings
// against the ones `rust/src/types/display.rs` produces. The two renderers
// are separate implementations of one display, and nothing but that test
// stops them drifting.
export const formatValue = (item: Value, depth: number): string => {
    if (!item || !item.type) return '?';

    switch (item.type) {
        case 'number':
            return formatNumberNode(item);
        case 'string':
            return `'${item.value}'`;
        case 'symbol':
            return String(item.value);
        case 'boolean':
            return item.value ? 'TRUE' : 'FALSE';
        case 'vector':
            return formatVector(item.value, depth);
        case 'record':
            return formatRecord(item.value, depth);
        case 'nil':
            return 'NIL';
        default:
            return JSON.stringify(item.value);
    }
};

// A Record (LANG.RECORDS.STRUCTURE) crosses the protocol as two aligned
// arrays of nodes, and renders as its own literal — `{ key value … }`, each
// key beside the value under it — which is the same display the engine's own
// stack rendering produces (`rust/src/types/display_source.rs`).
//
// The empty Record is `{ }`, which needs no case of its own.
//
// A short value array is padded with NIL rather than dropped, because the
// two arrays are aligned by position and a missing slot is the protocol
// having been malformed, not a Record with fewer values than keys — and
// neither `RECORD` nor the literal admits a length mismatch.
const formatRecord = (value: unknown, depth: number): string => {
    const { keys, values } = readRecordParts(value);
    if (keys.length === 0) return '{ }';
    const pairs: string[] = keys.map((key, index) => {
        const paired = values[index] ?? NIL;
        return `${formatElementAt(key, depth + 1)} ${formatElementAt(paired, depth + 1)}`;
    });
    return `{ ${pairs.join(' ')} }`;
};

// ── Math view (LaTeX) ───────────────────────────────────────────────────────
// Derives a LaTeX reading of a stack value from its structured protocol form
// (never by parsing display strings).
//
// Presentation only. The canonical display strings (`3/1`, `[ 1/1 2/1 ]`)
// remain the observable semantics the conformance suite checks; the LaTeX
// produced here is an alternate GUI rendering of the same structured `Value`.
// Values without a faithful math reading return `null`, and the caller falls
// back to the canonical text rendering.

// Beyond this many numeric lanes a matrix stops being readable and the
// bracket text form is the better surface.
const MAX_MATH_LANES = 64;

const INTEGER_PATTERN = /^-?\d+$/;

// Digit count at which a numerator or denominator stops being readable as
// a digit string and the math view switches to scientific notation.
const SCIENTIFIC_DIGIT_THRESHOLD = 10;
const MANTISSA_DIGITS = 6;

// Stricter than `checkFractionObject` above: the math view typesets only a
// rational whose parts are integers and whose denominator is not zero, and
// falls back to the canonical text for anything else.
const checkFractionShape = (value: unknown): Fraction | null => {
    if (!value || typeof value !== 'object') return null;
    const candidate = value as { numerator?: unknown; denominator?: unknown };
    const numerator = String(candidate.numerator ?? '');
    const denominator = String(candidate.denominator ?? '');
    if (!INTEGER_PATTERN.test(numerator) || !INTEGER_PATTERN.test(denominator)) return null;
    // A zero denominator is not a faithful rational (it is NIL occupancy /
    // malformed state, never a canonical number). Reject it here so the math
    // view falls back to the canonical text rendering instead of dividing by
    // zero inside `scientificLatex`. Matches INTEGER_PATTERN-allowed forms like
    // "0", "-0" and "0000000000".
    if (/^-?0+$/.test(denominator)) return null;
    return { numerator, denominator };
};

// Scientific reading of a huge ratio: mantissa times a power of ten,
// computed exactly with BigInt long division and prefixed with \approx
// whenever any precision is dropped — the math view never presents a
// truncated value as exact.
const scientificLatex = (numeratorStr: string, denominatorStr: string): string => {
    let numerator = BigInt(numeratorStr);
    let denominator = BigInt(denominatorStr);
    // Defensive: a zero denominator would divide by zero below. Internal callers
    // are pre-filtered by `checkFractionShape`, but `fractionToLatex` is exported
    // and may be called directly, so keep this primitive total.
    if (denominator === 0n) return '\\mathrm{NIL}';
    if (denominator < 0n) {
        denominator = -denominator;
        numerator = -numerator;
    }
    const negative = numerator < 0n;
    if (negative) numerator = -numerator;
    if (numerator === 0n) return '0';

    // Scale so the quotient carries one digit beyond the mantissa, then
    // read mantissa and exponent off the quotient's decimal digits.
    const digitGap = String(numerator).length - String(denominator).length;
    const scale = MANTISSA_DIGITS + 1 - digitGap;
    const scaled = scale >= 0
        ? (numerator * 10n ** BigInt(scale)) / denominator
        : numerator / (denominator * 10n ** BigInt(-scale));
    const dividesExactly = scale >= 0
        ? (numerator * 10n ** BigInt(scale)) % denominator === 0n
        : numerator % (denominator * 10n ** BigInt(-scale)) === 0n;

    const digits = String(scaled);
    const exponent = digits.length - 1 - scale;
    const kept = digits.slice(0, MANTISSA_DIGITS);
    const dropped = digits.slice(MANTISSA_DIGITS);
    const exact = dividesExactly && /^0*$/.test(dropped);

    let significand = kept;
    let exponentOut = exponent;
    if (!exact && dropped.length > 0 && dropped[0]! >= '5') {
        // Round half-up on the first dropped digit; a carry out of the top
        // digit (9.99999... -> 10) bumps the exponent instead.
        const rounded = String(BigInt(kept) + 1n);
        if (rounded.length > kept.length) {
            significand = '1';
            exponentOut = exponent + 1;
        } else {
            significand = rounded;
        }
    }
    significand = significand.replace(/0+$/, '') || '0';

    // Huge components do not imply a huge value (a best rational
    // approximation of sqrt(2) has ten-digit components and the value 1.41…),
    // so a human-scale exponent renders as a plain decimal and only a
    // genuinely large or tiny value gets the power of ten.
    const sign = negative ? '-' : '';
    let body: string;
    if (exponentOut >= 0 && exponentOut <= 5) {
        const integerLength = exponentOut + 1;
        const padded = significand.padEnd(integerLength, '0');
        const integerPart = padded.slice(0, integerLength);
        const fractionalPart = padded.slice(integerLength);
        body = `${sign}${integerPart}${fractionalPart ? `.${fractionalPart}` : ''}`;
    } else if (exponentOut < 0 && exponentOut >= -4) {
        body = `${sign}0.${'0'.repeat(-exponentOut - 1)}${significand}`;
    } else {
        const mantissa = significand.length > 1
            ? `${significand[0]}.${significand.slice(1)}`
            : significand;
        body = mantissa === '1'
            ? `${sign}10^{${exponentOut}}`
            : `${sign}${mantissa} \\times 10^{${exponentOut}}`;
    }
    return exact ? body : `\\approx ${body}`;
};

const checkHugeDigits = (frac: Fraction): boolean => {
    const numeratorDigits = frac.numerator.replace('-', '').length;
    const denominatorDigits = frac.denominator.replace('-', '').length;
    return numeratorDigits >= SCIENTIFIC_DIGIT_THRESHOLD
        || denominatorDigits >= SCIENTIFIC_DIGIT_THRESHOLD;
};

// `3/1` reads as the integer 3; `-3/4` keeps its sign outside the bar.
// Huge components switch to scientific notation so the rendering stays
// inside the Stack area instead of running off its right edge.
export const fractionToLatex = (frac: Fraction): string => {
    if (checkHugeDigits(frac)) return scientificLatex(frac.numerator, frac.denominator);
    if (frac.denominator === '1') return frac.numerator;
    const { negative, magnitude } = splitSign(frac.numerator);
    const body = `\\frac{${magnitude}}{${frac.denominator}}`;
    return negative ? `-${body}` : body;
};

const rowsToMatrixLatex = (rows: string[][]): string => {
    const body = rows.map(row => row.join(' & ')).join(' \\\\ ');
    return `\\begin{bmatrix} ${body} \\end{bmatrix}`;
};

const numberElementToLatex = (item: Value): string | null => {
    if (item.type !== 'number') return null;
    const frac = checkFractionShape(item.value);
    return frac === null ? null : fractionToLatex(frac);
};

const vectorToLatex = (elements: Value[]): string | null => {
    if (elements.length === 0 || elements.length > MAX_MATH_LANES) return null;

    // Homogeneous numeric vector: a one-row matrix.
    const scalarRow = elements.map(numberElementToLatex);
    if (scalarRow.every((tex): tex is string => tex !== null)) {
        return rowsToMatrixLatex([scalarRow]);
    }

    // Rectangular vector-of-numeric-vectors: a rank-2 matrix.
    const rows: string[][] = [];
    let width: number | null = null;
    for (const element of elements) {
        if (element.type !== 'vector' || !Array.isArray(element.value)) return null;
        const row = (element.value as Value[]).map(numberElementToLatex);
        if (!row.every((tex): tex is string => tex !== null)) return null;
        if (width === null) width = row.length;
        if (row.length !== width || width === 0) return null;
        rows.push(row);
    }
    if (rows.reduce((total, row) => total + row.length, 0) > MAX_MATH_LANES) return null;
    return rowsToMatrixLatex(rows);
};

// Σ c·√r as typeset mathematics. A coefficient of one is left implicit, the
// rational term (radicand 1) is drawn as an ordinary fraction, and a negative
// term joins with a minus rather than `+ -`.
const normalFormToLatex = (
    terms: ReadonlyArray<ExactTerm> | undefined
): string | null => {
    if (!terms || terms.length === 0) return null;
    let out = '';
    for (const term of terms) {
        const { negative, magnitude } = splitSign(term.numerator);
        const root = term.radicand === '1' ? '' : `\\sqrt{${term.radicand}}`;
        const unit = magnitude === '1' && term.denominator === '1' && root !== '';
        const coefficient = unit
            ? ''
            : fractionToLatex({ numerator: magnitude, denominator: term.denominator });
        if (out === '') {
            out = `${negative ? '-' : ''}${coefficient}${root}`;
        } else {
            out += ` ${negative ? '-' : '+'} ${coefficient}${root}`;
        }
    }
    return out;
};

// The LaTeX reading of a stack value, or `null` when the canonical text
// rendering is the only faithful surface.
export const valueToLatex = (item: Value): string | null => {
    if (!item || !item.type) return null;

    switch (item.type) {
        case 'number': {
            const semantics = item.semantics;
            // An algebraic irrational carries its exact normal form, and that
            // is what mathematics notation is for: `\sqrt{3}` says the whole
            // value, where the approximation below can only gesture at it.
            const exact = normalFormToLatex(semantics?.exactTerms);
            if (exact !== null) return exact;
            const frac = checkFractionShape(item.value);
            if (frac === null) return null;
            const tex = fractionToLatex(frac);
            // Best rational approximation of an exact irrational under a
            // lossy role (LANG.OBSERVATION.FIREWALL): make the approximation visible. The
            // scientific form may already carry its own \approx.
            const approximate = semantics?.approximate === true;
            return approximate && !tex.startsWith('\\approx') ? `\\approx ${tex}` : tex;
        }
        case 'vector':
            return Array.isArray(item.value) ? vectorToLatex(item.value as Value[]) : null;
        default:
            return null;
    }
};

// Math view (docs/dev/gui-current-design-memory.md): an alternate KaTeX
// rendering of stack values, derived from the structured protocol form.
// Presentation only — the canonical display strings stay untouched and
// remain the conformance observation; the toggle swaps the view, never
// the value. Output text is never scanned for delimiters.
const MATH_VIEW_STORAGE_KEY = 'ajisai-stack-math-view';

// The canonical protocol strings are the standard rendering; Math view is
// an opt-in alternate so Ajisai's observable surface never depends on
// KaTeX (portability: the GUI stays faithful without it).
const readMathViewPreference = (): boolean => {
    try {
        return globalThis.localStorage?.getItem(MATH_VIEW_STORAGE_KEY) === '1';
    } catch {
        return false;
    }
};

const writeMathViewPreference = (enabled: boolean): void => {
    try {
        globalThis.localStorage?.setItem(MATH_VIEW_STORAGE_KEY, enabled ? '1' : '0');
    } catch {
        // Preference is a convenience; rendering works without persistence.
    }
};

const renderMathValueNode = (item: Value): HTMLElement | null => {
    const tex = valueToLatex(item);
    if (tex === null) return null;
    const node = document.createElement('span');
    node.className = 'stack-node stack-node-math';
    // Tag the top-level node so the stack-top fill's selector
    // (`.stack-item:last-child .stack-node[data-depth="1"]`) matches in LaTeX
    // view exactly as it does for the canonical rendering.
    node.dataset.depth = '1';
    // Trusted markup: KaTeX output for TeX generated from the structured
    // value by valueToLatex, never from user-supplied text.
    node.innerHTML = katex.renderToString(tex, { throwOnError: false });
    return node;
};

// ── The Output and Stack areas ──────────────────────────────────────────────

const formatErrorMessage = (error: Error | { message?: string } | string): string =>
    `Error: ${typeof error === 'string' ? error : error.message || toError(error).message}`;

type OutputKind = 'debug' | 'program' | 'error' | 'info';

// The host writes each `PRINT` emission as one line, so the stream ends in the
// terminator of its last emission. Only that terminator is presentation; every
// other character is the observation itself (LANG.EFFECTS.OUTPUT), so nothing
// else is trimmed: `'' PRINT` is one empty emission and not a run that printed
// nothing, `'  x' PRINT` keeps its indentation, and `'' PRINT 'a' PRINT` keeps
// its first line. Trimming the whole string collapsed all three into their
// neighbours and made the Output projection disagree with the CLI's `output`.
const stripEmissionTerminator = (output: string): string =>
    output.endsWith('\n') ? output.slice(0, -1) : output;

const createSpanElement = (text: string, kind: OutputKind): HTMLSpanElement => {
    const span = document.createElement('span');
    span.className = `output-${kind}`;
    span.textContent = text;
    return span;
};

export const createDisplay = (elements: DisplayElements): Display => {
    let mainOutput = '';
    let mathViewEnabled = readMathViewPreference();
    let lastStack: Value[] = [];

    // The LaTeX toggle: a labeled checkbox at the bottom-right of the Stack
    // area. Checked means the LaTeX (KaTeX) rendering, unchecked the canonical
    // protocol strings. The unchecked mode is deliberately unnamed — a
    // checkbox states only what checking it adds.
    const init = (): void => {
        const panel = elements.stackDisplay.parentElement;
        if (!panel || panel.querySelector('.stack-latex-toggle')) return;

        const wrapper = document.createElement('label');
        wrapper.className = 'stack-latex-toggle';

        const checkbox = document.createElement('input');
        checkbox.type = 'checkbox';
        checkbox.checked = mathViewEnabled;
        checkbox.addEventListener('change', () => {
            mathViewEnabled = checkbox.checked;
            writeMathViewPreference(mathViewEnabled);
            renderStack(lastStack);
        });

        const caption = document.createElement('span');
        caption.textContent = 'LaTeX';

        wrapper.append(checkbox, caption);
        panel.appendChild(wrapper);
    };

    const appendSpan = (text: string, kind: OutputKind): HTMLSpanElement => {
        const span = createSpanElement(text, kind);
        elements.outputDisplay.appendChild(span);
        return span;
    };

    const renderExecutionResult = (result: ExecuteResult): void => {
        const output = result.output ?? '';
        const program = stripEmissionTerminator(output);

        mainOutput = program;
        elements.outputDisplay.replaceChildren();

        if (output) {
            appendSpan(program, 'program');
        }

        if (!output && !isFailure(result)) {
            appendSpan('OK', 'debug');
        }
    };

    // An error is written *below* whatever the run already printed, never in
    // place of it. `PRINT` is the language's trace tool, and the run that ends
    // in an error is the run whose trace is wanted, so clearing the area first
    // would blank `PRINT` at the one moment it matters most. `precedingOutput`
    // is what the failing run printed before it stopped, which the host
    // reports on the error path too.
    const renderError = (
        error: Error | { message?: string } | string,
        precedingOutput = ''
    ): void => {
        const errorMessage = formatErrorMessage(error);
        const printed = stripEmissionTerminator(precedingOutput);

        elements.outputDisplay.replaceChildren();
        if (precedingOutput) {
            appendSpan(printed, 'program');
            elements.outputDisplay.appendChild(document.createElement('br'));
        }
        mainOutput = precedingOutput ? `${printed}\n${errorMessage}` : errorMessage;

        appendSpan(errorMessage, 'error');
    };

    const renderInfo = (text: string, append = false): void => {
        if (append && elements.outputDisplay.innerHTML.trim() !== '') {
            mainOutput = `${mainOutput}\n${text}`;
            appendSpan('\n' + text, 'info');
        } else {
            mainOutput = text;
            elements.outputDisplay.replaceChildren();
            appendSpan(text, 'info');
        }
    };

    // A report that belongs in the record but not in the reader's way, folded
    // the way the cost summary is.
    //
    // A reasoned NIL is a value, not a failure: `1 0 DIV` answered what the
    // language says it answers. Printed in full, its diagnosis would put a
    // correct ten-line answer under a heading that reads like an error report.
    // Folding it puts the reason one click away and leaves the stance of the
    // language visible in the output. `mainOutput` still gets the text, so
    // Copy copies what was said whether or not it was opened.
    const renderFoldedInfo = (label: string, text: string): void => {
        mainOutput = mainOutput ? `${mainOutput}\n${text}` : text;

        const details = document.createElement('details');
        details.className = 'folded-info';

        const summary = document.createElement('summary');
        summary.textContent = label;
        details.appendChild(summary);

        const body = document.createElement('div');
        body.className = 'folded-info-body';
        body.textContent = text;
        details.appendChild(body);

        elements.outputDisplay.appendChild(details);
    };

    // A Core Word's reference entry, as the host's lookup answered it.
    // Reference text is read rather than run, so
    // it is shown here instead of being written into the editor over whatever
    // the user was writing. `pre-wrap` is already set on the area, so the
    // entry's own line structure survives verbatim.
    const renderDocumentation = (text: string): void => {
        mainOutput = text;
        elements.outputDisplay.replaceChildren();
        appendSpan(text, 'debug');
    };

    const renderStack = (stack: Value[]): void => {
        lastStack = Array.isArray(stack) ? stack : [];
        const display = elements.stackDisplay;
        display.replaceChildren();

        // The clear control follows the same rule the editor's does: it is not
        // drawn when there is nothing to clear. The flag goes on the panel
        // because the button is a sibling of the display, not a child.
        display.parentElement?.classList.toggle('is-empty-stack', lastStack.length === 0);

        if (lastStack.length === 0) {
            display.classList.add('is-empty');
            display.appendChild(createEmptyWordsElement('No values on the stack yet.'));
            return;
        }

        display.classList.remove('is-empty');

        const container = document.createElement('div');
        container.className = 'area-content-flow stack-content-flow';

        // One budget for the whole render: the Stack area draws a bounded
        // amount however the values on it are shaped.
        const budget = createRenderBudget();

        lastStack.forEach((item, index) => {
            const elem = document.createElement('span');
            elem.className = 'stack-item';
            try {
                const mathNode = mathViewEnabled ? renderMathValueNode(item) : null;
                elem.appendChild(mathNode ?? renderStackValueNode(item, 1, budget));
            } catch {
                console.error(`Error formatting item ${index}`);
                elem.textContent = 'ERROR';
            }
            container.appendChild(elem);
        });

        display.appendChild(container);
    };

    const extractState = (): DisplayState => ({ mainOutput });

    return {
        init,
        renderExecutionResult,
        renderError,
        renderInfo,
        renderFoldedInfo,
        renderDocumentation,
        renderStack,
        extractState
    };
};
