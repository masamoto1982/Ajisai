#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { indexTraces, validateCorpus } from "./evaluation-contract.js";

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
if (traces.size !== corpus.cases.length || repairTraces.size !== repairCorpus.cases.length) {
  throw new Error("committed reference traces must cover every evaluation case exactly once");
}
console.log(`evaluation contracts valid (${corpus.cases.length} selection, ${repairCorpus.cases.length} repair)`);
