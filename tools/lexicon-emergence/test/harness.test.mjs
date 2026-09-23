// The route-B harness against a scripted model: no API call is made, but the
// Ajisai MCP server, the prompt and the grader are the real ones, so a run
// that passes here differs from a paid run only in who writes the replies.

import assert from 'node:assert/strict';
import { after, before, test } from 'node:test';
import { connect } from '../lib/ajisai.mjs';
import { grade, loadFamilies } from '../lib/grade.mjs';
import { Budget, costOf, runSubject, SUBJECT_TOOLS, subjectSystem } from '../lib/harness.mjs';
import { subjectPrompt } from '../lib/prompt.mjs';

const families = loadFamilies(new URL('../tasks/', import.meta.url));
const tasks = families.flatMap((f) => f.tasks);
const usage = { input_tokens: 1000, output_tokens: 200, cache_creation_input_tokens: 0, cache_read_input_tokens: 0 };

/** A client whose replies are the next entry of `script`, recording every request it gets. */
function scripted(script) {
  const requests = [];
  return {
    requests,
    messages: {
      async create(params) {
        requests.push(structuredClone(params));
        const content = script.shift();
        if (!content) throw new Error('script exhausted');
        const stop = content.some((b) => b.type === 'tool_use') ? 'tool_use' : 'end_turn';
        return { model: params.model, stop_reason: stop, content, usage };
      },
    },
  };
}

const toolUse = (id, name, input) => ({ type: 'tool_use', id, name, input });
const meta = { run: 'test', condition: 'bottleneck', generation: 0, agent: 'G0A' };
let ajisai;

before(async () => {
  ajisai = await connect();
});
after(async () => {
  await ajisai.close();
});

test('a subject that computes and submits is graded like a pilot submission', async () => {
  const client = scripted([
    [{ type: 'text', text: 'Trying the sum.' }, toolUse('t1', 'compute', { source: '[ 3 1 4 1 5 ] G0A.SUM' })],
    [
      toolUse('t2', 'submit', {
        definitions: [{ name: 'G0A.SUM', body: '0 [ + ] FOLD', note: 'sum of a numeric Vector' }],
        solutions: tasks.map((t) => ({ task: t.id, code: t.reference })),
      }),
    ],
  ]);
  const { submission, record } = await runSubject({
    client,
    ajisai,
    model: 'claude-opus-5',
    system: 'reference',
    prompt: subjectPrompt(meta, { entries: [] }, families, 'harness'),
    meta,
  });

  assert.equal(record.ended, 'submitted');
  assert.equal(record.turns, 2);
  // The tools offered are the server's own, plus submit, and the compute call
  // came back with the server's text (here: an unknown-word error).
  assert.deepEqual(client.requests[0].tools.map((t) => t.name), [...SUBJECT_TOOLS, 'submit']);
  const toolResult = client.requests[1].messages.at(-1).content[0];
  assert.equal(toolResult.tool_use_id, 't1');
  assert.match(toolResult.content, /G0A\.SUM/);
  // Adaptive thinking and effort for a model that takes them.
  assert.deepEqual(client.requests[0].thinking, { type: 'adaptive' });
  assert.deepEqual(client.requests[0].output_config, { effort: 'high' });

  assert.equal(submission.agent, 'G0A');
  assert.equal(submission.definitions[0].body, '0 [ + ] FOLD');
  const graded = await grade(ajisai, submission, { entries: [] }, families);
  assert.equal(graded.results.filter((r) => r.correct).length, tasks.length);
});

test('a subject that stops without submitting is reminded once, then recorded as not submitting', async () => {
  const client = scripted([[{ type: 'text', text: 'Done.' }], [{ type: 'text', text: 'Really done.' }]]);
  const { submission, record } = await runSubject({
    client,
    ajisai,
    model: 'claude-haiku-4-5',
    system: 'reference',
    prompt: 'p',
    meta,
  });
  assert.equal(submission, null);
  assert.equal(record.ended, 'noSubmission');
  assert.match(client.requests[1].messages.at(-1).content, /submit/);
  // Haiku 4.5 takes neither adaptive thinking nor effort.
  assert.equal(client.requests[0].thinking, undefined);
  assert.equal(client.requests[0].output_config, undefined);
});

test('a call to a tool the subject was not given is an error result, not a crash', async () => {
  const client = scripted([[toolUse('t1', 'read_file', { path: 'SKILL.md' })], [{ type: 'text', text: 'ok' }], [{ type: 'text', text: 'ok' }]]);
  const { record } = await runSubject({ client, ajisai, model: 'claude-opus-5', system: 's', prompt: 'p', meta });
  const result = client.requests[1].messages.at(-1).content[0];
  assert.equal(result.is_error, true);
  assert.equal(record.ended, 'noSubmission');
});

test('the budget stops a run before the request that would exceed it', async () => {
  const budget = new Budget(costOf('claude-opus-5', usage) * 1.5);
  const client = scripted([
    [toolUse('t1', 'compute', { source: '1' })],
    [toolUse('t2', 'compute', { source: '2' })],
    [toolUse('t3', 'compute', { source: '3' })],
  ]);
  await assert.rejects(
    runSubject({ client, ajisai, model: 'claude-opus-5', system: 's', prompt: 'p', meta, budget }),
    /budget exhausted/,
  );
  assert.equal(client.requests.length, 2);
});

test('the shared system prompt carries SKILL.md, so it is the same bytes for every subject', () => {
  const system = subjectSystem(new URL('../../../SKILL.md', import.meta.url));
  assert.match(system, /DEF/);
  assert.equal(system, subjectSystem(new URL('../../../SKILL.md', import.meta.url)));
});

test('cost follows the listed rates, cache writes at 1.25x and reads at 0.1x', () => {
  const usd = costOf('claude-sonnet-5', {
    input_tokens: 1e6,
    output_tokens: 1e6,
    cache_creation_input_tokens: 1e6,
    cache_read_input_tokens: 1e6,
  });
  assert.equal(usd.toFixed(2), (2 + 10 + 2.5 + 0.2).toFixed(2));
  assert.equal(costOf('unknown-model', usage), null);
});
