import katex from 'katex';
import 'katex/dist/katex.min.css';
import type { Value, ExecuteResult } from '../wasm-interpreter-types';
import { valueToLatex } from './value-latex';
import {
    createRenderBudget,
    formatElision,
    planCollectionRender,
    type RenderBudget
} from './stack-render-budget';

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
    readonly appendExecutionResult: (result: ExecuteResult) => void;
    readonly renderOutput: (text: string) => void;
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

const lookupBracketsAtDepth = (_depth: number): [string, string] => ['[', ']'];


const BRACKET_DEPTH_COLORS: readonly string[] = [
    '#332288',
    '#88CCEE',
    '#44AA99',
    '#117733',
    '#999933',
    '#DDCC77',
    '#CC6677',
    '#882255',
    '#AA4499',
] as const;

const lookupBracketColor = (depth: number): string =>
    BRACKET_DEPTH_COLORS[depth - 1] ?? '#332288';

const createBracketSpan = (bracket: string, depth: number): HTMLSpanElement => {
    const span = document.createElement('span');
    span.className = 'stack-bracket';
    span.style.color = lookupBracketColor(depth);
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

/// A Vector renders as source that rebuilds it — a bracket literal, whatever
/// it holds — matching the engine's own renderer
/// (`rust/src/types/display_source.rs`). A nested Record is one element,
/// because it has a literal of its own (`formatRecord`).
///
/// The empty Vector is `[ ]` and not `[]`: a bracket must stand alone
/// (`spec/grammar.json`, `bracketMustStandAlone`), so `[]` is a source error
/// rather than an empty Vector. This panel used to print the glued form,
/// which no one noticed while a display was not expected to be source.
const formatVector = (value: unknown, depth: number): string => {
    const [open, close] = lookupBracketsAtDepth(depth);

    if (Array.isArray(value)) {
        if (value.length === 0) {
            return `${open} ${close}`;
        }
        const formatSingleElement = (v: Value): string => {
            try { return formatValue(v, depth + 1); } catch { return '?'; }
        };
        const elements: string = value.map(formatSingleElement).join(' ');
        return `${open} ${elements} ${close}`;
    }
    return `${open} ${close}`;
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
    // Tag the top-level node so the operation-target highlight selectors
    // (`.stack-node[data-depth="1"]`) match in LaTeX view exactly as they do
    // for the canonical rendering — keeping the grey background that shows
    // whether the target is the Stack top or the whole Stack.
    node.dataset.depth = '1';
    // Trusted markup: KaTeX output for TeX generated from the structured
    // value by valueToLatex, never from user-supplied text.
    node.innerHTML = katex.renderToString(tex, { throwOnError: false });
    return node;
};

// The undrawn tail of a collection, stated as a count rather than drawn. See
// stack-render-budget.ts for why the Stack area is bounded at all.
const createElisionSpan = (elided: number): HTMLSpanElement => {
    const span = document.createElement('span');
    span.className = 'stack-elision';
    span.textContent = formatElision(elided);
    span.title = `${elided} more element(s) are on the stack but not drawn; LENGTH reports the full length.`;
    return span;
};

interface ExactTerm {
    readonly numerator: string;
    readonly denominator: string;
    readonly radicand: string;
}

/// An irrational's normal form Σ c·√r written exactly as the engine writes it
/// (`rust/src/types/display.rs::render_algebraic_terms`): one spaceless token,
/// terms in the normal form's order, `sqrt(r)` for a unit coefficient,
/// `n/d*sqrt(r)` otherwise, and the rational term as `n/d`. The panel used to
/// write `√3` and `1/2√2` with spaced signs — a second spelling of a value
/// whose canonical display the Stack surface is required to show
/// (spec/gui-semantics.md), and one no program could have written back.
const formatNormalForm = (terms: ReadonlyArray<ExactTerm> | undefined): string | null => {
    if (!terms || terms.length === 0) return null;
    let out = '';
    terms.forEach((term, index) => {
        const negative = term.numerator.startsWith('-');
        const magnitude = negative ? term.numerator.slice(1) : term.numerator;
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

const renderStackValueNode = (item: Value, depth: number, budget: RenderBudget): HTMLElement => {
    const node = document.createElement('span');
    node.className = 'stack-node';

    if (item.type === 'vector' && Array.isArray(item.value)) {
        const children = item.value as Value[];
        const { shown, elided } = planCollectionRender(children.length, budget);
        node.classList.add('stack-node-vector');
        node.dataset.depth = String(depth);
        node.appendChild(createBracketSpan('[', depth));
        for (let index = 0; index < shown; index++) {
            if (index > 0) node.append(' ');
            node.appendChild(renderStackValueNode(children[index]!, depth + 1, budget));
        }
        if (elided > 0) {
            if (shown > 0) node.append(' ');
            node.appendChild(createElisionSpan(elided));
        }
        node.appendChild(createBracketSpan(']', depth));
        return node;
    }

    if (depth === 1) {
        node.dataset.depth = String(depth);
    }
    node.textContent = formatValue(item, depth);
    return node;
};

/// A number as the engine displays it. An irrational's `n/d` is only an
/// approximation, so its normal form (`semantics.exactTerms`) is written
/// instead; a host that sends none gets the approximation marked `≈`, never a
/// bare `n/d` that would read as exact.
const formatNumberNode = (item: Value): string => {
    const semantics = item?.semantics as
        | { approximate?: boolean; exactTerms?: ReadonlyArray<ExactTerm> }
        | undefined;
    const exact = formatNormalForm(semantics?.exactTerms);
    if (exact) return exact;
    const text = formatNumber(item.value);
    return semantics?.approximate === true ? `≈ ${text}` : text;
};

/// Exported for `output-display-renderer.test.ts`, which pins these strings
/// against the ones `rust/src/types/display.rs` produces. The two renderers
/// are separate implementations of one display, and nothing but that test
/// stops them drifting.
export const formatValue = (item: Value, depth: number): string => {
    if (!item || !item.type) return 'unknown';

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

/// A Record (LANG.RECORDS.STRUCTURE) crosses the protocol as two aligned
/// arrays of nodes, and renders as its own literal — `{ key value … }`, each
/// key beside the value under it — which is the same display the engine's own
/// stack rendering produces (`rust/src/types/display_source.rs`).
///
/// The empty Record is `{ }`, which needs no case of its own.
///
/// A short value array is padded with NIL rather than dropped, because the
/// two arrays are aligned by position and a missing slot is the protocol
/// having been malformed, not a Record with fewer values than keys — and
/// neither `RECORD` nor the literal admits a length mismatch.
const formatRecord = (value: unknown, depth: number): string => {
    const record = value as { keys?: Value[]; values?: Value[] } | null;
    const keys = Array.isArray(record?.keys) ? record!.keys : [];
    const values = Array.isArray(record?.values) ? record!.values : [];
    if (keys.length === 0) return '{ }';
    const formatSingleElement = (v: Value): string => {
        try { return formatValue(v, depth + 1); } catch { return '?'; }
    };
    const pairs: string[] = keys.map((key, index) => {
        const paired = values[index] ?? ({ type: 'nil' } as Value);
        return `${formatSingleElement(key)} ${formatSingleElement(paired)}`;
    });
    return `{ ${pairs.join(' ')} }`;
};


const formatErrorMessage = (error: Error | { message?: string } | string): string =>
    typeof error === 'string'
        ? `Error: ${error}`
        : `Error: ${(error as Error).message || error}`;

const createSpanElement = (text: string, color: string): HTMLSpanElement => {
    const span = document.createElement('span');
    span.style.color = color;
    span.textContent = text;
    return span;
};

const clearElement = (element: HTMLElement): void => {
    element.innerHTML = '';
};

const appendToElement = (parent: HTMLElement, child: HTMLElement): void => {
    parent.appendChild(child);
};

export const createDisplay = (elements: DisplayElements): Display => {
    let mainOutput = '';
    let mathViewEnabled = readMathViewPreference();
    let lastStack: Value[] = [];

    const createLatexToggle = (): void => {
        const panel = elements.stackDisplay.parentElement;
        if (!panel || panel.querySelector('.stack-latex-toggle')) return;

        // A labeled checkbox at the bottom-right of the Stack area: checked
        // means the LaTeX (KaTeX) rendering, unchecked the canonical
        // protocol strings. The unchecked mode is deliberately unnamed —
        // a checkbox states only what checking it adds.
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

    const init = (): void => {
        elements.outputDisplay.style.whiteSpace = 'pre-wrap';
        createLatexToggle();
    };

    const appendSpan = (text: string, color: string): HTMLSpanElement => {
        const span = createSpanElement(text.replace(/\\n/g, '\n'), color);
        appendToElement(elements.outputDisplay, span);
        return span;
    };

    const renderExecutionResult = (result: ExecuteResult): void => {
        const debug = (result.debugOutput || '').trim();
        const program = (result.output || '').trim();

        mainOutput = `${debug}\n${program}`;
        clearElement(elements.outputDisplay);

        if (debug) {
            appendSpan(debug, '#333');
        }

        if (debug && program) {
            appendToElement(elements.outputDisplay, document.createElement('br'));
        }

        if (program) {
            appendSpan(program, '#4DC4FF');
        }

        if (!debug && !program && result.status === 'OK') {
            appendSpan('OK', '#333');
        }

    };

    const appendExecutionResult = (result: ExecuteResult): void => {
        const filteredOutput = (result.output || '').trim();

        if (filteredOutput) {
            appendSpan(filteredOutput, '#4DC4FF');
        }
    };

    const renderOutput = (text: string): void => {
        mainOutput = text;
        clearElement(elements.outputDisplay);
        appendSpan(text, '#4DC4FF');
    };

    /// An error is written *below* whatever the run already printed, never in
    /// place of it. `PRINT` is the language's trace tool, and the run that ends
    /// in an error is the run whose trace is wanted; clearing the area first
    /// meant the one moment `PRINT` mattered most was the one moment it showed
    /// nothing. `precedingOutput` is what the failing run printed before it
    /// stopped (the host now reports it on the error path too).
    const renderError = (
        error: Error | { message?: string } | string,
        precedingOutput = ''
    ): void => {
        const errorMessage = formatErrorMessage(error);
        const printed = precedingOutput.trim();

        clearElement(elements.outputDisplay);
        if (printed) {
            appendSpan(printed, '#4DC4FF');
            appendToElement(elements.outputDisplay, document.createElement('br'));
        }
        mainOutput = printed ? `${printed}\n${errorMessage}` : errorMessage;

        const span = appendSpan(errorMessage, '#dc3545');
        span.style.fontWeight = 'bold';
    };

    const renderInfo = (text: string, append = false): void => {
        if (append && elements.outputDisplay.innerHTML.trim() !== '') {
            mainOutput = `${mainOutput}\n${text}`;
            appendSpan('\n' + text, '#666');
        } else {
            mainOutput = text;
            clearElement(elements.outputDisplay);
            appendSpan(text, '#666');
        }
    };

    // A report that belongs in the record but not in the reader's way, folded
    // the way the cost summary is.
    //
    // A reasoned NIL is a value, not a failure: `1 0 DIV` answered what the
    // language says it answers. Its diagnosis was printed in full anyway, so
    // a correct ten-line answer arrived under a heading that reads like an
    // error report. Folding it puts the reason one click away and leaves the
    // stance of the language visible in the output. `mainOutput` still gets
    // the text, so Copy copies what was said whether or not it was opened.
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

        appendToElement(elements.outputDisplay, details);
    };

    /// A Core Word's reference entry, as the host's lookup answered it.
    /// Reference text is read rather than run, so
    /// it is shown here instead of being written into the editor over whatever
    /// the user was writing. `pre-wrap` is already set on the area, so the
    /// entry's own line structure survives verbatim.
    const renderDocumentation = (text: string): void => {
        mainOutput = text;
        clearElement(elements.outputDisplay);
        appendSpan(text, '#333');
    };

    const renderStack = (stack: Value[]): void => {
        lastStack = Array.isArray(stack) ? stack : [];
        const display = elements.stackDisplay;
        clearElement(display);

        // The clear control follows the same rule the editor's does: it is not
        // drawn when there is nothing to clear. The flag goes on the panel
        // because the button is a sibling of the display, not a child.
        display.parentElement?.classList.toggle('is-empty-stack', lastStack.length === 0);

        if (lastStack.length === 0) {
            display.classList.add('is-empty');
            const message = document.createElement('div');
            message.className = 'empty-words-message';
            message.textContent = 'No values on the stack yet.';
            display.appendChild(message);
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
            appendToElement(container, elem);
        });

        appendToElement(display, container);
    };

    const extractState = (): DisplayState => ({ mainOutput });

    return {
        init,
        renderExecutionResult,
        appendExecutionResult,
        renderOutput,
        renderError,
        renderInfo,
        renderFoldedInfo,
        renderDocumentation,
        renderStack,
        extractState
    };
};
