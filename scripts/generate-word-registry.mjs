import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { dirname } from 'node:path';

// Project spec/words.json into a checked-in Rust Word registry — the single
// source of truth for the Word inventory, aliases, executor keys, and static
// contract fields (migration plan Phase 3). Run with `--check` in CI to fail on
// drift; run without it to regenerate.

const check = process.argv.includes('--check');
const outputPath = 'rust/src/kernel/generated/word_registry.rs';
const words = JSON.parse(readFileSync('spec/words.json', 'utf8'));
const entries = words.entries;
const schema = JSON.parse(readFileSync('spec/words.schema.json', 'utf8'));

// The contract vocabularies are generated from the schema's own `enum` lists
// rather than hand-written on the Rust side. That is what makes a narrower
// implementation vocabulary impossible: a value the specification admits is a
// variant here by construction, so it can never be silently collapsed into a
// neighbouring one (audit finding — the hand-written `NilPolicy` carried 5 of
// the specification's 7 values, so `passthroughThenProject` and `inspectNil`
// were inexpressible and 11 Words were mislabelled as a result).
const schemaEnum = (field) => {
  const found = [];
  const walk = (node) => {
    if (Array.isArray(node)) {
      node.forEach(walk);
    } else if (node && typeof node === 'object') {
      for (const [key, value] of Object.entries(node)) {
        if (key === field && value && typeof value === 'object' && Array.isArray(value.enum)) {
          found.push(value.enum);
        } else {
          walk(value);
        }
      }
    }
  };
  walk(schema);
  if (found.length !== 1) {
    throw new Error(`spec/words.schema.json must declare exactly one enum for "${field}" (found ${found.length})`);
  }
  return found[0];
};

// `starts-with?` / `passthroughThenProject` -> `StartsWith` / `PassthroughThenProject`.
const pascal = (value) =>
  value
    .replace(/[^A-Za-z0-9]+/g, ' ')
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .split(' ')
    .filter(Boolean)
    .map((part) => part[0].toUpperCase() + part.slice(1).toLowerCase())
    .join('');

// `ordered` derives `PartialOrd`/`Ord` for an enum whose spec declaration order
// *is* a lattice order, so the runtime can compare variants directly instead of
// mapping them through a hand-written rank. `parsed` additionally emits
// `from_spec_str`, for the vocabularies a caller writes by hand (a `#:contract`
// declaration) and the runtime therefore has to read back.
const CONTRACT_ENUMS = [
  { rustName: 'Family', field: 'family', doc: 'Semantic family the Word selects its shared laws from.' },
  { rustName: 'Consumption', field: 'consumption', doc: 'How the Word treats its operands under the default mode (SPEC §13.2).' },
  { rustName: 'NilPolicy', field: 'nilPolicy', doc: 'How the Word behaves when an operand is NIL (SPEC §7.12).' },
  { rustName: 'Partiality', field: 'partiality', doc: 'Whether well-formed application is total, partial, or NIL-projecting.' },
  { rustName: 'Purity', field: 'purity', doc: 'Observational purity class (SPEC §7.14).' },
  { rustName: 'Determinism', field: 'determinism', doc: 'What the Word\'s result may depend on beyond its operands.' },
  { rustName: 'VocabularyTier', field: 'vocabularyTier', doc: 'Where the Word sits in the public Core: the Semantic Kernel or the Standard vocabulary.' },
  { rustName: 'AcceptedDomain', field: 'acceptedDomain', doc: 'The shape of operand the Word accepts, where the specification narrows it.' },
  {
    rustName: 'CostClass',
    field: 'class',
    doc: 'Growth class of a Word\'s charge on one metered resource, as a function of its input.',
    ordered: true,
    parsed: true,
    note:
      'The variants are declared in the schema\'s own order, and that order **is** the\n'
      + '/// lattice order the cost join widens along (`const` < `linear` < `superlinear` <\n'
      + '/// `unbounded`), which is what the derived `Ord` means here. Reordering the\n'
      + '/// schema enum would silently reorder the lattice, so `word_cost_tests.rs`\n'
      + '/// asserts the chain directly.',
  },
];

const enumBlocks = CONTRACT_ENUMS.map(({ rustName, field, doc, ordered, parsed, note }) => {
  const values = schemaEnum(field);
  const variants = values.map((value) => `    /// \`${value}\`\n    ${pascal(value)},`).join('\n');
  const arms = values
    .map((value) => `            ${rustName}::${pascal(value)} => ${JSON.stringify(value)},`)
    .join('\n');
  const parseArms = values
    .map((value) => `            ${JSON.stringify(value)} => Some(${rustName}::${pascal(value)}),`)
    .join('\n');
  const derives = `Clone, Copy, Debug, PartialEq, Eq, Hash${ordered ? ', PartialOrd, Ord' : ''}`;
  const fromSpecStr = parsed
    ? `

    /// The variant a canonical spec string names, or \`None\` when the string is
    /// not one the specification admits.
    pub fn from_spec_str(value: &str) -> Option<${rustName}> {
        match value {
${parseArms}
            _ => None,
        }
    }`
    : '';
  return `/// ${doc}
///
/// Generated from the \`${field}\` enum in spec/words.schema.json: every value the
/// specification admits is a variant, so the implementation vocabulary cannot be
/// narrower than the canonical one.${note ? `\n///\n/// ${note}` : ''}
#[derive(${derives})]
pub enum ${rustName} {
${variants}
}

impl ${rustName} {
    /// The canonical spec string for this variant.
    pub const fn as_spec_str(self) -> &'static str {
        match self {
${arms}
        }
    }${fromSpecStr}
}`;
}).join('\n\n');

const enumRef = (rustName, value) => `${rustName}::${pascal(value)}`;

// Stack arity: a decimal count, or a data-dependent marker the specification
// names explicitly. Keeping the markers as variants (rather than collapsing
// both into one "dynamic" case, as the hand-written MassContract did) is what
// lets the static mass validator stay engaged for the 36 Words whose arity the
// specification pins exactly.
const arity = (value) =>
  typeof value === 'number' ? `Arity::Fixed(${value})` : `Arity::${pascal(String(value))}`;

// The executor key is a unique, valid PascalCase Rust identifier for every
// Word, so it doubles as the canonical WordId variant — the runtime dispatches
// on `WordId` itself rather than on a parallel hand-written key enum, which is
// why the key is not also emitted as a string field: one fact, one
// representation. Guard the uniqueness the enum relies on rather than emit a
// file that would not compile.
const ids = entries.map((word) => word.executorKey);
const duplicates = ids.filter((id, index) => ids.indexOf(id) !== index);
if (duplicates.length > 0) {
  throw new Error(`words.json has duplicate executorKey values: ${[...new Set(duplicates)].join(', ')}`);
}

// The three cost axes are projected in a fixed order rather than by iterating
// the entry's own key order, so a reordered `cost` object in spec/words.json
// cannot silently permute which resource a bound is attached to.
const COST_AXES = ['steps', 'numeric', 'collection'];
// Emitted in the shape rustfmt produces, since the generated file is checked in
// and CI runs `cargo fmt --check` over it: a one-line struct literal here would
// regenerate clean and then fail formatting.
const cost = (value) => {
  const axes = COST_AXES.map((axis) => {
    const declared = value[axis];
    return `            ${axis}: CostAxis {
                class: ${enumRef('CostClass', declared.class)},
                exact: ${declared.exact},
            },`;
  });
  return `WordCost {\n${axes.join('\n')}\n        }`;
};

// A projection condition of "never" is the absence of one, so it is projected
// as `None` rather than as a string every reader would have to compare against.
const projection = (when) => (when === 'never' ? 'None' : `Some(${JSON.stringify(when)})`);

// spec/words.json values are ASCII, so a JSON string literal is also a valid
// Rust string literal.
const rustStr = (value) => JSON.stringify(value);
// Emit the wrapped form ourselves when the one-line form would pass rustfmt's
// default 100-column width. The generated file is committed and CI runs both
// `cargo fmt --check` and `word-registry:check`, so a line rustfmt would break
// makes the two gates contradict each other: whichever ran last would be the
// one that failed. `field` is the field name the slice is assigned to, since
// it is part of the line being measured.
const RUST_MAX_WIDTH = 100;
const FIELD_INDENT = 8;
const rustStrSlice = (values, field) => {
  if (values.length === 0) return '&[]';
  const inline = `&[${values.map(rustStr).join(', ')}]`;
  const lineWidth = FIELD_INDENT + `${field}: `.length + inline.length + 1;
  if (lineWidth <= RUST_MAX_WIDTH) return inline;
  const items = values.map((value) => `${' '.repeat(FIELD_INDENT + 4)}${rustStr(value)},`);
  return `&[\n${items.join('\n')}\n${' '.repeat(FIELD_INDENT)}]`;
};

const variants = entries.map((word) => `    ${word.executorKey},`).join('\n');

const rows = entries
  .map(
    (word) => `    GeneratedWord {
        id: WordId::${word.executorKey},
        name: ${rustStr(word.name)},
        aliases: ${rustStrSlice(word.aliases, 'aliases')},
        family: ${enumRef('Family', word.family)},
        stack_inputs: ${arity(word.stack.inputs)},
        stack_outputs: ${arity(word.stack.outputs)},
        consumption: ${enumRef('Consumption', word.consumption)},
        nil_policy: ${enumRef('NilPolicy', word.nilPolicy)},
        projection: ${projection(word.projection.when)},
        partiality: ${enumRef('Partiality', word.partiality)},
        accepted_domain: ${word.acceptedDomain ? `Some(AcceptedDomain::${pascal(word.acceptedDomain)})` : 'None'},
        purity: ${enumRef('Purity', word.purity)},
        determinism: ${enumRef('Determinism', word.determinism)},
        cost: ${cost(word.cost)},
        vocabulary_tier: ${enumRef('VocabularyTier', word.vocabularyTier)},
        standard_kind: ${word.standardKind ? `Some(${rustStr(word.standardKind)})` : 'None'},
        effects: ${rustStrSlice(word.effects, 'effects')},
        error_when: ${rustStrSlice(word.errorWhen, 'error_when')},
        syntax: ${word.documentation.syntax ? `Some(${rustStr(word.documentation.syntax)})` : 'None'},
    },`,
  )
  .join('\n');

const output = `// @generated by scripts/generate-word-registry.mjs; do not edit.
//
// Canonical Word registry projected from spec/words.json — the single source of
// truth for the Word inventory, aliases, executor keys, and the static contract
// fields. Regenerate with \`npm run word-registry:generate\`; CI runs
// \`npm run word-registry:check\` to guard against drift.

/// Canonical identity of a Core Word, named by its executor key (unique across
/// the registry).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WordId {
${variants}
}

${enumBlocks}

/// Stack arity as declared in spec/words.json: an exact count, or one of the
/// data-dependent markers the specification names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Arity {
    /// An exact operand/result count.
    Fixed(u8),
    /// \`variable\` — data-dependent, not statically pinned.
    Variable,
    /// \`control\` — a positional control directive rather than a stack operation.
    Control,
}

impl Arity {
    /// The exact count when the specification pins one.
    pub const fn fixed(self) -> Option<u8> {
        match self {
            Arity::Fixed(n) => Some(n),
            Arity::Variable | Arity::Control => None,
        }
    }
}

/// A Word's declared charge on one metered resource.
///
/// \`class\` is a **sound upper bound**: the charge never grows faster than this
/// in the Word's input. \`exact\` records that some contribution provably
/// *attains* the class — the witness that lets a \`#:contract\` mismatch be
/// reported as an error rather than a note, since without it a looser class
/// might simply be an over-approximation the caller has every right to beat.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CostAxis {
    pub class: CostClass,
    pub exact: bool,
}

/// A Word's declared charge on all three metered resources, projected from
/// spec/words.json.
///
/// The three axes are the runtime's own counters (\`executionSteps\` /
/// \`numericWork\` / \`collectionWork\`), so a declared bound and a measured
/// \`resourceUsage\` are answers about the same quantity. They join pointwise
/// under concatenation, which is what lets a phrase's bound be computed from
/// its Words' bounds without running it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WordCost {
    pub steps: CostAxis,
    pub numeric: CostAxis,
    pub collection: CostAxis,
}

/// A Word's spec-declared contract, projected from spec/words.json with each
/// field typed by the vocabulary spec/words.schema.json declares for it.
#[derive(Debug)]
pub struct GeneratedWord {
    pub id: WordId,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub family: Family,
    pub stack_inputs: Arity,
    pub stack_outputs: Arity,
    pub consumption: Consumption,
    pub nil_policy: NilPolicy,
    /// The condition under which a *well-formed* operand yields a reasoned
    /// NIL, or \`None\` for the Words that declare \`never\`. Distinct from
    /// \`nil_policy\`, which is about NIL *operands*: a Word can pass a NIL
    /// through and still project one of its own (\`passthroughThenProject\`),
    /// or project without any NIL-operand rule engaging at all.
    pub projection: Option<&'static str>,
    pub partiality: Partiality,
    /// The operand shape the Word accepts, where the specification narrows it,
    /// and \`None\` where the Word takes whatever its family takes.
    ///
    /// \`partiality\` answers "what happens to an operand this Word accepts";
    /// without this field a reader had no way to ask which operands those are,
    /// so \`SORT\` could read as \`total\` — always produces a result — while
    /// rejecting a vector of strings outright.
    pub accepted_domain: Option<AcceptedDomain>,
    pub purity: Purity,
    pub determinism: Determinism,
    /// What the Word charges on each metered resource. Read by
    /// \`interpreter::word_cost\`, which joins these bounds along a phrase to
    /// bound it without executing it.
    pub cost: WordCost,
    /// Which half of the public Core the Word belongs to. Both halves are
    /// ordinary sealed-Core Words reached by their plain names; the tier is a
    /// design classification the reading surfaces report, never a namespace.
    pub vocabulary_tier: VocabularyTier,
    /// Why a Standard Word is native rather than left to a user definition, or
    /// \`None\` for a Semantic Kernel Word.
    pub standard_kind: Option<&'static str>,
    /// The effects the Word declares, in the specification's own spelling.
    /// Empty for every \`pure\` Word; the vocabulary is open (the schema types
    /// it as free strings), so this is projected as declared rather than
    /// narrowed into a Rust enum.
    pub effects: &'static [&'static str],
    /// The conditions under which the Word *raises*, in the specification's
    /// own spelling — the ERROR counterpart of \`projection\`, which names the
    /// condition under which it answers NIL instead.
    ///
    /// Projected so a diagnosis can be derived from the declaration rather
    /// than from a hand-written table: a Word that raises for a condition it
    /// declares used to reach the caller as \`why: "unknown"\` with "read the
    /// message" as its only next check.
    pub error_when: &'static [&'static str],
    /// One line of the Word written correctly, as the specification writes it.
    ///
    /// Read by the stack-underflow diagnosis, where "how many operands and in
    /// what shape" is exactly the question and \`Stack underflow\` alone
    /// answers none of it.
    pub syntax: Option<&'static str>,
}

pub const GENERATED_WORDS: &[GeneratedWord] = &[
${rows}
];
`;

if (check) {
  const current = readFileSync(outputPath, 'utf8');
  if (current !== output) {
    console.error(`[word-registry] ${outputPath} is stale. Run npm run word-registry:generate.`);
    process.exitCode = 1;
  } else {
    console.log(`[word-registry] ${entries.length} Words are current.`);
  }
} else {
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, output);
  console.log(`[word-registry] wrote ${entries.length} Words to ${outputPath}.`);
}
