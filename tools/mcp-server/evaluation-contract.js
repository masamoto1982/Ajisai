const TOOL_NAMES = new Set(["compute", "check", "infer_contracts", "word_contract"]);

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
