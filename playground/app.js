// The playground.
//
// Everything here is presentation. The rules that decide *which* surface you
// are looking at live in `presentation.js`, free of the DOM and tested on their
// own; this file wires them to elements, keys, and gestures.

import {
  MOBILE_BREAKPOINT,
  SWIPE_THRESHOLD,
  analyzeModes,
  applyRun,
  applyRunMobile,
  cycle,
  detectChanges,
  operandCells,
  selectSurface,
} from './presentation.js';

const $ = (id) => document.getElementById(id);

const el = {
  body: document.body,
  editor: $('editor'),
  suggestions: $('suggestions'),
  palette: $('palette'),
  output: $('output'),
  stack: $('stack'),
  modeHint: $('mode-hint'),
  dictionary: $('dictionary'),
  search: $('search'),
  sheet: $('sheet-select'),
  status: $('status'),
  leftSelect: $('left-select'),
  rightSelect: $('right-select'),
  mobileSelect: $('mobile-select'),
  areas: {
    input: $('input-area'),
    output: $('output-area'),
    stack: $('stack-area'),
    dictionary: $('dictionary-area'),
  },
};

// ------------------------------------------------------------------ session

let worker = null;
let pending = new Map();
let nextId = 0;
let running = false;

/** Fragments that ran without error, in order. Replayed after an abort. */
const journal = [];

const spawn = () => {
  worker = new Worker(new URL('./worker.js', import.meta.url), { type: 'module' });
  worker.onmessage = (event) => {
    const { id, ok, reply, error } = event.data;
    const settle = pending.get(id);
    if (!settle) return;
    pending.delete(id);
    if (ok) settle.resolve(reply);
    else settle.reject(new Error(error));
  };
};

const ask = (kind, source, extra) =>
  new Promise((resolve, reject) => {
    const id = (nextId += 1);
    pending.set(id, { resolve, reject });
    worker.postMessage({ id, kind, source, ...extra });
  });

/**
 * Abort by terminating the worker, then rebuild the session by replaying the
 * journal. The fragment that was running is not in the journal, so it is the
 * one thing that does not come back — which is what abort means.
 */
const abort = async () => {
  if (!running) return false;
  worker.terminate();
  for (const { reject } of pending.values()) reject(new Error('aborted'));
  pending = new Map();
  running = false;
  spawn();
  const snapshot = await ask('replay', null, { journal: [...journal] });
  render(snapshot);
  say('Aborted. The session was rebuilt from what had already run.');
  return true;
};

// -------------------------------------------------------------------- state

let layout = { left: 'input', right: 'stack' };
let mobileSurface = 'input';
let snapshot = { stack: [], definitions: [] };
let vocabulary = [];
let sheet = 'core';

const isMobile = () => window.innerWidth <= MOBILE_BREAKPOINT;

const say = (text) => {
  el.status.textContent = text;
};

// ------------------------------------------------------------------- layout

const applyLayout = () => {
  const mobile = isMobile();
  el.body.dataset.layout = mobile ? 'mobile' : 'desktop';
  if (mobile) {
    for (const [name, node] of Object.entries(el.areas)) {
      node.hidden = name !== mobileSurface;
    }
    el.body.dataset.activeArea = mobileSurface;
    el.mobileSelect.value = mobileSurface;
  } else {
    el.areas.input.hidden = layout.left !== 'input';
    el.areas.output.hidden = layout.left !== 'output';
    el.areas.stack.hidden = layout.right !== 'stack';
    el.areas.dictionary.hidden = layout.right !== 'dictionary';
    el.body.dataset.activeArea = layout.right;
    el.leftSelect.value = layout.left;
    el.rightSelect.value = layout.right;
  }
};

const show = (surface) => {
  if (isMobile()) {
    mobileSurface = surface;
  } else {
    layout = selectSurface(layout, surface);
  }
  applyLayout();
};

// ----------------------------------------------------------------- rendering

const renderStack = () => {
  const modes = analyzeModes(el.editor.value);
  const operands = new Set(operandCells(snapshot.stack.length, modes));
  el.stack.replaceChildren(
    ...snapshot.stack.map((text, index) => {
      const item = document.createElement('li');
      item.className = 'cell';
      item.textContent = text;
      if (operands.has(index)) {
        item.dataset.operand = modes.retention;
      }
      return item;
    }),
  );
  if (snapshot.stack.length === 0) {
    const empty = document.createElement('li');
    empty.className = 'empty';
    empty.textContent = 'the flow is empty';
    el.stack.replaceChildren(empty);
  }
  const target = modes.target === 'stak' ? 'the whole flow' : 'the surface';
  const fate = modes.retention === 'keep' ? 'kept' : 'eaten';
  el.modeHint.textContent = `${target} · ${fate}`;
  el.modeHint.dataset.retention = modes.retention;
};

const renderDiagnostics = (items) => {
  if (items.length === 0) {
    const ok = document.createElement('li');
    ok.className = 'ok';
    ok.textContent = 'nothing to report';
    el.output.replaceChildren(ok);
    return;
  }
  el.output.replaceChildren(
    ...items.map(({ severity, message }) => {
      const line = document.createElement('li');
      line.className = severity;
      line.textContent = message;
      return line;
    }),
  );
};

const wordEntry = (word) => {
  const item = document.createElement('li');
  item.className = 'word';

  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'word-name';
  button.textContent = word.name;
  button.addEventListener('click', () => insert(word.name));
  item.append(button);

  if (word.aliases?.length) {
    const aliases = document.createElement('span');
    aliases.className = 'word-aliases';
    aliases.textContent = word.aliases.join(' ');
    item.append(aliases);
  }
  const effect = document.createElement('code');
  effect.className = 'word-effect';
  effect.textContent = word.stack_effect ?? word.body ?? '';
  item.append(effect);

  if (word.summary) {
    const summary = document.createElement('p');
    summary.className = 'word-summary';
    summary.textContent = word.summary;
    item.append(summary);
  }
  return item;
};

const renderDictionary = () => {
  const filter = el.search.value.trim().toUpperCase();
  const source =
    sheet === 'user'
      ? snapshot.definitions.map((word) => ({ ...word, summary: '' }))
      : vocabulary;
  const shown = source.filter(
    (word) =>
      !filter ||
      word.name.includes(filter) ||
      (word.aliases ?? []).some((alias) => alias.includes(filter)),
  );
  if (shown.length === 0) {
    const empty = document.createElement('li');
    empty.className = 'empty';
    empty.textContent =
      sheet === 'user' ? 'no words defined yet' : 'no word matches that filter';
    el.dictionary.replaceChildren(empty);
    return;
  }
  el.dictionary.replaceChildren(...shown.map(wordEntry));
};

const render = (reply) => {
  snapshot = { stack: reply.stack ?? [], definitions: reply.definitions ?? [] };
  renderStack();
  renderDictionary();
};

// -------------------------------------------------------------------- editor

const insert = (text) => {
  const editor = el.editor;
  const at = editor.selectionStart ?? editor.value.length;
  const end = editor.selectionEnd ?? at;
  const before = editor.value.slice(0, at);
  const after = editor.value.slice(end);
  const lead = before && !/\s$/.test(before) ? ' ' : '';
  editor.value = `${before}${lead}${text} ${after}`;
  const caret = before.length + lead.length + text.length + 1;
  editor.setSelectionRange(caret, caret);
  editor.focus();
  renderStack();
  if (!isMobile()) show('input');
};

const currentPrefix = () => {
  const at = el.editor.selectionStart ?? 0;
  const before = el.editor.value.slice(0, at);
  const match = before.match(/(\S+)$/);
  return match ? match[1].toUpperCase() : '';
};

const hideSuggestions = () => {
  el.suggestions.hidden = true;
  el.suggestions.replaceChildren();
};

const showSuggestions = () => {
  const prefix = currentPrefix();
  if (!prefix) return hideSuggestions();
  const names = [
    ...vocabulary.map((word) => word.name),
    ...snapshot.definitions.map((word) => word.name),
  ];
  const matches = names.filter((name) => name.startsWith(prefix) && name !== prefix).slice(0, 8);
  if (matches.length === 0) return hideSuggestions();
  el.suggestions.replaceChildren(
    ...matches.map((name) => {
      const item = document.createElement('li');
      const button = document.createElement('button');
      button.type = 'button';
      button.textContent = name;
      button.addEventListener('click', () => {
        const at = el.editor.selectionStart ?? 0;
        const before = el.editor.value.slice(0, at).replace(/\S+$/, '');
        const after = el.editor.value.slice(at);
        el.editor.value = `${before}${name} ${after}`;
        const caret = before.length + name.length + 1;
        el.editor.setSelectionRange(caret, caret);
        el.editor.focus();
        hideSuggestions();
        renderStack();
      });
      item.append(button);
      return item;
    }),
  );
  el.suggestions.hidden = false;
};

// ------------------------------------------------------------------- actions

/** Run a fragment, then let what it touched decide where the layout goes. */
const runFragment = async (source, label) => {
  const trimmed = source.trim();
  if (!trimmed) return;
  const before = snapshot;
  running = true;
  say(`${label}…`);
  let reply;
  try {
    reply = await ask('execute', trimmed);
  } catch (error) {
    if (error.message === 'aborted') return;
    say(String(error.message));
    return;
  } finally {
    running = false;
  }
  if (reply.ok) journal.push(trimmed);

  const after = { stack: reply.stack, definitions: reply.definitions };
  const changes = detectChanges(before, after, reply);
  render(reply);
  renderDiagnostics(reply.ok ? [] : [{ severity: 'error', message: reply.error }]);

  if (isMobile()) {
    mobileSurface = applyRunMobile(mobileSurface, changes);
  } else {
    layout = applyRun(layout, changes);
  }
  if (changes.dictionary) {
    sheet = 'user';
    el.sheet.value = 'user';
    renderDictionary();
  }
  applyLayout();
  say(reply.ok ? `${label} complete.` : reply.error);
};

const run = () => runFragment(el.editor.value, 'Run');

/** Run one source unit and take it out of the editor, so Step can repeat. */
const step = async () => {
  const source = el.editor.value.trim();
  if (!source) return;
  const reply = await ask('steps', source);
  if (!reply.ok) {
    renderDiagnostics([{ severity: 'error', message: reply.error }]);
    layout = applyRun(layout, { output: true });
    mobileSurface = applyRunMobile(mobileSurface, { output: true });
    applyLayout();
    return;
  }
  const [head, ...rest] = reply.steps;
  if (!head) return;
  el.editor.value = rest.join(' ');
  await runFragment(head, `Step ${head}`);
  renderStack();
};

const format = async () => {
  const reply = await ask('format', el.editor.value);
  if (reply.ok) {
    el.editor.value = reply.text;
    renderStack();
    say('Formatted.');
  } else {
    renderDiagnostics([{ severity: 'error', message: reply.error }]);
    show('output');
    say(reply.error);
  }
};

const runLint = async () => {
  const reply = await ask('lint', el.editor.value);
  if (!reply.ok) {
    renderDiagnostics([{ severity: 'error', message: reply.error }]);
  } else {
    renderDiagnostics(reply.findings);
    say(
      reply.findings.length === 0
        ? 'Nothing obviously wrong — this is not a proof of success.'
        : `${reply.findings.length} finding(s).`,
    );
  }
  show('output');
};

const reset = async () => {
  if (!window.confirm('Discard the flow and every word you have defined?')) return;
  journal.length = 0;
  const reply = await ask('reset');
  render(reply);
  renderDiagnostics([]);
  el.editor.value = '';
  sheet = 'core';
  el.sheet.value = 'core';
  renderDictionary();
  // A full reset returns the layout to where a session starts, rather than
  // leaving you looking at the empty dictionary you just cleared.
  layout = { left: 'input', right: 'stack' };
  mobileSurface = 'input';
  applyLayout();
  el.editor.focus();
  say('Session reset.');
};

// -------------------------------------------------------------------- events

el.editor.addEventListener('input', () => {
  renderStack();
  if (!el.suggestions.hidden) showSuggestions();
});
el.editor.addEventListener('blur', () => window.setTimeout(hideSuggestions, 150));

document.addEventListener('keydown', (event) => {
  if (event.key === 'Escape') {
    if (!el.suggestions.hidden) return hideSuggestions();
    abort().then((aborted) => {
      if (!aborted && !isMobile()) show('input');
    });
    return;
  }
  if (event.key === 'Enter' && event.shiftKey && !event.ctrlKey && !event.altKey) {
    event.preventDefault();
    run();
    return;
  }
  if (event.key === 'Enter' && event.ctrlKey && event.altKey) {
    event.preventDefault();
    reset();
    return;
  }
  if (event.key === 'Enter' && event.ctrlKey) {
    event.preventDefault();
    step();
    return;
  }
  if (event.key === ' ' && event.ctrlKey) {
    event.preventDefault();
    showSuggestions();
    return;
  }
  if ((event.key === 'f' || event.key === 'F') && event.shiftKey && event.altKey) {
    event.preventDefault();
    format();
  }
});

$('run').addEventListener('click', run);
$('step').addEventListener('click', step);
$('format').addEventListener('click', format);
$('reset').addEventListener('click', reset);
$('lint').addEventListener('click', runLint);
$('copy-output').addEventListener('click', async () => {
  const text = [...el.output.children].map((line) => line.textContent).join('\n');
  try {
    await navigator.clipboard.writeText(text);
    say('Output copied.');
  } catch {
    say('The browser refused clipboard access.');
  }
});
$('export-words').addEventListener('click', async () => {
  const text = snapshot.definitions
    .map((word) => `${word.body} "${word.name}" DEF`)
    .join('\n');
  try {
    await navigator.clipboard.writeText(text);
    say(`Copied ${snapshot.definitions.length} definition(s).`);
  } catch {
    say('The browser refused clipboard access.');
  }
});

el.leftSelect.addEventListener('change', (event) => show(event.target.value));
el.rightSelect.addEventListener('change', (event) => show(event.target.value));
el.mobileSelect.addEventListener('change', (event) => show(event.target.value));
el.sheet.addEventListener('change', (event) => {
  sheet = event.target.value;
  renderDictionary();
});
el.search.addEventListener('input', renderDictionary);

// Clicking Output on a two-column layout goes back to editing.
el.areas.output.addEventListener('click', (event) => {
  if (isMobile() || event.target.closest('button')) return;
  show('input');
  el.editor.focus();
});

window.addEventListener('resize', applyLayout);

// ------------------------------------------------------------------ gestures

/** Count taps within a short window, so triple-tap and double-tap are distinct. */
const tapCounter = (node, count, action) => {
  let taps = 0;
  let timer = null;
  node.addEventListener('pointerup', (event) => {
    if (!isMobile() || event.target.closest('button, select, input')) return;
    taps += 1;
    window.clearTimeout(timer);
    if (taps >= count) {
      taps = 0;
      action();
      return;
    }
    timer = window.setTimeout(() => {
      taps = 0;
    }, 400);
  });
};

tapCounter(el.editor, 3, run);
tapCounter(el.areas.stack, 2, () => show('output'));
tapCounter(el.areas.output, 2, () => {
  show('input');
  el.editor.focus();
});

let swipeFrom = null;
document.addEventListener('pointerdown', (event) => {
  swipeFrom = { x: event.clientX, y: event.clientY };
});
document.addEventListener('pointerup', (event) => {
  if (!swipeFrom || !isMobile()) return;
  const dx = event.clientX - swipeFrom.x;
  const dy = event.clientY - swipeFrom.y;
  swipeFrom = null;
  if (Math.abs(dx) <= Math.abs(dy) || Math.abs(dx) <= SWIPE_THRESHOLD) return;
  show(cycle(mobileSurface, dx > 0 ? 'right' : 'left'));
});

// --------------------------------------------------------------------- start

const PALETTE = [
  'ADD', 'SUB', 'MUL', 'DIV', 'EQ', 'LT', 'GT',
  'NOT', 'AND', 'OR', 'TRUE', 'FALSE', 'NIL', 'UNKNOWN',
  'DUP', 'DROP', 'SWAP', 'TOP', 'STAK', 'EAT', 'KEEP', 'VENT',
  'LENGTH', 'NTH', 'FIRST', 'REST', 'APPEND', 'CONCAT', 'RANGE',
  'MAP', 'FILTER', 'FOLD', 'EXEC', 'DEF',
];

const buildPalette = () => {
  el.palette.replaceChildren(
    ...PALETTE.map((name) => {
      const button = document.createElement('button');
      button.type = 'button';
      button.textContent = name;
      button.addEventListener('click', () => insert(name));
      return button;
    }),
  );
};

const PLACEHOLDER_DESKTOP = [
  'Enter code here',
  '',
  'Run      → Shift+Enter',
  'Step     → Ctrl+Enter',
  'Format   → Shift+Alt+F',
  'Suggest  → Ctrl+Space',
  'Reset    → Ctrl+Alt+Enter',
  'Abort    → Escape',
].join('\n');

const PLACEHOLDER_MOBILE = [
  'Enter code here',
  '',
  'Run              → triple-tap here',
  'Stack → Output   → double-tap Stack',
  'Output → editor  → double-tap Output',
  'Change panel     → swipe, or use Panel',
  'Input assist     → tap the words below',
].join('\n');

const setPlaceholder = () => {
  el.editor.placeholder = isMobile() ? PLACEHOLDER_MOBILE : PLACEHOLDER_DESKTOP;
};
window.addEventListener('resize', setPlaceholder);

const start = async () => {
  spawn();
  buildPalette();
  setPlaceholder();
  applyLayout();
  renderDiagnostics([]);
  try {
    const manifest = await ask('vocabulary');
    vocabulary = manifest.words;
    const reply = await ask('snapshot');
    render(reply);
    say(`Ready — ${vocabulary.length} words.`);
    document.body.dataset.ready = 'true';
  } catch (error) {
    say(`Could not start: ${error.message}`);
  }
};

start();
