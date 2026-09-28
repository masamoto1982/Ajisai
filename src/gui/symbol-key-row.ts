// The mobile symbol key row: one thin row of keys under the editor, shown on a
// phone while the editor has focus.
//
// It replaces a 30-symbol palette that opened over the editor at every word
// boundary. Few of those symbols mean anything in Ajisai any more, and the
// palette took a block of an already small surface each time it opened. The
// row holds the same 30 symbols in five pages of six: the first page is the
// ones a program actually needs — the Vector brackets, the string quote and
// the characters of a number — and the page key walks the rest.
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

// A key that acts without moving focus: the press is swallowed before the
// browser can hand focus to the button, and the click still arrives.
const createKey = (label: string, onPress: () => void): HTMLButtonElement => {
    const key = document.createElement('button');
    key.type = 'button';
    key.className = 'symbol-key';
    key.textContent = label;
    key.addEventListener('pointerdown', (event) => event.preventDefault());
    key.addEventListener('click', onPress);
    return key;
};

export const createSymbolKeyRow = (
    root: HTMLElement,
    insert: (symbol: string) => void
): SymbolKeyRow => {
    let page = 0;

    const render = (): void => {
        const symbols = SYMBOL_PAGES[page]!;
        const keys = symbols.map((symbol) => createKey(symbol, () => insert(symbol)));
        const pageKey = createKey(`${page + 1}/${SYMBOL_PAGES.length}`, () => {
            page = (page + 1) % SYMBOL_PAGES.length;
            render();
        });
        pageKey.classList.add('symbol-key-page');
        pageKey.setAttribute(
            'aria-label',
            `Symbol page ${page + 1} of ${SYMBOL_PAGES.length}; next page`
        );
        root.replaceChildren(...keys, pageKey);
    };

    render();
    return { currentPage: () => page };
};
