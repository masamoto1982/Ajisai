pub(crate) fn lookup_detail_control_higher_order(name: &str) -> Option<String> {
    let result = match name {
        "IDLE" => r#"# IDLE - 何もしない

## 機能
スタックを変更しない no-op ワードです。COND の else 節などに使います。

## 使用例
[ 3 ] IDLE                 # → [ 3 ]

{ IDLE } { SAY-BANG } COND   # COND の else 節として"#,

        "MAP" => r#"# MAP - 各要素にコードを適用

## 機能
Form: ベクタまたはスタックの各要素に対して、指定したコードブロックを適用します。

## 使用法
[ a... ] { code } MAP
[ a... ] 'WORD' MAP

## 使用例
[ 1 2 3 4 ] { [ 2 ] * } MAP           # → [ 2 4 6 8 ]
[ 1 2 3 ] { [ 1 ] + } MAP             # → [ 2 3 4 ]

{ [ 2 ] * [ 1 ] + } 'DOUBLE-PLUS-ONE' DEF
[ 1 2 3 ] 'DOUBLE-PLUS-ONE' MAP       # → [ 3 5 7 ]

[ 1 2 3 4 5 ]
  == { [ 2 ] * } MAP
  == { [ 5 ] < NOT } FILTER
  == [ 0 ] { + } FOLD
# → [ 24 ]

## 注意
- コードブロック `{ ... }` またはワード名で指定
- 各要素に対して独立して実行されます"#,

        "FILTER" => r#"# FILTER - 条件に合う要素を抽出

## 機能
Form: ベクタの各要素に対して述語コードを評価し、TRUE を返した要素のみを残します。該当がない場合は NIL を返します。

## 使用法
[ a... ] { predicate } FILTER
[ a... ] 'WORD' FILTER

## 使用例
[ 1 2 3 4 5 6 ] { [ 2 ] MOD [ 0 ] = NOT } FILTER
# → [ 1 3 5 ]（奇数のみ）

[ 10 5 20 3 15 ] { [ 10 ] < NOT } FILTER
# → [ 10 20 15 ]（10以上）

{ [ 5 ] < NOT } 'IS-LARGE' DEF
[ 1 3 8 2 7 ] 'IS-LARGE' FILTER       # → [ 8 7 ]

[ 1 2 3 ] { [ 10 ] < NOT } FILTER => [ 0 ]
# → [ 0 ]（該当なしで NIL、=> でフォールバック）

## 注意
- 述語は TRUE/FALSE を返す必要があります
- 該当要素がない場合は NIL"#,

        "FOLD" => r#"# FOLD - 初期値付き畳み込み

## 機能
Form: ベクタまたはスタックの要素を、初期値から左から右へ二項演算コードで畳み込み、単一の結果にします。空ベクタでは初期値をそのまま返します。

## 使用法
[ a... ] [ init ] { op } FOLD
[ a... ] [ init ] 'WORD' FOLD

## 使用例
[ 1 2 3 4 ] [ 0 ] { + } FOLD    # → [ 10 ]
[ 1 2 3 4 ] [ 1 ] { * } FOLD    # → [ 24 ]
[ 1 2 3 ] [ 10 ] { - } FOLD     # → [ 4 ]（10-1-2-3）

NIL [ 42 ] { + } FOLD           # → [ 42 ]（空なら初期値）

## 畳み込み順序
[ a b c ] [ init ] { op } FOLD = ((init op a) op b) op c

## 注意
- 演算コードは2値を取り1値を返す必要があります
- 組み込み演算（+, -, *, /）もユーザーワードも使用可能"#,

        "UNFOLD" => r#"# UNFOLD - 状態遷移から列を生成

## 機能
Form: 初期状態からコードを反復実行し、`[ element next_state ]` を返す限り element を列に追加していきます。NIL を返した時点で終了します。

## 使用法
[ 初期状態 ] { code } UNFOLD
[ 初期状態 ] 'WORD' UNFOLD

## 使用例
[ 1 ]
  { { [ 1 ] = $ [ 1 2 ] }
    { [ 2 ] = $ [ 2 3 ] }
    { [ 3 ] = $ [ 3 NIL ] }
    { IDLE    $ NIL }
    COND }
  UNFOLD
# → [ 1 2 3 ]

## 注意
- 各反復の返り値は NIL または `[ element next_state ]`
- next_state が NIL なら、その要素を追加した直後に終了
- 10000 回連続で終了しない場合はエラー"#,

        "ANY" => r#"# ANY - 1つでも条件を満たすか判定

## 機能
Form: ベクタの各要素に述語を適用し、1つでも TRUE ならば TRUE を返します（短絡評価）。

## 使用法
[ a... ] { predicate } ANY
[ a... ] 'WORD' ANY

## 使用例
[ 1 3 5 8 ] { [ 2 ] MOD [ 0 ] = } ANY   # → TRUE
NIL ANY                                  # → FALSE

## 注意
- 述語は TRUE/FALSE を返す必要があります
- 非 Boolean を返すとエラー"#,

        "ALL" => r#"# ALL - 全要素が条件を満たすか判定

## 機能
Form: ベクタの各要素に述語を適用し、全要素が TRUE ならば TRUE を返します（短絡評価）。

## 使用法
[ a... ] { predicate } ALL
[ a... ] 'WORD' ALL

## 使用例
[ 2 4 6 8 ] { [ 2 ] MOD [ 0 ] = } ALL   # → TRUE
NIL ALL                                  # → TRUE（空は真）

## 注意
- 述語は TRUE/FALSE を返す必要があります
- 非 Boolean を返すとエラー"#,

        "COUNT" => r#"# COUNT - 条件一致件数を数える

## 機能
Form: ベクタの各要素に述語を適用し、TRUE になった件数を返します。中間ベクタは生成しません。

## 使用法
[ a... ] { predicate } COUNT
[ a... ] 'WORD' COUNT

## 使用例
[ 1 2 3 4 5 6 ] { [ 2 ] MOD [ 0 ] = } COUNT   # → [ 3 ]
NIL COUNT                                      # → [ 0 ]

## 注意
- 述語は TRUE/FALSE を返す必要があります"#,

        "SCAN" => r#"# SCAN - FOLD の途中経過を返す

## 機能
Form: 初期値から左畳み込みを行い、各ステップ後の累積値をベクタで返します。空入力の場合は NIL を返します。

## 使用法
[ a... ] [ init ] { op } SCAN
[ a... ] [ init ] 'WORD' SCAN

## 使用例
[ 1 2 3 4 ] [ 0 ] { + } SCAN   # → [ 1 3 6 10 ]
NIL [ 0 ] { + } SCAN           # → NIL

## 注意
- 演算コードは2値を取り1値を返す必要があります"#,

        _ => return None,
    };
    Some(result.to_string())
}
