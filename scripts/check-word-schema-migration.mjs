import { readFileSync, readdirSync } from 'node:fs';

const words = JSON.parse(readFileSync('spec/words.json', 'utf8'));
const schema = JSON.parse(readFileSync('spec/words.schema.json', 'utf8'));
const families = JSON.parse(readFileSync('spec/semantic-families.json', 'utf8'));
const manifest = JSON.parse(readFileSync('docs/word-manifest.json', 'utf8'));
const aliasesSource = readFileSync('rust/src/core_word_aliases.rs', 'utf8');
const compiledPlanSource = readFileSync('rust/src/interpreter/compiled_plan.rs', 'utf8');
const dispatchSource = readFileSync('rust/src/interpreter/execute_builtin.rs', 'utf8');
const language = readFileSync('spec/language-semantics.md', 'utf8');

const errors = [];
const fail = (message) => errors.push(message);
const required = schema.$defs.word.required;
const familyIds = new Set(families.families.map((family) => family.id));
const manifestNames = new Set(manifest.entries.map((entry) => entry.canonical));
const names = new Set();

if (words.migration.completeInventory !== true) fail('the canonical Word inventory must be marked complete');
if (words.migration.betaFreezePhase !== 1) fail('the beta vocabulary migration must be in phase 1');

for (const word of words.entries) {
  if (names.has(word.name)) fail(`duplicate Word: ${word.name}`);
  names.add(word.name);
  for (const field of required) if (!(field in word)) fail(`${word.name} lacks required field ${field}`);
  if (!familyIds.has(word.family)) fail(`${word.name} references unknown family ${word.family}`);
  if (word.vocabularyTier === 'standard' && !['shorthand', 'namedPattern', 'algorithm', 'operational'].includes(word.standardKind)) {
    fail(`${word.name} lacks a valid standardKind`);
  }
  if (word.vocabularyTier === 'kernel' && 'standardKind' in word) fail(`${word.name} is Kernel but declares standardKind`);
  if (!manifestNames.has(word.name)) fail(`${word.name} is absent from the frozen manifest`);
  for (const clause of word.clauses) if (!language.includes(`${clause} —`)) fail(`${word.name} references missing clause ${clause}`);

  // The executor key used to be written a second time on the Rust spec entry,
  // and this check reconciled the two copies. It is written once now — the
  // generated `WordId` *is* the executor key — so what is checked instead is
  // that the key reaches a runtime arm. The compiler already requires the
  // dispatch match to be total over `WordId`; this catches the case a total
  // match cannot, a Word folded into a neighbour's arm by mistake.
  const compiledModifiers = new Set(['KEEP']);
  const directive = new Set(['VENT', 'FLOW']);
  if (compiledModifiers.has(word.name)) {
    if (!compiledPlanSource.includes(`CompiledOp::${word.executorKey}`)) fail(`${word.name} compiled executorKey drift`);
  } else if (directive.has(word.name)) {
    if (word.executorKey !== 'LazyNextUnitFallback') fail(`${word.name} executorKey drift`);
  } else if (!dispatchSource.includes(`WordId::${word.executorKey} =>`)) {
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
