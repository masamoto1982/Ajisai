# Recordにリテラルを与える——`{ }` を割り当て、`COLLECT` 形を廃する（2026-09-22）

Status: 非正典。正典は `spec/grammar.json`・`spec/language-semantics.md` と、
そこから生成される `SPECIFICATION.html`。本書は割り当ての根拠と、その代わりに
何を手放したかを記録する。

## 1. 何をしたか

Record に字面（リテラル）を与えた。`{ key value … }` で、キーとその値が横に並ぶ。

| | 旧 | 新 |
| --- | --- | --- |
| Record | `[ 'x' 'y' ] [ 1/1 2/1 ] RECORD` | `{ 'x' 1/1 'y' 2/1 }` |
| 空 Record | `[ ] [ ] RECORD` | `{ }` |
| Record を含む Vector | `[ 'a' ] [ 1/1 ] RECORD 1 COLLECT` | `[ { 'a' 1/1 } ]` |
| `CONTRACT` の戻り値 | 9要素の `COLLECT` 形（`docs/dev/record-display-round-trip-2026-09.md` §5） | 素直な入れ子 |

`RECORD` は残る。字面は**定数**の Record を書く形、`RECORD` は**計算した**キー列と
値列から組み立てる語であり、片方で他方は書けない。語彙は 100 語のまま、
字句は `{` `}` の2文字だけ増えた。

## 2. なぜ [方針記録]

直前の2つの判断が、合わせて一つの穴を作っていた。

1. `record-display-round-trip-2026-09.md`——「表示はその値を再現するソースであれ」を
   採り、Record にリテラルが無いので表示を `[ keys ] [ values ] RECORD` にした。
   代償は可読性（キーと値が横に並ばない）で、`CONTRACT` が最悪ケース。
2. `source-character-liberation-2026-09.md`——`( ) { } |` の文字単位拒否規則を全廃し、
   普通の名前文字に戻した。

つまり **`{ }` は空いており、往復性のために可読性を払っていた**。所有者の判断は、
その2文字を払って両方を取ること。`( )` `|` は解放されたままである。

割り当ての根拠は `rust/src/surface_forms.rs` が既に書いていた規律そのもの——
「その文字が語の規則を超えた何かをするなら表に載る」。`{ }` は Record の字面を
区切るのだから載る。**死んだ概念名の再投入**（解放メモ §4 が警告した欠陥）とは
別物であり、その区別は `a_forms_kind_agrees_with_what_the_tokenizer_accepts` が
実測で守る。

## 3. 設計判断 [設計根拠]

**(a) 区切りは空白、`:` は使わない。** 理由は二つある。一つは文書側の負担で、
`:` は散文と道具立ての両方で稼働中の区切りなので、字句的意味を与えると Reference が
書きにくくなる（規準と実例は `character-allocation-and-prose-2026-09.md`）。
もう一つは、払っても得られる形が小さいことである——`{ 'x': 1 }` は**字句的に書けない**。
引用符は次が空白か入力末尾のときにだけ閉じるので（`spec/grammar.json`
stringLiteral.closeRule）、`'x':` は `unclosedLiteral` になる。これは解放メモ §3 の
実測が既に示していたことで、Text キーを持つ Record が読み戻せなかった真因である。
したがって要素は空白区切りで交互に並べる。

**(b) 字面は評価しない。** `[ ... ]` と同じ規則を共有する（`vector_literal.rs` の
`collect_literal_elements`）。裸の名前はシンボル、`TRUE`/`FALSE`/`NIL` だけが値。
これにより `{ 'op' ADD }` は辞書状態に依らず同じ値であり、また
**Record を含む Vector がリテラルとして正しく読み戻る**——`COLLECT` 形が要らなく
なった理由はここにある。

**(c) エラーは構成子と同じものだけ。** 要素数が奇数なら「キー列が値列より1つ長い」
＝ `vectorLengthMismatch`、キーの重複は `duplicateKey`。新しい結末を一つも
増やしていない。キーの同一性は値の同一性（`1` と `1/1` は同じキー）なので、
これらは字句の問題ではなく**構築時**に決まる。ゆえに source error ではない。

**(d) 静的予測は近似しない。** 字面は定数なので、`predict_program_outcomes` は
予測時に実際に組み立てて答える（`{ 'a' 1 'a' 2 }` は必ず `duplicateKey`、
`{ 'a' 1 'b' 2 }` は一つも足さない）。過大近似は健全だが、ここでは不要だった。

**(e) `[ }` は `mismatched code delimiter`。** 字句の前検査（`check_bracket_matching`）
は両ペアを一つのスタックで見るが、**交差したときは報告せず止まる**。前検査は
「取りこぼしてよいが捏造してはならない」と文法が規定しており、オープナを失った
スタックの先を読めば捏造が始まる。判定は token 側の `validate_code_tokens` が持つ。

## 4. 代償——何を手放したか

- **`{ }` は名前の一部に使えなくなった。** `a{b}` は `bracketMustStandAlone`
  （whole-lexeme 規則）で拒否される。per-character 拒否は復活していない：
  字句は「ちょうど一つの区切り文字」か「区切り文字を含まない」かのどちらかである。
- **`'{' DEF` が通らなくなった。** 区切り文字は名前ではないので定義名にできない
  （`a_delimiter_is_not_a_definable_name`）。
- **`{ ... }` の旧ブロック構文は「エラー」から「Record を作って `DEF` が拒む」に
  変わった。** 退役した形が動かないことは変わらない（`definable_name_tests.rs`）。
- **compiled plan は Record 字面を下ろさない。** 定数なのに `FallbackToken` で
  解釈経路に落とす。構成子の ERROR をコンパイル時に持ち込む設計判断を、
  測定が要求するまで保留した（`compiled_plan.rs`）。

## 5. 検査

- `rust/tests/round_trip_laws.rs`——表示を**実行**して同じ値が出ることを要求する法則。
  `a_collection_renders_as_its_own_literal` で綴りも固定した（往復するだけでは
  間違った綴りを許してしまう）。
- `rust/src/lexical_grammar_laws.rs::a_delimiter_stands_alone`——`delimiterPairs` を
  データとして読み、各ペアがトークンになることと、糊付きが拒否されることを実測。
  per-character 規則の削除が残した保証をここが引き継ぐ。
- `tests/conformance/index.html`——字面が構成子と同じ値・同じエラーを出す6件を追加。
- `src/gui/output-display-renderer.test.ts`——GUI 側の独立実装を、エンジンが実際に
  出力した文字列に対して固定（期待値は `ajisai run` から採取）。
- `scripts/generate-skill-md.mjs`——§2/§3 の波括弧断片を**実行して Record を答えるか**
  検査する。旧検査は「波括弧は正当なソースでない」を固定しており、解放後は
  それ自身が偽になっていた（SKILL.md §2 にその一文が残っていた）。
