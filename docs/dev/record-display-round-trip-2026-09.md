# Recordの表示を往復可能にする（2026-09-22）

Status: 非正典。正典は `spec/language-semantics.md` と `SPECIFICATION.html`。

## 1. 何をしたか

Record の表示を、それを組み立てる呼び出しそのものに変えた。

| | 旧 | 新 |
| --- | --- | --- |
| Record | `{ 'x': 1/1 'y': 2/1 }` | `[ 'x' 'y' ] [ 1/1 2/1 ] RECORD` |
| 空 Record | `{ }` | `[ ] [ ] RECORD` |
| Record を含む Vector | `[ { 'a': 1/1 } ]` | `[ 'a' ] [ 1/1 ] RECORD 1 COLLECT` |
| Record を含まない Vector | `[ 1/1 2/1 ]` | **変更なし** |

これで **Ajisai のあらゆる値の表示が、その値を再現するソースになった**。Stack から
コピーしてエディタに貼れば、同じ値が戻る。

## 2. なぜ [方針記録]

Vector（`[ 1/1 2/1 ]`）、Text（`'ab'`）、Boolean、NIL、有理数はもともと往復して
いた。**Record だけが穴**で、理由は「Record にリテラルは無い、`RECORD` だけが
Record を存在させる唯一の入口」（LANG.RECORDS.STRUCTURE）という意図的な規則
だった。

ここで効いた観察は、**往復性にリテラルは要らない**ということである。必要なのは
「表示がその値に評価される正当なソースであること」だけで、構成子呼び出しは
その条件を満たす。だから規則を曲げずに穴が塞がった。むしろ表示が規則そのものを
述べる形になっている。

## 3. `COLLECT` 形が必要になった理由 [設計根拠]

**これが実装上の要点である。** 素朴にやると、Record を含む Vector の表示が
壊れる。

ブラケットリテラルは中身を評価しない。`[ [ 'a' ] [ 1 ] RECORD ]` は
**3要素の Vector**（Vector 2つと名前 `RECORD`）であって、1要素の Vector では
ない。実測：

```
[ 'outer' ] [ [ 'a' ] [ 1 ] RECORD ] RECORD
=> error: Vector length mismatch: 1 vs 3
```

つまり Record を含む Vector をリテラル形で表示すると、**「読み戻せない」ではなく
「読み戻すと別の値になる」** 表示が生まれる。表示がやりうる最悪のことである。

そこで、Record を含む Vector だけ `e1 e2 … N COLLECT` に切り替えた。これが
成立するのは、**この表示器が書く断片がどれもスタックに正味1つの値を残す**ため
であり、その不変条件のおかげで断片が入れ子にできる（Ajisai にスタック操作語は
無いので、これが無いと合成できない）。

Record を含まない Vector は従来どおりリテラルのまま。圧倒的多数の表示は不変で
ある。

## 4. 法則として書いた

`rust/tests/round_trip_laws.rs` は、表示文字列を別の文字列と比較するのではなく
**実行する**。値を表示し、表示を走らせ、同じ値が出ることを要求する。綴りを
固定する検査は、まさに変わりうるものを固定してしまうので採らなかった。

空振りしないことを確認済み：`render_vector_source` の `COLLECT` 分岐を潰すと
`a_record_nested_in_a_vector_round_trips` だけが落ちる。

## 5. 代償——`CONTRACT` が最悪ケース

キーと値が横に並ばなくなるので、読むときに位置を数える必要がある。**キー数が
多いほど、入れ子があるほど悪化する。** 実測での最悪例は `CONTRACT`
（値に Record を含むので `COLLECT` 形も混ざる）：

```
旧: { 'inputs': 0/1 'outputs': 1/1 'nil': 'neverCreates' 'purity': 'pure'
      'determinism': 'deterministic'
      'cost': { 'steps': 'const' 'numeric': 'const' 'collection': 'const' }
      'effects': [ ] 'confidence': 'complete' 'gaps': [ ] }

新: [ 'inputs' 'outputs' 'nil' 'purity' 'determinism' 'cost' 'effects'
     'confidence' 'gaps' ]
    0/1 1/1 'neverCreates' 'pure' 'deterministic'
    [ 'steps' 'numeric' 'collection' ] [ 'const' 'const' 'const' ] RECORD
    [ ] 'complete' [ ] 9 COLLECT RECORD
```

所有者の判断で受け入れた代償。**負担するのは人間だけ**である：
LANG.OBSERVATION.PROTOCOL により Record は「整列した鍵列と値列」を持つ
`record` ノードとしてホストへ渡り、LANG.OBSERVATION.FIREWALL が表示テキストから
意味を推論することを禁じているので、エージェント側はこの変更の影響を受けない。

## 6. 副次的に見つかったもの

**GUI が空 Vector を `[]` と描いていた。** エンジンは `[ ]`。字句的に `[]` は
`bracketMustStandAlone` で拒否されるので、**GUI の表示はソースとして不正だった**。
表示がソースであることを要求しない間は誰も気づかなかった。修正済み。

この種のずれを二度と起こさないため、`src/gui/output-display-renderer.test.ts` を
新設した。GUI 側の描画（プロトコルノードから再構成する独立実装）を、**エンジンが
実際に出力した文字列**に対して固定する。期待値は手書きではなく `ajisai run` から
採取したものである。

## 7. 往復しないもの（意図的、主張もしていない）

- **Symbol**：裸の名前として表示されるので、書き戻すとワードを *呼ぶ*。値として
  積まれない。
- **役割依存の表示**（datetime、区間、連分数）：`format_with_hint` が作るもので、
  構造表示器の担当外。`display.rs` と `display_source.rs` を分けたのはこの線を
  引くためでもある——`display_source.rs` の出力だけが往復法則の対象。
