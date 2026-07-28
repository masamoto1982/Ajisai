// The presentation profile, as pure functions.
//
// This is a user interface, not the language (`SPECIFICATION.md` §14). Nothing
// here can change what a program means; it decides which of the four surfaces
// a person is looking at. It lives in its own module, free of the DOM, so the
// rules can be tested directly — see `playground/presentation.test.mjs`.

/** The four observation surfaces, in the order a single-panel device cycles. */
export const VIEW_ORDER = ['input', 'output', 'stack', 'dictionary'];

const LEFT_SURFACES = ['input', 'output'];
const RIGHT_SURFACES = ['stack', 'dictionary'];

/** Width at or below which the layout shows one surface at a time. */
export const MOBILE_BREAKPOINT = 768;
/** Horizontal travel that counts as a swipe rather than a tap. */
export const SWIPE_THRESHOLD = 50;

export const isLeftSurface = (surface) => LEFT_SURFACES.includes(surface);
export const isRightSurface = (surface) => RIGHT_SURFACES.includes(surface);

/**
 * Manual selection, two-column layout.
 *
 * The two coupling rules are the point of this function, and they are about
 * intent rather than cosmetics:
 *
 *   - choosing Output pulls the right column to Stack, because you asked to
 *     see what a run said and the flow is the other half of that answer;
 *   - choosing Dictionary pulls the left column to Input, because the reason
 *     to open the dictionary is to put a word into the editor.
 *
 * Together they keep Output and Dictionary — the two surfaces whose intents
 * conflict — out of the reachable configurations.
 */
export const selectSurface = (layout, surface) => {
  const next = { ...layout };
  if (isLeftSurface(surface)) {
    next.left = surface;
    if (surface === 'output') next.right = 'stack';
  }
  if (isRightSurface(surface)) {
    next.right = surface;
    if (surface === 'dictionary') next.left = 'input';
  }
  return next;
};

/**
 * Execution-driven transition, two-column layout.
 *
 * Distinct from manual selection: here the surfaces a run *touched* decide
 * where the layout moves. Dictionary outranks Stack for the right column,
 * because defining a word is the more notable structural change. When a run
 * changed nothing observable, both columns stay where they were — a run that
 * did nothing should not move the furniture.
 */
export const applyRun = (layout, changes) => {
  const next = { ...layout };
  if (changes.output) next.left = 'output';
  if (changes.stack) next.right = 'stack';
  if (changes.dictionary) next.right = 'dictionary';
  return next;
};

/**
 * Execution-driven transition, single-surface layout.
 *
 * One surface can be shown, so the most notable change wins, in the same
 * priority order the two-column rule implies. Nothing changed means stay put.
 */
export const applyRunMobile = (current, changes) => {
  if (changes.dictionary) return 'dictionary';
  if (changes.output) return 'output';
  if (changes.stack) return 'stack';
  return current;
};

/** The next surface when cycling — by swipe, or by the selector's arrows. */
export const cycle = (current, direction) => {
  const at = VIEW_ORDER.indexOf(current);
  const from = at === -1 ? 0 : at;
  const step = direction === 'left' ? 1 : -1;
  return VIEW_ORDER[(from + step + VIEW_ORDER.length) % VIEW_ORDER.length];
};

/**
 * What a run changed, by comparing two snapshots.
 *
 * A failed run always counts as changing Output, because the diagnostic is
 * what the Output surface shows — a program that fails silently would leave
 * the person looking at an unchanged screen.
 */
export const detectChanges = (before, after, result) => {
  const sameStack = JSON.stringify(before.stack) === JSON.stringify(after.stack);
  const key = (definitions) =>
    JSON.stringify(
      [...definitions]
        .map(({ name, body }) => [name, body])
        .sort((a, b) => a[0].localeCompare(b[0])),
    );
  const dictionary = key(before.definitions) !== key(after.definitions);
  const named = after.definitions.find(
    (word) => !before.definitions.some((had) => had.name === word.name),
  );
  return {
    output: !result.ok || Boolean(result.findings?.length),
    stack: !sameStack,
    dictionary,
    // The word that just appeared, so the dictionary can land on it.
    reveal: named?.name,
  };
};

/**
 * Which stack cells the armed modes would treat as operands, and what would
 * become of them.
 *
 * This is a *hint about the source being written*, not a simulation: a mode
 * governs one word, and this summarises which modes the whole fragment uses.
 * It reads whole tokens only, so `.5` and `5.` are numbers, never a `TOP`.
 *
 *   - `target`: `STAK` (or `:`) anywhere means every cell is an operand;
 *     otherwise only the surface cell is.
 *   - `retention`: `KEEP` (or `&`) anywhere means operands are retained;
 *     otherwise they are eaten.
 */
export const analyzeModes = (source) => {
  let target = 'top';
  let retention = 'eat';
  for (const raw of source.split(/\s+/)) {
    const token = raw.toUpperCase();
    if (token === ':' || token === 'STAK') target = 'stak';
    if (token === '&' || token === 'KEEP') retention = 'keep';
  }
  return { target, retention };
};

/** The cells `analyzeModes` says are operands, given a flow of `depth` cells. */
export const operandCells = (depth, modes) => {
  if (depth === 0) return [];
  if (modes.target === 'stak') return Array.from({ length: depth }, (_, i) => i);
  return [depth - 1];
};
