# docs/dev/ INDEX

Status: non-canonical. この索引を含め、`docs/dev/` 配下の全文書は Ajisai の
意味論・互換性方針を定義しない。正典は `spec/` 配下の各ソースと、そこから生成される
`SPECIFICATION.html` のみ。

状態タグの意味:

- `[執筆規約]` — 正典・Reference の執筆規律。
- `[設計根拠]` — 現行実装が依拠する設計文書。コード・CI から参照される。
- `[方針記録]` — 採用済みの設計判断とその理由の記録。
- `[観察ノート]` — 実装の記述的分析。方針を定めない。

## 執筆規約・形式化

| 文書 | 説明 | 状態 |
| --- | --- | --- |
| `ajisai-authoring-style.md` | 正典 HTML 文書の執筆規約（コード/数式チャネル分離、KaTeX） | `[執筆規約]` |
| `structured-prose-style.md` | 情報の形の選び方（文 vs label:value vs 表 vs 図）。多言語対応を理由とする | `[執筆規約]` |
| `reference-writing-style.md` | Reference 表面の執筆規約 | `[執筆規約]` |
| `specification-implementation-rules.md` | 実装の工学規律（命名、500行予算、制御フロー、コメント）。1件目は `check-file-size-budget.mjs` が強制 | `[執筆規約]` |
| `character-allocation-and-prose-2026-09.md` | 字句に文字を取ってよいかの規準——その文字が文書側で区切りとして稼働しているかを見る。空いている枠を消費するのと、使われている文字を取り上げる交換は別の判断である、という切り分け。`ajisai-authoring-style.md` §2・§8（散文は言語の字面を避けよ）の対として、言語が散文の区切りを避ける側を述べる。損失が現れる3つの形（実行で検査される読み物が例外を必要とする・`#:contract` と `#@` でコロンが既に稼働・`label:value` は執筆規約が指定した形）。実例は `{ }`（空いていた→取った）と `:`（稼働中→見送った） | `[方針記録]` |
| `dry-criterion-2026-09.md` | DRY を形ではなく知識で測る判断基準と、その基準による一回の点検結果（既定ステップ予算の言い直し、診断形式の二重化、永続化レコード形の言い直し） | `[方針記録]` |
| `three-layer-documentation-model.md` | ワードヘルプの三層モデル（Reference / LOOKUP / hover） | `[執筆規約]` |
| `devlog-format.md` | Blogger 開発ログ記事の形式（目的/手段/結果/課題の四項目・約500字）と誠実性の規律 | `[執筆規約]` |

## 言語・実装

| 文書 | 説明 | 状態 |
| --- | --- | --- |
| `spec-impl-alignment-methodology.md` | 仕様・実装整合化の4フェーズ手順とスイート裁定規則。`spec-impl-drift-tactic.md` の後継 | `[設計根拠]` |
| `semantic-spine-migration-plan.md` | 整理後正典と整理前実装の乖離を収束させる Semantic Spine 移行計画（9 Phase） | `[方針記録]` |
| `vocabulary-100-work-order-2026-09.md` | 語彙を 66 語から 100 語へ再設計する改修指示書。語を採る二つの級（書けないもの／族を閉じるもの）、三つの分岐（Tier 2 の払い切り・形と階の語彙化・第 7 の値領域 Record）、10 概念の組み替え、100 語の割り当て、Phase 1〜7。設計判断は本書で確定済み。Phase 1（`CEIL` `DROP` 追加・`SUM` 削除）・Phase 2（形と階の 5 語）・Phase 3（探索の 4 語）・Phase 4（結末の宣言 `ABSENT` `FAIL`）・Phase 5（第 7 領域 Record の 9 語）・Phase 6（反射と境界の 6 語、`PROBE` 再定義）・Phase 7（数の払い切り 8 語、数値カーネル拡張）実施済み——全 Phase 完了、100 語。§7.2 の事後調整（`PROBE`→`CONTRACT` 統合、`NEQ` 退役、`UPPER` `LOWER` 追加、丸め規則の統一）も実施済み。**§7.3 に alpha 期の採用規則**（追加は「言語内で書けない語」か「導出可能な語一つとの入れ替え」のどちらかを満たすこと。語数そのものは alpha の制約にしない）と、その最初の適用として τ を検討し入れないと決めた記録。**§7.4 に退役候補の列**（入れ替えに出す導出可能 10 語を失うものが少ない順に固定。`check-minimal-core.mjs` が列の不変条件を検査する。語数を 90 に抑えて枠を空ける案を検討し、採らなかった記録も含む）。添付 `_attachments/vocabulary-100-draft-contracts.json` に新語 35 件と再定義 3 件の契約スケルトン | `[設計根拠]` |
| `ajisai-minimal-core-identity.md` | 何が変われば Ajisai でなくなるか——同一性の幹の切り分け | `[方針記録]` |
| `record-display-round-trip-2026-09.md` | Record の表示を `{ 'x': 1/1 }` から構成子呼び出し `[ 'x' ] [ 1/1 ] RECORD` に変え、あらゆる値の表示をそれ自身を再現するソースにした記録。往復性にリテラルは要らないという観察。Record を含む Vector がリテラル形では「別の値に読み戻る」ため `COLLECT` 形が要ったこと、断片が正味1値を残すという合成の不変条件。代償（`CONTRACT` が最悪ケース）と、それを人間だけが負う理由。GUI が空 Vector を不正な `[]` と描いていた発見と、二実装を固定する新設テスト。**表示形と `COLLECT` 形は `record-literal-2026-09.md` が置き換えた**（往復性の要求と、それを満たす形の探索の記録として残す） | `[方針記録]` |
| `record-literal-2026-09.md` | Record に字面 `{ key value … }` を与え、`{` `}` を Record リテラルの区切りとして割り当てた記録。直前の2判断（表示＝ソース化と記号の解放）が合わせて作った穴——空いた2文字の代わりに可読性を払っていた——という見立て。`:` が字句的に書けない理由（文字列の閉じ規則）、字面が評価しないことで `COLLECT` 形が不要になった経緯、エラーを構成子と同じ2つに限った理由、静的予測が定数を実際に組み立てて答えること、交差ペアで前検査が止まる理由。手放したもの（名前に波括弧を使えない・`'{' DEF`・compiled plan の未下ろし） | `[方針記録]` |
| `source-character-liberation-2026-09.md` | 字句文法から文字単位の拒否規則を全廃した記録（`( ) { }` の `reservedMarker`/`retiredForm`、単独 `|` の `retiredCondSeparator`）。文法が自分の `nameCharacter` 注記と矛盾していたこと、`( )` の予約が設計の放棄した役割のためだったこと。唯一のブロッカーだった Record 表示の読み戻し保証を reference-lexer で実測し、実際に守っているのは波括弧の拒否ではなく文字列の閉じ規則だと判明した経緯。落ちた検査9件を符号反転で置き換えた方針と、`naming_convention_checker.rs` に分かれていた二つ目の `|` 規則 | `[方針記録]` |
| `vector-nesting-role-redefinition.md` | Vector ネストの役割（Lisp 的動機の廃止） | `[方針記録]` |
| `ajisai-single-axis-proposal-2026-08.md` | 中心概念を「絞り込み（narrowing）」一本に定める提案。到達不能契約の実測と、七つの改修案。うちⅡ・Ⅲ・Ⅴ・Ⅵ・Ⅶは実施済み（PR #1563/#1564/#1567）、Ⅰは指示書のみ | `[観察ノート]` |
| `type-unification-work-order-2026-08.md` | 改修Ⅰ（CodeBlock/Vector 統合）の指示書。読み取り専用の前提再検証（Phase 0）→ 使い捨てブランチでの測定スパイク（Phase 1）→ 実装（Phase 2）の三段階。破壊的変更であり、Phase 2 はユーザー承認を得て実施済み | `[設計根拠]` |

## エージェント/CLI・GUI

| 文書 | 説明 | 状態 |
| --- | --- | --- |
| `agent-cli-output-contract.md` | `ajisai` CLI の `--json` 出力契約 | `[設計根拠]` |
| `cli-repl-phase8a-design.md` | `ajisai repl` の設計メモ | `[設計根拠]` |
| `cli-test-phase8a-design.md` | `ajisai test` の設計メモ（`#@` directive コメント） | `[設計根拠]` |
| `gui-current-design-memory.md` | GUI 現行設計メモ | `[設計根拠]` |
| `mcp-host-profiles.md` | ホストごとの資源上限プロファイル対照表と、意図された差分 | `[設計根拠]` |
| `mcp-readiness.md` | MCP 製品化の実装トラッカー（達成した exit criteria のみを記録する） | `[方針記録]` |
| `mcp-claude-code-handoff.md` | MCP 開発の次担当への引き継ぎ（現行方針・禁止事項） | `[方針記録]` |
| `mcp-hard-use-findings-work-order-2026-09.md` | MCP を実際に使い込んで出た欠陥 9 件の改修指示書。F-1（NIL レーンを含む Vector への算術がプロセスを落とす）が最優先。所有者判断待ちの 2 件を明示 | `[設計根拠]` |
| `playground-report-validation-2026-09.md` | 外部セッションによる Playground 検証レポート 8 件の妥当性確認と、そこから実施した改修（PI の失敗原因の誤帰属、宣言済み条件の診断分類、実時間タイムアウトの診断、ja Reference の同期）。未着手項目も明示 | `[方針記録]` |
| `playground-mobile-report-validation-2026-09.md` | 外部セッションによる Playground モバイルモード改善提案書 12 件の妥当性確認と、そこから実施した改修。**§5 でその一部（タッチアクションバー・記号パレットのフロー配置・スワイプ除外リスト）を実機使用の報告により撤回**した経緯と、撤回しなかった改修の切り分けも記録 | `[方針記録]` |
| `host-profile-derivation-handoff.md` | ホスト間で上限の値ではなく導出を統一する作業の引き継ぎ。走査系の非二次化の後に着手 | `[方針記録]` |
| `competitive-advantage-work-order-2026-08.md` | 競争優位の研磨（観測ダイジェスト・全数意味論表・gap ID・三分法統一・コスト契約）の改修指示書。Phase 単位で実装する。設計判断は本書で確定済み | `[設計根拠]` |
| `outcome-space-bijection-work-order-2026-09.md` | 結末空間の全単射（結末レジストリの単一化・未宣言結末の表現不可能化・ドメイン拡張・両方向ゲート）の改修指示書。全域言語であることを検証可能性に変換する。設計判断は本書で確定済み。Phase 1〜3 実施済み、Phase 4 はゲート未着手（後継書へ） | `[設計根拠]` |
| `auditable-kernel-work-order-2026-09.md` | 監査可能実行カーネルの完成（generic 結末の掃討・全単射ゲート・COND の真理値強制除去・実行証明書・静的結末予測）の改修指示書。前掲書の Phase 4 を引き継ぎ、走らせる前の完全な結末集合と走らせた後の第三者検証可能な受領証を製品にする。設計判断は本書で確定済み | `[設計根拠]` |
| `trichotomy-unification.md` | 実行時三分法と静的検査三値の対応を統一した理由と、reason レジストリ統合（案(b)）を今やらない技術的理由・再検討条件 | `[方針記録]` |
| `cost-contract-design.md` | `#:contract` のコスト軸（steps/numeric/collection）の設計根拠。クラス格子・join規則・多項式を今やらない理由・機械非依存性の正確な意味 | `[設計根拠]` |
| `cost-discoverability-work-order-2026-08.md` | 推論されたコストを `ajisai contract` に出す改修指示書。付録 A に SHA-256→BLAKE3 置換を採用しない根拠と再検討条件 | `[設計根拠]` |
| `reference-ja-restructure-handoff.md` | **破棄・非推奨**（2026-09-14）。水のメタファー導入を計画していたが、方針が逆転し撤去された。制御構造の集約という §2.2 の指摘のみ今も有効 | `[方針記録]` |
