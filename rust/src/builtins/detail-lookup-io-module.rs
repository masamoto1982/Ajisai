




pub(crate) fn lookup_detail_io_module(name: &str) -> Option<String> {
    let result = match name {




        "PRINT" => r#"# PRINT - 要素を出力

## 機能
スタックトップの要素を出力バッファに書き込みます。
要素はスタックから除去されます。

## 使用法
要素 PRINT
→ 要素を出力し、スタックから削除

## 使用例
[ 42 ] PRINT              # 出力: 42
'Hello' PRINT             # 出力: Hello
[ 1 2 3 ] PRINT           # 出力: [1 2 3]

# 複数の値を出力
[ 1 ] [ 2 ] [ 3 ]
PRINT PRINT PRINT         # 出力: 1 2 3

## 注意
- 出力後、要素はスタックから削除されます
- 出力フォーマットは要素の型によって自動的に決定されます"#,





        "JSON@PARSE" => r#"# JSON@PARSE - JSON文字列をAjisai値に変換

## 機能
JSON文字列をパースし、Ajisaiのデータ構造に変換します。
パースエラー時はNILを返し、エラーメッセージを出力バッファに書き込みます。

## 型マッピング
- null → NIL
- true/false → TRUE/FALSE（Boolean hint）
- Number (整数) → Scalar(Fraction)
- Number (浮動小数点) → Scalar(Fraction)
- String → Vector<Scalar> + String hint
- Array → Vector（空配列はNIL）
- Object → Vector<[key, value]>（キーはString hint付きVector）

## 使用法
'JSON文字列' JSON@PARSE
→ 変換されたAjisai値

## 使用例
'42' JSON@PARSE                           # → [ 42 ]
'"hello"' JSON@PARSE                      # → 'hello'
'true' JSON@PARSE                         # → TRUE
'null' JSON@PARSE                         # → NIL
'[1, 2, 3]' JSON@PARSE                    # → [ 1 2 3 ]
'{"name": "Ajisai"}' JSON@PARSE           # → [['name' 'Ajisai']]

# パースエラー時
'invalid json' JSON@PARSE                 # → NIL（エラーメッセージが出力される）

## 注意
- ネスト深さは10次元まで（超過時はNILを返す）
- パースエラーはRustエラーとして伝播せず、NIL + エラーメッセージ
- ,, (Keep) モードをサポート"#,

        "JSON@STRINGIFY" => r#"# JSON@STRINGIFY - Ajisai値をJSON文字列に変換

## 機能
Ajisaiのデータ構造をJSON文字列に変換します。

## 型マッピング（逆変換）
- NIL → "null"
- Scalar (Boolean hint) → "true" / "false"
- Scalar (整数) → "42"
- Scalar (分数) → 浮動小数点近似値
- Vector (String hint) → "\"hello\""
- Vector (Object形式) → "{\"key\": value}"
- Vector (その他) → "[1, 2, 3]"
- CodeBlock → "null"

## 使用法
value JSON@STRINGIFY
→ JSON文字列

## 使用例
[ 42 ] JSON@STRINGIFY                     # → '42'
'hello' JSON@STRINGIFY                    # → '"hello"'
TRUE JSON@STRINGIFY                       # → 'true'
NIL JSON@STRINGIFY                        # → 'null'
[ 1 2 3 ] JSON@STRINGIFY                  # → '[1,2,3]'

# ラウンドトリップ
'{"name": "Ajisai"}' JSON@PARSE JSON@STRINGIFY # → '{"name":"Ajisai"}'

## 注意
- ,, (Keep) モードをサポート"#,

        "IO@INPUT" => r#"# IO@INPUT - 入力バッファからテキストを読み取り

## 機能
GUIの入力パネルから設定されたテキストをスタックに文字列としてプッシュします。
WASM環境ではJavaScript側から set_input_buffer() で設定された内容を読み取ります。

## 使用法
IO@INPUT
→ 入力バッファの内容（String hint付きVector）

## 使用例
# 入力バッファに '42' が設定されている場合
IO@INPUT                                # → '42'
IO@INPUT NUM                            # → [ 42 ]（数値に変換）
IO@INPUT JSON@PARSE                    # → JSONとしてパース

# 入力バッファが空の場合
IO@INPUT                                # → ''（空文字列）

## 注意
- 入力バッファは読み取り後もクリアされません
- スタックモード (..) はサポートされません"#,

        "IO@OUTPUT" => r#"# IO@OUTPUT - 出力バッファにテキストを書き込み

## 機能
スタックトップの値をテキストに変換し、GUIの出力パネル用バッファに書き込みます。
PRINTとは異なる専用バッファ（io_output_buffer）に出力されます。

## 使用法
value IO@OUTPUT
→ 値をテキスト変換して出力バッファに追加

## 使用例
'Hello' IO@OUTPUT                       # 出力バッファに 'Hello' を書き込み
[ 42 ] IO@OUTPUT                        # 出力バッファに '[ 42 ]' を書き込み

# JSONデータを処理して出力
IO@INPUT JSON@PARSE                    # 入力JSONをパース
: [ 2 ] * ; MAP                          # 各要素を2倍
JSON@STRINGIFY IO@OUTPUT               # JSON文字列として出力

## 注意
- PRINTとは別のバッファに出力されます
- WASM環境では get_io_output_buffer() で取得可能
- ,, (Keep) モードをサポート"#,

        "JSON@GET" => r#"# JSON@GET - JSONオブジェクトから値を取得

## 機能
JSONオブジェクト（[key, value] ペアのベクタ）から、指定したキーに対応する値を取得します。
キーが見つからない場合はNILを返します。

## 使用法
object 'key' JSON@GET
→ キーに対応する値、またはNIL

## 使用例
'{"name": "Ajisai", "version": 1}' JSON@PARSE
== 'name' JSON@GET                   # → 'Ajisai'

'{"x": 10, "y": 20}' JSON@PARSE
== 'x' JSON@GET                      # → [ 10 ]

# キーが存在しない場合
'{"a": 1}' JSON@PARSE 'b' JSON@GET        # → NIL

# Nil Coalescingと組み合わせ
'{"a": 1}' JSON@PARSE 'b' JSON@GET => [ 0 ]  # → [ 0 ]

## 注意
- ,, (Keep) モードをサポート
- オブジェクトでない値に対してはNILを返します"#,

        "JSON@KEYS" => r#"# JSON@KEYS - JSONオブジェクトの全キーを取得

## 機能
JSONオブジェクト（[key, value] ペアのベクタ）から、全てのキーをベクタとして取得します。
キーがない場合はNILを返します。

## 使用法
object JSON@KEYS
→ キーのベクタ、またはNIL

## 使用例
'{"name": "Ajisai", "version": 1}' JSON@PARSE
== JSON@KEYS                         # → ['name' 'version']

# キーの数を取得
'{"a": 1, "b": 2, "c": 3}' JSON@PARSE
== JSON@KEYS LENGTH                  # → [ 3 ]

## 注意
- ,, (Keep) モードをサポート
- オブジェクトでない値に対してはNILを返します"#,

        "JSON@SET" => r#"# JSON@SET - JSONオブジェクトにキー・値を設定

## 機能
JSONオブジェクト（[key, value] ペアのベクタ）に、キーと値のペアを追加または更新します。
既存のキーの場合は値を更新、新規キーの場合は追加します。

## 使用法
object 'key' value JSON@SET
→ 更新されたオブジェクト

## 使用例
# 新規キーの追加
'{"name": "Ajisai"}' JSON@PARSE
== 'version' [ 1 ] JSON@SET          # → [['name' 'Ajisai'] ['version' 1]]

# 既存キーの更新
'{"x": 1, "y": 2}' JSON@PARSE
== 'x' [ 10 ] JSON@SET               # → [['x' 10] ['y' 2]]

# 空からオブジェクトを構築
NIL 'key' 'value' JSON@SET           # → [['key' 'value']]

## 注意
- ,, (Keep) モードをサポート
- 非オブジェクト値に対しても新しい1要素オブジェクトを作成"#,

        "JSON@EXPORT" => r#"# JSON@EXPORT - JSONファイルとしてエクスポート

## 機能
スタックトップの値をJSON形式に変換し、Outputエリアにダウンロードリンクを表示します。
ダウンロードリンクをクリックすると、整形されたJSONファイルがダウンロードされます。

## 使用法
value JSON@EXPORT
→ Outputにダウンロードリンクを表示

## 使用例
# 配列のエクスポート
[ 1 2 3 ] JSON@EXPORT              # → [1, 2, 3] のダウンロードリンク

# オブジェクトのエクスポート
NIL 'name' 'Ajisai' JSON@SET
== 'version' [ 1 ] JSON@SET
== JSON@EXPORT                      # → {"name": "Ajisai", "version": 1}

# Keep モードでスタックを保持
[ 1 2 3 ] ,, JSON@EXPORT           # → [ 1 2 3 ] はスタックに残る

## 注意
- ,, (Keep) モードをサポート
- CodeBlock は null として出力されます
- ネストされたベクタはJSONの配列として出力されます
- [key, value] ペアのベクタはJSONオブジェクトとして出力されます"#,

        "MUSIC@SEQ" => r#"# MUSIC@SEQ - 順次再生モード

## 機能
続くPLAYコマンドで、Vector内の要素を順番に再生します。
これはデフォルトモードです。

## 使用法
[ 音1 音2 音3 ] MUSIC@SEQ MUSIC@PLAY
→ 音1 → 音2 → 音3 の順に再生

## 使用例
[ 440 550 660 ] MUSIC@SEQ MUSIC@PLAY           # 3音を順番に
[ 440/2 550/1 660/2 ] MUSIC@SEQ MUSIC@PLAY     # 音長指定付き

## 注意
- MUSIC@SEQはデフォルトモードです
- MUSIC@PLAYの後、モードはSEQにリセットされます"#,

        "MUSIC@SIM" => r#"# MUSIC@SIM - 同時再生モード

## 機能
続くPLAYコマンドで、Vector内の要素を同時に再生します。
和音やマルチトラック再生に使用します。

## 使用法
[ 音1 音2 音3 ] MUSIC@SIM MUSIC@PLAY
→ 音1 + 音2 + 音3 を同時に再生

## 使用例
# 和音
[ 440 550 660 ] MUSIC@SIM MUSIC@PLAY           # 3音同時

# マルチトラック
[ 440 550 ] [ 220 275 ] .. MUSIC@SIM MUSIC@PLAY
→ トラック1と2が同時進行

## 注意
- MUSIC@PLAYの後、モードはSEQにリセットされます"#,

        "MUSIC@PLAY" => r#"# MUSIC@PLAY - 音声再生

## 機能
スタック上のVectorを音声として再生します。
MUSIC@SEQ/MUSIC@SIMモードとオペレーションターゲットに従って動作します。

## 分数の解釈
- n/d = nHz を dスロット再生
- n = nHz を 1スロット再生
- 0/d = dスロット休符
- NIL = 1スロット休符
- 文字列 = Outputに出力（歌詞）

## 使用例
# 基本
[ 440 550 660 ] MUSIC@PLAY               # 順次再生
[ 440 550 660 ] MUSIC@SIM MUSIC@PLAY     # 和音

# 音長指定
[ 440/2 550/1 660/4 ] MUSIC@PLAY         # 各音の長さを指定

# 休符
[ 440 NIL 550 ] MUSIC@PLAY               # 440 → 休符 → 550
[ 440 0/4 550 ] MUSIC@PLAY               # 440 → 4スロット休符 → 550

# 歌詞
[ 440/2 'き' 550/2 'ら' ] MUSIC@PLAY     # 音と共に歌詞出力

# マルチトラック
[ 440 550 ] [ 220 275 ] .. MUSIC@SIM MUSIC@PLAY

## オペレーションターゲット
- . (デフォルト): スタックトップを再生
- ..: スタック全体を再生

## 注意
- 入力ベクタはスタックから消費されます
- 周波数は正の数値である必要があります
- 20Hz未満、20kHz超の周波数には警告が出力されます"#,

        "MUSIC@SLOT" => r#"# MUSIC@SLOT - スロットデュレーション設定

## 機能
1スロットあたりの秒数を設定します。
MUSIC@PLAYワードで再生される音の基準時間単位を変更します。

## 使用法
秒数 MUSIC@SLOT
→ 1スロット = 指定秒数

## 使用例
# 直接指定
0.5 MUSIC@SLOT               # 1スロット = 0.5秒（デフォルト）
0.25 MUSIC@SLOT              # 1スロット = 0.25秒（速い）
1 MUSIC@SLOT                 # 1スロット = 1秒（遅い）

# 分数で指定（精度保持）
1/4 MUSIC@SLOT               # 1スロット = 0.25秒
1/8 MUSIC@SLOT               # 1スロット = 0.125秒
3/4 MUSIC@SLOT               # 1スロット = 0.75秒

# BPMワードの定義例
: [ 60 ] SWAP / MUSIC@SLOT ; 'BPM' DEF
120 BPM                # 120 BPM → 0.5秒/スロット
60 BPM                 # 60 BPM → 1秒/スロット

# 音長との組み合わせ
0.125 MUSIC@SLOT             # 32分音符を基準
[ 440/4 550/2 660/1 ] MUSIC@PLAY   # 4分音符, 8分音符, 32分音符

## 注意
- グローバル設定です（全ての再生に影響）
- 正の値のみ有効（0以下はエラー）
- 極端に小さい（<0.01秒）/大きい（>10秒）値は警告"#,

        "MUSIC@GAIN" => r#"# MUSIC@GAIN - 音量設定

## 機能
再生音量を設定します。設定は次の変更まで持続します。

## 使用法
音量値 MUSIC@GAIN
→ 音量を設定（0.0〜1.0）

## 使用例
0.5 MUSIC@GAIN               # 50%音量
[ 440 550 660 ] MUSIC@PLAY   # 50%で再生

0.3 MUSIC@GAIN               # 30%音量
[ 880 ] MUSIC@PLAY           # 30%で再生

1 MUSIC@GAIN                 # 100%（デフォルト）
[ 440 ] MUSIC@PLAY           # 通常音量で再生

# 分数で指定
1/2 MUSIC@GAIN               # 50%音量
3/4 MUSIC@GAIN               # 75%音量

## 範囲
- 0.0: 無音
- 0.5: 50%音量
- 1.0: 100%（デフォルト、最大）
- 範囲外の値は自動的にクランプされます

## 注意
- グローバル設定です（全ての再生に影響）
- MUSIC@GAIN-RESET でデフォルトに戻せます"#,

        "MUSIC@GAIN-RESET" => r#"# MUSIC@GAIN-RESET - 音量リセット

## 機能
音量をデフォルト値（1.0 = 100%）に戻します。

## 使用法
MUSIC@GAIN-RESET
→ 音量を100%に設定

## 使用例
0.3 MUSIC@GAIN
[ 440 ] MUSIC@PLAY           # 30%で再生
MUSIC@GAIN-RESET
[ 440 ] MUSIC@PLAY           # 100%で再生"#,

        "MUSIC@PAN" => r#"# MUSIC@PAN - 定位（パンニング）設定

## 機能
ステレオの左右定位を設定します。設定は次の変更まで持続します。

## 使用法
定位値 MUSIC@PAN
→ 定位を設定（-1.0〜1.0）

## 使用例
-1 MUSIC@PAN                 # 完全に左
[ 440 ] MUSIC@PLAY           # 左から再生

0 MUSIC@PAN                  # 中央（デフォルト）
[ 440 ] MUSIC@PLAY           # 中央から再生

1 MUSIC@PAN                  # 完全に右
[ 440 ] MUSIC@PLAY           # 右から再生

0.5 MUSIC@PAN                # やや右寄り
[ 440 550 660 ] MUSIC@PLAY   # やや右から再生

# 分数で指定
-1/2 MUSIC@PAN               # やや左（-0.5）

## 範囲
- -1.0: 完全に左
- 0.0: 中央（デフォルト）
- 1.0: 完全に右
- 範囲外の値は自動的にクランプされます

## 注意
- グローバル設定です（全ての再生に影響）
- ヘッドフォンで効果が明確になります
- MUSIC@PAN-RESET でデフォルトに戻せます"#,

        "MUSIC@PAN-RESET" => r#"# MUSIC@PAN-RESET - 定位リセット

## 機能
定位をデフォルト値（0.0 = 中央）に戻します。

## 使用法
MUSIC@PAN-RESET
→ 定位を中央に設定

## 使用例
-1 MUSIC@PAN
[ 440 ] MUSIC@PLAY           # 左から再生
MUSIC@PAN-RESET
[ 440 ] MUSIC@PLAY           # 中央から再生"#,

        "MUSIC@FX-RESET" => r#"# MUSIC@FX-RESET - 全エフェクトリセット

## 機能
全てのオーディオエフェクト設定をデフォルトに戻します。

## 使用法
MUSIC@FX-RESET
→ MUSIC@GAIN=1.0, MUSIC@PAN=0.0 に設定

## 使用例
0.3 MUSIC@GAIN -0.7 MUSIC@PAN
[ 440 ] MUSIC@PLAY           # 30%音量、左寄りで再生
MUSIC@FX-RESET
[ 440 ] MUSIC@PLAY           # 100%音量、中央で再生

## 現在リセットされるエフェクト
- MUSIC@GAIN → 1.0（100%音量）
- MUSIC@PAN → 0.0（中央）"#,

        _ => return None,
    };
    Some(result.to_string())
}
