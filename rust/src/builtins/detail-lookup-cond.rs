pub(crate) fn lookup_detail_cond(name: &str) -> Option<String> {
    let result = match name {
        "COND" => {
            r#"# COND - 条件分岐

## 機能
ガードと本体のペアを順に評価し、最初にTRUEになった本体を実行します。

## 使用法
value { guard1 } { body1 } { guard2 } { body2 } ... COND
value { guard1 $ body1 } { guard2 $ body2 } ... COND

## 使用例
[ 42 ]
  { [ 0 ] < $ 'negative' }
  { [ 0 ] = $ 'zero' }
  { IDLE    $ 'positive' }
  COND

## 旧構文と等価な例
[ 42 ]
  { [ 0 ] < }   { 'negative' }
  { [ 0 ] = }   { 'zero' }
  { IDLE }      { 'positive' }
  COND

## 注意
- `$` を使う場合は `{ guard $ body }` を1節として書きます
- `$` 節は1行に1節のみです（同一行に複数節は禁止）
- 旧構文 `{ guard } { body }` と `$` 構文の混在は禁止です
- else節は `{ IDLE }` または `{ IDLE $ body }` で表現します
- 一致なし・elseなしはエラーになります"#
        }
        _ => return None,
    };
    Some(result.to_string())
}
