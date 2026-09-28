// The mobile symbol key row: one thin row of keys under the editor, shown on a
// phone while the editor has focus.
//
// It replaces a 30-symbol palette that opened over the editor at every word
// boundary. Few of those symbols mean anything in Ajisai any more, and the
// palette took a block of an already small surface each time it opened. The
// row holds the same 30 symbols in five pages of six: the first page is the
// ones a program actually needs — the Vector brackets, the string quote and
// the characters of a number — and the chevrons at the two ends walk the rest.
//
// The chevrons are drawn lines, not labelled keys. A page key that read "2/5"
// looked like one more symbol to type, and "<" or ">" as text would be one:
// both are on page 2.
//
// A key never takes focus. The editor keeps it, so the phone's keyboard stays
// up and the symbol lands at the caret, as typing it would.

export const SYMBOL_PAGES: readonly (readonly string[])[] = Object.freeze([
    Object.freeze(['[', ']', "'", '/', '.', '-']),
    Object.freeze(['(', ')', '{', '}', '<', '>']),
    Object.freeze(['+', '*', '%', '=', '!', '?']),
    Object.freeze(['&', '|', '~', '@', '#', '$']),
    Object.freeze(['_', '\\', ':', ';', ',', '"']),
]);

export interface SymbolKeyRow {
    /** The page shown, from 0. */
    readonly currentPage: () => number;
}

const SVG_NS = 'http://www.w3.org/2000/svg';

// A key that acts without moving focus: the press is swallowed before the
// browser can hand focus to the button, and the click still arrives.
const createKey = (className: string, onPress: () => void): HTMLButtonElement => {
    const key = document.createElement('button');
    key.type = 'button';
    key.className = className;
    key.addEventListener('pointerdown', (event) => event.preventDefault());
    key.addEventListener('click', onPress);
    return key;
};

// One stroke the height of the row, pointing the way the page turns.
const createChevron = (direction: 'previous' | 'next'): SVGSVGElement => {
    const svg = document.createElementNS(SVG_NS, 'svg');
    svg.setAttribute('viewBox', '0 0 10 44');
    svg.setAttribute('preserveAspectRatio', 'none');
    svg.setAttribute('aria-hidden', 'true');
    const line = document.createElementNS(SVG_NS, 'polyline');
    line.setAttribute('points', direction === 'previous' ? '9,1 1,22 9,43' : '1,1 9,22 1,43');
    line.setAttribute('vector-effect', 'non-scaling-stroke');
    svg.append(line);
    return svg;
};

export const createSymbolKeyRow = (
    root: HTMLElement,
    insert: (symbol: string) => void
): SymbolKeyRow => {
    let page = 0;
    const pageCount = SYMBOL_PAGES.length;

    const turn = (step: number): void => {
        page = (page + step + pageCount) % pageCount;
        render();
    };

    const createPageKey = (direction: 'previous' | 'next'): HTMLButtonElement => {
        const key = createKey('symbol-page-key', () => turn(direction === 'previous' ? -1 : 1));
        key.setAttribute('aria-label', direction === 'previous' ? 'Previous symbols' : 'Next symbols');
        key.append(createChevron(direction));
        return key;
    };

    const previousKey = createPageKey('previous');
    const nextKey = createPageKey('next');

    const render = (): void => {
        const keys = SYMBOL_PAGES[page]!.map((symbol) => {
            const key = createKey('symbol-key', () => insert(symbol));
            key.textContent = symbol;
            return key;
        });
        root.setAttribute('aria-label', `Symbols, page ${page + 1} of ${pageCount}`);
        root.replaceChildren(previousKey, ...keys, nextKey);
    };

    render();
    return { currentPage: () => page };
};
