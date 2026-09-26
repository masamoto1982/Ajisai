import { readFileSync, readdirSync } from 'node:fs';

const words = JSON.parse(readFileSync('spec/words.json', 'utf8'));
const schema = JSON.parse(readFileSync('spec/words.schema.json', 'utf8'));
const families = JSON.parse(readFileSync('spec/semantic-families.json', 'utf8'));
const manifest = JSON.parse(readFileSync('docs/word-manifest.json', 'utf8'));
const aliasesSource = readFileSync('rust/src/core_word_aliases.rs', 'utf8');
const dispatchSource = readFileSync('rust/src/interpreter/execute_builtin.rs', 'utf8');
const language = readFileSync('spec/language-semantics.md', 'utf8');
const outcomes = JSON.parse(readFileSync('spec/outcomes.json', 'utf8'));

const errors = [];
const fail = (message) => errors.push(message);
const required = schema.$defs.word.required;
const familyIds = new Set(families.families.map((family) => family.id));
const manifestNames = new Set(manifest.entries.map((entry) => entry.canonical));
const names = new Set();

// `nilPolicy` is a summary of the operand roles (LANG.FAILURE.PASSTHROUGH),
// never a choice of its own: two Words that treat their operands alike treat
// a NIL alike. The summary names the strongest role present, in the order a
// NIL meets them at dispatch.
function derivedNilPolicy(roles, projecting) {
  if (roles.length === 0) return 'preserveReason';
  if (roles.includes('truth')) return 'kleeneAbsorbing';
  if (roles.includes('data') || roles.includes('leaf')) return projecting ? 'passthroughThenProject' : 'passthrough';
  if (!roles.includes('element')) return projecting ? 'createsNil' : 'rejectNil';
  return 'consumeNil';
}

// `partiality` summarizes the outcomes the Word declares, never a choice of
// its own: a Word that can project is `projecting`; one that can raise even on
// operands of the right kind — it runs a block, or declares a condition
// repaired in the program (spec/outcomes.json `repair`) — is `partial`; any
// other Word raises only on an operand of the wrong kind, and is `total`.
const programConditions = new Set(
  outcomes.errorCategories.filter((category) => category.repair === 'program').map((category) => category.id),
);
function derivedPartiality(word) {
  if (word.projection.when !== 'never') return 'projecting';
  if (word.purity === 'conditional' || (word.errorWhen ?? []).some((c) => programConditions.has(c))) return 'partial';
  return 'total';
}

for (const word of words.entries) {
  if (word.partiality !== derivedPartiality(word)) {
    fail(`${word.name} declares partiality ${word.partiality}, but its outcomes derive ${derivedPartiality(word)}`);
  }
  if (names.has(word.name)) fail(`duplicate Word: ${word.name}`);
  names.add(word.name);
  for (const field of required) if (!(field in word)) fail(`${word.name} lacks required field ${field}`);
  if (!familyIds.has(word.family)) fail(`${word.name} references unknown family ${word.family}`);
  if (word.vocabularyTier === 'standard' && !['shorthand', 'namedPattern', 'algorithm', 'operational'].includes(word.standardKind)) {
    fail(`${word.name} lacks a valid standardKind`);
  }
  if (word.vocabularyTier === 'kernel' && 'standardKind' in word) fail(`${word.name} is Kernel but declares standardKind`);
  if (!manifestNames.has(word.name)) fail(`${word.name} is absent from the frozen manifest`);
  // A Word that tests its operands and answers a truth value is named with a
  // trailing `?` (NIL?, HAS?, MEMBER?), and a `?` name always answers one.
  // Relations and connectives (EQ LT GT, AND NOT SELECT) are operators, named
  // for the operation, and exempt.
  const answersTruth = /-> \[ (TRUE \| FALSE|truths?) \]/.test(word.documentation.stackEffect);
  const operator = ['comparison', 'booleanLogic'].includes(word.family);
  if (!operator && answersTruth !== word.name.endsWith('?')) {
    fail(
      answersTruth
        ? `${word.name} answers a truth value, so its name ends in ?`
        : `${word.name} ends in ? but does not answer a truth value`,
    );
  }
  // One notation for every stack effect: every operand and result in its own
  // `[ … ]` group with inner spaces, named rather than quoted, in one
  // vocabulary — `text` for a String, `TRUE | FALSE` for a truth value, `...`
  // for "and more", alternatives spaced around `|`.
  const effect = word.documentation.stackEffect;
  const [inputs = ''] = effect.split('->');
  if (
    /\[\]|'[A-Za-z.]+'/.test(effect) ||
    /\b(str|bool)\b/.test(effect) ||
    /(?<!\.)\.\.(?!\.)/.test(effect) ||
    /\S\||\|\S/.test(effect) ||
    // A projection is declared in `projection`; the stack effect names the
    // value answered, so `| NIL` alternatives are not written there.
    /\| NIL\b/.test(effect) ||
    !/^(\[ .* \] )?$/.test(inputs)
  ) {
    fail(`${word.name} stack effect \`${effect}\` departs from the one notation (bracketed, unquoted, text, TRUE | FALSE, ...)`);
  }
  // Two lifted operands are aligned lane by lane (LANG.COLLECTIONS.LIFT), and
  // two containers that do not align are `shapeMismatch`; a Word with fewer
  // has nothing to align and does not declare it for lifting's sake.
  const liftedCount = (word.stack.operands ?? []).filter((role) => role === 'leaf' || role === 'truth').length;
  if (liftedCount >= 2 && !(word.errorWhen ?? []).includes('shapeMismatch')) {
    fail(`${word.name} lifts ${liftedCount} operands but does not declare shapeMismatch`);
  }
  // A lifted operand is only meaningful for a Word with one result: the
  // lift assembles one result per element.
  if ((word.stack.operands ?? []).some((role) => role === 'leaf' || role === 'truth') && word.stack.outputs !== 1) {
    fail(`${word.name} lifts over an operand but does not answer exactly one value`);
  }
  const operands = word.stack.operands;
  if (typeof word.stack.inputs === 'number') {
    if (!Array.isArray(operands) || operands.length !== word.stack.inputs) {
      fail(`${word.name} declares ${word.stack.inputs} input(s) but ${operands?.length ?? 'no'} operand role(s)`);
    } else if (word.nilPolicy !== derivedNilPolicy(operands, word.projection.when !== 'never')) {
      fail(
        `${word.name} declares nilPolicy ${word.nilPolicy}, but its operand roles [${operands.join(', ')}] ` +
          `derive ${derivedNilPolicy(operands, word.projection.when !== 'never')}`,
      );
    }
  } else if (operands !== undefined) {
    fail(`${word.name} has data-dependent arity and must not declare operand roles`);
  }
  for (const clause of word.clauses) if (!language.includes(`${clause} —`)) fail(`${word.name} references missing clause ${clause}`);
  // Two clauses restate a fact the declaration already records, so the
  // citation is derived from it rather than chosen: a Word is subject to the
  // lifting law exactly when it has a lifted operand, and to the projection
  // law exactly when it declares a projection.
  const derivedClauses = [
    ['LANG.COLLECTIONS.LIFT', (word.stack.operands ?? []).some((role) => role === 'leaf' || role === 'truth')],
    ['LANG.FAILURE.PROJECT', word.projection.when !== 'never'],
  ];
  for (const [clause, applies] of derivedClauses) {
    if (applies !== word.clauses.includes(clause)) {
      fail(`${word.name} ${applies ? 'must' : 'must not'} cite ${clause}: its declaration ${applies ? 'has' : 'has no'} the property the clause governs`);
    }
  }

  // The executor key used to be written a second time on the Rust spec entry,
  // and this check reconciled the two copies. It is written once now — the
  // generated `WordId` *is* the executor key — so what is checked instead is
  // that the key reaches a runtime arm. The compiler already requires the
  // dispatch match to be total over `WordId`; this catches the case a total
  // match cannot, a Word folded into a neighbour's arm by mistake.
  if (!dispatchSource.includes(`WordId::${word.executorKey} =>`)) {
    fail(`${word.name} has no dispatch arm for WordId::${word.executorKey}`);
  }
  // Canonical documentation and effects now have one generated spelling.
  // Aliases remain a separate source-level surface, so reconcile those here.
  for (const alias of word.aliases) {
    const aliasPattern = `alias: "${alias}",`;
    const canonicalPattern = `canonical: Some("${word.name}"),`;
    const aliasStart = aliasesSource.indexOf(aliasPattern);
    const aliasBlock = aliasesSource.slice(aliasStart, aliasesSource.indexOf('},', aliasStart));
    if (aliasStart < 0 || !aliasBlock.includes(canonicalPattern)) fail(`${word.name} alias drift: ${alias}`);
  }
}

// A family is the laws its Words share (spec/semantic-families.json), so its
// clauses are exactly the clauses every member cites — no more, which would
// claim a law some member is not subject to, and no fewer.
for (const family of families.families) {
  const members = words.entries.filter((word) => word.family === family.id);
  const shared = members.length
    ? members[0].clauses.filter((clause) => members.every((word) => word.clauses.includes(clause)))
    : [];
  const declared = [...family.clauses].sort();
  if (JSON.stringify(declared) !== JSON.stringify([...shared].sort())) {
    fail(`family ${family.id} declares clauses [${declared.join(', ')}] but its Words share [${[...shared].sort().join(', ')}]`);
  }
}

const canonicalManifestEntries = manifest.entries.filter((entry) => entry.kind === 'coreword');
const expected = new Set(canonicalManifestEntries.map((entry) => entry.canonical));
for (const name of expected) if (!names.has(name)) fail(`contract scope omits ${name}`);
for (const name of names) if (!expected.has(name)) fail(`contract scope contains unexpected Word ${name}`);
if (names.size !== expected.size) fail(`contract scope has ${names.size} entries; expected ${expected.size}`);

if (errors.length) {
  for (const error of errors) console.error(`[word-schema] ${error}`);
  process.exitCode = 1;
} else {
  console.log(`[word-schema] all ${canonicalManifestEntries.length} Core contracts cover the ${manifest.entries.length}-surface manifest and current executors.`);
}
