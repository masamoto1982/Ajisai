# Ajisai 仕様 vs 実装 乖離分析（検証済み）

**日付**: 2026-06-30
**対象正典**: `SPECIFICATION.html`（Status: Canonical, Version: 2026-06-11）
**入力資料**: `tools/ajisai-repro/FINDINGS.md`（Python 再現版による 79 本比較、15 本相違）
**本書の役割**: FINDINGS.md が挙げた相違を、正典本文・適合性スイート（`tests/conformance/index.html`）・
Rust 実装挙動に当たって**一件ずつ検証**し直し、各乖離について「仕様に実装を合わせる（impl→spec）」か
「実装に仕様を合わせる（spec→impl）」かの推奨方針を付したもの。

---

## 0. 前提の訂正：権威順位の読み直し（重要）

FINDINGS.md は「正典 §2.5 が `散文 → 数学的形式化 → 適合性スイート` を canonical と定め、
散文と適合性スイートという**二つの canonical 層**が矛盾している」を最重要発見としている。
しかしこれは正典の権威構造の読み違いである。

`SPECIFICATION.html` §2.4「Documentation layers」が定める権威は **4 層**で、表は次のとおり：

| 層 | 所在 | 権威 |
|---|---|---|
| Mathematical formalization | Math blocks ほか | **Descriptive**（本文を制約・動機づけるが、決して上書きしない） |
| **Specification（本文）** | `SPECIFICATION.html` | **Canonical** |
| Reference | `public/docs/` | Derived（衝突時は本文が勝つ） |
| Authoring discipline | `docs/dev/...` | Non-canonical |

さらに本文冒頭は「**If any other document conflicts with this document, this document takes precedence.**」と明言する。

つまり：

- **適合性スイート（`tests/conformance/`）は権威 4 層に含まれない。** スイート自身も各ケースに
  「matches SPECIFICATION.html」と注記しており、スイートは*本文から導かれた検証物*という位置づけである。
- したがって「スイートが散文を上書きする canonical 層」という構図は成立しない。本文と矛盾するスイートは、
  本文の precedence 規定により**スイート側も是正対象**になる。
- 一方でスイートは L5「Conformance equivalence（= 実装が同じ Ajisai であることの運用的定義）」を担い、
  正典は実装間の同一性をスイートに依存している（§"Conformance and Identity"）。
  よってスイートが pin した挙動の多くは**意図的な設計判断**であり、本文がそれを*書き漏らしている*と読むのが自然。

**この訂正の帰結**：乖離は「散文 vs スイートの canonical 衝突」ではなく、次の 3 類型に分かれる。

- **A 類（仕様の穴／書き漏らし）**: 実装＋スイートが一貫した設計を持つが、本文が署名・規則を書いていない。
  → 原則 **spec→impl**（本文を実装に合わせて加筆）。同時にスイートはそのまま正典化を裏づける。
- **B 類（実装が明確な散文規定に違反、スイート沈黙）**: 本文に明確な規定があり、実装が外れ、スイートが守っていない。
  → 原則 **impl→spec**（実装を本文に合わせる）。
- **C 類（本文内・規約間の不整合）**: 本文の複数箇所が食い違う。→ 個別判断。

---

## 1. 検証済み乖離一覧（compare-output.txt の 15 本＋ FINDINGS §3.1）

凡例: 方向欄 = 推奨方針。`impl→spec`=実装を直す / `spec→impl`=本文を直す / `要判断`=設計者の決定が必要。

| # | プログラム | 原典実装 | 散文の規定 | 類型 | 方向 |
|---|---|---|---|---|---|
| 1 | `5 0 MOD` | `custom` エラー | §7.3「MOD = x − ⌊x/y⌋·y」かつ §11.2「DIV のゼロ除算→NIL」 → **NIL が含意される** | C | **impl→spec**（NIL 化） |
| 2 | `-2.5 ROUND` | `-3`（ゼロから遠い方） | §7.x「最も近い整数」のみ。タイ規則**未定義** | A | **spec→impl**（後述・要選択） |
| 3 | `1 2 3 .. LT` | `stackUnderflow` | §6.1「STAK=スタック全体が被演算子」。カウントも n 項 fold も未記述 | A | **spec→impl** |
| 4 | `3 2 1 .. LT` | `3/1 2/1 TRUE` | 同上（STAK は先頭のカウントを取り n 項の単調性述語を適用） | A | **spec→impl** |
| 5 | `1 1 1 .. EQ` | `1/1 1/1 TRUE` | 同上（STAK n 項述語） | A | **spec→impl** |
| 6 | `[ 1 2 3 ] LENGTH` | ベクタを**残す** + `3/1` | §6.2 既定 EAT=被演算子を消費 → 矛盾 | C/A | **spec→impl** |
| 7 | `[ 1 2 3 ] 1 GET` | ベクタを残す + `2/1`（裸インデックス可） | §7.1「インデックスの要素を取得」署名未定義、§6.2 EAT と矛盾 | A | **spec→impl** |
| 8 | `[ 1 2 3 ] 9 GET` | ベクタを残す + `NIL` | §11.2 範囲外→NIL は両者一致。差分はベクタ保持のみ | A | **spec→impl** |
| 9 | `[ 1 2 3 ] -1 GET` | ベクタを残す + `3/1`（負index=末尾から） | §7.1 署名・負index未定義 | A | **spec→impl** |
| 10 | `0 5 RANGE` | `custom` エラー | §7.1「start〜end の整数列（任意で step）」。署名（`[start end]` 包み）・端点包含未定義 | A | **spec→impl** |
| 11 | `[ 1 2 3 ] 1 5 REPLACE` | `custom` エラー | §7.1「インデックスの要素を置換」。署名（`[index element]` 包み必須）未定義 | A | **spec→impl** |
| 12 | `[ 1 2 3 ] 1 9 INSERT` | `custom` エラー | §7.1「インデックスに挿入」。署名（`[index element]` 包み必須）未定義 | A | **spec→impl** |
| 13 | `1 2 3 COLLECT` | `stackUnderflow`（カウント要求） | §7.1「**全**スタック値を 1 本のベクタに集める」 → カウント要求と**明確に矛盾** | B/C | **要判断** |
| 14 | `'ab' 'cd' CONCAT` | `[ 97 98 99 100 ]`（Text ロール喪失） | §12.2「ネストした Text は引用符付きのまま、codepoint fraction に崩落しない」 | B | **impl→spec** |
| 15 | `1114112 CHR` | `''`（空文字列） | §11.2「コードポイントが Unicode スカラ範囲外→`NilReason::InvalidEncoding`（=NIL）」 | B | **impl→spec** |
| 16 | `MATH@SQRT`（IMPORT 前） | `unknownWord` | §2.3.1.1 の例は IMPORT を伴わない／§7 の修飾名解決は import 状態への依存を明記せず | C（曖昧） | **要判断**（本文の曖昧さ解消） |

---

## 2. 類型別の所見と推奨

### 2.1 A 類 — 仕様の穴（書き漏らし）: #2〜#12

実装＋スイートが一貫した設計を持つのに、本文が**スタック署名・端点規則・タイ規則・保持規則**を
書いていないために、散文だけから移植した再現版と必然的に割れる。
**いずれも spec→impl（本文を加筆して実装に合わせる）を推奨。** 加筆すべき内容を具体化する。

- **#3〜#5 STAK の意味**: §6.1 は「スタック全体が被演算子」とだけ書く。実際の実装は
  - 先頭にカウント `N` を置き（`1 2 3 3 STAK ADD`）、その `N` 個を被演算子に取り、
  - 二項語（ADD）は **n 項 fold**、比較語（LT/EQ）は **n 項の単調性／全等述語** として適用する。
  - スイート case `core-stak-add` / `core-stak-keep-add` がこのカウント前置形を pin 済み。
  - → §6.1（または §7.4）に「STAK はスタック先頭のカウント `N` を取り、直下の `N` 値を被演算子集合として
    語を適用する。二項語は左畳み込み、順序比較は狭義単調増加の述語、等値比較は全要素一致の述語」と明記する。

- **#6〜#9 GET/LENGTH のソース保持**: 実装は GET/LENGTH で**元ベクタを残す**が、REVERSE/REMOVE/REPLACE/CONCAT は
  消費する。§6.2 の既定 EAT と表面的に矛盾するが、これは「検査系（projecting）語はソースを残す」という
  別カテゴリの設計とみるのが妥当（partiality 欄に `Projecting` が既に存在する）。
  - → §6.2 もしくは §7.1 に「GET・LENGTH 等の検査語はソースコレクションを消費せず、結果のみを上に積む
    （EAT 既定の例外）」と明記。あわせて GET の署名（裸スカラ・`[index]` ベクタ包みの両方を受理）と
    負インデックス（末尾から）も記述する。

- **#10 RANGE / #11 REPLACE / #12 INSERT のスタック署名**: 本文は一行説明のみで署名を欠く。
  - RANGE: `[ start end ]`（任意で step）ベクタ包み・**end 包含**。スイート `core-range` が pin 済み。
  - REPLACE / INSERT: `vector [ index element ]`（2 要素ベクタ必須）。
  - → §7.1 各語に署名を加筆。

- **#2 ROUND のタイ規則**: 本文は「最も近い整数」だけ。実装は **half-away-from-zero**（`-2.5→-3`, `0.5→1`）。
  ただし §4.2.5 の NICF タイ規則は **half-down（下側の整数へ）** と*正規的に*定義されており、
  語間で丸め規約が食い違う潜在的不整合がある。→ **要選択**（下記 3 章）。

### 2.2 B 類 — 実装が明確な散文に違反（スイート沈黙）: #14, #15

スイートにガードが無く、本文の明確な規定に実装が反している。**impl→spec を推奨。**

- **#15 `CHR` 範囲外 → NIL**: §11.2 の NIL-reason 表が「CHR: コードポイントが Unicode スカラ範囲外
  → `NilReason::InvalidEncoding`」と明記。実装は NIL を作らず `''` を返す。明確な実装バグ。
  → 実装を NIL（`InvalidEncoding`）に修正。`-1 CHR` / `55296 CHR`（サロゲート）/ `1114112 CHR` すべて NIL。

- **#14 文字列 `CONCAT` の Text ロール喪失**: §12.2 は「ネストした Text 要素は引用符付き文字列のまま、
  codepoint fraction に崩落しない」と規定。`'ab' 'cd' CONCAT` が `[ 97 98 99 100 ]` になるのは §12.2 違反。
  → 実装を修正し、文字列同士の CONCAT は Text を保持（`'abcd'`）する。
  ※ 本文は「CONCAT が文字列を受けるか」自体を明記していないため、§7.1 にも「文字列 CONCAT は連結した
    Text を返す」旨を併せて加筆するのが望ましい（B＋A 複合）。

### 2.3 C 類 — 本文内・規約間の不整合: #1, #13, #16

- **#1 `MOD` by 0**: §7.3 は「MOD = x − ⌊x/y⌋·y」と定義し、§11.2 は「DIV のゼロ除算→NIL」。
  MOD は DIV を内包する定義であり、§4.5.1 NIL passthrough（MOD は NIL 透過に列挙）からも、
  ゼロ除数では **NIL** が含意される。実装は `custom` エラーを返し、DIV（NIL）と非対称。
  → **impl→spec**: `MOD` by 0 を NIL（`DivisionByZero`）に修正。あわせて §11.2 の Bubble/NIL 表に
    MOD 行を追加して対称性を明示。

- **#13 `COLLECT`**: §7.1 は「**全**スタック値を 1 本のベクタに集める」と*無条件*に書くが、実装は
  カウント `N` を要求する（`1 2 3 COLLECT → stackUnderflow`, `1 2 3 3 COLLECT → [1 2 3]`）。
  これは**明確な散文との矛盾**であり、A 類の「書き漏らし」とは違って本文が積極的に別挙動を規定している。
  - 選択肢 (a) impl→spec: COLLECT を「全スタック」に戻す（STAK と役割が重複する点に注意）。
  - 選択肢 (b) spec→impl: COLLECT もカウント前置（STAK と統一）に本文を書き換える。
  - → **要判断**。STAK がカウント方式である以上 (b) の一貫性も妥当だが、本文の現行文言とは非互換。

- **#16 `MATH@SQRT` の修飾名解決**: FINDINGS は「§2.3.1.1 の看板例 `2 MATH@SQRT 2 MATH@SQRT SUB 0 EQ` が
  IMPORT 無しで動かない＝最も象徴的な乖離」とするが、本書の検証では**過大評価**と判断する。
  - §2.3.1.1 の当該文の主旨は「**比較が exact かつ total** なので、表現や比較予算を知らずに TRUE を信頼できる」
    ことであり、修飾名が import 無しで解決される、と*述べてはいない*。修飾記法は例示に使われているだけ。
  - §7 の修飾名規定は「`MODULE@WORD` はそのモジュールの canonical エントリのみに解決される」と書くが、
    **import 状態への依存有無を明記していない**。スイート（`core-sqrt2-*`）は一律 `'math' IMPORT 2 SQRT` を使う。
  - すなわちこれは「実装が看板例を壊している」のではなく、**本文が修飾名と IMPORT の関係を曖昧にしている**穴。
  - → **要判断**（本文の明確化）: 設計意図が
    (a)「修飾名は import 不要で常に解決」なら実装修正＋§7 加筆、
    (b)「修飾名も import されたモジュールにのみ解決」なら §2.3.1.1 の例を `IMPORT` 付きに直す（spec 側修正）。
    スイートが (b) を前提にしている点から、(b)（spec 明確化）が現実的。

---

## 3. 設計者の決定が必要な論点（要判断 3 件）

1. **ROUND のタイ規則（#2）**: half-away-from-zero（実装現状）/ half-down（§4.2.5 NICF と統一）/ half-even のいずれを正典化するか。
2. **COLLECT（#13）**: 「全スタック」に実装を戻すか、カウント前置に本文を合わせるか。
3. **修飾名と IMPORT（#16）**: 修飾名は import 不要で解決か、import 済みモジュールのみか。

その他 13 件は方向が明確：
- **impl→spec（実装修正）**: #1 MOD/0→NIL、#14 CONCAT Text 保持、#15 CHR→NIL。
- **spec→impl（本文加筆）**: #2 を除く A 類すべて（STAK 署名 #3-5、GET/LENGTH 保持＆署名 #6-9、
  RANGE/REPLACE/INSERT 署名 #10-12）。加えて、加筆した署名・規則は適合性スイートに新規ケースとして固定する。

---

## 4. 一致領域（参考）

64/79 本一致。厳密有理数算術・表示、SQRT の連分数表示と厳密代数比較、三値論理 K3、
Bubble/NIL と `^`(VENT) コアレッシング、出力境界の引用符除去は散文どおり機能している（FINDINGS §5）。

---

## 5. 結論

ユーザ仮説――「差異が出るなら仕様の作り込みが甘いか、仕様と実装に乖離がある」――は実証された。
ただし FINDINGS.md の「散文 vs 適合性スイートという二つの canonical 層の衝突」という診断は、
権威順位（§2.4）の読み違いに基づく。正しくは：

1. **大半（A 類 9 件）は本文の書き漏らし**で、実装＋スイートが一貫設計を持つ。→ **本文を加筆**して同一性を回復する。
2. **少数（B 類 2 件）は実装が明確な散文に違反**（CHR・CONCAT）。→ **実装を修正**する。
3. **C 類 3 件は本文内の不整合**（MOD/0、COLLECT、修飾名×IMPORT）で、設計者の決定を要する。

総じて「`SPECIFICATION.html` 単体では実装非依存に同一言語を再現できない」（§2.4 の目標未達）は妥当だが、
是正は**主に本文加筆（spec→impl）**で達成でき、実装修正が要るのは CHR・CONCAT・MOD/0 の 3 点に限られる。

---

## 6. 決定と対応（2026-06-30 追記・確定版）

**決定（ユーザ指示）**: すべて **spec→impl（現行実装を正典化）**。実装は一切変更せず、本文を実装挙動に合わせて加筆・訂正する。

実装の現挙動を release CLI で精密に再観測し、**正典 `Display`（適合性ランナーが用いる正規表示）** を基準に確定したうえで `SPECIFICATION.html` を加筆した。あわせて新挙動を `tests/conformance/index.html` に 12 ケース固定し、`conformance_suite_passes` の全件パスを確認した。

### 6.1 CHR は乖離ではなかった（重要訂正）

FINDINGS.md #15／本書 §2.2 の「`CHR` 範囲外は NIL ではなく空文字列 `''`」は **誤判定**。原因は比較ハーネス（`probe.py`）が headless CLI の **JSON `stackDisplay`** を読んでおり、そこでは CHR 由来の NIL が `''` と表示されるため。実装は `convert_codepoint_to_char`（`rust/src/interpreter/cast/cast_conversions.rs:151`）で範囲外コードポイントに対し `Value::bubble_with_reason(NilReason::InvalidEncoding, …)` すなわち **NIL** を返しており、正典 `Display` では `NIL` と表示される（適合性ランナーで確認）。つまり **§11.2 の規定どおりで、実装と仕様は元から一致**。本書 §2.2／FINDINGS の当該結論は撤回する。
（副次的論点として、headless CLI の JSON 出力経路が CHR-NIL を `''` と表示するのは表示上の不整合だが、コア言語意味論ではなく CLI 表示の問題であり本対応の対象外。）

### 6.2 確定対応一覧

| # | 項目 | 対応（spec→impl） |
|---|---|---|
| 3-5 | STAK | §6.1 に「先頭カウント `N`＋n項 fold（二項語は左畳み込み）／連鎖比較述語（LT 厳格増加・LTE 非減少・GT 厳格減少・GTE 非増加・EQ 全等・NEQ 隣接相異）」を明記。`EAT`/`KEEP` の消費規則も記述。 |
| 6-9 | GET/LENGTH | §7.1.1 を新設。検査語はソースを残す（EAT 既定の例外）。GET は裸／`[i]` 包みの両形・負index＝末尾から・範囲外＝NIL。 |
| 10 | RANGE | §7.1.1 に署名 `[ start end (step) ]`・end 包含を明記。 |
| 11-12 | REPLACE/INSERT | §7.1.1 に署名 `vector [ index element ]`（平坦形は不可）を明記。 |
| 13 | COLLECT | §7.1 セル＋§7.1.1 を「先頭カウント `N` を取り N 値を集める」に修正（STAK と統一）。 |
| 14 | CONCAT | §7.1.1 に「Text 被演算子は code-point ベクタに coerce され、結果は数値ベクタ」と明記（§12.2 の表示規則は維持）。 |
| 2 | ROUND | タイ＝ゼロから遠い方（half-away-from-zero）をセルに明記。 |
| 1 | MOD by 0 | §7.3 に「`custom`／"Modulo by zero"、DIV と非対称」を明記。§11.2 に MOD 行追加。 |
| 16 | 修飾名×IMPORT | §7 に「`MODULE@WORD` は import 後のみ解決、未 import は `unknownWord`」を明記。§2.3.1.1 の看板例に `'math' IMPORT` を前置。 |
| 15 | CHR | **対応不要**（§6.1 のとおり元から一致。誤判定を撤回）。 |

### 6.3 検証

- `cargo build --bin ajisai --release` 済み、上表の全挙動を CLI で再観測。
- `tests/conformance/index.html` に 12 ケース追加（STAK LT×2、GET 裸index、REPLACE、INSERT、RANGE step、COLLECT、CONCAT、ROUND×2、CHR→NIL、修飾名 IMPORT）。
- `cargo test --release --lib conformance` → `conformance_suite_passes ... ok`（66 ケース全件パス）。
