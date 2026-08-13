#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { LANGUAGES, indexTraces, validateCorpus } from "./evaluation-contract.js";

function read(relative) {
  return JSON.parse(readFileSync(new URL(relative, import.meta.url), "utf8"));
}

const corpus = read("./eval/cases.json");
const repairCorpus = read("./eval/repair-cases.json");
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
  `evaluation contracts valid (${corpus.cases.length} selection, ${repairCorpus.cases.length} repair, ` +
    `asked in ${LANGUAGES.join("/")} = ${expectedSelection + expectedRepair} prompts)`,
);
