pub(crate) fn lookup_detail_cond(name: &str) -> Option<String> {
    let result = match name {
        "COND" => {
            r#"# COND - 条件分岐

## 機能
ガードと本体のペアを順に評価し、最初にTRUEになった本体を実行します。

## 使用法
value { guard1 } { body1 } { guard2 } { body2 } ... COND

## 使用例
[ 42 ]
  { [ 0 ] < }   { 'negative' }
  { IDLE }      { 'positive' }
  COND

## 注意
- ガード/本体は必ずペアで指定します
- else節は `{ IDLE }` ガードで表現します
- 一致なし・elseなしはエラーになります"#
        }
        _ => return None,
    };
    Some(result.to_string())
}
