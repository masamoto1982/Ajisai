import { readFileSync, writeFileSync } from 'node:fs';

const check = process.argv.includes('--check');
const read = (path) => readFileSync(path, 'utf8');
const fragments = new Map([
  ['presentation-profile', read('spec/gui-semantics.md')],
  ['14-ai-first-implementation-rules', read('docs/dev/specification-implementation-rules.html')],
  ['15-test-discipline', read('docs/quality/specification-test-discipline.html')],
]);

let content = read('spec/language-semantics.md');
for (const [id, fragment] of fragments) {
  const marker = `<!-- INCLUDE:${id} -->`;
  if (!content.includes(marker)) throw new Error(`Missing specification marker: ${marker}`);
  content = content.replace(marker, fragment.trimEnd());
}
if (/<!-- INCLUDE:/.test(content)) throw new Error('Unresolved specification include marker');

const generated = read('spec/specification.template.html').replace('{{SPECIFICATION_CONTENT}}', content);
const destination = 'SPECIFICATION.html';
if (check) {
  if (read(destination) !== generated) {
    console.error(`[specification] ${destination} is stale. Run npm run specification:generate and commit the result.`);
    process.exitCode = 1;
  } else {
    console.log(`[specification] ${destination} is up to date.`);
  }
} else {
  writeFileSync(destination, generated);
  console.log(`[specification] wrote ${destination}.`);
}
