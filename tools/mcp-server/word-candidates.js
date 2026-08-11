// "Did you mean" for a Word name the packaged registry does not hold.
//
// The engine answers this for an unknown Word at runtime (`diagnosis
// .candidates`, from `rust/src/interpreter/word_candidates.rs`). The registry
// lookup tool needs the same answer for the same reason — a caller that
// mistypes a name is the overwhelmingly likely cause of an empty result, and
// the complete name list is sitting in the same process.
//
// Deliberately the same ranking rules as the engine's: distance ceiling by
// name length, at most three suggestions, ties broken alphabetically so the
// same misspelling always produces the same list.

const MAX_CANDIDATES = 3;

function distanceCeiling(length) {
  if (length <= 3) return 1;
  if (length <= 7) return 2;
  return 3;
}

/** Levenshtein distance, two rows at a time. */
export function editDistance(left, right) {
  const a = [...left];
  const b = [...right];
  if (a.length === 0) return b.length;
  if (b.length === 0) return a.length;

  let previous = Array.from({ length: b.length + 1 }, (_, i) => i);
  let current = new Array(b.length + 1);
  for (let i = 0; i < a.length; i += 1) {
    current[0] = i + 1;
    for (let j = 0; j < b.length; j += 1) {
      const substitution = previous[j] + (a[i] === b[j] ? 0 : 1);
      current[j + 1] = Math.min(substitution, previous[j + 1] + 1, current[j] + 1);
    }
    [previous, current] = [current, previous];
  }
  return previous[b.length];
}

/**
 * Registry names within a plausible typo distance of `word`, best match first.
 * `entries` are `spec/words.json` entries; both canonical names and aliases
 * are considered, and a purely symbolic alias (`+`, `^`) is never offered as
 * the correction of an alphabetic name.
 */
export function suggestWords(word, entries) {
  const needle = String(word ?? "").trim().toUpperCase();
  if (!needle) return [];
  const ceiling = distanceCeiling([...needle].length);

  const scored = [];
  const seen = new Set();
  for (const entry of entries ?? []) {
    for (const name of [entry.name, ...(entry.aliases ?? [])]) {
      const upper = String(name).toUpperCase();
      if (upper === needle || seen.has(upper)) continue;
      if (!/[a-z0-9]/i.test(upper)) continue;
      const distance = editDistance(needle, upper);
      if (distance > ceiling) continue;
      seen.add(upper);
      scored.push([distance, upper]);
    }
  }
  scored.sort((left, right) => left[0] - right[0] || left[1].localeCompare(right[1]));
  return scored.slice(0, MAX_CANDIDATES).map(([, name]) => name);
}
