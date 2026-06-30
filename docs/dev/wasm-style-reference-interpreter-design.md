# WASM 方式の Ajisai 向けアレンジ — 参照実装と五層の相互拘束

> Status: **Non-canonical / 設計メモ（§2.2）.** 本書は言語意味論を一切定義しない。
> 正典は `SPECIFICATION.html` のみ。本書は乖離抑制の*運用設計*を述べる手続き文書である。
> 前提: `docs/dev/spec-impl-drift-tactic.md`（タイムスタンプ裁定の却下と追跡可能性ゲート、PR #1212 でマージ済み）。
> 関連正典: `SPECIFICATION.html` §2.4（四層）・§2.5（権威順位）・§16.1（第二権威の禁止）・"Conformance and Identity"。
> 関連メタ仕様: `docs/dev/ajisai-authoring-style.md`（「仕様書の仕様」）。

## 0. 採用方針

WebAssembly 方式（散文仕様・形式意味論・参照実装・テストスイートを同居させ相互拘束する）を、
Ajisai の権威構造（単一正典）を壊さない形でアレンジして採用する。**「実装そのものが仕様」**
（CPython/MRI 方式）は採らない——`PORTABILITY.md` 原則1・2・10・11 および §2.4 の実装非依存目標と
非両立だからである。代わりに、実装の魅力（実行可能ゆえの無曖昧さ）は**参照実装＝仕様の実行可能な影**
として回収する。これが WASM 方式の核心であり、Ajisai に欠けている唯一のピースである。

## 1. WebAssembly の構造を Ajisai に写す

WebAssembly は正典を「単一の文書」ではなく**相互拘束する束**として持つ。各資産を Ajisai に対応させる:

| WebAssembly | Ajisai での対応物 | 権威（§2.4/§2.5） |
|---|---|---|
| **SpecTec**（形式仕様を書く記述言語＝"仕様の仕様"） | `docs/dev/ajisai-authoring-style.md` ＋ 姉妹 style 文書 | Non-canonical（記法のみ、意味を定義しない） |
| formal semantics | `docs/dev/ajisai-mathematical-formalization.md` | Descriptive |
| prose specification | `SPECIFICATION.html` | **Canonical** |
| reference interpreter | **`tools/ajisai-repro/ajisai.py`（要昇格）** | 検証物（spec の影。spec が勝つ） |
| conformance test suite | `tests/conformance/` | 検証物（L5 同一性） |
| production engines（V8 等） | Rust ・ WASM ・ TypeScript | 実装（spec の表現） |

5 つの本質的洞察:

1. **`ajisai-authoring-style.md` は Ajisai の SpecTec である。** SpecTec が「Wasm の形式意味論を
   機械処理可能・無曖昧に書くための記法」であるのと同様、authoring-style は「Ajisai の仕様を
   機械可読・無曖昧に書くための記法」を定める——Ajisai トークンは灰色 code チャネル、数式は
   KaTeX チャネル、列挙構造は table（§3/§4/§5）。**この層が無ければ参照実装は成立しない**(後述 §3)。
2. **不足は参照実装ただ一つ。** 他の4資産は既に存在する。`ajisai-repro` はその胚だが、今は
   使い捨ての乖離 probe であり、保守される正典拘束資産になっていない。
3. **参照実装は権威ではない。** Wasm の reference interpreter が仕様の下位にあるのと同じく、
   `ajisai-repro` も spec の**実行可能な影**にとどめる。spec と食い違えば spec が勝ち、参照実装を直す
   （§2.4 の Reference と同じ規律）。これにより §16.1「第二権威の禁止」を侵さない。
4. **production 実装は spec の表現にすぎない**(§2.1)。参照実装も production も、ともに spec へ従う
   別個の実装であり、互いを差分テストで突き合わせる(differential testing)。
5. **§2.5 の権威順位は不変。** 参照実装は conformance suite・law tests と同じ「検証物」の段に置き、
   spec と数式の**下**に位置づける。順位は一切変えない。

## 2. なぜ参照実装が WASM 方式の心臓か

`SPECIFICATION.html` ＋ conformance suite だけでは、`spec-impl-drift-tactic.md` §2.2 が示した
**A 類（仕様の穴、乖離の 60%）**を仕様文面だけからは塞げない。穴は「散文が書いていない領域」であり、
静的検査では「書かれていないこと」を網羅的には捕まえられない。参照実装はこれを**実行可能オラクル**で
解決する:

- **差分テスト**: production Rust ⇔ 参照実装 を多数プログラムで突き合わせる。
  - 両者が一致しない入力 = **仕様が決めていない領域の自動発見**（A 類）。`ajisai-repro` は
    79 本中 15 本の相違を*まさにこの方法で*検出した実績がある。
  - 参照実装だけが正典違反 → 参照実装のバグ（spec の読み違い）。
  - production だけが正典違反 → **B 類（実装バグ）**。
- これは `spec-impl-drift-tactic.md` §3.1 の追跡可能性ゲートを**強化**する: 静的アンカーに加え、
  各 conformance ケースを**実行可能な spec の影**にも照合できる。タイムスタンプより遥かに強い乖離検出。

## 3. 参照実装とメタ仕様の結合（authoring-style を含める理由）

ここが本メモの中心的主張である。**参照実装の忠実度は、散文の無曖昧さに完全に依存する。**

`ajisai-repro/README.md` は「**SPECIFICATION.html の散文だけから、Rust を見ずに**」書かれたと明記する。
すなわち参照実装は、§2.4 が掲げる移植性テスト――「ソースを読まず文書だけから同じ言語を再現できるか」
――を**実際に実行した産物**である。その成否は、散文が `ajisai-authoring-style.md` の規律どおり
無曖昧に書けているかに掛かる。したがって:

> **参照実装は authoring discipline の実行可能なテストである。**
> 独立実装者（人間、または Python 再現）が同じ言語を再現できれば、散文は十分に無曖昧。
> 再現が割れたら、原因は二つに一つ——**散文が曖昧（メタ仕様・spec の穴 = A 類）**か、
> **どちらかの実装が違反（B 類）**。

この結合から、五層は一方向の依存で閉じる:

```
authoring-style（記法）  →  spec を機械可読・無曖昧に書ける
        ↓
spec（正典）＋ 数式（記述）  →  参照実装を文書だけから書ける
        ↓
参照実装（spec の影）  ⇔  production 実装   （differential testing）
        ↓
conformance suite  →  両実装が L5 同一性で一致することを pin
```

`ajisai-repro` が検出する乖離の一部は、本質的に**authoring-style 失敗の測定値**である
（散文が署名・端点・タイ規則を書き漏らした箇所）。よって参照実装の保守は、spec と
メタ仕様の品質を継続的に測る計器になる。

## 4. 「実装そのものが仕様」を採らない理由（再掲・確定）

| 論点 | 帰結 |
|---|---|
| `PORTABILITY.md` 原則1・2 | 「正典は特定実装ではない」「Rust は参照実装の一つ」と明言。実装=仕様は非両立 |
| §2.4 目標 | 「文書だけから移植して同じ言語」が消える。第二実装は bug-for-bug クローンでしか作れない |
| §2.3 firewall | Rust enum 名・`Debug`・内部表現・最適化経路を観測禁止。実装=仕様はこれらを正典化 |
| バグ概念 | B 類（CHR・CONCAT 等）を「バグ」と呼べなくなる。是正のテコを失う |
| 実証 | `ajisai-repro` の存在が「spec を正典とすれば実装非依存に再現できる」を既に証明済み。捨てる理由がない |

参照実装は「実行可能ゆえ無曖昧」という実装=仕様の*唯一の長所*を、正典を侵さずに回収する。

## 5. 昇格設計 — `ajisai-repro` を保守される参照実装へ

### 5.1 スコープ

- **Ajisai Core のみ**（ホスト非依存。`PORTABILITY.md` の Core/Hosted 分割に一致）。
  Hosted 効果（IO・SERIAL 等）は host capability を要するため参照実装の対象外とし、
  conformance suite と production 実装が担う。
- **最小・spec 形**: 最適化を持たず、`SPECIFICATION.html` の節に1対1で対応する構造を保つ
  （production Rust とは目的が逆——あちらは速度、こちらは明白な対応）。
- **予算意味論を露出**: 比較予算 `β` と `U`（§7.4.1/§7.4.2 `COMPARE-WITHIN`）を参照実装でも
  観測可能にする。これは spec に予算意味論を精密に書かせる圧力になる（利点）。

### 5.2 正典上の位置づけ（§16.1 を侵さないための明文）

- 参照実装は**検証物**であり、§2.5 で conformance suite・law tests と同じ段に置く（spec・数式の下）。
- spec と食い違えば**spec が勝ち、参照実装を直す**。参照実装は第二権威ではない。
- production 実装と参照実装が食い違い、かつ suite が沈黙する場合、裁定は
  `spec-impl-drift-tactic.md` §3.3 の**スイート裁定規則**に従う（時計ではなくスイートが裁く）。

### 5.3 CI 結線

- `tools/ajisai-repro/compare.py` を土台に、production CLI（`cargo build --bin ajisai --release`）と
  参照実装の差分を**プログラム集合**に対して走らせる job を追加。
- conformance の全ケースは参照実装でも一致せねばならない（参照実装が suite を通す）。
- 差分集合が空でない＝**A 類の穴または B 類のバグの検出**として、レビュー対象に上げる
  （いきなり fail にするか advisory にするかは段階導入。`AJISAI_STRICT_QUALITY` 方式に倣う）。

### 5.4 保守規律（変更プロセスの相互拘束）

- **spec の規範項目を加筆／改訂** → 同一 PR で参照実装と conformance を追従。三者が割れたまま
  マージしない（authoring-style と spec の品質を参照実装が継続検証する）。
- **数式の改訂** → Descriptive のまま。参照実装は spec に従い、数式に直接は縛られない。
- **production 最適化** → 観測挙動を変えないことを参照実装との差分ゼロで担保（§2.3 firewall の運用的検査）。

## 6. 最小実装ステップ（提案）

1. `tools/ajisai-repro/ajisai.py` を「使い捨て probe」から「保守される Core 参照実装」へ位置づけ直す
   README 改訂（目的・スコープ・正典上の位置＝検証物・spec が勝つ規律を明記）。
2. 差分テストドライバを整備（`compare.py` を CI 実行可能な終了コード付き形へ）。プログラム集合は
   conformance ケース ＋ 法則ベース生成（`spec-impl-drift-tactic.md` の生成器思想を流用）。
3. `.github/workflows/test.yml` に differential job を追加（まず advisory）。
4. `SPECIFICATION.html` §2.4 の四層表に**参照実装**を検証物として注記する案を起票
   （正典本文の改訂のため別 PR・設計者承認のうえ。本書は提案に留める）。§2.5 の段にも追記。
5. `PORTABILITY.md` 原則2「Rust は参照実装の一つ」に、最小 Core 参照実装（`ajisai-repro`）が
   差分テストの基準として存在することを追記（別 PR）。

## 7. 一行サマリ

> WebAssembly の五資産（SpecTec・形式意味論・散文仕様・参照実装・テストスイート）を Ajisai に写すと、
> **SpecTec ＝ `ajisai-authoring-style.md`（仕様の仕様）**、不足は**参照実装ただ一つ**。
> `ajisai-repro` を使い捨て probe から **spec の実行可能な影**へ昇格し、production 実装との
> 差分テストを CI 化する。参照実装は authoring discipline の実行可能なテストであり、
> その差分が A 類の穴・B 類のバグを機械的に炙り出す。権威は §2.5 のまま——参照実装は
> 第二権威ではなく検証物。「実装そのものが仕様」は採らないが、その唯一の長所（実行可能ゆえの
> 無曖昧さ）は参照実装が正典を侵さずに回収する。
