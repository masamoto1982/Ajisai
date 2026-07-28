import { readFileSync } from 'node:fs';

const words = JSON.parse(readFileSync('spec/words.json', 'utf8'));
const schema = JSON.parse(readFileSync('spec/words.schema.json', 'utf8'));
const families = JSON.parse(readFileSync('spec/semantic-families.json', 'utf8'));
const manifest = JSON.parse(readFileSync('docs/word-manifest.json', 'utf8'));
const rust = readFileSync('rust/src/builtins/builtin_word_definitions.rs', 'utf8');
const aliasesSource = readFileSync('rust/src/core_word_aliases.rs', 'utf8');
const compiledPlanSource = readFileSync('rust/src/interpreter/compiled_plan.rs', 'utf8');
const language = readFileSync('spec/language-semantics.md', 'utf8');

const errors = [];
const fail = (message) => errors.push(message);
const required = schema.$defs.word.required;
const familyIds = new Set(families.families.map((family) => family.id));
const manifestNames = new Set(manifest.entries.filter((entry) => entry.kind === 'coreword').map((entry) => entry.canonical));
const names = new Set();

if (words.migration.entryCount !== words.entries.length) fail('migration.entryCount does not match entries');
if (words.migration.completeInventory !== false) fail('the family rollout must not claim complete inventory');

for (const word of words.entries) {
  if (names.has(word.name)) fail(`duplicate Word: ${word.name}`);
  names.add(word.name);
  for (const field of required) if (!(field in word)) fail(`${word.name} lacks required field ${field}`);
  if (!familyIds.has(word.family)) fail(`${word.name} references unknown family ${word.family}`);
  if (!manifestNames.has(word.name)) fail(`${word.name} is absent from the frozen manifest`);
  for (const clause of word.clauses) if (!language.includes(`${clause} —`)) fail(`${word.name} references missing clause ${clause}`);

  const escaped = word.name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const start = rust.search(new RegExp(`name: "${escaped}"`));
  if (start < 0) {
    fail(`${word.name} has no Rust registry entry`);
    continue;
  }
  const block = rust.slice(start, rust.indexOf('..SPEC_DEFAULT', start));
  const compiledModifiers = new Set(['TOP', 'STAK', 'EAT', 'KEEP']);
  const directive = new Set(['VENT', 'FLOW']);
  if (compiledModifiers.has(word.name)) {
    if (!compiledPlanSource.includes(`CompiledOp::${word.executorKey}`)) fail(`${word.name} compiled executorKey drift`);
  } else {
    const executorMarker = directive.has(word.name)
      ? `execution_form: ExecutionForm::${word.executorKey}`
      : `executor_key: Some(BuiltinExecutorKey::${word.executorKey})`;
    if (!block.includes(executorMarker)) fail(`${word.name} executorKey drift`);
  }
  const normalizedBlock = block.replace(/\\\s*\n\s*/g, '').replace(/\s+/g, ' ');
  if (!normalizedBlock.includes(word.documentation.summary)) fail(`${word.name} summary drift`);
  if (!block.includes(`hover_syntax: "${word.documentation.syntax}"`)) fail(`${word.name} syntax drift`);
  for (const effect of word.effects) {
    const rustEffect = effect.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);
    if (!block.includes(`"${rustEffect}"`)) fail(`${word.name} effect drift: ${effect}`);
  }
  for (const alias of word.aliases) {
    const aliasPattern = `alias: "${alias}",`;
    const canonicalPattern = `canonical: Some("${word.name}"),`;
    const aliasStart = aliasesSource.indexOf(aliasPattern);
    const aliasBlock = aliasesSource.slice(aliasStart, aliasesSource.indexOf('},', aliasStart));
    if (aliasStart < 0 || !aliasBlock.includes(canonicalPattern)) fail(`${word.name} alias drift: ${alias}`);
  }
}

const priorSlice = ['TRUE', 'FALSE', 'NIL', 'NIL?', 'NIL-REASON', 'NIL-ORIGIN', 'NIL-RECOVERABLE?', 'NIL-DIAGNOSIS', 'BOOL', 'COMPARE-WITHIN', 'EQ', 'LT', 'LTE', 'GT', 'GTE', 'NEQ', 'AND', 'OR', 'NOT', 'VENT', 'TOP', 'STAK', 'EAT', 'KEEP', 'IDLE', 'COND', 'FLOW', 'FORC', 'EXEC', 'CONSERVE', 'EVAL', 'OR-ELSE', 'DEF', 'DEL', 'LOOKUP', 'IMPORT', 'IMPORT-ONLY', 'UNIMPORT', 'UNIMPORT-ONLY'];
const collectionSlice = manifest.entries
  .filter((entry) => entry.kind === 'coreword' && ['vector', 'tensor', 'higher-order'].includes(entry.category))
  .map((entry) => entry.canonical);
const expected = new Set([...priorSlice, ...collectionSlice]);
for (const name of expected) if (!names.has(name)) fail(`migration scope omits ${name}`);
if (names.size !== expected.size) fail(`migration scope has ${names.size} entries; expected ${expected.size}`);

if (errors.length) {
  for (const error of errors) console.error(`[word-schema] ${error}`);
  process.exitCode = 1;
} else {
  console.log(`[word-schema] ${names.size} migrated contracts match the 224-surface manifest and current executors.`);
}
