const TOOL_NAMES = new Set(["compute", "check", "infer_contracts", "word_contract"]);

/**
 * What produced a set of traces — the field that decides what its score means.
 *
 * `referenceFixture` is a hand-written conformance trace: scoring it proves the
 * scorer runs, and its perfect result says nothing whatever about a model.
 * `model` is a real capture. Until this field existed the two were the same
 * shape, so a fixture's `toolSelectionAccuracy: 1` could be read — or reported
 * — as a model result, which is the single claim this evaluation harness is
 * least entitled to make. The scorers now refuse to blur them.
 */
export const TRACE_SOURCES = Object.freeze(["referenceFixture", "model"]);

/**
 * What a `model` trace must record to be re-runnable and comparable.
 *
 * A number without these is not a measurement: comparing PR 2 and PR 3 means
 * comparing the same corpus under the same model, prompt and tool-choice
 * setting, against the same engine. Anything missing here is a comparison that
 * cannot be made honestly later.
 */
const MODEL_PROVENANCE_FIELDS = Object.freeze([
  "modelId",
  "promptTemplateDigest",
  "toolChoice",
  "capturedAt",
  "serverVersion",
  "engineVersion",
  "registryDigest",
]);

function requireProvenance(document, label) {
  const provenance = document?.provenance;
  if (!provenance || typeof provenance !== "object" || Array.isArray(provenance)) {
    throw new Error(`${label} must carry a provenance block naming what produced it`);
  }
  if (!TRACE_SOURCES.includes(provenance.source)) {
    throw new Error(
      `${label} provenance.source must be one of ${TRACE_SOURCES.join(", ")}`,
    );
  }
  if (provenance.source !== "model") return provenance;
  const missing = MODEL_PROVENANCE_FIELDS.filter((field) =>
    typeof provenance[field] !== "string" || provenance[field].length === 0
  );
  if (missing.length) {
    throw new Error(
      `${label} is a model trace and must record ${missing.join(", ")}`,
    );
  }
  return provenance;
}

function requireDocument(document, label, collection) {
  if (document?.schemaVersion !== 1 || !Array.isArray(document?.[collection])) {
    throw new Error(`${label} must have schemaVersion 1 and a ${collection} array`);
  }
}

function requirePointers(expect, label) {
  if (!expect || typeof expect !== "object" || Array.isArray(expect)) {
    throw new Error(`${label}.expect must be an object`);
  }
  for (const pointer of Object.keys(expect)) {
    if (!pointer.startsWith("/")) throw new Error(`${label} has invalid JSON pointer: ${pointer}`);
  }
}

export function validateCorpus(document, { repair = false } = {}) {
  requireDocument(document, "evaluation corpus", "cases");
  const ids = new Set();
  for (const testCase of document.cases) {
    if (typeof testCase?.id !== "string" || testCase.id.length === 0) {
      throw new Error("every evaluation case needs a non-empty id");
    }
    if (ids.has(testCase.id)) throw new Error(`duplicate evaluation case id: ${testCase.id}`);
    ids.add(testCase.id);
    if (typeof testCase.prompt !== "string" || testCase.prompt.length === 0) {
      throw new Error(`${testCase.id} needs a non-empty prompt`);
    }
    if (repair) {
      requirePointers(testCase.firstAttempt?.expect, `${testCase.id}.firstAttempt`);
      requirePointers(testCase.repairedAttempt?.expect, `${testCase.id}.repairedAttempt`);
    } else {
      if (testCase.expectedTool !== null && !TOOL_NAMES.has(testCase.expectedTool)) {
        throw new Error(`${testCase.id} has unknown expectedTool: ${testCase.expectedTool}`);
      }
      if (testCase.expectedTool === null) {
        if (testCase.expect !== null || testCase.arguments !== null) {
          throw new Error(`${testCase.id} negative case must have null arguments and expect`);
        }
      } else {
        if (!testCase.arguments || typeof testCase.arguments !== "object" ||
            Array.isArray(testCase.arguments)) {
          throw new Error(`${testCase.id} needs an arguments object`);
        }
        requirePointers(testCase.expect, testCase.id);
      }
    }
  }
  return ids;
}

function validateAttempt(attempt, label, allowNull) {
  if (allowNull && attempt?.selectedTool === null) return;
  if (!attempt || !TOOL_NAMES.has(attempt.selectedTool) ||
      !attempt.arguments || typeof attempt.arguments !== "object" || Array.isArray(attempt.arguments)) {
    throw new Error(`${label} must select a known tool with an arguments object`);
  }
}

export function indexTraces(document, caseIds, { repair = false } = {}) {
  requireDocument(document, "evaluation traces", "traces");
  requireProvenance(document, "evaluation traces");
  const traces = new Map();
  for (const trace of document.traces) {
    if (!caseIds.has(trace?.caseId)) throw new Error(`trace has unknown caseId: ${trace?.caseId}`);
    if (traces.has(trace.caseId)) throw new Error(`duplicate trace caseId: ${trace.caseId}`);
    if (repair) {
      validateAttempt(trace.firstAttempt, `${trace.caseId}.firstAttempt`, false);
      validateAttempt(trace.repairedAttempt, `${trace.caseId}.repairedAttempt`, false);
    } else {
      validateAttempt(trace, trace.caseId, true);
    }
    traces.set(trace.caseId, trace);
  }
  return traces;
}

/** The provenance of a validated trace document. */
export function traceProvenance(document) {
  return requireProvenance(document, "evaluation traces");
}

/**
 * Whether a perfect score may be *asserted* for this document.
 *
 * `--require-perfect` exists to prove the scorer end-to-end against a fixture
 * built to pass it. Pointing it at a model trace turns a measurement into a
 * pass/fail gate on the model, which converts "we have not measured this" into
 * "this is perfect" the first time a run happens to be clean. A model trace is
 * scored and reported; it is never asserted.
 */
export function mayAssertPerfect(document) {
  return traceProvenance(document).source === "referenceFixture";
}
