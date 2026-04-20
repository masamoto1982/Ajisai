pub(crate) fn lookup_detail_vector_ops(name: &str) -> Option<String> {
    let result = match name {
        "GET" => r#"# GET - 指定位置の要素を取得

## 機能
Form: ベクタまたはスタックから、指定インデックス（0オリジン）の要素を取得します。負のインデックスは末尾からの位置を表します。

## 使用法
[ a... ] [ index ] GET
a... [ index ] .. GET

## 使用例
[ 10 20 30 ] [ 0 ] GET     # → [ 10 20 30 ] [ 10 ]
[ 10 20 30 ] [ -1 ] GET    # → [ 10 20 30 ] [ 30 ]（末尾）

a b c [ 1 ] .. GET         # → a b c [ b ]

## 注意
- インデックス範囲外はエラー（`~` モードで NIL に変換可能）
- 空のベクタに対する操作はエラー"#,

        "INSERT" => r#"# INSERT - 指定位置に要素を挿入

## 機能
Form: ベクタまたはスタックの指定位置（0オリジン）に要素を挿入します。負のインデックスは末尾からの位置を表します。

## 使用法
[ a... ] [ index value ] INSERT
a... [ index value ] .. INSERT

## 使用例
[ 1 3 ] [ 1 2 ] INSERT       # → [ 1 2 3 ]
[ a c ] [ 0 b ] INSERT       # → [ b a c ]（先頭に挿入）
[ 1 2 3 ] [ 3 4 ] INSERT     # → [ 1 2 3 4 ]（末尾に追加）

a c [ 1 b ] .. INSERT        # → a b c

## 注意
- 引数は `[ index value ]` の形式で指定します
- 負のインデックスは要素位置を指します（-1 = 末尾の要素の位置）"#,

        "REPLACE" => r#"# REPLACE - 指定位置の要素を置換

## 機能
Form: ベクタまたはスタックの指定位置（0オリジン）の要素を置き換えます。

## 使用法
[ a... ] [ index value ] REPLACE
a... [ index value ] .. REPLACE

## 使用例
[ 1 2 3 ] [ 0 9 ] REPLACE    # → [ 9 2 3 ]
[ 1 2 3 ] [ -1 9 ] REPLACE   # → [ 1 2 9 ]

a b c [ 1 X ] .. REPLACE     # → a X c

## 注意
- 引数は `[ index value ]` の形式で指定します
- インデックス範囲外はエラー"#,

        "REMOVE" => r#"# REMOVE - 指定位置の要素を削除

## 機能
Form: ベクタまたはスタックの指定位置（0オリジン）の要素を削除します。

## 使用法
[ a... ] [ index ] REMOVE
a... [ index ] .. REMOVE

## 使用例
[ 1 2 3 ] [ 0 ] REMOVE       # → [ 2 3 ]
[ 1 2 3 ] [ -1 ] REMOVE      # → [ 1 2 ]（末尾）

a b c [ 1 ] .. REMOVE        # → a c

## 注意
- インデックス範囲外はエラー"#,

        "LENGTH" => r#"# LENGTH - 要素数を取得

## 機能
Form: ベクタまたはスタック全体の要素数を返します。

## 使用法
[ a... ] LENGTH
a... .. LENGTH

## 使用例
[ 1 2 3 4 5 ] LENGTH       # → [ 1 2 3 4 5 ] [ 5 ]
[ ] LENGTH                 # → NIL [ 0 ]

a b c d .. LENGTH          # → a b c d [ 4 ]

## 注意
- 空ベクタ `[ ]` は NIL として扱われ、長さは 0 です"#,

        "TAKE" => r#"# TAKE - 先頭または末尾から N 個の要素を取得

## 機能
Form: 正の数なら先頭から、負の数なら末尾から、指定個数の要素を取り出します。

## 使用法
[ a... ] [ n ] TAKE
a... [ n ] .. TAKE

## 使用例
[ 1 2 3 4 5 ] [ 3 ] TAKE     # → [ 1 2 3 ]
[ 1 2 3 4 5 ] [ -2 ] TAKE    # → [ 4 5 ]

a b c d e [ 3 ] .. TAKE      # → a b c

## 注意
- 指定個数が要素数を超える場合はエラー"#,

        "SPLIT" => r#"# SPLIT - 指定サイズで分割

## 機能
Form: ベクタまたはスタックを、指定サイズの組に分割します。余りは最後の組に含まれます。

## 使用法
[ a... ] [ size1 size2 ... ] SPLIT
a... [ size1 size2 ... ] .. SPLIT

## 使用例
[ 1 2 3 4 5 6 ] [ 2 3 ] SPLIT
# → [ 1 2 ] [ 3 4 5 ] [ 6 ]

a b c d e [ 2 1 ] .. SPLIT
# → [ a b ] [ c ] [ d e ]

## 注意
- サイズの合計が要素数を超える場合はエラー
- 最低1つのサイズ指定が必要です"#,

        "CONCAT" => r#"# CONCAT - ベクタを連結

## 機能
Form: 複数のベクタを1つに連結します。デフォルトは2個、個数指定可能で、負数を指定すると逆順で連結します。

## 使用法
vec1 vec2 CONCAT                # 2個を連結
vec1 vec2 vec3 [ 3 ] CONCAT     # 3個を連結
vec1 vec2 vec3 [ -3 ] CONCAT    # 3個を逆順で連結
a... .. CONCAT                  # スタック全体

## 使用例
[ a ] [ b ] CONCAT                # → [ a b ]
[ a ] [ b ] [ c ] [ 3 ] CONCAT    # → [ a b c ]
[ a ] [ b ] [ c ] [ -3 ] CONCAT   # → [ c b a ]

a b c .. CONCAT                   # → [ a b c ]

## 注意
- 最初のベクタの括弧タイプが結果に適用されます"#,

        "REVERSE" => r#"# REVERSE - 要素の順序を反転

## 機能
Form: ベクタまたはスタックの要素順を反転します。変化が生じない場合はエラーになります（「変化なしはエラー」原則）。

## 使用法
[ a... ] REVERSE
a... .. REVERSE

## 使用例
[ a b c ] REVERSE          # → [ c b a ]
[ 1 2 3 4 ] REVERSE        # → [ 4 3 2 1 ]

a b c .. REVERSE           # → c b a

## 注意
- 1要素のみ、空ベクタ、回文などは変化がないためエラー"#,

        "RANGE" => r#"# RANGE - 数値範囲を生成

## 機能
Form: start から end までの等差数列を生成します（end を含む）。オプションで step（増分）を指定できます。

## 使用法
[ start end ] RANGE
[ start end step ] RANGE

## 使用例
[ 0 5 ] RANGE              # → [ 0 1 2 3 4 5 ]
[ 0 10 2 ] RANGE           # → [ 0 2 4 6 8 10 ]
[ 10 0 -2 ] RANGE          # → [ 10 8 6 4 2 0 ]
[ 5 5 ] RANGE              # → [ 5 ]

## 注意
- step のデフォルトは自動判定（start <= end なら 1、そうでなければ -1）
- 整数のみサポート
- 無限シーケンスになる組み合わせ（例: `[ 0 10 -1 ]`）はエラー"#,

        "REORDER" => r#"# REORDER - インデックスリストで並べ替え

## 機能
Form: インデックスリストで指定した順序に要素を並べ替えます。部分選択、重複選択、負インデックスに対応します。

## 使用法
[ a... ] [ indices... ] REORDER
a... [ indices... ] .. REORDER

## 使用例
[ a b c ] [ 2 0 1 ] REORDER     # → [ c a b ]
[ a b c ] [ 0 0 0 ] REORDER     # → [ a a a ]（複製）
[ a b c ] [ -1 -2 -3 ] REORDER  # → [ c b a ]（逆順）
[ a b c ] [ 0 2 ] REORDER       # → [ a c ]（間引き）

a b c [ 1 2 0 ] .. REORDER      # → b c a（左ローテート）

## 注意
- インデックスリストが空、または範囲外の場合はエラー
- 結果が空になる場合は NIL を返します"#,

        "SORT" => r#"# SORT - ベクタの要素を昇順に並べ替え

## 機能
Form: ベクタまたはスタック内の数値を昇順にソートします。整数・分数・混在データを正確にソートできます。変化が生じない場合はエラーになります。

## 使用法
[ a... ] SORT
a... .. SORT

## 使用例
[ 32 8 2 18 ] SORT         # → [ 2 8 18 32 ]
[ 1/2 1/3 2/3 ] SORT       # → [ 1/3 1/2 2/3 ]
[ 3 1/2 2 1/4 ] SORT       # → [ 1/4 1/2 2 3 ]

64 25 12 22 11 .. SORT     # スタック全体を昇順に

## 注意
- 数値以外を含むとエラー
- 既にソート済み、1要素のみ、全要素が同じなどはエラー"#,

        _ => return None,
    };
    Some(result.to_string())
}
