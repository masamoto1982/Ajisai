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
  [join(repoRoot, "SKILL.md"), join(assetsDir, "quickstart.md")],
];
const rootPackage = JSON.parse(readFileSync(join(repoRoot, "package.json"), "utf8"));
const words = readFileSync(sources[0][0]);
const metadata = `${JSON.stringify({
  schemaVersion: 1,
  engineVersion: rootPackage.version,
  registryDigest: createHash("sha256").update(words).digest("hex"),
}, null, 2)}\n`;
const outputs = [...sources.map(([source, target]) => [readFileSync(source), target]),
  [Buffer.from(metadata), join(assetsDir, "metadata.json")]];
const check = process.argv.includes("--check");

if (check) {
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
