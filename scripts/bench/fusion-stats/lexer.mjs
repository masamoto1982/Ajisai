// Tokens with source spans (1-based line/column in characters), following
// rust/src/tokenizer.rs: whitespace (Unicode White_Space, as spec/grammar.json
// enumerates it) is the only delimiter, `#` at a word position comments to end
// of line, `'` opens a string that a quote followed by whitespace or end of
// input closes. Token values are cross-checked against
// scripts/lib/reference-lexer.mjs by lexProgram().
import { loadGrammar, makeLexer } from "../../lib/reference-lexer.mjs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const reference = makeLexer(loadGrammar(repoRoot));
const WS = new RegExp("[\\u0009-\\u000D\\u0020\\u0085\\u00A0\\u1680\\u2000-\\u200A\\u2028\\u2029\\u202F\\u205F\\u3000]", "u");
const NUMBER = /^[+-]?\d+(\.\d+)?([eE][+-]?\d+)?(\/[+-]?\d+(\.\d+)?([eE][+-]?\d+)?)?$/;

function spans(src) {
  const chars = [...src]; const out = []; let i = 0, line = 1, col = 1;
  const bump = () => { if (chars[i] === "\n") { line++; col = 1; } else col++; i++; };
  while (i < chars.length) {
    const c = chars[i];
    if (WS.test(c)) { bump(); continue; }
    if (c === "#") { while (i < chars.length && chars[i] !== "\n") bump(); continue; }
    const at = { line, col };
    if (c === "'") {
      let j = i + 1; let close = -1;
      for (; j < chars.length; j++) if (chars[j] === "'" && (j + 1 >= chars.length || WS.test(chars[j + 1]))) { close = j; break; }
      if (close < 0) return null;
      const text = chars.slice(i + 1, close).join("");
      while (i <= close) bump();
      out.push({ kind: "string", value: text, ...at }); continue;
    }
    let j = i; while (j < chars.length && !WS.test(chars[j])) j++;
    const lexeme = chars.slice(i, j).join("");
    while (i < j) bump();
    if (lexeme === "[") out.push({ kind: "open", value: lexeme, ...at });
    else if (lexeme === "]") out.push({ kind: "close", value: lexeme, ...at });
    else if (NUMBER.test(lexeme)) out.push({ kind: "number", value: lexeme, ...at });
    else out.push({ kind: "symbol", value: lexeme, ...at });
  }
  return out;
}

const KIND = { Number: "number", Symbol: "symbol", String: "string", VectorStart: "open", VectorEnd: "close" };

// → { tokens } or { error }
export function lexProgram(src) {
  const ref = reference(src);
  if (ref.condition) return { error: ref.condition };
  const toks = spans(src);
  if (!toks || toks.length !== ref.tokens.length) return { error: "span lexer disagrees with reference lexer (count)" };
  for (let k = 0; k < toks.length; k++) {
    const r = ref.tokens[k];
    if (KIND[r.id] !== toks[k].kind || r.value !== toks[k].value) return { error: `span lexer disagrees at token ${k}: ${r.id} ${r.value} vs ${toks[k].kind} ${toks[k].value}` };
  }
  return { tokens: toks };
}
