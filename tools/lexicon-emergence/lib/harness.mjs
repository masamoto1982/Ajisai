// Route B (work order §1.2): a subject agent driven through the Claude API.
//
// The harness stands where Claude Code stood in the pilot. It gives the model
// the same Ajisai MCP tools the pilot's subagents had — their definitions are
// read from the server, not rewritten here — relays each call to the server
// and returns the server's text unchanged. The model hands in its work through
// a `submit` tool instead of writing a file. Everything else a subject sees is
// the SKILL.md reference and the one prompt from `prompt.mjs`.
//
// The loop is written by hand rather than with the SDK's tool runner because
// the experiment needs what the runner does not expose in one place: a fixed
// turn cap, a spending ceiling checked before every request, the served model
// of every response, and the whole transcript on disk.

import { readFileSync } from 'node:fs';

/** The three model sizes the main experiment mixes (work order §5.3). */
export const MODELS = { large: 'claude-opus-5', medium: 'claude-sonnet-5', small: 'claude-haiku-4-5' };

/** Base rates in USD per million tokens. Cache writes cost 1.25× input, cache reads 0.1×. */
export const PRICES = {
  'claude-opus-5': { input: 5, output: 25 },
  'claude-sonnet-5': { input: 2, output: 10 },
  'claude-haiku-4-5': { input: 1, output: 5 },
};

/** The Ajisai tools a subject may call — the pilot's set (prompt.mjs). */
export const SUBJECT_TOOLS = ['compute', 'check', 'word_contract'];

const SUBMIT_TOOL = {
  name: 'submit',
  description:
    'Hand in your definitions and solutions. Call it once, when every solution has been run through `compute` exactly as it will be graded. The study ends for you when this call succeeds.',
  strict: true,
  input_schema: {
    type: 'object',
    additionalProperties: false,
    required: ['definitions', 'solutions'],
    properties: {
      definitions: {
        type: 'array',
        description: 'Your Words, each after any of your own Words it calls.',
        items: {
          type: 'object',
          additionalProperties: false,
          required: ['name', 'body', 'note'],
          properties: {
            name: { type: 'string', description: 'The Word name, starting with your agent id and a dot.' },
            body: { type: 'string', description: 'The code between `[` and `]` in `[ body ] \'NAME\' DEF`.' },
            note: { type: 'string', description: 'What the Word does, one line.' },
          },
        },
      },
      solutions: {
        type: 'array',
        description: 'One entry per task you solved.',
        items: {
          type: 'object',
          additionalProperties: false,
          required: ['task', 'code'],
          properties: {
            task: { type: 'string', description: 'The task id.' },
            code: { type: 'string', description: 'The code that follows the input.' },
          },
        },
      },
    },
  },
};

/** The system prompt every subject shares, so it is cached across agents. */
export function subjectSystem(skillPath) {
  const skill = readFileSync(skillPath, 'utf8');
  return [
    'You are a subject agent in a language-use study. The reference for the language, Ajisai, follows.',
    'It is the only documentation you have; the tools are the only way to run code.',
    '',
    skill,
  ].join('\n');
}

/** USD for one response's usage at `model`'s rates; null for a model without a listed price. */
export function costOf(model, usage) {
  const price = PRICES[model];
  if (!price) return null;
  const perToken = (rate) => rate / 1e6;
  return (
    (usage.input_tokens ?? 0) * perToken(price.input) +
    (usage.cache_creation_input_tokens ?? 0) * perToken(price.input * 1.25) +
    (usage.cache_read_input_tokens ?? 0) * perToken(price.input * 0.1) +
    (usage.output_tokens ?? 0) * perToken(price.output)
  );
}

/** A spending ceiling shared by every agent of a run; checked before each request. */
export class Budget {
  constructor(limitUsd) {
    this.limitUsd = limitUsd;
    this.spentUsd = 0;
  }
  charge(usd) {
    this.spentUsd += usd ?? 0;
  }
  assertRoom() {
    if (this.limitUsd != null && this.spentUsd >= this.limitUsd) {
      throw new Error(`budget exhausted: $${this.spentUsd.toFixed(2)} of $${this.limitUsd.toFixed(2)} spent`);
    }
  }
}

/** Request parameters that depend on the model: Haiku 4.5 takes neither adaptive thinking nor effort. */
function modelParams(model, effort) {
  if (model.startsWith('claude-haiku-4-5')) return {};
  return { thinking: { type: 'adaptive' }, output_config: { effort } };
}

function toSubmission(input, meta) {
  const solutions = {};
  for (const { task, code } of input.solutions ?? []) solutions[task] = code;
  return {
    agent: meta.agent,
    generation: meta.generation,
    condition: meta.condition,
    definitions: (input.definitions ?? []).map(({ name, body, note }) => ({ name, body, note })),
    solutions,
  };
}

/**
 * Run one subject to a submission. `client` is an Anthropic client (or any
 * object with the same `messages.create`); `ajisai` is `connect()`'s client.
 * Returns the submission (null when the subject never submitted) and a record
 * of the run: every request's served model and usage, the cost, and why it ended.
 */
export async function runSubject({ client, ajisai, model, effort = 'high', system, prompt, meta, budget, maxTurns = 60 }) {
  const ajisaiTools = await ajisai.tools(SUBJECT_TOOLS);
  const tools = [
    ...ajisaiTools.map((t) => ({ name: t.name, description: t.description, input_schema: t.inputSchema })),
    SUBMIT_TOOL,
  ];
  const messages = [{ role: 'user', content: prompt }];
  const requests = [];
  let submission = null;
  let reminded = false;
  let ended = 'turnCap';

  for (let turn = 0; turn < maxTurns; turn += 1) {
    budget?.assertRoom();
    const response = await client.messages.create({
      model,
      max_tokens: 16000,
      system: [{ type: 'text', text: system, cache_control: { type: 'ephemeral' } }],
      tools,
      messages,
      cache_control: { type: 'ephemeral' },
      ...modelParams(model, effort),
    });
    const cost = costOf(model, response.usage);
    budget?.charge(cost);
    requests.push({ servedModel: response.model, stopReason: response.stop_reason, usage: response.usage, cost });
    messages.push({ role: 'assistant', content: response.content });

    if (response.stop_reason === 'refusal' || response.stop_reason === 'max_tokens') {
      ended = response.stop_reason;
      break;
    }
    const calls = response.content.filter((b) => b.type === 'tool_use');
    if (calls.length === 0) {
      if (reminded) {
        ended = 'noSubmission';
        break;
      }
      reminded = true;
      messages.push({ role: 'user', content: 'Hand in your work by calling the `submit` tool.' });
      continue;
    }

    const results = [];
    for (const call of calls) {
      if (call.name === 'submit') {
        submission = toSubmission(call.input, meta);
        results.push({ type: 'tool_result', tool_use_id: call.id, content: 'Submitted.' });
      } else if (SUBJECT_TOOLS.includes(call.name)) {
        try {
          results.push({ type: 'tool_result', tool_use_id: call.id, content: await ajisai.callText(call.name, call.input) });
        } catch (error) {
          results.push({ type: 'tool_result', tool_use_id: call.id, content: String(error.message ?? error), is_error: true });
        }
      } else {
        results.push({ type: 'tool_result', tool_use_id: call.id, content: `There is no tool named ${call.name}.`, is_error: true });
      }
    }
    messages.push({ role: 'user', content: results });
    if (submission) {
      ended = 'submitted';
      break;
    }
  }

  const totalCost = requests.reduce((sum, r) => sum + (r.cost ?? 0), 0);
  return {
    submission,
    record: { agent: meta.agent, model, effort, ended, turns: requests.length, costUsd: totalCost, requests, messages },
  };
}

/** A real client, loaded only when a run needs one, so tests and grading work without the SDK's credentials. */
export async function anthropicClient() {
  const { default: Anthropic } = await import('@anthropic-ai/sdk');
  return new Anthropic();
}
