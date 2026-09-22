# 記号の解放——`( ) { } |` を普通の名前文字に戻す（2026-09-22）

Status: 非正典。正典は `spec/grammar.json` と、そこから生成される
`SPECIFICATION.html`。

> **一部被覆（2026-09-22）**: 本書が解放した5文字のうち `{` `}` は、その後
> Record の字面の区切りとして**割り当て**られた（`record-literal-2026-09.md`）。
> 文字単位の拒否規則が全廃されたことは変わらない——`{` `}` を withhold するのは
> `[` `]` と同じ whole-lexeme 規則である。`( )` `|` は解放されたまま。

## 1. 何をしたか

字句文法から**文字単位の拒否規則を全廃**した。

| 記号 | 旧 | 新 |
| --- | --- | --- |
| `(` `)` | `reservedMarker`——語中のどこにあってもエラー | 普通の名前文字 |
| `{` `}` | `retiredForm`——語中のどこにあってもエラー | 普通の名前文字 |
| `\|` | `retiredCondSeparator`——単独の字句のときのみエラー | 普通の名前文字 |

`spec/grammar.json` から `rejectedCharacters` と `retiredCondClauseSep` が、
`sourceErrors` から3条件が消えた。`characterClasses.nameCharacter` の定義は
「空白でも拒否文字でもないもの」から「空白でないもの」になった。

`[` `]` `'` `#` は**変わらない**。これらは字句規則で働いているので、
`SURFACE_FORMS` に残る。

## 2. なぜ [方針記録]

所有者の指摘：「以下二つはレガシーな仕組みの名残という気がします。私の理想としては、
『名前の一部にしかならない普通の文字』にまとめたいです。」

その読みは正しかった。三つの根拠。

**(a) 文法が自分と矛盾していた。** `nameCharacter` の注記はこう言っていた——
"Ajisai allocates no identifier character class." その同じファイルが四文字に
per-character 規則を割り当てていた。`tokenizer.rs` でも、拒否ループの20行下に
"This is why no character needs special treatment" と書かれていた。

**(b) `( )` を選ぶ原理が無かった。** `<` `>` も `«` `»` も昔から普通の名前文字で、
`( )` だけが予約されていたのは他言語の習慣による。しかも予約の理由として出る
メッセージ自身が "'[' and ']' are the sole bracket in Ajisai" と言っており、
**設計が既に放棄した役割のために枠を押さえていた**。

**(c) 退役は移行支援で、移行は終わっていた。** COND→SELECT は 2026-09-18、
0.2.0-alpha.1 の時点。「older material で `|` に出会う読者」の older material は
実質このリポジトリの dev メモだけだった。

## 3. 唯一のブロッカーと、その実測 [設計根拠]

`rust/src/types/display.rs` の Record 表示が、波括弧が書けないことに寄りかかって
いた——「Braces are retired lexemes … so this display can never be read back as a
literal」。`is_symbol_token_lexeme`（`tokenizer.rs`）は「トークナイザが単一 Symbol
として受理するか」そのものであり、`DEF` がそれを命名可否に使っている
（`execute_def.rs`）。よって波括弧を解放すると `'{' DEF` が通る。

**この論拠を実測したところ、二重に誤っていた。** `scripts/lib/reference-lexer.mjs`
（`spec/grammar.json` を実行する正典側の字句器）で `rejectedCharacters` を空にした
文法を作り、実際に通した結果：

| 入力 | 結果 |
| --- | --- |
| `{ 'a': 1 'b': 2 }`（Text キー） | **`unclosedLiteral`** |
| `{ }`（空 Record） | `Symbol {` `Symbol }` |
| `{ 1: 2 }`（数値キー） | `Symbol {` `Symbol 1:` `Number 2` `Symbol }` |

`'a':` が通らないのは**文字列の閉じ規則**（引用符は次が空白か入力末尾のときにだけ
閉じる）のため。**Text キーを持つ Record は、波括弧の扱いに関係なく読み戻せない。**
保証を供給していたのは波括弧の拒否ではなく文字列規則だった。読み戻せてしまうのは
空 Record と非 Text キーの場合だけで、それも未知ワードとして解決に失敗する。

さらに、この話は**機械には無関係**である。`LANG.OBSERVATION.PROTOCOL` により
Record は「整列した鍵列と値列」を持つ `record` ノードとして渡され、
`LANG.OBSERVATION.FIREWALL` が表示テキストからの意味推論を禁じている。
表示形は Stack 面を読む人間のためのものである。

この実測により、当初検討した二案——(A) 表示形を `[ keys ] [ values ] RECORD` に
変えて往復性を得る、(ii) 保証を `DEF` の命名規則へ移す——はどちらも不要になった。
所有者の採択は「表示は `{ ... }` のまま、波括弧を解放し、`display.rs` のコメントを
実測どおりに書き直す」。

## 4. 検査をどう置き換えたか

拒否規則を消すと、それを固定していた検査が全て落ちる（9件）。**消さずに符号を
反転させた**——同じドリフトを反対側から捕まえる。

| 旧 | 新 |
| --- | --- |
| `rejected_characters_are_refused_anywhere_in_a_word`（`lexical_grammar_laws.rs`） | `the_grammar_declares_no_rejected_character` + `a_freed_character_is_an_ordinary_name_anywhere_in_a_word` |
| `a_retired_form_is_refused_by_name_and_points_at_its_replacement`（`surface_forms.rs`） | `a_freed_character_is_an_ordinary_name_and_is_not_listed` |
| `test_brace_is_rejected_as_source` ほか字句レベル6件 | 字句としては名前になることを固定。**「退役ブロック構文が動かない」性質は解釈器レベルへ移設**（`definable_name_tests.rs` の `retired_brace_block_syntax_still_does_not_define_a_word`） |

`surface_forms.rs` の新しい検査が要点である。この表は word manifest・SKILL.md・
quickstart に生成されるので、ここに項目があることは「その文字は語の規則を超えた
何かをする」という主張になる。死んだ概念名を再投入すれば、`{ }` が SKILL.md §9 に
「使える区切り文字」として載った過去の欠陥と**符号違いの同じ欠陥**になる。

## 5. 副次的に見つかったもの

`naming_convention_checker.rs` が `|` を「tokenizer-level syntax」として `DEF` から
弾いていた。トークナイザ側の規則を消しても、こちらが残れば `'|' DEF` は通らない。
規則が二箇所に分かれていたということで、あわせて削除した。

## 6. 残っている非対称（意図的）

`[` `]` は `bracketMustStandAlone` により、糊付きの語（`[1`、`2]`）を拒否し続ける。
これは per-character 規則ではなく**whole-lexeme 規則**であり、空白を唯一の区切りと
したまま括弧を機能させるために load-bearing である。解放の対象ではない。
