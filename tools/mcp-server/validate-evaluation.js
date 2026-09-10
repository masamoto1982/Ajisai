#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { LANGUAGES, indexTraces, validateCorpus } from "./evaluation-contract.js";

function read(relative) {
  return JSON.parse(readFileSync(new URL(relative, import.meta.url), "utf8"));
}

const corpus = read("./eval/cases.json");
const repairCorpus = read("./eval/repair-cases.json");

/**
 * No prompt may show a source character the language does not accept.
 *
 * The `static-check` and `contract-inference` prompts asked, in both locales,
 * about `{ [ 1 ] + } 'INC' DEF` — retired syntax the tokenizer refuses — while
 * their own `arguments.source` carried the correct `[ [ 1 ] + ] 'INC' DEF`.
 * The fixtures therefore passed: the corpus scores the tool call, and nothing
 * read the prompt. But this corpus is the text a model is shown, and what a
 * model is shown is what it imitates. A corpus for an AI-first language is the
 * last place a retired lexeme should survive.
 *
 * The forbidden set is read from the packaged manifest rather than written out
 * here, so retiring another form extends this gate for free.
 *
 * Two deliberate narrowings, each of which a first draft of this gate got
 * wrong:
 *
 * - Retired forms only, not reserved markers. `(` and `)` are reserved in
 *   Ajisai but are ordinary punctuation in both prompt languages, so forbidding
 *   them rejects correct prose.
 * - Only cases that expect an Ajisai tool call. A negative case exists to check
 *   that a question about *another* language does not reach an Ajisai tool, so
 *   `irrelevant-debug` shows a C-style `for` loop on purpose. Code that is not
 *   claiming to be Ajisai is not a lexical claim about Ajisai.
 *
 * Scope is the characters and not the whole fragment for the same reason:
 * prompts legitimately carry English prose like "the vector [1, 2]", which is
 * mathematical notation, so a tokenize-everything rule would reject them.
 */
function retiredSourceCharacters() {
  const manifest = read("./assets/word-manifest.json");
  const entries = manifest.entries ?? manifest.words ?? manifest;
  return (Array.isArray(entries) ? entries : [])
    .filter((entry) => entry.kind === "retired_form")
    .map((entry) => entry.surface)
    .filter((surface) => typeof surface === "string" && surface.length === 1);
}

function assertPromptsAreLexicallyValid(cases, label) {
  const retired = retiredSourceCharacters();
  if (retired.length === 0) {
    throw new Error(
      "no retired surface forms found in the packaged manifest: " +
        "the prompt lexis gate would pass vacuously",
    );
  }
  const findings = [];
  for (const testCase of cases) {
    // A repair case names the tool through its own fixture shape; a selection
    // case that expects no tool call is a negative case, exempt per above.
    if (label === "cases" && testCase.expectedTool === null) continue;
    for (const [language, prompt] of Object.entries(testCase.prompts ?? {})) {
      for (const surface of retired) {
        if (prompt.includes(surface)) {
          findings.push(
            `${label}/${testCase.id}/${language}: prompt shows '${surface}', a retired ` +
              "form the tokenizer refuses — a model shown it will imitate it",
          );
        }
      }
    }
  }
  if (findings.length > 0) {
    throw new Error(
      `evaluation prompts must not show invalid Ajisai source:\n  ${findings.join("\n  ")}`,
    );
  }
}

assertPromptsAreLexicallyValid(corpus.cases, "cases");
assertPromptsAreLexicallyValid(repairCorpus.cases, "repair-cases");
const traces = indexTraces(read("./eval/reference-traces.json"), validateCorpus(corpus));
const repairTraces = indexTraces(
  read("./eval/reference-repair-traces.json"),
  validateCorpus(repairCorpus, { repair: true }),
  { repair: true },
);
// Once per case *per language*: a fixture missing one half of a pair scores as
// a language gap rather than as the incomplete fixture it is, which is the one
// way this document could report a finding it never measured.
const expectedSelection = corpus.cases.length * LANGUAGES.length;
const expectedRepair = repairCorpus.cases.length * LANGUAGES.length;
if (traces.size !== expectedSelection || repairTraces.size !== expectedRepair) {
  throw new Error(
    "committed reference traces must cover every evaluation case once per language: " +
      `expected ${expectedSelection} selection and ${expectedRepair} repair, ` +
      `got ${traces.size} and ${repairTraces.size}`,
  );
}
console.log(
  `evaluation prompt lexis valid (no retired source characters shown to a model)`,
);
console.log(
  `evaluation contracts valid (${corpus.cases.length} selection, ${repairCorpus.cases.length} repair, ` +
    `asked in ${LANGUAGES.join("/")} = ${expectedSelection + expectedRepair} prompts)`,
);
