// The playground, driven in a real browser.
//
// The rules live in `presentation.js` and are tested on their own; this checks
// that they are actually wired to the elements, the keys, and the gestures —
// the part a pure test cannot see. It found two defects the first time it ran:
// a `Lint` button that lived on a panel you cannot see while typing, and a
// `display: flex` that beat `[hidden]`.
//
// Requires the module to have been built:
//   cargo build -p ajisai-wasm --target wasm32-unknown-unknown --release

import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('.', import.meta.url));

const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.wasm': 'application/wasm',
  '.json': 'application/json',
};

let server;
let origin;
let browser;
let page;
const complaints = [];

before(async () => {
  server = createServer(async (request, response) => {
    const path = normalize(decodeURIComponent(new URL(request.url, 'http://x').pathname));
    const file = join(root, path === '/' ? 'index.html' : path);
    if (!file.startsWith(root)) {
      response.writeHead(403).end();
      return;
    }
    try {
      const body = await readFile(file);
      response.writeHead(200, { 'content-type': TYPES[extname(file)] ?? 'application/octet-stream' });
      response.end(body);
    } catch {
      response.writeHead(404).end();
    }
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  origin = `http://127.0.0.1:${server.address().port}`;

  const { chromium } = await import('playwright');
  browser = await chromium.launch();
});

after(async () => {
  await browser?.close();
  await new Promise((resolve) => server.close(resolve));
  assert.deepEqual(complaints, [], 'the console should stay quiet');
});

const open = async ({ width, height, touch = false }) => {
  page = await browser.newPage({
    viewport: { width, height },
    hasTouch: touch,
    isMobile: touch,
  });
  page.on('pageerror', (error) => complaints.push(String(error)));
  page.on('console', (message) => {
    if (message.type() === 'error') complaints.push(message.text());
  });
  page.on('dialog', (dialog) => dialog.accept());
  await page.goto(`${origin}/index.html`);
  await page.waitForSelector('body[data-ready="true"]', { timeout: 30000 });
  return page;
};

const visible = async () => {
  const shown = [];
  for (const name of ['input', 'output', 'stack', 'dictionary']) {
    if (!(await page.locator(`#${name}-area`).isHidden())) shown.push(name);
  }
  return shown;
};

const stack = () => page.locator('#stack .cell').allTextContents();

const settle = () => page.waitForTimeout(300);

// --------------------------------------------------------------- two columns

test('a two-column layout starts on Input and Stack', async () => {
  await open({ width: 1280, height: 800 });
  assert.equal(await page.getAttribute('body', 'data-layout'), 'desktop');
  assert.deepEqual(await visible(), ['input', 'stack']);
  await page.close();
});

test('Shift+Enter runs, and the flow appears', async () => {
  await open({ width: 1280, height: 800 });
  await page.fill('#editor', '1 2 3 STAK ADD');
  await page.locator('#editor').press('Shift+Enter');
  await settle();
  assert.deepEqual(await stack(), ['6']);
  await page.close();
});

test('the stack shows which cells are operands and what becomes of them', async () => {
  await open({ width: 1280, height: 800 });
  await page.fill('#editor', '1 2 3');
  await page.locator('#editor').press('Shift+Enter');
  await settle();

  // Default: the surface cell is the operand, and it is eaten.
  await page.fill('#editor', 'ADD');
  await settle();
  assert.deepEqual(await page.locator('#stack .cell').evaluateAll((cells) =>
    cells.map((cell) => cell.dataset.operand ?? null),
  ), [null, null, 'eat']);
  assert.equal(await page.textContent('#mode-hint'), 'the surface · eaten');

  // STAK KEEP: every cell is an operand, and they are kept.
  await page.fill('#editor', ': & +');
  await settle();
  assert.deepEqual(await page.locator('#stack .cell').evaluateAll((cells) =>
    cells.map((cell) => cell.dataset.operand ?? null),
  ), ['keep', 'keep', 'keep']);
  assert.equal(await page.textContent('#mode-hint'), 'the whole flow · kept');
  await page.close();
});

test('a definition moves the right column to the words you defined', async () => {
  await open({ width: 1280, height: 800 });
  await page.fill('#editor', '{ 2 MUL } "DOUBLE" DEF');
  await page.locator('#editor').press('Shift+Enter');
  await settle();
  assert.deepEqual(await visible(), ['input', 'dictionary']);
  assert.equal(await page.inputValue('#sheet-select'), 'user');
  assert.deepEqual(await page.locator('#dictionary .word-name').allTextContents(), ['DOUBLE']);
  await page.close();
});

test('a failure moves the left column to the diagnostic', async () => {
  await open({ width: 1280, height: 800 });
  await page.fill('#editor', '1 0 DIV');
  await page.locator('#editor').press('Shift+Enter');
  await settle();
  assert.ok((await visible()).includes('output'));
  assert.match(await page.textContent('#output li'), /division by zero/);
  await page.close();
});

test('Ctrl+Enter runs one unit, and a vent keeps its unit', async () => {
  await open({ width: 1280, height: 800 });
  await page.fill('#editor', 'TRUE VENT { 1 2 ADD } 7');
  await page.locator('#editor').press('Control+Enter');
  await settle();
  assert.equal(await page.inputValue('#editor'), 'VENT { 1 2 ADD } 7');
  await page.locator('#editor').press('Control+Enter');
  await settle();
  // The vent released its unit in one step, rather than being split from it.
  assert.deepEqual(await stack(), ['3']);
  await page.close();
});

test('Shift+Alt+F formats to canonical words', async () => {
  await open({ width: 1280, height: 800 });
  await page.fill('#editor', '1 2 & + : ^ { 3 }');
  await page.keyboard.press('Shift+Alt+KeyF');
  await settle();
  assert.equal(await page.inputValue('#editor'), '1 2 KEEP ADD STAK VENT { 3 }');
  await page.close();
});

test('Lint reports without refusing to run, and is reachable while typing', async () => {
  await open({ width: 1280, height: 800 });
  await page.fill('#editor', '[ 1 ] 2 ADD');
  // The button must be on the panel you are typing on.
  assert.ok(await page.locator('#lint').isVisible());
  await page.click('#lint');
  await settle();
  assert.match(await page.textContent('#output li'), /expected number/);
  // ...and the program still runs.
  await page.locator('#editor').press('Shift+Enter');
  await settle();
  assert.match(await page.textContent('#status'), /expected number/);
  await page.close();
});

test('Ctrl+Space suggests words by prefix', async () => {
  await open({ width: 1280, height: 800 });
  await page.fill('#editor', 'CON');
  await page.locator('#editor').press('Control+Space');
  await settle();
  const shown = await page.locator('#suggestions button').allTextContents();
  assert.ok(shown.includes('CONCAT'), shown.join(' '));
  await page.click('#suggestions button');
  assert.equal(await page.inputValue('#editor'), 'CONCAT ');
  await page.close();
});

test('clicking a dictionary word puts it in the editor', async () => {
  await open({ width: 1280, height: 800 });
  await page.selectOption('#right-select', 'dictionary');
  // Opening the dictionary returns the left column to the editor.
  assert.deepEqual(await visible(), ['input', 'dictionary']);
  await page.fill('#search', 'VENT');
  await settle();
  await page.locator('#dictionary .word-name').first().click();
  assert.equal(await page.inputValue('#editor'), 'VENT ');
  await page.close();
});

test('Reset clears the flow and the words', async () => {
  await open({ width: 1280, height: 800 });
  await page.fill('#editor', '{ 1 } "X" DEF 9');
  await page.locator('#editor').press('Shift+Enter');
  await settle();
  await page.keyboard.press('Control+Alt+Enter');
  await settle();
  assert.equal(await page.inputValue('#editor'), '');
  assert.deepEqual(await stack(), []);
  assert.deepEqual(await visible(), ['input', 'stack']);
  await page.close();
});

test('the session survives across separate runs', async () => {
  await open({ width: 1280, height: 800 });
  await page.fill('#editor', '1 2');
  await page.locator('#editor').press('Shift+Enter');
  await settle();
  await page.fill('#editor', 'ADD');
  await page.locator('#editor').press('Shift+Enter');
  await settle();
  assert.deepEqual(await stack(), ['3']);
  await page.close();
});

// ------------------------------------------------------------ single surface

test('a narrow layout shows one surface at a time', async () => {
  await open({ width: 390, height: 844, touch: true });
  assert.equal(await page.getAttribute('body', 'data-layout'), 'mobile');
  assert.deepEqual(await visible(), ['input']);
  await page.close();
});

test('swiping cycles the surfaces both ways', async () => {
  await open({ width: 390, height: 844, touch: true });
  await page.selectOption('#mobile-select', 'stack');
  const box = await page.locator('#main').boundingBox();
  const swipe = async (direction) => {
    const y = box.y + box.height / 2;
    const [from, to] =
      direction === 'left'
        ? [box.x + box.width - 30, box.x + 30]
        : [box.x + 30, box.x + box.width - 30];
    await page.mouse.move(from, y);
    await page.mouse.down();
    await page.mouse.move(to, y, { steps: 8 });
    await page.mouse.up();
    await page.waitForTimeout(150);
  };
  await swipe('left');
  assert.deepEqual(await visible(), ['dictionary']);
  await swipe('left');
  assert.deepEqual(await visible(), ['input'], 'cycling wraps around');
  await swipe('right');
  assert.deepEqual(await visible(), ['dictionary']);
  await page.close();
});

test('a run surfaces the most notable change', async () => {
  await open({ width: 390, height: 844, touch: true });
  await page.fill('#editor', '1 2 ADD');
  await page.click('#run');
  await settle();
  assert.deepEqual(await visible(), ['stack']);

  await page.selectOption('#mobile-select', 'input');
  await page.fill('#editor', '{ 2 MUL } "DOUBLE" DEF');
  await page.click('#run');
  await settle();
  assert.deepEqual(await visible(), ['dictionary'], 'a definition outranks the flow');
  await page.close();
});

test('the tap gestures move between surfaces', async () => {
  await open({ width: 390, height: 844, touch: true });
  await page.fill('#editor', '10 20 ADD');

  // Triple-tap the editor to run.
  const editor = await page.locator('#editor').boundingBox();
  for (let i = 0; i < 3; i += 1) {
    await page.mouse.click(editor.x + editor.width / 2, editor.y + editor.height / 2);
  }
  await settle();
  assert.deepEqual(await visible(), ['stack']);
  assert.deepEqual(await stack(), ['30']);

  // Double-tap the flow to read the diagnostics.
  const area = await page.locator('#stack-area').boundingBox();
  for (let i = 0; i < 2; i += 1) {
    await page.mouse.click(area.x + area.width / 2, area.y + area.height - 20);
  }
  await settle();
  assert.deepEqual(await visible(), ['output']);

  // Double-tap the diagnostics to get back to the editor.
  const output = await page.locator('#output-area').boundingBox();
  for (let i = 0; i < 2; i += 1) {
    await page.mouse.click(output.x + output.width / 2, output.y + output.height - 20);
  }
  await settle();
  assert.deepEqual(await visible(), ['input']);
  await page.close();
});

test('the narrow layout never scrolls sideways', async () => {
  await open({ width: 320, height: 568, touch: true });
  for (const surface of ['input', 'output', 'stack', 'dictionary']) {
    await page.selectOption('#mobile-select', surface);
    const overflows = await page.evaluate(
      () => document.documentElement.scrollWidth > window.innerWidth,
    );
    assert.equal(overflows, false, `${surface} overflows`);
  }
  await page.close();
});
