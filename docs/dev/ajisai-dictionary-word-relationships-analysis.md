# 辞書における Words 同士の優先順位・格付け — 現状分析

## 位置づけ（非正典・分析のみ）

- 本書は **非正典 (non-canonical)**。Ajisai の意味論・ランタイム挙動・互換性方針を定義しない。
  正典は `SPECIFICATION.html` のみ（§2.1 / §2.2）。
- 本書は **記述的** なメモであり、現状の辞書モデルの整理と「わかりにくさ」の診断、
  および将来の簡潔化に向けた検討材料を残すことだけを目的とする。
  **正典にも実装にも変更を加えない**。本書と正典が食い違う場合は正典が優先する。
- 参照した正典セクション: §7（Built-in Words / canonical home / boundary words）、
  §7.14（Coreword contract metadata / listing fields）、§8.6（Word identity）、
  §9.3（Dictionary word tiers）。参照した実装: `rust/src/interpreter/resolve_word.rs`。

## 出発点（問題意識）

Ajisai は「数値を内部的に連分数で扱う」「操作対象モード／消費モード」など独自性が強いが、
これらは GUI で効果が可視化されるため実用上の難所にはなりにくい。
**最もわかりにくいのは、辞書における Words 同士の優先順位・格付けである** —— という観察が出発点。

本分析の結論を先に述べる:

> **「優先順位」そのものは 4 段のはしご 1 本で、実は単純。
> わかりにくさは優先順位ではなく、"格付けを語る語彙が多重定義されていること" から来ている。**

---

## 1. 実際の解決順序は 4 段のはしご（単純）

裸の名前（bare name）の解決順序は `resolve_word.rs::resolve_short_name` が唯一の定義であり、
次の順で最初に当たったものが勝つ:

1. **Core**（`core_vocabulary`）
2. **インポート済み Module 語**（`is_module_word_imported` が真のもののみ）
3. **所有辞書**（いま実行中の語が属する辞書 = 自己参照。`owning_dictionary_context`、§8.6）
4. **その他の User 辞書**（同名衝突は `registration_order` で最も古い登録が勝つ）

修飾名 `X@WORD` はこのはしごを通らず、`X`（`CORE` / モジュール / ユーザー辞書）へ直行する
（`resolve_word_entry_readonly` の layers 分岐）。

### 1.1 ここから出る強い不変条件

- **裸の名前では User 語が Core / Module を上書き（shadow）できない。**
  自分の所有辞書内ですら Core が先に勝つ（順序が Core → Module → owning dict だから）。
  ユーザー視点では **「組み込みは絶対に裏切らない」** の一文で言い切れる。
- User 辞書間の衝突は **登録順（最古勝ち）** で決まり、名前の再解決は実行時に起こらない
  （依存は content identity で固定。§8.6）。

優先順位の本体はこれで全部である。ここは追加説明をほとんど要しない。

---

## 2. わかりにくさの正体 — 3 本の独立した軸がもつれている

正典では、本来独立した 3 つの軸が同じ表・同じ用語の密度で語られている:

| 軸 | 何を決めるか | 解決への影響 | 登場する語 |
|---|---|---|---|
| **Tier（格）** | 変更可能性 | なし | Core(永続) / Module(着脱) / User(編集可)（§9.3） |
| **Canonical home** | 解決・IMPORT 先 | **あり** | Core / Module(name)（§7, §7.14） |
| **Listing（掲載）** | 表示・ブラウズ | なし（presentation のみ） | listed_in_core / _modules / _categories（§7.14） |

**解決に効くのは canonical home だけ**。にもかかわらず、解決に効かない listing が
同じ密度で書かれているため、読者は「これも優先順位に関わるのか?」と毎回身構える。
これが認知コストの主因。

### 2.1 "Core" が 4 通りの意味で使われている

| 用法 | 意味 | 出典 |
|---|---|---|
| Core Words **tier** | 永続ランタイム語（削除・改名・再定義不可） | §9.3 |
| **Coreword** | 契約メタを持つ組み込み = Core 語 *または* Module 語 | §7.14（§9.3 自身が「紛らわしい」と注記） |
| Core **listing view** | ブラウズ用の掲載先（解決に無関係） | §7, §7.14 |
| `CORE@` **名前空間** | 修飾解決のターゲット | `resolve_word_entry_readonly` |

§9.3 が明示的に「Core Words tier は Coreword より狭い」と断り書きを入れている点が、
語の衝突が実在することの証左。

### 2.2 boundary word の 4 分類と category

- **boundary word（4 分類）**: canonical home(Core/Module) × その逆側 listing 有無 の直積を
  名前付きクラスにしたもの（§7 の表）。本質は 2 bit の情報。
- **category**（CAST / TEXT / TENSOR / RUNTIME）: モジュールに見えてモジュールではない。
  `MODULE_SPECS` に登録されず IMPORT 不可、ドキュメント専用ラベル（§7, §7.14）。

いずれも **presentation 層**（解決に無関係）でありながら、解決軸と同じ場所で語られている。

---

## 3. 将来の簡潔化に向けた 2 つの独立したレバー

> ※ 本書はメモであり、以下は採否未定の検討材料。実施は別途判断する。

- **(a) 用語の脱・多重定義（挙動不変・低コスト・効果大）**
  "Core" の 4 義を割り当て直す。例: 契約メタの "Coreword" を *Built-in* / *Contracted word*
  に改名し、Tier の "Core" と分離する。挙動・正典構造には踏み込まない範囲でも実施可能。
- **(b) presentation 層の構造的削減**
  boundary word の 4 分類を名前付きクラスとして廃し、
  「語は canonical home を 1 つ持ち、tag（掲載先）を任意個持つ」だけに還元。
  category も同じ tag 機構に吸収。**解決セマンティクスは触らない。**
  これは §7 / §7.14 に変更が入るため、正典改訂を伴う。

両者は独立しており、(a) のみ・(a)+(b)・どちらもやらない、のいずれも選べる。

---

## 4. Reference 構成への提案（参考）

簡潔化に踏み込むか否かにかかわらず、Reference 側は次の順で書くと 3 軸が分離されて読みやすい:

1. **優先順位 = はしご 1 本**（§1 の 4 段）を冒頭に置く。
   併せて「組み込みは shadow されない」「User 辞書間は最古勝ち」の 2 不変条件を太字で。
2. **格付け = 変更可能性の 3 段**（Core 永続 / Module 着脱 / User 編集可）を次に。
3. **掲載（listing / category）= 解決に無関係な棚**であることを明示し、後段へ降格。

「優先順位」「格付け」「掲載」を別物として並べることで、
読者が listing/category を優先順位の一部と誤解する余地を最初から断てる。

---

## 5. User Word の同名衝突とインポート（現状の確証）

Core / Module は Ajisai 公式が管理するため悩みは少ない。問題は **User Word**
（ユーザーが自由に作り、有力ライブラリの出現が望まれる層）に集中する。
以下は実装で確証した現状の挙動。

### 5.1 懸念A — 複数 User 辞書に同名（例: XXX@TEST と YYY@TEST）

**現状は「黙って、登録が最も古い辞書が勝つ」。エラーも警告も出ない。**

- `resolve_short_name`（裸名の唯一の解決経路）は
  Core → インポート済み Module → 所有辞書（§8.6） → その他 User 辞書 の順。
  最後の段で同名が複数あれば `registration_order` 昇順で**最古を選んで `Some` を返す**。
- 親切な曖昧エラー（`"Ambiguous word 'TEST': found in XXX@TEST, YYY@TEST. Use a qualified path."`）
  を返す `check_ambiguity` は存在するが、呼ばれるのは `resolve_word_entry` が `None` の
  ときの `.ok_or_else` 内だけ（`execute_builtin.rs`）。User 辞書に同名があれば必ず `Some`
  が返るため、**この曖昧検出は衝突ケースで事実上発火しない（死にコード）**。
- 修飾名 `XXX@TEST` / `YYY@TEST` なら確実に指定できる。問題は裸名が順番依存で静かに
  解決される点。これは「ユーザーが User word の扱いに悩む」状況そのもの。

### 5.2 懸念B — Example をリネームせず再インポート

**推測どおりマージ（上書き）。** 仕組みは §8.6 + §9.3：

- 辞書名は**ファイル名**から決まる（リネームしなければ `EXAMPLE` のまま）。
- インポートは **content identity でマージ**し、中身が同一の語は自動 dedup
  → だからセレクタが2個に増えなかった（観測と一致）。
- エクスポートファイルを**編集して** `EXAMPLE` のまま戻すと、編集語は新 identity を持ち、
  名前→identity の map である辞書内で**同名キーを張り替える**。バンドル Example が壊れうる。

## 6. content-first 化（名前より内容を優先）— 既に土台はある

`rust/src/interpreter/word_identity.rs` は §8.6 の content-addressing を**実装済み**：

- `recompute_word_identities`: 各 User Word の `id = H(normalize(body) ⊕ {id(deps)})` を算出。
  参照は名前ではなく**参照先 identity** に置換してからハッシュ（名前スコープと独立）。
  再帰群は Tarjan の SCC で単位ハッシュ。結果は `self.word_identities`（fq名→id）。
- `body_store`: 同一 body を `Arc` 共有し、`gc_body_store` で孤児を回収。
- `body_content_key`: 名前非依存の content key（`1` と `1/1` も正規化で一致）。

**つまり codebase / identity 層は content-first。乖離はランタイム解決層だけ** ——
`resolve_short_name` は `word_identities` を一切参照せず、名前 + `registration_order` で引く
（name-first）。懸念A・B の根因はこの一点に集約される。

### 6.1 content-first にすると2つの懸念が1本化する

- **懸念A の精密化**: 「曖昧ならエラー」は正しいが、content を見るとさらに正確になる。
  - 一致する複数エントリの **identity が同一** → それは同じ語。曖昧ではないので解決してよい。
  - identity が **相違** → 真の曖昧 → エラーで修飾を促す。
  - すなわち **ambiguous = 名前の一致ではなく「内容の相違」**。
- **懸念B の無害化**: content-addressing を徹底すれば再インポートは codebase レベルで非破壊。
  - 同一 Example → 同 identity → 完全 dedup（真の no-op）。
  - 編集して `EXAMPLE` のまま戻す → 新 identity が**併存**。旧定義は残り依存者は旧 id に pin
    （§8.6「previous identity is unaffected」）。**消えない**。
  - よって「リネームなし=エラー」は安全策としては不要に縮退。残る論点は naming 層の UX
    のみ（予約棚 `EXAMPLE` が黙って張り替わると混乱）→「予約名保護 + 警告」で足りる。

## 7. 設計判断（このセッションで合意）

> ※ 本書はメモ。実装は別途着手判断する（本書時点では正典・実装とも未変更）。

1. **裸名の辞書間曖昧 → エラー**。死んでいる `check_ambiguity` を生かす。
   content-first 版では「identity が相違する複数辞書に同名」のときのみエラー、
   identity 一致なら解決。
2. **リネームなしインポート → content-first を徹底**し、再インポートを非破壊化。
   `EXAMPLE` は予約名としてインポート先から保護し、同名辞書への取り込みは警告。
   ハードエラーには頼らない方針。
3. ご提案「同じ階層に同じ名前を許さない」は次のように整理して採用：
   - **辞書名（層の名前）は User 辞書間で一意**、`EXAMPLE` は予約（懸念B の解）。
   - **語名の辞書間併存は許す**（§8.6 の設計）。ただし裸名が **content 相違で曖昧**なら
     黙選せずエラー（懸念A の解）。

## 8. 未解決の論点（次に決めること）

- content-first 解決を実装する場合、`resolve_short_name` を `word_identities` 参照に
  拡張する範囲と、`resolve_cache` / `registration_order` タイブレークとの整合。
- `EXAMPLE` 予約名保護と「同名辞書マージ時の警告」を engine 側と GUI 側のどちらで担うか。
- 用語整理（レバー a）にどこまで踏み込むか。"Coreword" の改名は正典・実装・テストの
  広範囲に波及しうるため、影響範囲の棚卸しが先。
- presentation 層の構造削減（レバー b）を行う場合、§7 の boundary 表と §7.14 の
  listing fields をどう tag モデルへ書き換えるか。
- Reference を `public/docs/` のどの粒度で新設・改訂するか。
