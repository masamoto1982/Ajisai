// The rule of scripts/lib/vocabulary-counts.mjs, exercised without the tree:
// each case is a line in a file, and whether the gate must refuse it.
//
//   npm run test:scripts
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { scanText } from './lib/vocabulary-counts.mjs';

const RECORD = { canonical: 77, kernel: 49, standard: 28 };
const refused = (path, text) => scanText(path, text, RECORD).mismatches.map((m) => m.written);

test('a current count passes in prose and in code', () => {
  assert.deepEqual(refused('README.md', 'Core holds the 77 canonical Words and the 49-Word Semantic Kernel.'), []);
  assert.deepEqual(refused('public/docs/ja/index.html', 'この77個のワードは、49語のSemantic Kernelと28語のStandard'), []);
  assert.deepEqual(refused('docs/dev/x.md', '語彙は 77 語（Kernel 49・Standard 28）。'), []);
  assert.deepEqual(refused('rust/src/x.rs', '/// all 77 Words, 49 Kernel Words'), []);
});

test('a stale whole-vocabulary count is refused in any text file', () => {
  assert.deepEqual(refused('spec/language-semantics.md', '<td>78</td><td>Canonical Words</td>'), [78]);
  assert.deepEqual(refused('docs/dev/x.md', '現在の語彙は 78 語（Kernel 50・Standard 28）'), [78, 50]);
  assert.deepEqual(refused('public/docs/ja/index.html', '78個すべては'), [78]);
  assert.deepEqual(refused('README.md', 'a 50-Word Semantic Kernel'), [50]);
});

test('a stale bare count is refused in code and CI comments', () => {
  assert.deepEqual(refused('.github/workflows/test.yml', '        # 65語 × 入力ドメイン組の全数表。'), [65]);
  assert.deepEqual(refused('scripts/x.mjs', '// each of the 86 Words is run'), [86]);
});

test('a bare count in prose is not a claim about the vocabulary', () => {
  assert.deepEqual(refused('docs/dev/lexicon.md', '容量なしは約 40 語で固定点に近づき、辞書は 8 語のまま'), []);
  assert.deepEqual(refused('public/docs/en/index.html', '<a href="#words">2.4.2 Words</a>'), []);
});

test('a count narrated in the past tense is history', () => {
  assert.deepEqual(refused('docs/dev/x.md', '語は増やさない（当時 78 語のまま。のちに 77 語）。'), []);
  assert.deepEqual(refused('rust/tests/x.rs', '/// a hand copy here once said 48 Words'), []);
});

test('only the listed exemption passes, and only in its file', () => {
  assert.deepEqual(refused('rust/src/runtime_limits_tests.rs', '// down a chain of 22 Words'), []);
  assert.deepEqual(refused('rust/src/other.rs', '// down a chain of 22 Words'), [22]);
});

test('the part of a larger number is not a count', () => {
  assert.deepEqual(refused('rust/src/x.rs', '/// a chain of 2,000 one-token Words, see §9.3 Core Words'), []);
});
