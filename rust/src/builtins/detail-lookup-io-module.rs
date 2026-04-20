pub(crate) fn lookup_detail_io_module(name: &str) -> Option<String> {
    let result = match name {
        "PRINT" => r#"# PRINT - 要素を出力

## 機能
Map: スタックトップの要素を出力バッファに書き込みます。要素はスタックから消費されますが、`,,` モードで保持もできます。

## 使用法
value PRINT
value ,, PRINT

## 使用例
[ 42 ] PRINT              # 出力: 42
'Hello' PRINT             # 出力: Hello
[ 1 2 3 ] PRINT           # 出力: [ 1 2 3 ]

'Hello' ,, PRINT          # 出力: Hello、スタックに 'Hello' は残る

## 注意
- 出力フォーマットは要素の型によって自動的に決定されます"#,

        "NOW" => r#"# NOW - 現在時刻取得

## 機能
現在の Unix タイムスタンプ（秒、ミリ秒精度の分数）を返します。タイムゾーン情報は含みません。

## 使用法
NOW

## 使用例
NOW                       # → [ 1732531200500/1000 ]

# 処理時間の計測
NOW
... 何らかの処理 ...
NOW
-                         # 経過時間（秒）

## 注意
- Stackモード (..) はサポートされません
- 戻り値は単なる分数（タイムゾーンなし）"#,

        "DATETIME" => r#"# DATETIME - タイムスタンプを日時ベクタに変換

## 機能
Unix タイムスタンプを、指定タイムゾーンでの日時ベクタ `[ 年 月 日 時 分 秒 ]` に変換します。

## 使用法
[ timestamp ] 'timezone' DATETIME

## タイムゾーン
- 'LOCAL': ブラウザのローカルタイムゾーン
- （将来: 'UTC', 'Asia/Tokyo' など）

## 使用例
[ 1732531200 ] 'LOCAL' DATETIME
# → [ 2024 11 25 23 0 0 ]（日本時間の場合）

[ 1732531200 ] 'LOCAL' DATETIME [ 0 ] GET
# → [ ... ] [ 2024 ]（年だけ取得）

## 注意
- タイムゾーン指定は必須（省略するとエラー）
- Stackモード (..) はサポートされません
- 月は 1-12、日は 1-31、時は 0-23"#,

        "TIMESTAMP" => r#"# TIMESTAMP - 日時ベクタをタイムスタンプに変換

## 機能
指定タイムゾーンでの日時ベクタ `[ 年 月 日 時 分 秒 ]` を Unix タイムスタンプに変換します。

## 使用法
[ [ 年 月 日 時 分 秒 ] ] 'timezone' TIMESTAMP

## タイムゾーン
- 'LOCAL': ブラウザのローカルタイムゾーン
- （将来: 'UTC', 'Asia/Tokyo' など）

## 使用例
[ [ 2024 11 25 23 0 0 ] ] 'LOCAL' TIMESTAMP
# → [ 1732531200 ]（日本時間の場合）

## 注意
- タイムゾーン指定は必須（省略するとエラー）
- Stackモード (..) はサポートされません
- 実在しない日時はエラー（自動補正しない）"#,

        "CSPRNG" => r#"# CSPRNG - 暗号論的擬似乱数を生成

## 機能
指定範囲の暗号論的擬似乱数を生成します。`[ 上限 ]` で 0〜上限未満、`[ 上限 ][ 個数 ]` で個数指定も可能です。

## 使用法
[ max ] CSPRNG
[ max ] [ count ] CSPRNG

## 使用例
[ 6 ] [ 1 ] CSPRNG         # → [ 0 ] から [ 5 ] のいずれか
[ 5 ] CSPRNG               # → 5個の乱数

## 注意
- ブラウザの Web Crypto API を使用します"#,

        "HASH" => r#"# HASH - ハッシュ値を計算

## 機能
任意の値からハッシュ値（数値）を計算します。オプションでビット長を指定できます。

## 使用法
value HASH
[ bits ] value HASH

## 使用例
'hello' HASH               # → 分数のハッシュ値
[ 128 ] 'hello' HASH       # → 128-bit のハッシュ値

## 注意
- 同じ入力からは常に同じハッシュ値が得られます"#,

        "IMPORT" => r#"# IMPORT - 標準ライブラリモジュールの読み込み

## 機能
指定した標準ライブラリモジュールの全ての公開ワードを辞書に取り込みます。

## 使用法
'module' IMPORT

## 使用例
'music' IMPORT             # MUSIC@* のワードを全て取り込み
'json' IMPORT              # JSON@* のワードを全て取り込み
'io' IMPORT                # IO@* のワードを全て取り込み

## 注意
- モジュール名は小文字で指定します"#,

        "IMPORT-ONLY" => r#"# IMPORT-ONLY - 選択的に公開ワードを読み込み

## 機能
指定モジュールから、選択したワードのみを辞書に取り込みます。

## 使用法
'module' [ 'name1' 'name2' ... ] IMPORT-ONLY

## 使用例
'json' [ 'parse' ] IMPORT-ONLY          # JSON@PARSE のみ
'music' [ 'play' 'seq' ] IMPORT-ONLY    # MUSIC@PLAY と MUSIC@SEQ のみ

## 注意
- ワード名は小文字で指定します
- 取り込まれるワード名は `MODULE@NAME` 形式の大文字表記です"#,

        _ => return None,
    };
    Some(result.to_string())
}
