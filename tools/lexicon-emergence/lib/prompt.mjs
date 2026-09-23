// The one message a subject agent receives. It carries the tasks, one
// example input per family (the grader also uses inputs the subject never
// sees, so a hard-coded answer fails), and the inherited lexicon — and
// nothing of any earlier agent's conversation (work order §5.1 step 7).

import { prelude } from './source.mjs';

/**
 * `route` is how the subject runs (work order §1.2): `subagent` for a Claude
 * Code subagent that reads SKILL.md and writes a file (the pilot), `harness`
 * for the API harness, whose system prompt already carries SKILL.md and whose
 * subject hands in through a `submit` tool. The tasks and the lexicon read the
 * same either way.
 */
export function subjectPrompt({ run, condition, generation, agent }, lexicon, families, route = 'subagent') {
  const harness = route === 'harness';
  const path = `tools/lexicon-emergence/runs/${run}/${condition}/gen${generation}/submissions/${agent}.json`;
  const lines = [];
  lines.push(`You are subject agent ${agent} in a language-use study. Solve the tasks below in Ajisai.`);
  lines.push('');
  lines.push('## Rules');
  if (harness) {
    lines.push('- Learn Ajisai from the reference in your instructions.');
    lines.push('- Run code only with the `compute` tool. You may also use `check` and `word_contract`.');
  } else {
    lines.push('- Learn Ajisai from `SKILL.md` at the repository root. Read no other file in the repository —');
    lines.push('  in particular nothing under `tools/lexicon-emergence/`, `docs/`, `spec/` or `rust/`.');
    lines.push('- Run code only with the `mcp__ajisai__compute` tool (load it with ToolSearch first). You may');
    lines.push('  also use `mcp__ajisai__check` and `mcp__ajisai__word_contract`.');
  }
  lines.push("- Each task's input is already on the stack when your solution runs. Your solution is the code");
  lines.push('  that follows it and must leave exactly the requested value as the only item on the stack.');
  lines.push('  It is graded on more inputs than the example shown, so it must work for any input of the kind');
  lines.push('  described.');
  lines.push(`- You may define your own Words. Every name must start with \`${agent}.\` (for example`);
  lines.push(`  \`${agent}.MEAN\`). Definitions made in this round may be offered to later agents.`);
  lines.push('');

  if (lexicon.entries.length > 0) {
    lines.push('## Inherited Words');
    lines.push('These Words are already defined before your solution runs, and you may call them by name:');
    lines.push('');
    lines.push('| Name | Body | Note |');
    lines.push('| --- | --- | --- |');
    for (const e of lexicon.entries) lines.push(`| \`${e.name}\` | \`${e.body}\` | ${e.note || '—'} |`);
    lines.push('');
    lines.push('To test with them, put this text before your own code in a `compute` call:');
    lines.push('');
    lines.push('```');
    lines.push(prelude(lexicon.entries));
    lines.push('```');
    lines.push('');
  }

  lines.push('## Tasks');
  for (const family of families) {
    lines.push('');
    lines.push(`### Input: ${family.inputKind}`);
    lines.push(`Example input: \`${family.inputs[0]}\``);
    lines.push('');
    for (const task of family.tasks) lines.push(`- \`${task.id}\`: ${task.prompt}`);
  }
  lines.push('');
  lines.push('## Submission');
  if (harness) {
    lines.push('Call the `submit` tool once, with your definitions (name, body, note) and one solution per task');
    lines.push('(task id, code that follows the input).');
  } else {
    lines.push(`Write one JSON file to \`${path}\` with this shape, then reply with the single word DONE:`);
    lines.push('');
    lines.push('```json');
    lines.push('{');
    lines.push(`  "agent": "${agent}", "generation": ${generation}, "condition": "${condition}",`);
    lines.push(`  "definitions": [ { "name": "${agent}.EXAMPLE", "body": "code inside the brackets", "note": "what it does, one line" } ],`);
    lines.push('  "solutions": { "<task id>": "code that follows the input" }');
    lines.push('}');
    lines.push('```');
  }
  lines.push('');
  lines.push('`body` is the code that goes between `[` and `]` in `[ body ] \'NAME\' DEF`, starting with its parameter');
  lines.push('header: the names of the operands it takes, then `|` — `X Y | X Y +`, or `| 42` for none. List definitions so that each');
  lines.push('comes after any of your own Words it calls. Solutions may call your Words and the inherited Words.');
  lines.push(`Before ${harness ? 'submitting' : 'writing the file'}, run each solution through \`compute\` exactly as it will be graded: the`);
  lines.push('inherited-Word text above (if any), your definitions as DEF lines, the example input, the solution.');
  return `${lines.join('\n')}\n`;
}
