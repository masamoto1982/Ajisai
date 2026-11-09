// The rule of scripts/check-vocabulary-counts.mjs, kept pure so that
// scripts/check-vocabulary-counts.test.mjs can exercise it without the tree.
//
// A count of Words written in a file is a copy of docs/word-manifest.json's
// `counts`, and nothing regenerates the copy. `scanText` finds the copies in
// one file's text and returns each one that differs from the record.
//
// Two layers:
//
//  - STATED: phrasings that can only mean the whole vocabulary or one of its
//    two tiers ("77 canonical Words", "49-Word Semantic Kernel", "語彙は77個
//    のワード", "77 語（Kernel 49・Standard 28）"). They are read in every
//    tracked text file, and each is tied to the count it restates.
//
//  - BARE: any "N Words" / "N語" / "N個のワード". In prose that is too often
//    something else (a lexicon-emergence result, "2.4.2 Words"), so it is read
//    only in code and configuration — source comments, CI workflow comments —
//    where a number of Words is almost always a count of the vocabulary or of
//    some part of it, and a stale one survives longest because no reader of
//    the Reference ever sees it. A bare number passes if it equals any count of
//    record; otherwise it must be exempt below or sit on a historical line.
//
// A line that narrates the past (HISTORICAL) is skipped by both layers: "当時
// 78 語のまま" and "a hand copy here once said 48 Words" are true records of a
// count that was, not claims about the vocabulary now.

/** Which tracked files are read at all. JSON and lockfiles are data, not prose. */
export const TEXT_FILE = /\.(md|html|rs|ts|tsx|js|mjs|cjs|ya?ml|toml|lean|sh|txt)$/;

/** The rule and its test quote stale counts on purpose, as fixtures and exemptions. */
export const SELF = ['scripts/lib/vocabulary-counts.mjs', 'scripts/check-vocabulary-counts.test.mjs'];

/** Files whose bare counts are read too: code and configuration, not prose. */
export const CODE_FILE = /\.(rs|ts|tsx|js|mjs|cjs|ya?ml|toml|lean|sh)$/;

/** A count stated in the past tense is history, not a claim about the present. */
export const HISTORICAL = /\b(?:once|formerly|previously|used to|was|were|had been)\b|当時|かつて|以前は|旧語彙/;

// A number that is not the tail of a larger one ("2,000", "2.4.2", "§9.3").
const N = String.raw`(?<![\d.,§])(\d+)`;
const re = (source) => new RegExp(source.replaceAll('N', N), 'g');

/** [pattern, which count it restates]. Each pattern's first group is the number. */
export const STATED = [
  // the Specification's count table and dictionary clause
  [re(String.raw`<td>N<\/td><td>Canonical Words`), 'canonical'],
  [re(String.raw`<td>N<\/td><td>Semantic Kernel Words`), 'kernel'],
  [re(String.raw`<td>N<\/td><td>Standard Words`), 'standard'],
  [re(String.raw`Words, within the N`), 'canonical'],
  // English phrasings
  [re(String.raw`These N Words divide`), 'canonical'],
  [re(String.raw`N-Word Semantic Kernel`), 'kernel'],
  [re(String.raw`N Standard Words`), 'standard'],
  [re(String.raw`All N are ordinary Core Words`), 'canonical'],
  [re(String.raw`vocabulary is N Words`), 'canonical'],
  [re(String.raw`The N Words of the canonical vocabulary`), 'canonical'],
  [re(String.raw`Canonical inventory: \*\*N Words`), 'canonical'],
  [re(String.raw`N (?:canonical|Core) Words`), 'canonical'],
  [re(String.raw`N\s+(?:Semantic\s+)?Kernel\s+Words`), 'kernel'],
  // Japanese phrasings
  [re(String.raw`このN個のワード`), 'canonical'],
  [re(String.raw`N\s*語のSemantic Kernel`), 'kernel'],
  [re(String.raw`N\s*語のStandard`), 'standard'],
  [re(String.raw`N個すべて`), 'canonical'],
  [re(String.raw`語彙は\s*N\s*(?:個のワード|語)`), 'canonical'],
  [re(String.raw`N個のCoreワード`), 'canonical'],
  [re(String.raw`全\s*N\s*語`), 'canonical'],
  [re(String.raw`[（(]\s*N\s*語のまま`), 'canonical'],
  [re(String.raw`N\s*語[（(]\s*Kernel`), 'canonical'],
  [re(String.raw`Kernel\s*N\s*[・、,]\s*Standard`), 'kernel'],
  [re(String.raw`Standard\s*N\s*[）)]`), 'standard'],
];

/** Any number of Words, in either language. */
export const BARE = re(String.raw`N\s*(?:-\s*)?(?:canonical\s+|Core\s+|Kernel\s+|Standard\s+|derived\s+)?(?:Words\b|語(?![句彙数])|個の(?:Core)?ワード)`);

/**
 * Bare counts in code that are not counts of the vocabulary, each with why.
 * [path, text on the line, reason]. Keep this short: a new entry is a claim
 * that the number is about something else, and a reviewer should agree.
 */
export const EXEMPT = [
  ['rust/src/interpreter/execute_builtin.rs', 'chain of 22 Words', 'the length of a test chain of user Words, not the vocabulary'],
  ['rust/src/runtime_limits_tests.rs', 'chain of 22 Words', 'the length of a test chain of user Words, not the vocabulary'],
  ['scripts/check-semantic-kernel.mjs', 'ten concepts and 100 Words', 'the vocabulary-100 work order of 2026-09, a past design target'],
  ['tools/mcp-server/tool-description.test.js', '"78 Words"', 'quotes the stale prose this test was written to catch'],
];

/**
 * Every stated count in `text` (the contents of `path`) that differs from
 * `record` ({ canonical, kernel, standard }). Returns { stated, mismatches },
 * where each mismatch is { line, written, expected, which, phrase }; `which`
 * is 'any' for a bare count, whose `expected` lists every count of record.
 */
export function scanText(path, text, record) {
  const lines = text.split('\n');
  const recorded = new Set(Object.values(record));
  const exempt = EXEMPT.filter(([p]) => p === path).map(([, phrase]) => phrase);
  const code = CODE_FILE.test(path);
  const mismatches = [];
  let stated = 0;

  lines.forEach((line, index) => {
    if (HISTORICAL.test(line)) return;
    const claimed = new Set();
    for (const [pattern, which] of STATED) {
      for (const match of line.matchAll(pattern)) {
        const at = match.index + match[0].indexOf(match[1]);
        if (claimed.has(at)) continue; // two phrasings around one number
        claimed.add(at);
        stated += 1;
        const written = Number(match[1]);
        if (written === record[which]) continue;
        mismatches.push({ line: index + 1, written, expected: record[which], which, phrase: match[0] });
      }
    }
    if (!code) return;
    for (const match of line.matchAll(BARE)) {
      if (claimed.has(match.index)) continue;
      stated += 1;
      const written = Number(match[1]);
      if (recorded.has(written)) continue;
      if (exempt.some((phrase) => line.includes(phrase))) continue;
      mismatches.push({ line: index + 1, written, expected: [...recorded], which: 'any', phrase: match[0] });
    }
  });

  return { stated, mismatches };
}
