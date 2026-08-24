#!/usr/bin/env node
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import Ajv2020 from "ajv/dist/2020.js";
import {
  CAPACITY_WAIT_MS,
  createBackend,
  createServer,
  ExecutionGate,
  LIMITS,
  serverVersion,
} from "./index.js";
import { runCli } from "./doctor.js";
import { coveredLimitNames, limitCases } from "./golden/limit-cases.js";
import { HOST_ERRORS } from "./host-error.js";
import { SKILL_BOUNDARY } from "./sync-assets.js";
import { readFileSync } from "node:fs";

let failures = 0;
function check(label, condition) {
  console.log(`${condition ? "PASS" : "FAIL"}  ${label}`);
  if (!condition) failures += 1;
}

function canonicalJson(value) {
  if (Array.isArray(value)) return value.map(canonicalJson);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, child]) => [key, canonicalJson(child)]),
    );
  }
  return value;
}

function atPointer(document, pointer) {
  return pointer
    .split("/")
    .slice(1)
    .map((part) => part.replaceAll("~1", "/").replaceAll("~0", "~"))
    .reduce((value, part) => value?.[part], document);
}

const gate = new ExecutionGate(2);
check("execution gate admits up to its capacity", gate.tryAcquire() && gate.tryAcquire());
check("execution gate rejects excess concurrent work", gate.tryAcquire() === false);
gate.release();
check("execution gate restores capacity on release", gate.tryAcquire());
gate.release();
gate.release();

// Back-pressure: a saturated gate holds a caller briefly rather than turning
// every burst into a retry loop, and a released slot goes to whoever has been
// waiting longest.
const queued = new ExecutionGate(1);
queued.tryAcquire();
const waiting = queued.acquire(1_000);
queued.release();
check("execution gate hands a released slot to a queued caller", (await waiting) === true);
queued.release();
const timedOut = new ExecutionGate(1);
timedOut.tryAcquire();
check("execution gate gives up after its wait window", (await timedOut.acquire(20)) === false);
timedOut.release();

const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
const server = createServer();
const client = new Client({ name: "selftest", version: "0.0.0" });
await Promise.all([server.connect(serverTransport), client.connect(clientTransport)]);

const { tools } = await client.listTools();
check(
  "exposes the four focused agent tools",
  JSON.stringify(tools.map(({ name }) => name).sort()) ===
    JSON.stringify(["check", "compute", "infer_contracts", "word_contract"]),
);
check(
  "every tool rejects undeclared input",
  tools.every(({ inputSchema }) => inputSchema.additionalProperties === false),
);
check(
  "every tool advertises the same structured output contract",
  tools.every(({ outputSchema }) =>
    outputSchema?.$id === "ajisai://schema/result" &&
    outputSchema?.required?.includes("status")
  ),
);
check(
  "the source input schema states the unit its limit is measured in",
  tools
    .filter(({ name }) => name !== "word_contract")
    .every(({ inputSchema }) =>
      inputSchema.properties.source.maxLength === LIMITS.sourceBytes &&
      inputSchema.properties.source.description.includes("UTF-8 bytes")
    ),
);
check(
  "tools advertise safe selection hints",
  tools.every(({ annotations }) =>
    annotations?.readOnlyHint === true &&
    annotations?.destructiveHint === false &&
    annotations?.idempotentHint === true &&
    annotations?.openWorldHint === false
  ),
);

const contract = await client.callTool({ name: "word_contract", arguments: { word: "map" } });
check(
  "word_contract returns the complete canonical contract",
  contract.structuredContent?.matches?.some((entry) =>
    entry.name === "MAP" &&
    entry.purity === "conditional" &&
    entry.stack?.inputs === 2
  ),
);
const missWithSuggestion = await client.callTool({
  name: "word_contract",
  arguments: { word: "LENGHT" },
});
check(
  "word_contract answers an unmatched name with its closest known Words",
  missWithSuggestion.structuredContent?.matches?.length === 0 &&
    missWithSuggestion.structuredContent?.suggestions?.[0] === "LENGTH",
);
check(
  "word_contract carries the same provenance as an execution result",
  contract.structuredContent?.mcp?.registryDigest?.length === 64 &&
    contract.structuredContent?.mcp?.serverVersion === serverVersion() &&
    contract.structuredContent?.suggestions?.length === 0,
);
// A Word's cost is what lets a caller budget a phrase before running it, so it
// has to be on the entry the caller already reads — not derivable only by
// executing something.
check(
  "word_contract publishes what the Word charges, per axis",
  contract.structuredContent?.matches?.[0]?.cost?.steps?.class === "unbounded" &&
    contract.structuredContent?.matches?.[0]?.cost?.numeric?.class === "unbounded" &&
    contract.structuredContent?.matches?.[0]?.cost?.collection?.class === "unbounded",
);
const addContract = await client.callTool({ name: "word_contract", arguments: { word: "+" } });
// `MAP` is unbounded on every axis, so it cannot show that the axes are read
// independently; `ADD` is the case where they differ, and its `numeric` bound
// is the one `runtime_limits.rs` prices limb×limb and therefore attains.
check(
  "a Word's cost axes are published independently, with their exactness",
  (() => {
    const cost = addContract.structuredContent?.matches?.[0]?.cost;
    return cost?.steps?.class === "const" && cost.steps.exact === true &&
      cost.numeric?.class === "linear" && cost.numeric.exact === true &&
      cost.collection?.class === "const" && cost.collection.exact === true;
  })(),
);

const resources = await client.listResources();
check(
  "publishes guide, vocabulary, contracts, result schema and host profile as resources",
  JSON.stringify(resources.resources.map(({ uri }) => uri).sort()) ===
    JSON.stringify([
      "ajisai://contracts",
      "ajisai://guide/quickstart",
      "ajisai://limits",
      "ajisai://schema/result",
      "ajisai://vocabulary",
    ]),
);
// Bounding a phrase means joining the bounds of every Word in it, which 65
// separate `word_contract` probes cannot practically deliver. The bulk read is
// what makes the published cost usable, so it is checked as such: every entry
// carries a complete, well-formed cost.
const contractsResource = JSON.parse(
  (await client.readResource({ uri: "ajisai://contracts" })).contents[0]?.text ?? "{}",
);
const COST_CLASSES = ["const", "linear", "superlinear", "unbounded"];
check(
  "the contracts resource serves every Word's cost in one read",
  Array.isArray(contractsResource.entries) &&
    contractsResource.entries.length > 0 &&
    contractsResource.entries.every((entry) =>
      ["steps", "numeric", "collection"].every((axis) =>
        COST_CLASSES.includes(entry.cost?.[axis]?.class) &&
        typeof entry.cost?.[axis]?.exact === "boolean"
      )
    ),
);
check(
  "the contracts resource and word_contract answer with the same entry",
  JSON.stringify(contractsResource.entries.find((entry) => entry.name === "MAP")) ===
    JSON.stringify(contract.structuredContent?.matches?.[0]),
);
// The property that makes a published bound worth reading: a caller that joins
// the atoms' classes gets the same answer the engine infers for the phrase.
// `[ 1 ] +` is `ADD` against a literal, so `numeric` stays `linear` while every
// other axis stays at the lattice bottom.
const composed = await client.callTool({
  name: "infer_contracts",
  arguments: { source: "{ [ 1 ] + } 'BUDGETED' DEF" },
});
check(
  "an inferred phrase bound agrees with the join of its Words' published bounds",
  (() => {
    const add = contractsResource.entries.find((entry) => entry.name === "ADD")?.cost;
    const inferred = composed.structuredContent?.contracts?.find(
      (entry) => entry.name === "BUDGETED",
    )?.cost;
    return add && inferred &&
      ["steps", "numeric", "collection"].every((axis) => inferred[axis] === add[axis].class);
  })(),
);
const limitsResource = JSON.parse(
  (await client.readResource({ uri: "ajisai://limits" })).contents[0]?.text ?? "{}",
);
check(
  "the host profile resource serves exactly the limits tool results report",
  limitsResource.profile === "mcp-local-stdio" &&
    JSON.stringify(limitsResource.limits) === JSON.stringify(LIMITS),
);
// The adapter and the engine are separately versioned, and a stored result
// naming only the engine cannot say which adapter produced it.
check(
  "the host profile resource names the adapter version as well as the engine",
  limitsResource.serverVersion === serverVersion() &&
    limitsResource.engineVersion !== undefined &&
    typeof serverVersion() === "string",
);
const guide = await client.readResource({ uri: "ajisai://guide/quickstart" });
const guideText = guide.contents[0]?.text ?? "";
check("quickstart resource reads generated guidance", guideText.includes("Agent Writing Protocol"));

// The served guide leads with the MCP interface, not with the CLI run loop the
// generated protocol opens on. A connected client cannot issue `ajisai run`,
// and a model that met that first learned the language before learning which
// of the four tools to call.
const [preface] = guideText.split(SKILL_BOUNDARY);
check(
  "the quickstart opens with MCP tool selection, not the CLI run loop",
  preface.length > 0 && preface.length < guideText.length &&
    ["compute", "check", "infer_contracts", "word_contract"].every((tool) =>
      preface.includes(`\`${tool}\``)
    ) &&
    !preface.includes("ajisai run"),
);
check(
  "the quickstart distinguishes every outcome class a caller must branch on",
  ["`ok`", "`error`", "`hostError`", "NIL", "exactTerms"].every((token) =>
    preface.includes(token)
  ),
);
// The short rendering only helps if the section teaching algebraic values
// reaches it before the two renderings that mislead, so the order is part of
// the fix rather than a stylistic choice. Scoped to that section: `§2` names
// `stackDisplay` first for good reason, and that is not what this is about.
const algebraicSection = preface
  .split(/^## /m)
  .find((section) => section.includes("exactDisplay")) ?? "";
check(
  "the quickstart reaches the short algebraic rendering before the misleading ones",
  algebraicSection.indexOf("exactDisplay") <= algebraicSection.indexOf("exactTerms") &&
    algebraicSection.indexOf("exactDisplay") < algebraicSection.indexOf("stackDisplay") &&
    algebraicSection.includes("approximate"),
);

// Every example in the hand-written preface runs against the live backend. The
// generated half is verified by its generator against the real interpreter;
// this is the same guarantee for the half a human wrote, so guidance the model
// is told to trust cannot quietly stop being true.
const prefaceExamples = [...preface.matchAll(/^```ajisai([^\n]*)\n([\s\S]*?)^```$/gm)].map(
  ([, attributes, source]) => ({
    source: source.trim(),
    expect: Object.fromEntries(
      [...attributes.matchAll(/(\w+)=("[^"]*"|\S+)/g)].map(([, key, raw]) => [
        key,
        raw.startsWith('"') ? raw.slice(1, -1) : raw,
      ]),
    ),
  }),
);
check("the quickstart preface carries executable examples", prefaceExamples.length >= 5);
for (const { source, expect } of prefaceExamples) {
  const observed = await client.callTool({
    name: expect.tool ?? "compute",
    arguments: { source },
  });
  const structured = observed.structuredContent;
  const statusAgrees = structured?.status === (expect.status ?? "ok");
  const stackAgrees =
    expect.stack === undefined || (structured?.stackDisplay ?? []).join(" ") === expect.stack;
  check(`quickstart example: ${source.replaceAll("\n", " ")}`, statusAgrees && stackAgrees);
  if (!statusAgrees || !stackAgrees) {
    console.error(`  got status=${structured?.status} stack=${JSON.stringify(structured?.stackDisplay)}`);
  }
}
const schemaResource = await client.readResource({ uri: "ajisai://schema/result" });
const resultSchema = JSON.parse(schemaResource.contents[0]?.text ?? "{}");
const validateResult = new Ajv2020().compile(resultSchema);
check(
  "result resource publishes the exact algebraic wire schema",
  resultSchema.$id === "ajisai://schema/result" &&
    resultSchema.$defs?.exactTerm?.required?.includes("radicand"),
);
check(
  "tool output and result resource use the same schema",
  JSON.stringify(canonicalJson(tools.find(({ name }) => name === "compute")?.outputSchema)) ===
    JSON.stringify(canonicalJson(resultSchema)),
);
const templates = await client.listResourceTemplates();
check(
  "publishes canonical Word contracts as a resource template",
  templates.resourceTemplates.some(({ uriTemplate }) => uriTemplate === "ajisai://words/{name}"),
);
const mapResource = await client.readResource({ uri: "ajisai://words/MAP" });
const mapContract = JSON.parse(mapResource.contents[0]?.text ?? "{}");
check(
  "Word resource resolves the canonical contract",
  mapContract.matches?.[0]?.name === "MAP" && mapContract.matches[0]?.nilPolicy,
);

const golden = JSON.parse(
  readFileSync(new URL("./golden/cases.json", import.meta.url), "utf8"),
);
for (const goldenCase of golden.cases) {
  const observed = await client.callTool({
    name: "compute",
    arguments: { source: goldenCase.source },
  });
  const mismatches = Object.entries(goldenCase.expect).filter(
    ([pointer, expected]) =>
      JSON.stringify(atPointer(observed.structuredContent, pointer)) !==
      JSON.stringify(expected),
  );
  // A field that must *not* be there. `expect` cannot say this: a missing
  // pointer and a pointer holding `null` both stringify to the same thing, so
  // "absent" and "present and null" were indistinguishable. `exactDisplay` and
  // `exactTerms` are meaningless on a rational — a short algebraic rendering
  // of a number that has no radical would be a field inviting a reader to
  // wonder what it means — and this is what pins their absence.
  const unexpected = (goldenCase.expectAbsent ?? []).filter(
    (pointer) => atPointer(observed.structuredContent, pointer) !== undefined,
  );
  check(
    `golden: ${goldenCase.name}`,
    observed.isError !== true && mismatches.length === 0 && unexpected.length === 0,
  );
  if (mismatches.length || unexpected.length) {
    console.error(`  mismatched=${JSON.stringify(mismatches)} unexpectedly present=${JSON.stringify(unexpected)}`);
  }
}

// Every declared limit must be accounted for by name. A limit an agent plans
// against and a limit the regression suite pins have to be the same set —
// otherwise the server can add a ceiling nobody ever exercises, which is how
// `responseBytes` came to be declared for two backends and enforced by one.
check(
  "every declared limit has a coverage entry, and every entry a declared limit",
  JSON.stringify(coveredLimitNames()) === JSON.stringify(Object.keys(LIMITS).sort()),
);
for (const limit of limitCases()) {
  if (limit.coverage === "boundary") {
    check(
      `limit ${limit.name}: declared value matches the served profile`,
      limit.declared === LIMITS[limit.name],
    );
    for (const probe of limit.probes) {
      const observed = await client.callTool({
        name: "compute",
        arguments: { source: probe.source },
      });
      const mismatches = Object.entries(probe.expect).filter(
        ([pointer, expected]) =>
          JSON.stringify(atPointer(observed.structuredContent, pointer)) !==
          JSON.stringify(expected),
      );
      check(`limit ${limit.name} (${probe.edge})`, mismatches.length === 0);
      if (mismatches.length) console.error(`  ${JSON.stringify(mismatches)}`);
    }
  } else {
    check(
      `limit ${limit.name}: declared value matches the served profile`,
      limit.declared === LIMITS[limit.name],
    );
    check(
      `limit ${limit.name}: non-boundary coverage explains itself`,
      typeof limit.note === "string" && limit.note.length > 0 &&
        (limit.coverage !== "injectedLimit" || typeof limit.rustTest === "string"),
    );
  }
}

// `wallTimeMs` is a deadline the adapter holds around execution, not a budget
// the engine spends, and since the collection meter landed no source program
// reaches it — the named ceilings answer first, which is the outcome they exist
// for. So it is exercised the way the other host gate is: through the server's
// own machinery, with the deadline moved to where a real program misses it. A
// ceiling that cannot be observed at all is a claim, whichever kind it is.
const impatient = createBackend({ wallTimeMs: 1 });
if (impatient) {
  let timedOut = null;
  try {
    await impatient.compute("[ 0 99999 ] RANGE SORT LENGTH");
  } catch (error) {
    timedOut = error;
  }
  check(
    "limit wallTimeMs (hostGate): the adapter deadline answers `timeout`",
    timedOut?.code === "timeout" && timedOut?.retryable === true,
  );
} else {
  check("limit wallTimeMs (hostGate): a backend is available to exercise it", false);
}

// The two halves of a result, and the padding between them.
//
// MCP asks a tool with an output schema to also return the serialized JSON in
// a text block, so a text-only client receives the whole result rather than a
// summary of it. That makes the text a *mirror*: if it ever stops being the
// same object, the two kinds of client stop seeing the same answer. These pin
// the mirror, and pin the padding that used to cost a third of it.
const mirrored = await client.callTool({ name: "compute", arguments: { source: "1 3 /" } });
check(
  "the text block is the structured result, not a summary of it",
  JSON.stringify(JSON.parse(mirrored.content?.[0]?.text ?? "null")) ===
    JSON.stringify(mirrored.structuredContent),
);
check(
  "the text block carries no indentation padding",
  typeof mirrored.content?.[0]?.text === "string" &&
    !/\n\s/.test(mirrored.content[0].text),
);
// An optional field with nothing in it is absent, not `null`. A plain success
// used to advertise `diagnosis`, `aiDiagnostic`, `message` and
// `contractDecls`, all empty — four diagnostic-sounding fields pointing at
// nothing.
check(
  "a successful result carries no null-valued fields",
  Object.values(mirrored.structuredContent ?? {}).every((field) => field !== null) &&
    !("diagnosis" in (mirrored.structuredContent ?? {})),
);
// Compaction must not cost a text-only client the outcome distinction, which
// is the whole reason the text is the serialized result rather than prose.
for (const [label, call] of [
  ["a value", { name: "compute", arguments: { source: "1 3 /" } }],
  ["a reason-carrying NIL", { name: "compute", arguments: { source: "1 0 /" } }],
  ["a language error", { name: "compute", arguments: { source: "FROBNICATE" } }],
  ["a host failure", { name: "compute", arguments: { source: "" } }],
]) {
  const observed = await client.callTool(call);
  const text = JSON.parse(observed.content?.[0]?.text ?? "null");
  check(
    `a text-only client can still identify ${label}`,
    typeof text?.status === "string" &&
      text.status === observed.structuredContent?.status &&
      (text.status !== "hostError" || typeof text.error?.code === "string") &&
      (text.status !== "error" || typeof text.diagnosis?.why === "string"),
  );
}

const compute = await client.callTool({
  name: "compute",
  arguments: { source: "[ 2 ] SQRT" },
});
const oversized = await client.callTool({
  name: "compute",
  arguments: { source: " ".repeat(LIMITS.sourceBytes + 1) },
});
check("compute rejects source beyond its UTF-8 byte limit", oversized.isError === true);
// Host failures are structured, so a caller branches on a code rather than on
// an English sentence — this file used to match on "CLI not found" itself.
check(
  "a host failure is machine-readable and schema-valid",
  oversized.structuredContent?.status === "hostError" &&
    oversized.structuredContent?.error?.code === "sourceTooLarge" &&
    oversized.structuredContent?.error?.retryable === false &&
    validateResult(oversized.structuredContent),
);
check(
  "a host failure message carries no host paths or environment names",
  !/AJISAI_(BIN|REPO)|\/(home|usr|tmp)\//.test(oversized.structuredContent?.error?.message ?? ""),
);
const badRequest = await client.callTool({ name: "compute", arguments: { source: "" } });
check(
  "an invalid request is a host error, not a language error",
  badRequest.structuredContent?.error?.code === "invalidRequest",
);
check(
  "every declared host-error code is retryable or not, explicitly",
  Object.values(HOST_ERRORS).every(({ retryable }) => typeof retryable === "boolean") &&
    HOST_ERRORS.capacityExhausted.retryable === true &&
    HOST_ERRORS.timeout.retryable === true &&
    CAPACITY_WAIT_MS > 0,
);
if (compute.structuredContent?.error?.code === "backendUnavailable") {
  check("compute requires a real Ajisai backend", false);
} else {
  const sqrt = compute.structuredContent?.stack?.[0]?.value?.[0];
  const [exactTerm] = sqrt?.semantics?.exactTerms ?? [];
  check(
    "compute preserves the exact algebraic normal form",
    exactTerm?.numerator === "1" &&
      exactTerm?.denominator === "1" &&
      exactTerm?.radicand === "2",
  );
  // The same normal form written short. Everything else on this result that
  // looks like the value is not: `stackDisplay` is a continued fraction cut
  // off at a display budget, and `value` is a rational approximation.
  check(
    "compute writes the algebraic value short beside the terms it renders",
    sqrt?.semantics?.exactDisplay === "sqrt(2)" &&
      compute.structuredContent?.stackDisplay?.[0]?.includes("...)") === true,
  );
  check(
    "compute reports engine provenance and applied limits",
    compute.structuredContent?.mcp?.serverVersion === serverVersion() &&
      compute.structuredContent?.mcp?.engineVersion === "0.2.0-beta.1" &&
      compute.structuredContent?.mcp?.limits?.wallTimeMs === 5000 &&
      compute.structuredContent?.mcp?.limits?.materializedElements === 100000 &&
      compute.structuredContent?.mcp?.limits?.bigintBits === 262144 &&
      compute.structuredContent?.mcp?.limits?.algebraicTerms === 512,
  );
  check(
    "compute names which backend answered",
    ["nativeCli", "wasmWorker"].includes(compute.structuredContent?.mcp?.backend?.kind),
  );
  const second = await client.callTool({ name: "compute", arguments: { source: "[ 2 ] SQRT" } });
  check(
    "the backend is fixed for the life of the server",
    second.structuredContent?.mcp?.backend?.kind ===
      compute.structuredContent?.mcp?.backend?.kind,
  );
  check("compute satisfies the published result schema", validateResult(compute.structuredContent));

  const languageError = await client.callTool({
    name: "compute",
    arguments: { source: "FROBNICATE" },
  });
  check(
    "Ajisai language errors remain structured non-MCP errors",
    languageError.isError !== true && languageError.structuredContent?.status === "error",
  );
  // The stable half of a next-check is its code; the display text is free to
  // be reworded or translated without breaking a consumer or a repair scorer.
  const [firstCheck] = languageError.structuredContent?.diagnosis?.nextChecks ?? [];
  check(
    "diagnostic checks carry a stable code and per-locale display text",
    firstCheck?.code === "checkSpelling" &&
      typeof firstCheck?.title?.en === "string" &&
      typeof firstCheck?.title?.ja === "string" &&
      typeof firstCheck?.detail?.en === "string" &&
      typeof firstCheck?.detail?.ja === "string" &&
      !/[぀-ヿ一-鿿]/.test(firstCheck.detail.en),
  );
  const misspelled = await client.callTool({
    name: "compute",
    arguments: { source: "[ 1 2 3 ] LENGHT" },
  });
  check(
    "an unknown Word diagnosis names its likely correction",
    misspelled.structuredContent?.diagnosis?.candidates?.[0] === "LENGTH" &&
      validateResult(misspelled.structuredContent),
  );

  // A ceiling an agent plans against has to report the number it is judged by.
  // `runtimeMetrics.executionSteps` was read from a struct field nothing ever
  // wrote, so every program that ever ran reported 0 steps — beside the counter
  // every limit check increments.
  const spent = await client.callTool({
    name: "compute",
    arguments: { source: "[ 1 20 ] RANGE 1 { * } FOLD" },
  });
  const usage = spent.structuredContent?.resourceUsage;
  check(
    "a run reports the resources it actually spent",
    usage?.executionSteps === 22 &&
      usage?.numericWork === 20 &&
      // 20 elements materialized by `RANGE`, at 17 units each: the collection
      // meter sees the vector the fold walks, which no ceiling counted before.
      usage?.collectionWork === 340 &&
      validateResult(spent.structuredContent),
  );
  check(
    "every reported resource names a declared limit",
    Object.keys(usage ?? {}).every((key) => key in LIMITS),
  );
  check(
    "the compatibility alias agrees with the resource it mirrors",
    spent.structuredContent?.runtimeMetrics?.executionSteps === usage?.executionSteps,
  );

  // An error report's answer is its diagnosis; the stack is residual state.
  // `[ 0 99999 ] RANGE LENGHT` is a one-character typo holding a
  // 100,000-element vector, which serialized in full is ~27 MB — so before the
  // stack was elided the whole result became `responseTooLarge` and the reader
  // was told its answer was too big rather than that it had misspelled
  // `LENGTH`. The residue is what gives way, never the reason.
  const hugeResidue = await client.callTool({
    name: "compute",
    arguments: { source: "[ 0 99999 ] RANGE LENGHT" },
  });
  const hugeResidueBytes = Buffer.byteLength(
    JSON.stringify(hugeResidue.structuredContent ?? {}),
    "utf8",
  );
  check(
    "a diagnosis survives a failing stack too large to send",
    hugeResidue.isError !== true &&
      hugeResidue.structuredContent?.status === "error" &&
      hugeResidue.structuredContent?.diagnosis?.candidates?.[0] === "LENGTH" &&
      hugeResidueBytes < LIMITS.responseBytes &&
      validateResult(hugeResidue.structuredContent),
  );
  check(
    "an elided slot keeps its position and says what it dropped",
    hugeResidue.structuredContent?.stackElided?.reason === "errorStackBudget" &&
      hugeResidue.structuredContent?.stackElided?.slots?.[0]?.index === 0 &&
      hugeResidue.structuredContent?.stackElided?.slots?.[0]?.elements === 100000 &&
      hugeResidue.structuredContent?.stack?.[0]?.type === "vector" &&
      hugeResidue.structuredContent?.stack?.[0]?.value === null &&
      hugeResidue.structuredContent?.stack?.length ===
        hugeResidue.structuredContent?.stackDisplay?.length,
  );
  check(
    "an ordinary error is not elided at all",
    misspelled.structuredContent?.stackElided === undefined &&
      !("elided" in (misspelled.structuredContent?.stack?.[0] ?? {})),
  );

  const checked = await client.callTool({
    name: "check",
    arguments: { source: "{ [ 1 ] + } 'INC' DEF" },
  });
  check("check is execution-free and structured", checked.structuredContent?.status === "ok");

  const inferred = await client.callTool({
    name: "infer_contracts",
    arguments: { source: "{ [ 1 ] + } 'INC' DEF" },
  });
  check(
    "infer_contracts returns the user Word contract",
    inferred.structuredContent?.contracts?.some((entry) => entry.name === "INC"),
  );
  check(
    "infer_contracts satisfies the published result schema",
    validateResult(inferred.structuredContent),
  );
}

// The terminal command line. A stdio server is silent whether it is healthy or
// wedged, so `--doctor` is the only thing that tells an operator which; and it
// has to answer through an exit code, because that is what a script reads.
async function cli(...argv) {
  const lines = [];
  const code = await runCli(argv, (line) => lines.push(line));
  return { code, text: lines.join("\n") };
}
const version = await cli("--version");
check(
  "--version names the adapter, the engine and the registry",
  version.code === 0 &&
    version.text.includes(`ajisai-mcp-server ${serverVersion()}`) &&
    /engine \d/.test(version.text) &&
    /registry [a-f0-9]{16}/.test(version.text),
);
const help = await cli("--help");
check(
  "--help documents every command the binary accepts",
  help.code === 0 && ["--doctor", "--version", "--help"].every((flag) => help.text.includes(flag)),
);
const doctorRun = await cli("--doctor");
check(
  "--doctor exercises the real backend and exits 0 when healthy",
  doctorRun.code === 0 &&
    doctorRun.text.includes("checks passed") &&
    !doctorRun.text.includes("FAIL"),
);
check(
  "--doctor proves exactness rather than only reporting that it started",
  doctorRun.text.includes("2 3 / 1 3 / + = 1/1") && doctorRun.text.includes("exactTerms"),
);
const unknownFlag = await cli("--frobnicate");
check(
  "an unrecognized option is named and exits non-zero rather than serving",
  unknownFlag.code === 2 && unknownFlag.text.includes("--frobnicate"),
);

await client.close();
await server.close();
if (failures) process.exit(1);
console.log("all checks passed");
