# セルフホスティングの位置づけ — 亜種を生まない自己記述の批判的再構成

> Status: **Non-canonical / 設計メモ（§2.2）.** 本書は言語意味論を一切定義しない。
> 正典は `SPECIFICATION.html` のみ。本書は「セルフホスティングをどう位置づけるか」という
> 運用・アーキテクチャ方針を論じる手続き文書である。
> 関連正典: `SPECIFICATION.html` §2.1（正典順位）・§2.4/§2.5（四層・権威順位）・
> §7.14（Coreword contract metadata、`safety_level`）・§9.3（辞書語彙階層）・
> Portability Profiles・"Conformance and Identity"。
> 関連設計メモ: `docs/dev/spec-impl-drift-tactic.md`（タイムスタンプ裁定の却下と
> 追跡可能性ゲート）・`docs/dev/wasm-style-reference-interpreter-design.md`（参照実装の
> 権威上の位置づけ）・`PORTABILITY.md`（原則1・2・10・11）・`python/README.md` ・
> `tools/ajisai-repro/README.md`（既存の「移植による仕様洗練」の実例）。

## 0. 本書の立場

ChatGPT との議論（"Ajisai はメタプログラミング性の追求を避けているが、セルフホスティングには
関心がある。両立できるか" という問い、および Surface Ajisai / Self-hosting Ajisai / Sealed Core
の三層モデル、capability-gated primitive、"Official Self-hosting Subset" 提案）を、
**Ajisai が既に持っている権威構造と語彙構造**に当てて再構成する。結論を先に置く:

1. ChatGPT 案が守ろうとしている性質（「一般利用者は構文・評価規則・Core Words を書き換えられない」）
   は **Ajisai に既に存在する**。固定表層文法（§3）・封印された Core Words（§7/§8）・単一の
   辞書解決アルゴリズム（§9.3）・メタプログラミング用の構文拡張手段の不在によって、これは
   新しい壁を作らずとも最初から成立している。
2. ChatGPT 案の「capability-gated primitive（`requires official-kernel-capability` で公式ビルドだけに
   実行を許す語）」と「新しい Ajisai Kernel Profile」は **採らない**。これは Ajisai の移植性目標
   （§2.4: 文書だけから同じ言語を再現できる）と `PORTABILITY.md` 原則2（実装は参照実装の一つで
   あり唯一の正統実装ではない）に反し、「公式ビルドだけが実行できる語」という特権階層——
   まさに防ぎたかった亜種を、実装の特権という別軸で生み出してしまう。
3. セルフホスティングは、新しい権威層でも新しい Core Word 群でもなく、**実装言語の選択が一つ
   増えるだけ**の話である。Ajisai で書かれた Ajisai 実装は、Python 移植（`python/`）や
   参照実装（`tools/ajisai-repro/`）と同じ資格・同じ規律で扱う: Conformance and Identity
   がその適合性を判定し、spec と食い違えば spec が勝つ。新しい正典機構は要らない。
4. 唯一の実務的ギャップは、`safety_level: Quarantined` の意味として既に本文に登場している
   **"self-host execution" という用語が定義されないまま使われていた**ことである
   （SPECIFICATION.html §7.14、`safety_level` 表の `Quarantined` 行）。これは
   `spec-impl-drift-tactic.md` の分類でいう A 類（仕様の穴）そのものであり、本書はその定義を
   §7.14 に追記する提案を行う（本 PR で反映）。

## 1. ChatGPT 提案の要約

議論は次を提案していた:

- **三層モデル**: Surface Ajisai（利用者が書く通常の Ajisai。メタプログラミング不可）／
  Self-hosting Ajisai（処理系記述用の Ajisai。構文・意味論は Surface と同一だが、扱う対象が
  トークナイザ・パーサ・評価器そのもの）／ Sealed Core（構文・評価順序・Core Words・辞書解決・
  数値モデル・Sugar 展開規則。Ajisai プログラムから直接書き換え不能）。
- **Ajisai Kernel Profile** という「公式のみのプロファイル」を設け、Surface Profile と合わせて
  二つだけを許す（Community Profile・User-defined Profile は認めない）。
- **capability-gated メタ操作**: word metadata の参照・stack effect 検査・dictionary entry の
  列挙などを通常ユーザーに開放せず、`requires official-kernel-capability` のような封印で
  公式ビルド・公式署名済みソースだけに限定する。
- **セルフホスト実装は正典ではない**: 正典はあくまで数式・仕様・Reference であり、セルフホスト
  実装が仕様に反すれば Ajisai ではない。
- **拡張は Words/Modules に閉じる**: 独自構文・独自評価規則・独自 Core・独自 Sugar・独自
  辞書解決・独自数値モデルは避ける。
- **Fork-friendly but dialect-hostile**: フォークは歓迎するが、仕様・数式・Reference・公式
  検証に一致しないものは Ajisai ではなく Ajisai 派生言語と呼ぶ。

## 2. 何が既に Ajisai に存在するか（新規性のない部分）

`spec-impl-drift-tactic.md` §1 が数式の権威付けに対して行ったのと同じ検査をここでも行う:
ChatGPT 案の要素を一つずつ、既存の正典条文と照合する。

### 2.1 「Surface Ajisai はメタプログラミングできない」は既に真

Ajisai の表層文法は §3 で完全に固定されている。ユーザーに開放されている拡張点は
`DEF`/`DEL` による **User Word の追加**（§7.8、§8）だけであり、これは:

- 新しい構文を導入できない（§3.7 構文制約は不変）。
- 既存の Core Word の意味を上書きできない — `DEF` は辞書に新しいエントリを追加するだけで、
  Core Words の解決規則自体（§9.3、bare name はまず Canonical Core を見る、§7.14 末尾）は
  変更不能。
- 辞書解決アルゴリズムそのものを差し替える手段がない。
- Sugar 展開規則（§6.4、§16 チェックリスト項目7）はユーザーに開放されていない。

**したがって ChatGPT が Surface 層に求めた制約は、新しい禁止規則を追加するまでもなく、
既存の言語設計がそのまま体現している。** これは「三層モデルの半分」がそもそも Ajisai の
出発点であることを意味する——防御すべき対象が最初から攻撃面に存在しない。

### 2.2 「Sealed Core」も新設不要

ChatGPT の Sealed Core（構文・評価順序・Core Words・辞書解決・数値モデル・Sugar 展開規則）は、
Ajisai では単に **§2.1 が定める「正典」の内容そのもの**である。これを「書き換え不能にする」
新しい機構は要らない。なぜなら Ajisai プログラムには、そもそもそれらを書き換える文法的・
意味論的な手段が存在しないからである（2.1 節で確認した通り）。「Sealed」という形容詞は
Ajisai にとって新しい制約ではなく、既存設計の**呼び名**にすぎない。

### 2.3 「セルフホスト実装は正典ではない」も既存の §2.1/§2.5 の言い換え

ChatGPT 案の「セルフホスト実装が仕様に反すれば Ajisai ではない」は、Ajisai の
Conformance and Identity のまま:

> An implementation is an Ajisai implementation if and only if it passes the conformance
> suite (`tests/conformance/`).

および §2.1 の「実装コードはこの仕様の表現であり、独立した意味の源泉では決してない」と
一字一句同じ主張である。新しい規則は不要——ただ**適用対象をセルフホスト実装にも及ぼす**と
明言すればよいだけである（§4 で提案）。

### 2.4 「Fork-friendly but dialect-hostile」も既存

`PORTABILITY.md` 原則10「ある実装が Ajisai であることは conformance suite を通すことで
証明される」がまさにこの規律である。フォークやセルフホスト実装がスイートを通れば Ajisai、
通らなければ Ajisai ではない——判定は自動化されており、宣言文を追加する必要はない。

## 3. 何を却下するか、そしてなぜか

### 3.1 capability-gated primitive（`requires official-kernel-capability`）— 却下

これは Ajisai に **新しい特権階層** を導入する。理由により却下する:

1. **§2.4 の移植性目標と非両立。** §2.4 の四層モデルは「文書だけから同じ言語を再現できる」
   ことを目的に掲げる。ある語が文書に書かれていても公式ビルドでしか実行できないなら、
   独立実装者はその語を**文書だけからは動かせない**。これは「文書が言語を決める」という
   Ajisai の根本前提を破る。
2. **`PORTABILITY.md` 原則2 と衝突。** 「Rust/WASM 実装は参照実装のひとつであり、唯一の
   正統実装ではない」という原則は、capability gate が事実上作り出す「公式ビルドだけが
   完全な Ajisai を実行できる」という構造と正面から矛盾する。
3. **観測可能な現象の単一性を壊す。** Conformance and Identity は「同じソースは同じ入出力
   対応を持つ」ことを前提にする。capability gate はこれを「誰が実行するか」で分岐させ、
   同一ソースが実行者によって異なる意味を持つ——これは方言化そのものであり、ChatGPT 案が
   防ぎたかった「亜種の乱立」を、構文の分岐ではなく**実行権限の分岐**という形で再導入する。
4. **代替が既にある。** 「公式実装だけが持つ内部データ」を守りたいなら、それは
   §2.3 の Semantic Firewall（内部表現は観測対象外、protocol field だけが観測可能）が
   既に担っている。新しい capability 機構は不要。

### 3.2 新しい "Ajisai Kernel Profile"（Portability Profiles への追加）— 却下、ただし再解釈して部分採用

`SPECIFICATION.html` の Portability Profiles（Core / Hosted / Platform / Presentation）は
**実行環境が持つ能力**を分類する軸である。ChatGPT の Kernel Profile は「どの言語で
処理系自体を書くか」という**実装言語選択の軸**であり、これは能力プロファイルとは直交する
別の軸である。既存 Profile の列に第五の Profile として追加するのは分類を混同させる。

代わりに、セルフホスティングは既存の「移植による仕様洗練」の枠——`python/`（Python 移植）・
`tools/ajisai-repro/`（Python 製参照実装）——に**もう一つの実装言語としての Ajisai 自身**を
加えるだけと位置づける（§4）。新しい Profile 軸は導入しない。

### 3.3 "Community Profile" / "User-defined Profile" の明示的禁止 — 不要（既に構造的に不可能）

ChatGPT 案はこれらを「認めない方がよい」と述べるが、Ajisai には利用者が新しい構文・評価
規則・辞書解決を定義する手段が最初から存在しない（§2.1）。「禁止する」条文を追加する
必要はない——**存在しない機能を禁止する規則は空虚**である。

## 4. 採用するモデル — セルフホスティングは「実装言語の選択」

結論として、セルフホスティングは次の一文に還元される:

> **セルフホスト実装とは、トークナイザ・辞書・評価器そのものが Ajisai で書かれた Ajisai の
> 独立実装である。** これは Rust 実装・Python 移植（`python/`）・Python 製参照実装
> （`tools/ajisai-repro/`）と同じ資格を持つ——§2.1 の意味で「この文書に従う限りにおいて
> 正典」であり、§2.5 の権威順位はまったく変わらない。判定基準は Conformance and Identity
> ただ一つ: スイートを通すか通さないか。spec と食い違えば spec が勝ち、セルフホスト実装を
> 直す。セルフホスト実装が正典になることはない。

この実装は、ホストの `CodeBlock`/`EXEC`/`EVAL` を借りない。`python/` や `ajisai-repro` が
Python の実行モデルではなく独自の AST/dictionary/evaluator データ構造を構築するのと同様に、
セルフホスト実装は Ajisai の Vector・Record・Text・User Word 再帰だけを使って**独自の**
トークン列・AST 相当データ・辞書表現を組み立てる。ホスト側の `EXEC`/`EVAL` は「別の評価器を
呼ぶ」機能であって、セルフホスト実装がそれ自身を記述するのに使う対象ではない——両者は
独立に動く。**この構成に、追加の Core Word も追加の構文も一切要らない。** 既存の値モデル
（§4）とユーザー辞書（§8）だけで十分である。

## 5. 実際に見つかった仕様の穴 — "self-host execution" の未定義

本書を書く過程で、`spec-impl-drift-tactic.md` の方法論（既存本文の用語がどこまで定義されて
いるかを機械的に確認する）を適用した結果、実際に A 類の穴を発見した:

`SPECIFICATION.html` §7.14 の `safety_level` 表は次のように書いている:

> `Quarantined`: not eligible for self-host execution.

しかし **"self-host execution" という用語自体は本文のどこにも定義されていない**。
本 PR はこの穴を、次の定義を §7.14 に追記することで塞ぐ（§0 で述べた「本文に加筆する
場合は同一 PR で追跡可能性を満たす」規律に従う、小さく自己完結した追記）:

- self-hosted implementation の定義（§4 のもの）。
- self-host execution は、セルフホスト実装自身が Core Profile 上で走る Ajisai プログラム
  であるため、**Core Profile の語彙にのみ及ぶ**——ホストの子ランタイム機構に依存する
  `Quarantined` な語（現状すべて Section 10 の子ランタイム制御語: `SPAWN` `AWAIT` `STATUS`
  `KILL` `MONITOR` `SUPERVISE`）は self-host execution の対象外であり、セルフホスト実装は
  これらを再現する義務を負わない。
- これはホスト言語実装（Rust・Python 等）における当該語の契約を一切縮小しない。除外は
  self-host execution という文脈にのみ及ぶ。

この追記は新しい規則を作らない——既に登録されている `Quarantined` という分類の意味を、
本文が使っていながら定義していなかった言葉として補うだけである。

## 6. ChatGPT 案との対応表

| ChatGPT 提案 | Ajisai での扱い |
|---|---|
| Surface / Self-hosting / Sealed Core の三層 | 大部分**既存**。固定表層文法（§3）・封印 Core Words（§7/§8）・単一辞書解決（§9.3）が既に同じ性質を与えている。新規機構は不要 |
| capability-gated primitive（`requires official-kernel-capability`） | **却下**。公式ビルドの特権化により§2.4 の移植性目標・`PORTABILITY.md` 原則2・Conformance and Identity の単一性と衝突する |
| 新しい "Ajisai Kernel Profile" | **却下→再解釈**。実行環境能力の軸ではなく実装言語選択の軸なので Portability Profiles には追加しない。移植の一資産として位置づける |
| Community/User-defined Profile の明示的禁止 | **不要**。禁止対象の機能自体が最初から存在しない |
| セルフホスト実装は仕様に従属し正典にならない | **採用**。§2.1/§2.5/Conformance and Identity の言い換えであり新規則は不要 |
| 拡張は Words/Modules に閉じる | **既存**。`DEF` は新しい語彙しか作れず、構文・Core Word・辞書解決規則は最初から書き換え不能 |
| Fork-friendly but dialect-hostile | **既存**。`PORTABILITY.md` 原則10（conformance suite 通過が Ajisai であることの証明）がそのまま担う |
| `safety_level = Quarantined` と self-host execution の関係 | **新規に必要**（実際に発見した仕様の穴）。§7.14 に定義を追記（本 PR） |

## 7. 最小実装ステップ（提案）

1. `SPECIFICATION.html` §7.14 に self-host execution の定義を追記し、`Quarantined` の
   対象語（Section 10 の子ランタイム制御語）を明示する。**本 PR で実施。**
2. 将来、実際にセルフホスト実装を書く場合は `tools/ajisai-selfhost/`（仮称）のような
   独立ディレクトリを設け、`python/README.md` および `tools/ajisai-repro/README.md` と
   同型の冒頭注記（非正典・検証物・spec が勝つ・conformance suite が唯一の判定者）を
   置く。これは本書の対象外であり、着手時に別 PR とする。
3. セルフホスト実装のスコープは Portability Profiles の Core Profile のみとし、Hosted 効果
   および `Quarantined` 語を対象外とする——`ajisai-repro` と同じスコープ規律。
4. 差分テストを追加する場合は `tools/ajisai-repro/compare.py` の方式を流用し、production
   Rust・`ajisai-repro`・セルフホスト実装の三者比較に拡張する（advisory から開始）。
5. 亜種防止の運用規律は新設せず、`spec-impl-drift-tactic.md` §3.3 のスイート裁定規則と
   `PORTABILITY.md` 原則10 をそのまま適用する。

## 8. 一行サマリ

> ChatGPT 案の「三層 + capability gate」は目的（メタプログラミング拒否とセルフホスティングの
> 両立）は正しいが、そのための新しい機構は Ajisai には要らない——固定表層文法・封印 Core
> Words・単一辞書解決・Conformance and Identity が既にその両立を無償で与えている。
> capability-gated primitive と新しい Kernel Profile は、公式ビルドを特権化することで
> かえって「実行権限による方言」を生むため却下する。セルフホスティングは新しい権威層では
> なく、Python 移植・`ajisai-repro` 参照実装と同じ資格を持つ**実装言語の選択肢が一つ増える
> だけ**の話であり、判定は Conformance and Identity ただ一つに委ねる。唯一の実務的ギャップ
> だった "self-host execution" という未定義用語は、本 PR で §7.14 に定義を追記して塞ぐ。
