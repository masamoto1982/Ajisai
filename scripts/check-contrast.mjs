#!/usr/bin/env node
// Gate the Playground's colour tokens on WCAG 2.x contrast.
//
// Every colour token the stylesheets use as text (`color: var(--…)`) must be
// listed below with the surfaces it is drawn on, and must reach 4.5:1
// (WCAG 1.4.3) on each of them. A token used as text that is not listed fails
// the check too, so a new text colour cannot arrive unmeasured. The few
// non-text marks listed separately — focus rings and the top-of-stack edge —
// must reach 3:1 (WCAG 1.4.11).
//
// This exists because contrast was never measured: printed output shipped at
// 1.97:1, the editor's placeholder — its only help — at 1.77:1, User Word
// chips at 2.43:1, and the top of the stack was marked by a fill 1.21:1 from
// the panel around it.
//
// Tokens are read from src/styles/tokens.css and converted OKLCH → sRGB
// (gamut-clipped, as a browser renders them) → relative luminance.
//
//   node scripts/check-contrast.mjs

import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const stylesDir = join(repoRoot, 'src', 'styles');
const tokensCss = readFileSync(join(stylesDir, 'tokens.css'), 'utf8');

const TEXT_MINIMUM = 4.5;
const NON_TEXT_MINIMUM = 3;

// Text token → the surfaces it is drawn on.
const PANEL = ['--color-white', '--color-light'];
const TEXT = {
  '--color-text': [...PANEL, '--color-symbol', '--color-consume-eat'],
  '--color-text-light': [...PANEL, '--color-symbol', '--color-consume-eat'],
  '--input-text': ['--color-symbol'],
  '--input-placeholder': ['--color-symbol'],
  '--color-primary': PANEL,
  '--color-white': ['--color-primary'],
  '--color-stack': ['--color-white', '--color-consume-eat'],
  '--color-core': PANEL,
  '--color-dependency': PANEL,
  '--color-non-dependency': PANEL,
  '--color-output-debug': PANEL,
  '--color-output-program': PANEL,
  '--color-output-error': PANEL,
  '--color-output-info': PANEL,
  // Brackets are drawn in the Stack, and the top item sits on the consume fill.
  ...Object.fromEntries(
    [1, 2, 3, 4, 5, 6, 7, 8, 9].map((depth) => [
      `--bracket-depth-${depth}`,
      ['--color-white', '--color-consume-eat'],
    ]),
  ),
};

// Non-text marks → the surfaces they must stand out from.
const NON_TEXT = {
  '--color-primary': [...PANEL, '--color-symbol'], // focus rings
  '--color-consume-edge': ['--color-white', '--color-consume-eat'], // top of the stack
};

function readTokens(css) {
  const raw = new Map();
  for (const match of css.matchAll(/(--[\w-]+):\s*([^;]+);/g)) raw.set(match[1], match[2].trim());
  const resolveToken = (name, seen = new Set()) => {
    if (seen.has(name)) throw new Error(`token cycle at ${name}`);
    const value = raw.get(name);
    if (value === undefined) throw new Error(`tokens.css declares no ${name}`);
    const alias = value.match(/^var\((--[\w-]+)\)$/);
    if (alias) return resolveToken(alias[1], new Set([...seen, name]));
    const oklch = value.match(/^oklch\(\s*([\d.]+)%\s+([\d.]+)\s+([\d.]+)\s*\)$/);
    if (!oklch) throw new Error(`${name} is not an opaque oklch() colour: ${value}`);
    return [Number(oklch[1]) / 100, Number(oklch[2]), Number(oklch[3])];
  };
  return resolveToken;
}

function linearSrgb([lightness, chroma, hue]) {
  const a = chroma * Math.cos((hue * Math.PI) / 180);
  const b = chroma * Math.sin((hue * Math.PI) / 180);
  const l = (lightness + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (lightness - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (lightness - 0.0894841775 * a - 1.291485548 * b) ** 3;
  return [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ].map((channel) => Math.min(1, Math.max(0, channel)));
}

function luminance(colour) {
  const [r, g, b] = linearSrgb(colour);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(a, b) {
  const [lighter, darker] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (lighter + 0.05) / (darker + 0.05);
}

const token = readTokens(tokensCss);
const failures = [];
let measured = 0;
for (const [pairs, minimum, kind] of [
  [TEXT, TEXT_MINIMUM, 'text'],
  [NON_TEXT, NON_TEXT_MINIMUM, 'non-text'],
]) {
  for (const [foreground, surfaces] of Object.entries(pairs)) {
    for (const surface of surfaces) {
      const ratio = contrast(token(foreground), token(surface));
      measured += 1;
      if (ratio < minimum) {
        failures.push(`${kind} ${foreground} on ${surface}: ${ratio.toFixed(2)}:1, needs ${minimum}:1`);
      }
    }
  }
}

// Every token the stylesheets draw text in must be measured above.
const usedAsText = new Set();
for (const file of readdirSync(stylesDir).filter((name) => name.endsWith('.css'))) {
  const css = readFileSync(join(stylesDir, file), 'utf8');
  for (const match of css.matchAll(/(?:^|[\s;{])color:\s*var\((--[\w-]+)\)/g)) usedAsText.add(match[1]);
}
for (const name of usedAsText) {
  if (!(name in TEXT)) failures.push(`${name} is used as text colour but not measured in check-contrast.mjs`);
}

if (failures.length) {
  console.error('[contrast] below the WCAG minimum:');
  for (const failure of failures) console.error(`  ${failure}`);
  process.exit(1);
}
console.log(
  `[contrast] ${measured} colour pairs meet WCAG contrast (text ${TEXT_MINIMUM}:1, non-text ${NON_TEXT_MINIMUM}:1); ` +
    `all ${usedAsText.size} text-colour tokens are measured.`,
);
