# Safe Mode 廃止指示書のレビューと改訂案（Gate / Water Level モデル）

本書は、外部から提示された「包括的 Safe Mode の廃止と『水門・水位』モデルへの移行」指示書を
現行 `SPECIFICATION.md`・実装・README に照らして評価し、安全に実行できる形へ改訂したものである。

- 種別: 非 canonical な開発メモ（`docs/dev/`）。`SPECIFICATION.md` と矛盾する場合は仕様が優先。
- 結論: **元指示書はそのまま実行してはならない。** 中心的前提が実態と異なり、推奨手順が破壊的かつ自己矛盾を含む。
  ただし「安全性を通常意味論と境界制御へ分解する」という最終方針は妥当であり、その大半は**すでに実現済み**である。

### 実施状況（owner 承認済み・非破壊分を反映）

本書 §4.1 のうち、破壊を伴わない以下を実装済み。`SAFE`(`~`)・意味論・トークン・型・テストは無変更。

- README に「Safety model: safe by design, with gates and water levels」節を追加。`SAFE` を「廃止」ではなく
  「水路エラーの明示的スピルウェイ」として正確に説明し、Gate / Water Level を既存機構の総称として導入。
- `SPECIFICATION.md` に **非規範の** 「Appendix A. Gates and Water Levels」を追加（既存節へのインデックスのみ。
  新規規範・型・word・protocol field は無し）。既存番号は不変。
- 付随バグ修正: `SPECIFICATION.md` の重複していた節番号 `11.3`（Equal-value output と Safe mode behavior）を
  Safe mode behavior 側で `11.4` へ是正し、README と Conformance 項目 9 の参照を追随。

### 追補（owner 決定）: `SAFE`(`~`) 修飾子の完全削除

本書 §2.2 / §4.2 は当初「`SAFE`(`~`) は safe-by-design の一部であり残す」を推奨し、削除は「独立した意味論変更
として別途決定すべき」とした。その後 owner が、Ajisai の核である「**できなかった → 泡 / 使い方が違う → エラー**」
という泡/エラー分離の crispness を優先し、`SAFE`(`~`) を**言語から完全に撤去する**ことを選択した（誤用は常に
可視のエラーとして伝播させ、値へ化けさせない）。

これは意図的な**破壊的・意味論変更**であり、「Safe Mode 整理のついで」ではない独立変更として実施する。本 PR は
これを反映し、上記の非破壊分に加えて以下を行う。`SAFE` は「残す」のではなく「無い」状態が正となる。

- 実装: `~` トークン / `Token::SafeMode` / `interp.safe_mode` / `NilReason::SafeCaught` /
  `AbsenceOrigin::SafeProjection` / `ErrorPhase::SafeProjection` / `ErrorFlowEventKind::Safe*` /
  `SAFE` core word を撤去。誤用エラーは射影せず伝播する。`elastic-safe`（`ElasticMode`、無関係）は不変。
- 仕様: §6.3「Safe mode modifier」を削除し §6.4/§6.5 を §6.3/§6.4 へ繰り上げ。§11.4 を「Error propagation」に置換。
  `caughtCategory` 絶対metadata欄・`safeCaught` reason・関連 Test Discipline（§15.2/§15.3/旧§15.4）・
  Conformance 項目 9 から SAFE 記述を除去。token 表・sugar 表・modifier 表から `~`/`SAFE` を除去。
- README: 「Safe mode」修飾子の記述を撤去し、「error-swallowing modifier は無い」と明記。
- 不変条件（§4.3）は維持: Bubble Rule、K3 真理表、comparison budget → UNKNOWN、step budget →
  `ExecutionLimitExceeded`、operational NIL passthrough は変更しない。

なお、`SAFE` が埋めていた唯一のニッチ（形を静的に保証できない入力の total 化）は撤去により失われる。異種入力を
扱う場合は、適用前の明示的な検証、または Bubble Rule で NIL 化される演算の利用へ寄せること。

---

## 1. 評価サマリ

元指示書の最終方針 ——「Safe Mode を消すこと自体が目的ではなく、安全性を通常の意味論と境界制御に分解する」——
は Ajisai の設計思想と一致する。しかし指示書の本文は、Ajisai に存在しない機構を前提に書かれているため、
記述どおりに実行するとコンパイル不能・仕様後退・テスト崩壊・意味論変更を招く。

問題は大きく 5 点。

1. **中心的前提が誤り**: 「通常評価全体を覆う包括的 Safe Mode」は Ajisai に存在しない。
2. **削除対象の取り違え**: 実在する "Safe Mode" は `~` / `SAFE` という単一の修飾子であり、これは
   廃止すべき危険機構ではなく、safe-by-design を支える既存部品である。
3. **自己矛盾**: §4.3 / §4.4 が新型 `GatePolicy` / `WaterLevelPolicy` を提案する一方、§11 非目標で
   「capability system の過剰な新規設計」「外部作用 word の追加」を禁止している。
4. **既実装の見落とし**: Gate 相当（IO/host 境界、module trust boundary）と
   Water Level 相当（step budget、comparison budget、`COMPARE-WITHIN`）はすでに仕様化・実装済み。
5. **非目標との衝突**: `SAFE` 削除は `NilReason::SafeCaught` を伴う NIL を消すため、
   非目標「Bubble/NIL 意味論を変えない」に反する。

したがって本改訂版は、**新規サブシステムの導入ではなく、既存機構への語彙統一（主にドキュメント整備）**として
タスクを再定義する。

---

## 2. 事実確認（現行 Ajisai における「安全性」の所在）

### 2.1 包括的 Safe Mode は存在しない

- グローバルな Safe Mode トグル・フラグは存在しない。フロントエンド（`src/`, `index.html`, `public/`）に
  該当 UI は無い。
- 実在するのは `interpreter_core.rs` の一過性フラグ `safe_mode: bool` のみで、これは `~` 修飾子の
  実装内部状態である。「次の 1 語」を実行する間だけ真になり、語の実行後に必ず偽へ戻る
  （`execution_loop.rs`、`control_cond.rs` でも save/restore される）。グローバルな実行モードではない。

→ 元指示書 §1・§2 の「通常モードは危険で Safe Mode だけが安全という誤解を招く」という動機は、
  そもそも該当する機構が無いため**前提が成立しない**。「A. 通常評価の安全切り替え」に分類される対象は存在しない。

### 2.2 実在する `SAFE`（`~`）は safe-by-design の一部

- `SPECIFICATION.md` §6.3「Safe mode modifier」、§11.3「Safe mode behavior」で定義される修飾子。
  `tokenizer.rs:187` が `~` を `Token::SafeMode` に写像する。
- 振る舞い（§11.3）: ガード対象の語が**エラーを投げた場合のみ**、スタックを直前スナップショットへ復元し、
  `absence.reason = safeCaught` かつ `caughtCategory`（例 `structureError`）を保持した NIL を 1 個積む。
  元のエラーは伝播しない。直接 Bubble/NIL は再ラップしない。
- 仕様は明確に「`SAFE` は通常の部分操作の主機構ではない（README §4 / §11.2 Bubble Rule が主機構）。
  `SAFE` はチャネルを壊しかねないエラーのための明示的スピルウェイ」と位置づける。
- すなわち `SAFE` は **泡/淀み/水路エラーの 4 区分（README "language in one picture"）を補完する
  「水路エラーをわざと泡へ落とす排水口」**であり、廃止対象ではない。

### 2.3 「水のメタファー」と安全 4 区分はすでに canonical

`README.md` の "The language in one picture" 表（Flow / Bubble / Stagnation / Channel error）と
`§4 Modifiers: gates, branches, and spillways` が、元指示書 §5 が求める統一表現を**すでに**提供している。

```
Flow          = 通常の値の流れ
Bubble / NIL  = 整形式だが値を生めなかった（operational absence）
Stagnation/U  = 値はあるが観測予算内で決められない（logical Unknown, K3）
Channel error = 使い方・形が不正（raised error、必要なら SAFE で射影）
```

### 2.4 「Water Level（量の制御）」はすでに存在する

| 元指示書の Water Level 項目 | 現行の実体 | 仕様参照 |
|---|---|---|
| evaluation step budget | step budget 100,000、超過で `ExecutionLimitExceeded` | §5.3, §11 |
| comparison budget / observation depth | 比較予算。**超過は U/Stagnation を返す** | §7.4.1 |
| 比較予算の明示制御 | `COMPARE-WITHIN`（budget を第一級パラメータ化） | §7.4.2 |
| comparison-undecided は NIL でなく UNKNOWN | すでにそう規定（U は `TruthValue`、NIL ではない） | §4.5.2, §7.4.1, §7.4.3 |

→ 元指示書 §4.4 が掲げる中核要件「比較予算切れは Bubble/NIL でなく Stagnation/UNKNOWN」は**達成済み**。
  step/expansion budget 超過時の structured Error（`ExecutionLimitExceeded`）も既存。

### 2.5 「Gate（境界の制御）」はすでに存在する

- 外向き: 外部副作用は Core に置かず、IO/semantic 境界で「host command」として発行され、host が実行する
  （§5.2、§9.4 SERIAL「serial access は host environment の性質であり runtime は port を直接開かない」）。
  host capability の不在は言語の意味エラーではなく環境条件、と明記済み。
- 内向き: module dictionary の `IMPORT` / `IMPORT-ONLY` / `UNIMPORT` による可視性制御と依存追跡（§7, §9）が
  trust-boundary crossing を担う。Core / Module / User の三層（§7）も境界の一部。

→ Gate は「新設すべき型」ではなく、**既存の境界群に与える総称**として扱うのが正しい。

---

## 3. 元指示書の項目別判定

| 元 § | 内容 | 判定 | 理由 / 対応 |
|---|---|---|---|
| 1, 2 | 包括的 Safe Mode の廃止 | **却下（前提誤り）** | 該当機構が存在しない |
| 3.1 | Core は safe-by-design | **採用（実現済み）** | README/§4/§11 で既述。明文の強化のみ可 |
| 3.2 | Gate = 境界制御 | **採用（命名のみ）** | 既存の IO/host・IMPORT 境界の総称として導入 |
| 3.3 | Water Level = 量制御 | **採用（命名のみ）** | step/comparison budget 等の総称として導入 |
| 4.1 | Safe Mode 参照の棚卸し | **限定採用** | 分類 A は空集合。B/C は「新設」でなく「既存への対応付け」 |
| 4.2 | Safe Mode の削除/legacy 化 | **却下** | `~`/`SAFE` は削除不可。意味論・トークン・`SafeCaught`・テストを破壊 |
| 4.3 | `GatePolicy` 新型 | **却下/保留** | §11 非目標と矛盾。新型は作らず既存境界の文書化に留める |
| 4.4 | `WaterLevelPolicy` 新型 | **却下/保留** | 同上。既存 budget の命名・診断統一に留める |
| 5 | 水メタファー公式表現 | **採用** | ただし「Safe Mode に依存しない」より「`SAFE` は排水口」と正確化 |
| 6 | README 更新 | **限定採用** | 「Safe Mode protects evaluation」という記述は**存在しない**。置換不要、追記のみ |
| 7 | SPECIFICATION 更新 | **限定採用** | §6.3/§11.3 は削除せず保持。Gate/Water Level の総称節を任意で追加 |
| 8 | UI 更新 | **却下（対象なし）** | Safe Mode トグル UI は存在しない |
| 9 | エラーメッセージ | **限定採用** | 「disabled in Safe Mode」文言は存在しない。境界/予算メッセージの整備は可 |
| 10 | テスト | **限定採用** | 既存 `interpreter_mode_tests.rs` の `~` テストは保持。命名変更は不可逆性に注意 |
| 11 | 非目標 | **採用** | 本改訂はこれを厳守する |
| 12, 13 | 完了条件/順序 | **要改訂** | 「Safe Mode 削除」前提の手順を「語彙統一」前提へ置換（本書 §5） |

特記:

- 元指示書 §6 の「旧: `Safe Mode protects evaluation.`」という置換元文字列は README に存在しない。
  README はすでに「`SAFE` is not the main mechanism」と書いており、置換ではなく**枠組みの追記**で足りる。
- 元指示書 §10.1 の「Safe Mode API が存在しないこと」を確認するテストは、実際には `~`/`SAFE` API が
  存在し**続ける**ため、要件として誤り。維持すべきは「グローバル実行モードが存在しないこと」。

---

## 4. 改訂後のタスク定義（安全・非破壊）

ゴールを次のように再定義する。

> **既存の安全機構（safe-by-design な通常評価、IO/host・IMPORT 境界、各種 budget）に対し、
> 「Gate（どこへ流すか）」「Water Level（どれだけ流すか）」という水のメタファー語彙を与えて
> ドキュメント上で統一する。意味論・トークン・型・テストの破壊的変更は行わない。**

### 4.1 やること（許可される変更）

1. **README**: 「Safety model」小節を追記し、4 区分（Flow/Bubble/Stagnation/Channel error）に加えて
   2 つの制御 ——「Gates = 境界（外: IO/host、内: module import）」「Water Levels = 量（step / comparison /
   expansion / collection size budget）」—— を**既存機構の総称として**説明する。`SAFE` は「水路エラーの
   明示的排水口」として正確に位置づける（廃止と書かない）。既存の spec リンクへ繋ぐ。
2. **SPECIFICATION.md（任意・要 owner 承認）**: 既存節を削除せず、解説用の総称節
   「Gates and Water Levels（既存機構へのマッピング表）」を追加してよい。§6.3 / §11.3 / §5.3 / §7.4 は不変。
   追加する場合、それが**新たな規範ではなく既存規範のインデックス**であることを明記する。
3. **エラーメッセージ（任意）**: 境界拒否・budget 超過のユーザー向け文言を、内部理由ではなく
   「どの Gate / どの Water Level か」で説明する方向に整える。ただし `safeCaught` / `caughtCategory` /
   `ExecutionLimitExceeded` / `agreedPrefix` などの**機械可読フィールドは変更しない**。
4. **用語集（任意）**: 日本語設計メモに対応表を置く。
   `泡=Bubble/NIL` / `淀み=Stagnation/UNKNOWN` / `水路エラー=Channel Error` /
   `水門=Gate（境界の総称）` / `水位=Water Level（量の総称）` / `排水口=SAFE(~)`。

### 4.2 やらないこと（禁止される変更）

- `~` / `SAFE` / `Token::SafeMode` / `interp.safe_mode` フラグの削除・改名・意味変更。
- `NilReason::SafeCaught` および `caughtCategory` の削除・意味変更（§4.5.2 / §11.3 を壊す）。
- 新型 `GatePolicy` / `WaterLevelPolicy` の追加（§11 非目標）。既存に統合できる場合のみ、別途設計レビューを経る。
- 外部作用 word の追加、capability system の新規大規模設計、UI 大規模再設計。
- Bubble/NIL・Stagnation/U・K3 真理表・連分数表現・comparison budget の意味論変更。
- `SPECIFICATION.md` の §6.3 / §11.3 を「legacy」化または削除すること。

### 4.3 保持すべき不変条件（回帰防止）

- グローバル実行モードは存在しない（`safe_mode` は `~` の一過性内部状態に限る）。
- `FALSE AND UNKNOWN = FALSE`、`TRUE OR UNKNOWN = TRUE`（K3, §7.5）。
- comparison-undecided は NIL ではなく U（§4.5.2, §7.4.1）。
- operational NIL は passthrough で流れ、U を吸収しない（§4.5.1, §4.5.2）。
- `~` ガードはエラー時のみ介入し、直接 Bubble/NIL は再ラップしない（§11.3）。
- step budget 超過は `ExecutionLimitExceeded`（§5.3）。

---

## 5. 改訂後の実行順序

1. 本書の事実確認（§2）と判定（§3）を owner が承認する。
2. README に「Safety model（Gates & Water Levels）」小節を追記（§4.1-1）。破壊的変更なし。
3. 既存の安全機構 → Gate / Water Level 対応表を作成（§2.4 / §2.5 の表を流用）。
4. （任意・承認後）SPECIFICATION に総称インデックス節を追加。既存節は不変。
5. （任意）境界/予算のユーザー向けメッセージ文言を整える（機械可読フィールドは不変）。
6. 既存テスト（`interpreter_mode_tests.rs`、`arithmetic_operation_tests.rs` の `~` 系、K3、comparison budget）を
   実行し、§4.3 の不変条件がすべて維持されていることを確認する。テストの削除・改名は行わない。

---

## 6. 完了条件（改訂版）

> 注: 下記 §6 は当初の「`SAFE` を残す」前提で書かれた完了条件である。冒頭の「追補（owner 決定）」により
> `SAFE`(`~`) は完全削除へ方針変更された。`SAFE` の存続に関する項目（取り消し線）は追補の完了条件で置き換える。

- README に Gates / Water Levels の枠組みが、**既存機構の総称として**説明されている。
- ~~`~` / `SAFE` が「水路エラーの明示的排水口」として正しく説明され、廃止されていない。~~
  → 追補により撤回。**`~` / `SAFE` は言語から完全に削除され、誤用エラーは射影されず伝播する**ことが正。
- ~~`SPECIFICATION.md` §6.3 / §11.3 / §5.3 / §7.4 が無変更で残っている。~~
  → §6.3「Safe mode modifier」は削除（§6.4/§6.5 を繰り上げ）、§11.4 は「Error propagation」へ置換。
  `§5.3`（step budget）/ `§7.4`（comparison budget）は無変更。
- 新型 policy・新規外部作用 word が追加されていない。
- §4.3 の不変条件（Bubble Rule / K3 / comparison budget→UNKNOWN / step budget / NIL passthrough）が
  テストで維持されている。
- ドキュメントが「通常モードは危険／Safe Mode だけ安全」という誤った二分法を導入していない。

---

## 付録: 元指示書の正しい読み替え早見表

| 元指示書の主張 | 現実 | 正しい対応 |
|---|---|---|
| 包括的 Safe Mode を廃止する | 存在しない | 何も削除しない |
| `safe_mode` 分岐を通常評価から削除 | `~` の一過性状態のみ | 保持（削除すると `~` が壊れる） |
| `SafeMode` 型/`safe_mode` フラグを legacy 化 | `~` 修飾子の実装 | 保持 |
| Gate を新規導入 | IO/host・IMPORT 境界が既存 | 既存への命名（総称） |
| Water Level を新規導入 | step/comparison budget が既存 | 既存への命名（総称） |
| 比較予算切れを UNKNOWN にする | すでに UNKNOWN | 維持確認のみ |
| UI の Safe Mode トグル削除 | トグルは存在しない | 対象なし |
| README の "Safe Mode protects evaluation" を置換 | その文言は無い | 枠組みを追記 |
