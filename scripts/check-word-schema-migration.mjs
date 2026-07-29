import { readFileSync } from 'node:fs';

const words = JSON.parse(readFileSync('spec/words.json', 'utf8'));
const schema = JSON.parse(readFileSync('spec/words.schema.json', 'utf8'));
const families = JSON.parse(readFileSync('spec/semantic-families.json', 'utf8'));
const manifest = JSON.parse(readFileSync('docs/word-manifest.json', 'utf8'));
const rust = readFileSync('rust/src/builtins/builtin_word_definitions.rs', 'utf8');
const aliasesSource = readFileSync('rust/src/core_word_aliases.rs', 'utf8');
const compiledPlanSource = readFileSync('rust/src/interpreter/compiled_plan.rs', 'utf8');
const language = readFileSync('spec/language-semantics.md', 'utf8');
const moduleBuiltins = readFileSync('rust/src/interpreter/modules/module_builtins.rs', 'utf8');
const moduleDocs = readFileSync('rust/src/interpreter/modules/module_word_docs.rs', 'utf8');

function moduleWordBlock(moduleName, shortName) {
  const arrayStart = moduleBuiltins.indexOf(`const ${moduleName}_WORDS`);
  const arrayEnd = moduleBuiltins.indexOf('\n];', arrayStart);
  if (arrayStart < 0 || arrayEnd < 0) return undefined;
  const body = moduleBuiltins.slice(arrayStart, arrayEnd);
  return [...body.matchAll(/module_word!\(([\s\S]*?)\n\s*\),/g)]
    .map((match) => match[1])
    .find((block) => block.match(/^\s*"([^"]+)"/)?.[1] === shortName);
}

function moduleDocBlock(moduleName, shortName) {
  return [...moduleDocs.matchAll(/ModuleWordDoc\s*\{([\s\S]*?)\n\s*\},/g)]
    .map((match) => match[1])
    .find((block) => block.includes(`module: "${moduleName}"`) && block.includes(`word: "${shortName}"`));
}

const errors = [];
const fail = (message) => errors.push(message);
const required = schema.$defs.word.required;
const familyIds = new Set(families.families.map((family) => family.id));
const manifestNames = new Set(manifest.entries.map((entry) => entry.canonical));
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

  if (word.module) {
    const [moduleName, shortName] = word.name.split('@');
    if (moduleName !== word.module || !shortName) fail(`${word.name} has inconsistent module ownership`);
    const block = moduleWordBlock(moduleName, shortName);
    if (!block || !block.includes(word.executorKey)) fail(`${word.name} module executorKey drift`);
    const doc = moduleDocBlock(moduleName, shortName);
    if (!doc) fail(`${word.name} has no module documentation`);
    else if (!doc.includes(`summary: "${word.documentation.summary}"`)) fail(`${word.name} summary drift`);
    for (const effect of word.effects) {
      const rustEffect = effect.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);
      if (!block?.includes(`"${rustEffect}"`)) fail(`${word.name} effect drift: ${effect}`);
    }
    continue;
  }

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

const expected = new Set(manifest.entries
  .filter((entry) => entry.kind === 'coreword' || (entry.kind === 'moduleword' && ['DATA', 'JSON', 'IO', 'CRYPTO'].includes(entry.module)))
  .map((entry) => entry.canonical));
for (const name of expected) if (!names.has(name)) fail(`migration scope omits ${name}`);
for (const name of names) if (!expected.has(name)) fail(`migration scope contains unexpected Word ${name}`);
if (names.size !== expected.size) fail(`migration scope has ${names.size} entries; expected ${expected.size}`);

if (errors.length) {
  for (const error of errors) console.error(`[word-schema] ${error}`);
  process.exitCode = 1;
} else {
  console.log('[word-schema] all 98 Core and 20 migrated Module contracts match the 224-surface manifest and current executors.');
}
