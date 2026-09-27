#!/usr/bin/env node
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, "..", "..");
const assetsDir = join(here, "assets");
const sources = [
  [join(repoRoot, "spec", "words.json"), join(assetsDir, "words.json")],
  [join(repoRoot, "docs", "word-manifest.json"), join(assetsDir, "word-manifest.json")],
];
const rootPackage = JSON.parse(readFileSync(join(repoRoot, "package.json"), "utf8"));
const words = readFileSync(sources[0][0]);
const metadata = `${JSON.stringify({
  schemaVersion: 1,
  engineVersion: rootPackage.version,
  registryDigest: createHash("sha256").update(words).digest("hex"),
}, null, 2)}\n`;

/**
 * The served quickstart is an MCP preface followed by the generated writing
 * protocol, not a copy of `SKILL.md`.
 *
 * `SKILL.md` opens with a CLI run loop (`ajisai agent compute file`) — commands a
 * connected MCP client cannot issue and has no reason to read about — and says
 * nothing about which of the four tools to call. A model that read it first
 * learned the language before learning the interface. The preface answers the
 * interface question in one screen and hands off; the generated half stays
 * verbatim, so its examples remain the ones the generator verified.
 *
 * `SKILL_BOUNDARY` is what the self-test slices on to run the preface's own
 * examples against the live backend, and what a reader sees as the seam.
 */
export const SKILL_BOUNDARY = "<!-- BEGIN GENERATED SKILL.md -->\n";
const quickstart = Buffer.concat([
  readFileSync(join(here, "mcp-quickstart.md")),
  Buffer.from(`${SKILL_BOUNDARY}\n`),
  readFileSync(join(repoRoot, "SKILL.md")),
]);

/**
 * The MCP result schema's value node is spec/host-protocol.schema.json's, not
 * a second description of it.
 *
 * `result.schema.json` used to describe the stack node by hand, loosely
 * (`additionalProperties: true`, an untyped `value`), beside a canonical
 * schema that nothing validated. The two drifted — the canonical one never
 * declared `elided` or `absence.detail`, both of which the server emitted —
 * and neither caught it. The canonical definitions are copied in here under
 * the names the result schema uses, so the self-test's validation of every
 * live result is also a validation of the protocol, and `--check` fails the
 * moment the two differ.
 */
const protocolDefNames = {
  observedValue: "protocolNode",
  semantics: "protocolSemantics",
  absence: "protocolAbsence",
  exactTerm: "exactTerm",
  elided: "protocolElided",
  integerString: "integerString",
  positiveIntegerString: "positiveIntegerString",
};
const resultSchemaPath = join(here, "result.schema.json");
function resultSchemaWithProtocol() {
  const protocol = JSON.parse(readFileSync(join(repoRoot, "spec", "host-protocol.schema.json"), "utf8"));
  const result = JSON.parse(readFileSync(resultSchemaPath, "utf8"));
  const renamed = JSON.parse(
    JSON.stringify(protocol.$defs).replace(/"#\/\$defs\/([A-Za-z]+)"/g, (whole, name) => {
      if (!(name in protocolDefNames)) throw new Error(`host protocol defines no ${name}`);
      return `"#/$defs/${protocolDefNames[name]}"`;
    }),
  );
  for (const [name, target] of Object.entries(protocolDefNames)) {
    result.$defs[target] = renamed[name];
  }
  return Buffer.from(`${JSON.stringify(result, null, 1)}\n`);
}

const outputs = [...sources.map(([source, target]) => [readFileSync(source), target]),
  [resultSchemaWithProtocol(), resultSchemaPath],
  [quickstart, join(assetsDir, "quickstart.md")],
  [Buffer.from(metadata), join(assetsDir, "metadata.json")]];
// Generating or checking is the entry-point behaviour; importing this module
// for `SKILL_BOUNDARY` — which the self-test does, so the seam it slices on is
// the one that was written — must not rewrite the working tree as a side
// effect of the import.
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (process.argv.includes("--check")) {
    const stale = outputs.filter(([content, target]) =>
      !existsSync(target) || !content.equals(readFileSync(target)));
    if (stale.length) {
      console.error(`MCP packaged assets are stale: ${stale.map(([, path]) => path).join(", ")}`);
      process.exit(1);
    }
    // npm includes prepack stdout before `npm pack --json`, which would corrupt
    // machine-readable pack output consumed by the smoke test.
    if (process.env.npm_lifecycle_event !== "prepack") {
      console.log("MCP packaged assets are current");
    }
  } else {
    mkdirSync(assetsDir, { recursive: true });
    for (const [content, target] of outputs) writeFileSync(target, content);
    console.log("updated MCP packaged assets");
  }
}
