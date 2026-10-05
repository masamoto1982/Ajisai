#!/usr/bin/env node
// Step 3/4 tables from blocks.json + meta.json (stats.mjs).
//   node scripts/bench/fusion-stats/report.mjs <out-dir>   → <out-dir>/tables.md and tables.json
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const dir = process.argv[2];
const blocks = JSON.parse(readFileSync(join(dir, "blocks.json"), "utf8"));
const meta = JSON.parse(readFileSync(join(dir, "meta.json"), "utf8"));
const KINDS = ["test", "example", "bench", "business"];
const work = blocks.filter((b) => b.n > 0);

// classification of a block that did work
function klass(b) {
  if (b.fusedCalls === b.workCalls) return "fused";
  if (b.loweredCalls === 0) return b.reasons.length ? "word" : "value-lowering"; // lowering declined with no static reason: an outer name whose value is not plain
  if (b.loweredCalls === b.workCalls) return "value-run";
  return "mixed";
}
const key = (r) => r.reason.replace(/^word /, "");
const keysOf = (b) => [...new Set(b.reasons.map(key))];
const pct = (a, b) => (b ? (100 * a / b).toFixed(1) + "%" : "–");
const md = []; const J = {};

// 1. fusion rate
md.push("## 融合率", "", "| 種別 | 実行されたブロック | 融合したブロック | ブロック率 | 処理要素 | 融合した要素 | 要素率 | 語で止まる | 値で止まる（実行時） | 値で止まる（外部束縛） | 混在 | 要素0のブロック |", "|---|---|---|---|---|---|---|---|---|---|---|---|");
for (const k of [...KINDS, "all"]) {
  const bs = blocks.filter((b) => k === "all" || b.kind === k); const ws = bs.filter((b) => b.n > 0);
  const c = (x) => ws.filter((b) => klass(b) === x).length;
  const n = ws.reduce((a, b) => a + b.n, 0), nf = ws.reduce((a, b) => a + b.nFused, 0);
  J[`rate_${k}`] = { blocks: ws.length, fused: c("fused"), n, nf };
  md.push(`| ${k} | ${ws.length} | ${c("fused")} | ${pct(c("fused"), ws.length)} | ${n} | ${nf} | ${pct(nf, n)} | ${c("word")} | ${c("value-run")} | ${c("value-lowering")} | ${c("mixed")} | ${bs.length - ws.length} |`);
}
md.push("", `By Word (all kinds): ` + ["MAP", "FILTER", "FOLD", "SCAN"].map((h) => { const ws = work.filter((b) => b.hof === h); return `${h} ${ws.filter((b) => klass(b) === "fused").length}/${ws.length}`; }).join(", "));
md.push("", `Blocks handed to DEF ${meta.nonHof.DEF}, to EXEC ${meta.nonHof.EXEC} (not fusion candidates). Calls linked exact ${meta.link.exact}, ordinal ${meta.link.ordinal}, unlinked ${meta.link.unlinked} (${meta.unlinkedN} elements).`);
md.push(`Static lowering vs runtime: agree ${meta.validation.lowerAgree}, disagree ${meta.validation.lowerDisagree}.`, "");

// 2. ranking (word-declined blocks)
const wordBlocks = work.filter((b) => klass(b) === "word");
function ranking(set, label) {
  const first = new Map(), single = new Map(), any = new Map();
  const bump = (m, k, b) => { const e = m.get(k) ?? { blocks: 0, n: 0, byKind: {} }; e.blocks++; e.n += b.n; e.byKind[b.kind] = (e.byKind[b.kind] ?? 0) + 1; m.set(k, e); };
  for (const b of set) { const ks = keysOf(b); bump(first, key(b.reasons[0]), b); for (const k of ks) bump(any, k, b); if (ks.length === 1) bump(single, ks[0], b); }
  const all = new Set([...first.keys(), ...any.keys()]);
  const rows = [...all].map((k) => ({ k, first: first.get(k) ?? { blocks: 0, n: 0, byKind: {} }, single: single.get(k) ?? { blocks: 0, n: 0, byKind: {} }, any: any.get(k) ?? { blocks: 0, n: 0, byKind: {} } }))
    .sort((a, b) => b.single.blocks - a.single.blocks || b.first.blocks - a.first.blocks || b.any.blocks - a.any.blocks);
  md.push(`## 止めた原因のランキング（${label}）`, "", "| 原因 | 最初に止めた（ブロック） | 同（要素） | それだけ追加で乗る（ブロック） | 同（要素） | 含まれる（ブロック） | 追加しても別の原因で止まる | 種別内訳（最初に止めた） |", "|---|---|---|---|---|---|---|---|");
  for (const r of rows) md.push(`| ${r.k} | ${r.first.blocks} | ${r.first.n} | ${r.single.blocks} | ${r.single.n} | ${r.any.blocks} | ${r.any.blocks - r.single.blocks} | ${Object.entries(r.first.byKind).map(([a, b]) => `${a} ${b}`).join(", ")} |`);
  md.push("");
  return rows;
}
J.rankingAll = ranking(wordBlocks, "全種別");
J.rankingNonTest = ranking(wordBlocks.filter((b) => b.kind !== "test"), "テスト以外");
J.rankingTest = ranking(wordBlocks.filter((b) => b.kind === "test"), "テストのみ");

// 3. combinations
const combos = new Map();
for (const b of wordBlocks) { const ks = keysOf(b).sort(); if (ks.length < 2) continue; const k = ks.join(" + "); const e = combos.get(k) ?? { blocks: 0, n: 0, kinds: {} }; e.blocks++; e.n += b.n; e.kinds[b.kind] = (e.kinds[b.kind] ?? 0) + 1; combos.set(k, e); }
md.push("## 2 つ以上の原因の組み合わせ（上位 10）", "", "| 組み合わせ | ブロック | 要素 | 種別 |", "|---|---|---|---|");
for (const [k, e] of [...combos].sort((a, b) => b[1].blocks - a[1].blocks).slice(0, 10)) md.push(`| ${k} | ${e.blocks} | ${e.n} | ${Object.entries(e.kinds).map(([a, b]) => `${a} ${b}`).join(", ")} |`);
md.push(`| （組み合わせの総数） | ${[...combos.values()].reduce((a, e) => a + e.blocks, 0)} | | |`, "");
J.combos = [...combos];

// 4. value-type causes
const vrows = new Map();
const vbump = (k, b, n) => { const e = vrows.get(k) ?? { blocks: new Set(), calls: 0, n: 0, kinds: {} }; if (!e.blocks.has(b)) e.kinds[b.kind] = (e.kinds[b.kind] ?? 0) + 1; e.blocks.add(b); e.calls++; e.n += n; vrows.set(k, e); };
for (const b of work) {
  if (klass(b) === "value-lowering") vbump("外部の名前の値が有理数・真偽値でない（lowering 時）", b, b.n);
  for (const v of b.valueDeclined) {
    const t = v.target ?? {}, s = v.seed;
    const el = t.elements ?? {};
    let why;
    if (v.elems < v.n) why = "ERROR で中断（演算に型違いの値: 文字列・真偽値など）";
    else if (Object.keys(el).some((x) => /Text|Symbol/.test(x))) why = "要素に文字列";
    else if (Object.keys(el).some((x) => /Nil|absent/.test(x))) why = "要素に NIL";
    else if (Object.keys(el).some((x) => /ExactScalar/.test(x))) why = "要素に代数的数（SQRT）";
    else if (Object.keys(el).some((x) => /Vector|Tensor/.test(x))) why = "要素がベクトル（ネスト）";
    else if (Object.keys(el).some((x) => /Record/.test(x))) why = "要素に Record";
    else if (s && (s.tag === "Tensor" || s.tag === "Vector")) why = s.n === 1 && s.rank !== 2
      ? (b.laneWalkable ? "seed が 1 要素ベクトル: 途中の値が機械語を超えた（lane walk は一般 tier を使わない）" : "seed が 1 要素ベクトル: ブロックの形が lane walk の条件外（比較・丸め等が累積値に触れる）")
      : "seed がベクトル（複数要素・ネスト）";
    else if (s && s.tag !== "Scalar" && s.tag !== "Boolean") why = `seed が ${s.tag}`;
    else why = "要素・seed とも素の値（ゼロ除算の NIL・上限など。内訳は測っていない）";
    vbump(why, b, v.n);
  }
}
md.push("## 値が原因で止まったブロック", "", "| 原因 | ブロック | 呼び出し | 要素 | 種別 |", "|---|---|---|---|---|");
for (const [k, e] of [...vrows].sort((a, b) => b[1].blocks.size - a[1].blocks.size)) md.push(`| ${k} | ${e.blocks.size} | ${e.calls} | ${e.n} | ${Object.entries(e.kinds).map(([a, b]) => `${a} ${b}`).join(", ")} |`);
md.push("");
J.valueCauses = [...vrows].map(([k, e]) => ({ k, blocks: e.blocks.size, calls: e.calls, n: e.n, kinds: e.kinds }));

// 5. nesting and User Words
const nested = wordBlocks.filter((b) => keysOf(b).some((k) => k.startsWith("nested block")));
const nestedOnly = nested.filter((b) => keysOf(b).every((k) => k.startsWith("nested block")));
const uw = work.filter((b) => b.userWords.length);
const uwStop = wordBlocks.filter((b) => b.reasons.some((r) => r.via));
const uwStopOnly = uwStop.filter((b) => b.reasons.every((r) => r.via));
md.push("## ネストしたブロックとユーザー定義語", "",
  `- 中に別のブロック（MAP/FILTER/FOLD/SCAN/EXEC/DEF に渡すもの）を含むために止まったブロック: ${nested.length}（うちそれだけが原因: ${nestedOnly.length}、要素 ${nested.reduce((a, b) => a + b.n, 0)}）`,
  `- ユーザー定義語を呼ぶブロック: ${uw.length}（融合した ${uw.filter((b) => klass(b) === "fused").length}）`,
  `- ユーザー定義語の本体が原因で止まったブロック: ${uwStop.length}（本体だけが原因: ${uwStopOnly.length}）`, "",
  "| ブロック | 呼ぶ語 | 本体が融合サブセット内か | 結果 | 種別 |", "|---|---|---|---|---|");
for (const b of uw) md.push(`| \`${b.text.slice(0, 70)}\` | ${b.userWords.join(", ")} | ${Object.entries(b.bodyFusible).map(([n, f]) => `${n}: ${f ? "内" : "外（" + b.reasons.filter((r) => r.via === n).map(key).join(", ") + "）"}`).join("; ")} | ${klass(b)} | ${b.kind} |`);
md.push("");

writeFileSync(join(dir, "tables.md"), md.join("\n"));
writeFileSync(join(dir, "tables.json"), JSON.stringify(J, null, 1));
console.log(md.join("\n"));
