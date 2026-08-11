#!/usr/bin/env node
import assert from "node:assert/strict";
import { indexTraces, mayAssertPerfect, validateCorpus } from "./evaluation-contract.js";

const validCase = {
  id: "one",
  prompt: "compute",
  expectedTool: "compute",
  arguments: { source: "1" },
  expect: { "/status": "ok" },
};
const corpus = { schemaVersion: 1, cases: [validCase] };
// Every trace document now declares what produced it. `fixture` is the shape a
// hand-written conformance trace takes; `modelDoc` is a real capture.
const fixture = (traces) => ({
  schemaVersion: 1,
  provenance: { source: "referenceFixture" },
  traces,
});
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
  () => indexTraces(fixture([{ caseId: "other", selectedTool: null }]), ids),
  /unknown caseId/,
);
assert.throws(
  () => indexTraces(fixture([
    { caseId: "one", selectedTool: "compute", arguments: {} },
    { caseId: "one", selectedTool: "compute", arguments: {} },
  ]), ids),
  /duplicate trace caseId/,
);
assert.throws(
  () => indexTraces(fixture([
    { caseId: "one", selectedTool: "shell", arguments: {} },
  ]), ids),
  /known tool/,
);
// A trace document that does not say what produced it cannot be scored: the
// same numbers mean "the scorer works" or "the model performs this well"
// depending on an answer the file would not be carrying.
assert.throws(
  () => indexTraces({ schemaVersion: 1, traces: [] }, ids),
  /provenance block/,
);
assert.throws(
  () => indexTraces({ schemaVersion: 1, provenance: { source: "vibes" }, traces: [] }, ids),
  /provenance.source must be one of/,
);
// A model trace without the metadata that makes it re-runnable is not a
// measurement — a number nobody can reproduce or compare a later run against.
assert.throws(
  () => indexTraces({ schemaVersion: 1, provenance: { source: "model" }, traces: [] }, ids),
  /must record modelId, promptTemplateDigest, toolChoice, capturedAt/,
);
const modelDoc = {
  schemaVersion: 1,
  provenance: {
    source: "model",
    modelId: "claude-opus-5",
    promptTemplateDigest: "abc123",
    toolChoice: "auto",
    capturedAt: "2026-08-11T00:00:00Z",
    serverVersion: "0.3.0",
    engineVersion: "0.2.0-beta.1",
    registryDigest: "a67241e0",
  },
  traces: [],
};
assert.deepEqual(indexTraces(modelDoc, ids).size, 0, "a complete model trace validates");
// The asymmetry that keeps a lucky run from becoming a committed claim.
assert.equal(mayAssertPerfect(fixture([])), true);
assert.equal(mayAssertPerfect(modelDoc), false);
console.log("evaluation contract rejection checks passed");
