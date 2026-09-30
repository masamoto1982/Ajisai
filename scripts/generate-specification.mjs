import { readText, writeOrCheck } from './lib/common.mjs';

const fragments = new Map([
  ['presentation-profile', readText('spec/gui-semantics.md')],
]);

let content = readText('spec/language-semantics.md');
for (const [id, fragment] of fragments) {
  const marker = `<!-- INCLUDE:${id} -->`;
  if (!content.includes(marker)) throw new Error(`Missing specification marker: ${marker}`);
  content = content.replace(marker, fragment.trimEnd());
}
if (/<!-- INCLUDE:/.test(content)) throw new Error('Unresolved specification include marker');

const generated = readText('spec/specification.template.html').replace('{{SPECIFICATION_CONTENT}}', content);
writeOrCheck(
  'specification',
  [{
    path: 'SPECIFICATION.html',
    content: generated,
    stale: 'SPECIFICATION.html is stale. Run npm run specification:generate and commit the result.',
  }],
  { current: 'SPECIFICATION.html is up to date.', wrote: 'wrote SPECIFICATION.html.' },
);
