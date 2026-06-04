#!/usr/bin/env node
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');
const suitePath = resolve(repoRoot, 'tests/conformance/index.html');
const html = readFileSync(suitePath, 'utf8');

function decodeEntities(value) {
  return value
    .replaceAll('&lt;', '<')
    .replaceAll('&gt;', '>')
    .replaceAll('&quot;', '"')
    .replaceAll('&#39;', "'")
    .replaceAll('&apos;', "'")
    .replaceAll('&amp;', '&');
}

function attr(attrs, name) {
  const pattern = new RegExp(`(?:^|\\s)${name}\\s*=\\s*(["'])([\\s\\S]*?)\\1`);
  const match = attrs.match(pattern);
  return match ? decodeEntities(match[2]) : undefined;
}

function extractClass(section, className, required = true) {
  const pattern = new RegExp(
    `<([a-zA-Z0-9-]+)(?=[^>]*\\bclass=["'][^"']*\\b${className}\\b)[^>]*>([\\s\\S]*?)<\\/\\1>`,
  );
  const match = section.match(pattern);
  if (!match) {
    if (required) throw new Error(`missing .${className}`);
    return undefined;
  }
  return decodeEntities(match[2]).trim();
}

function extractEffects(section) {
  const container = extractClass(section, 'ajisai-expect-effects');
  const effects = [];
  const effectPattern = /<span\b([^>]*\bclass=["'][^"']*\bajisai-effect\b[^"']*["'][^>]*)><\/span>/g;
  for (const match of container.matchAll(effectPattern)) {
    effects.push({
      kind: attr(match[1], 'data-kind'),
      payload: attr(match[1], 'data-payload'),
    });
  }
  return effects;
}

const cases = [];
const casePattern = /<section\b([^>]*\bclass=["'][^"']*\bajisai-case\b[^"']*["'][^>]*)>([\s\S]*?)<\/section>/g;
for (const match of html.matchAll(casePattern)) {
  const attrs = match[1];
  const body = match[2];
  const host = {};
  const nowMillis = attr(attrs, 'data-host-now-millis');
  const randomHex = attr(attrs, 'data-host-random-hex');
  const capabilities = attr(attrs, 'data-host-capabilities');
  if (nowMillis !== undefined) host.nowMillis = Number(nowMillis);
  if (randomHex !== undefined) host.randomHex = randomHex;
  if (capabilities !== undefined) {
    host.capabilities = capabilities.split(/[\s,]+/).filter(Boolean);
  }

  cases.push({
    id: attr(attrs, 'id'),
    category: attr(attrs, 'data-category'),
    source: extractClass(body, 'ajisai-source'),
    expectResult: extractClass(body, 'ajisai-expect-result'),
    expectError: extractClass(body, 'ajisai-expect-error', false),
    effects: extractEffects(body),
    ...(Object.keys(host).length ? { host } : {}),
  });
}

if (cases.length === 0) {
  throw new Error('conformance manifest would be empty');
}

process.stdout.write(`${JSON.stringify({ schemaVersion: 1, cases }, null, 2)}\n`);
