pub(crate) fn lookup_detail_string_cast(name: &str) -> Option<String> {
    let result = match name {
        "CHARS" => r#"# CHARS - 文字列を文字ベクタに分解

## 機能
Map: 文字列を1文字ごとの文字列ベクタに分解します。UTF-8 マルチバイト文字（日本語など）も正しく処理されます。

## 使用法
[ 'string' ] CHARS
str1 str2 ... .. CHARS

## 使用例
[ 'hello' ] CHARS          # → [ 'h' 'e' 'l' 'l' 'o' ]
[ '日本語' ] CHARS         # → [ '日' '本' '語' ]

[ 'hello' ] CHARS REVERSE JOIN   # → [ 'olleh' ]（文字列反転）
[ 'hello' ] CHARS [ 3 ] TAKE JOIN # → [ 'hel' ]（部分文字列）

## 注意
- String 型のみ受け付けます
- 空文字列はエラー
- JOIN ワードで元の文字列に戻せます"#,

        "JOIN" => r#"# JOIN - 文字列ベクタを連結

## 機能
Map: 文字列のベクタを連結して単一の文字列にします。CHARS の逆操作です。

## 使用法
[ str1 str2 ... ] JOIN
vec1 vec2 ... .. JOIN

## 使用例
[ 'h' 'e' 'l' 'l' 'o' ] JOIN     # → [ 'hello' ]
[ 'hel' 'lo' ] JOIN              # → [ 'hello' ]
[ '日' '本' '語' ] JOIN          # → [ '日本語' ]

## 注意
- 全要素が String 型である必要があります
- 空ベクタや数値・他の型を含むとエラー"#,

        "NUM" => r#"# NUM - 文字列を数値にパース

## 機能
Map: 文字列を解析して数値（分数）を生成します。パースに失敗した場合は NIL を返します（エラーにはなりません）。

## 使用法
'文字列' NUM

## 使用例
'123' NUM              # → [ 123 ]
'1/3' NUM              # → [ 1/3 ]
'-42' NUM              # → [ -42 ]
'3.14' NUM             # → [ 157/50 ]（小数も分数として解釈）
'ABC' NUM              # → NIL（パース失敗）
'' NUM                 # → NIL（空文字列）

## 注意
- 入力が String でない場合はエラー
- 分数表記と小数表記の両方に対応"#,

        "STR" => r#"# STR - 値を文字列に変換

## 機能
Map: 任意の値を人間が読める文字列に変換します。既に文字列の場合はエラーになります（「変化なしはエラー」原則）。

## 使用法
value STR

## 使用例
123 STR                # → '123'
1/3 STR                # → '1/3'
TRUE STR               # → 'TRUE'
NIL STR                # → 'NIL'
[ 1 2 3 ] STR          # → '1 2 3'

123 STR NUM            # → [ 123 ]（往復変換）

## 注意
- 入力が既に String の場合はエラー
- ベクタは要素を空白区切りで連結
- 分数は約分された形で出力"#,

        "BOOL" => r#"# BOOL - 真偽値に正規化

## 機能
Map: 文字列または数値を真偽値に正規化します。文字列 `'true'/'false'` をパース、数値は 0 を FALSE、その他を TRUE として扱います。

## 使用法
value BOOL

## 使用例
# 文字列からのパース（大文字小文字無視）
'true' BOOL            # → TRUE
'False' BOOL           # → FALSE
'other' BOOL           # → NIL（パース失敗）

# 数値からの正規化（Truthiness）
100 BOOL               # → TRUE
0 BOOL                 # → FALSE
1/2 BOOL               # → TRUE

## 注意
- 既に Boolean の場合はエラー（「変化なしはエラー」原則）
- NIL に対してはエラー
- 文字列は `'true'/'false'` のみパース（それ以外は NIL）"#,

        "CHR" => r#"# CHR - 数値を Unicode 文字に変換

## 機能
Map: 数値（Unicode コードポイント）を1文字の文字列に変換します。

## 使用法
value CHR

## 使用例
65 CHR                 # → 'A'
97 CHR                 # → 'a'
48 CHR                 # → '0'
10 CHR                 # → '\n'（改行）
12354 CHR              # → 'あ'
20320 CHR              # → '你'

## 注意
- 有効な範囲は 0〜0x10FFFF
- サロゲートペア範囲（0xD800〜0xDFFF）は無効
- 整数のみ受け付け（分数はエラー）"#,

        _ => return None,
    };
    Some(result.to_string())
}
