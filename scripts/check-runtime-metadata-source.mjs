#!/usr/bin/env node
// Enforces the invariant that `BuiltinSpec` is a *projection*, never a source.
//
// The failure this exists to prevent is not a particular retired type name; it
// is the shape of the bug. A hand-authored Core Word metadata table
// (historically `RuntimeSpec` + `SPEC_DEFAULT`) reappears alongside the
// projection, and the two disagree — or, as happened once, one side is deleted
// while the other keeps importing it and the tree stops compiling. Grepping for
// the old names would only catch a repeat under the same spelling, and would
// pass the moment someone calls the parallel table something else.
//
// So the check is a positive whitelist over the construction of `BuiltinSpec`
// rather than a blocklist over names: every field must be traceable, at the
// syntax level, to one of the two canonical sources.
//
//   prose      -> `doc.<field>`, a field of the generated `GeneratedCoreWordDoc`
//   contract   -> `<path>::<name>_from_contract(word)`, defined in
//                 rust/src/coreword_registry/contract.rs
//
// Anything else — a string literal, a `const`, a lookup into a second table, a
// `..Default::default()` spread — fails, whatever it is named. Together with
// `core-word-docs:check` (generated docs match spec/words.json) this closes the
// path from the canonical source to the runtime view.

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const DEFINITIONS = 'rust/src/builtins/builtin_word_definitions.rs';
const GENERATED_DOCS = 'rust/src/builtins/generated_core_word_docs.rs';
const CONTRACT = 'rust/src/coreword_registry/contract.rs';

const errors = [];
const read = (path) => readFileSync(resolve(repoRoot, path), 'utf8');

// Rust source is scanned with a brace matcher rather than a regex: field
// initializers contain nested calls and generics, so a non-greedy match would
// stop at the first `}` inside the body.
function matchBrace(source, openIndex) {
  let depth = 0;
  let inString = false;
  for (let i = openIndex; i < source.length; i += 1) {
    const char = source[i];
    if (inString) {
      if (char === '\\') i += 1;
      else if (char === '"') inString = false;
      continue;
    }
    if (char === '"') inString = true;
    else if (char === '{') depth += 1;
    else if (char === '}') {
      depth -= 1;
      if (depth === 0) return i;
    }
  }
  return -1;
}

// Split `a: x, b: f(y, z)` on top-level commas only, so an argument list does
// not read as a field boundary.
function splitTopLevel(body) {
  const parts = [];
  let depth = 0;
  let inString = false;
  let current = '';
  for (let i = 0; i < body.length; i += 1) {
    const char = body[i];
    if (inString) {
      current += char;
      if (char === '\\') current += body[(i += 1)];
      else if (char === '"') inString = false;
      continue;
    }
    if (char === '"') inString = true;
    else if ('([{<'.includes(char)) depth += 1;
    else if (')]}>'.includes(char)) depth -= 1;
    if (char === ',' && depth === 0) {
      parts.push(current);
      current = '';
      continue;
    }
    current += char;
  }
  parts.push(current);
  return parts.map((part) => part.trim()).filter(Boolean);
}

// Strip line comments and attributes so `#[allow(dead_code)]` between fields
// does not read as a field.
function structFields(source, declaration, label) {
  const at = source.indexOf(declaration);
  if (at === -1) {
    errors.push(`${label}: could not find \`${declaration}\``);
    return [];
  }
  const open = source.indexOf('{', at);
  const close = matchBrace(source, open);
  const body = source
    .slice(open + 1, close)
    .replace(/\/\/[^\n]*/g, '')
    .replace(/#\[[^\]]*\]/g, '');
  return splitTopLevel(body)
    .map((field) => field.match(/^(?:pub(?:\([^)]*\))?\s+)?([a-z_][a-z0-9_]*)\s*:/)?.[1])
    .filter(Boolean);
}

const definitions = read(DEFINITIONS);
const generatedDocs = read(GENERATED_DOCS);
const contract = read(CONTRACT);

// The generated docs file must actually be generator output. If it were
// hand-editable, "sourced from `doc.x`" would guarantee nothing.
if (!generatedDocs.startsWith('// @generated')) {
  errors.push(`${GENERATED_DOCS}: missing \`// @generated\` banner; it must be generator output`);
}

const specFields = structFields(definitions, 'pub struct BuiltinSpec', DEFINITIONS);
const docFields = new Set(structFields(generatedDocs, 'struct GeneratedCoreWordDoc', GENERATED_DOCS));
const contractFns = new Set(
  [...contract.matchAll(/\bfn\s+([a-z_][a-z0-9_]*_from_contract)\s*\(/g)].map((match) => match[1]),
);

if (specFields.length === 0) errors.push(`${DEFINITIONS}: BuiltinSpec declares no fields`);
if (docFields.size === 0) errors.push(`${GENERATED_DOCS}: GeneratedCoreWordDoc declares no fields`);
if (contractFns.size === 0) errors.push(`${CONTRACT}: no \`*_from_contract\` projections found`);

// `builtin_specs()` must iterate the generated docs. Iterating anything else
// would mean the inventory has a second source, even if every field checked out.
const assembler = definitions.slice(definitions.indexOf('fn builtin_specs('));
if (!/GENERATED_CORE_WORD_DOCS\s*\n?\s*\.iter\(\)/.test(assembler)) {
  errors.push(`${DEFINITIONS}: builtin_specs() must iterate GENERATED_CORE_WORD_DOCS`);
}

// Every construction of BuiltinSpec, anywhere in the crate. More than one means
// a second assembly path exists; a 69-entry authored table would show up here as
// 69 sites.
const sites = [];
const pattern = /(?<!struct\s)\bBuiltinSpec\s*\{/g;
for (const match of definitions.matchAll(pattern)) {
  const open = definitions.indexOf('{', match.index);
  const close = matchBrace(definitions, open);
  sites.push({ index: match.index, body: definitions.slice(open + 1, close) });
}

if (sites.length !== 1) {
  errors.push(
    `${DEFINITIONS}: expected exactly 1 BuiltinSpec construction site, found ${sites.length}. ` +
      'A second site means Core Word metadata is assembled in more than one place.',
  );
}

for (const site of sites) {
  const line = definitions.slice(0, site.index).split('\n').length;
  const seen = new Set();
  for (const entry of splitTopLevel(site.body.replace(/\/\/[^\n]*/g, ''))) {
    if (entry.startsWith('..')) {
      errors.push(
        `${DEFINITIONS}:${line}: struct-update syntax \`${entry}\` is not allowed; ` +
          'every field must name its canonical source explicitly.',
      );
      continue;
    }
    const field = entry.match(/^([a-z_][a-z0-9_]*)\s*:/)?.[1];
    if (!field) {
      errors.push(`${DEFINITIONS}:${line}: could not parse field initializer \`${entry}\``);
      continue;
    }
    seen.add(field);
    const value = entry.slice(entry.indexOf(':') + 1).trim();

    const fromDocs = value.match(/^doc\.([a-z_][a-z0-9_]*)$/);
    if (fromDocs) {
      if (!docFields.has(fromDocs[1])) {
        errors.push(
          `${DEFINITIONS}:${line}: ${field} reads \`doc.${fromDocs[1]}\`, ` +
            `which is not a field of GeneratedCoreWordDoc in ${GENERATED_DOCS}`,
        );
      }
      continue;
    }

    const fromContract = value.match(
      /^(?:[A-Za-z_][A-Za-z0-9_]*::)*([a-z_][a-z0-9_]*_from_contract)\(\s*word\s*\)$/,
    );
    if (fromContract) {
      if (!contractFns.has(fromContract[1])) {
        errors.push(
          `${DEFINITIONS}:${line}: ${field} calls \`${fromContract[1]}\`, ` +
            `which is not defined in ${CONTRACT}`,
        );
      }
      continue;
    }

    errors.push(
      `${DEFINITIONS}:${line}: ${field} is set from \`${value}\`, which is neither ` +
        'generated documentation (`doc.<field>`) nor a canonical contract projection ' +
        '(`<name>_from_contract(word)`). BuiltinSpec is a projection of spec/words.json, ' +
        'not a place to author Core Word metadata.',
    );
  }

  // A field that is declared but never initialized here cannot have come from a
  // canonical source, so the whitelist above would not have seen it.
  for (const field of specFields) {
    if (!seen.has(field)) errors.push(`${DEFINITIONS}:${line}: BuiltinSpec.${field} is never assigned`);
  }
}

if (errors.length > 0) {
  console.error('[runtime-metadata] BuiltinSpec must be projected from canonical sources only:');
  for (const error of errors) console.error(`  - ${error}`);
  process.exit(1);
}

console.log(
  `[runtime-metadata] BuiltinSpec projects ${specFields.length} fields from generated docs ` +
    `and ${contractFns.size} canonical contract projections; no authored metadata table.`,
);
