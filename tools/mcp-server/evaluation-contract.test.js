#!/usr/bin/env node
import assert from "node:assert/strict";
import { indexTraces, validateCorpus } from "./evaluation-contract.js";

const validCase = {
  id: "one",
  prompt: "compute",
  expectedTool: "compute",
  arguments: { source: "1" },
  expect: { "/status": "ok" },
};
const corpus = { schemaVersion: 1, cases: [validCase] };
const ids = validateCorpus(corpus);
assert.throws(
  () => validateCorpus({ schemaVersion: 1, cases: [validCase, validCase] }),
  /duplicate evaluation case id/,
);
assert.throws(
  () => validateCorpus({ schemaVersion: 1, cases: [{ ...validCase, expect: { status: "ok" } }] }),
  /invalid JSON pointer/,
);
assert.throws(
  () => indexTraces({ schemaVersion: 1, traces: [{ caseId: "other", selectedTool: null }] }, ids),
  /unknown caseId/,
);
assert.throws(
  () => indexTraces({ schemaVersion: 1, traces: [
    { caseId: "one", selectedTool: "compute", arguments: {} },
    { caseId: "one", selectedTool: "compute", arguments: {} },
  ] }, ids),
  /duplicate trace caseId/,
);
assert.throws(
  () => indexTraces({ schemaVersion: 1, traces: [
    { caseId: "one", selectedTool: "shell", arguments: {} },
  ] }, ids),
  /known tool/,
);
console.log("evaluation contract rejection checks passed");
