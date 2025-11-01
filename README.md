# Ajisai - Vectorベース言語

## 核心概念：BLOOM（開花）

Ajisaiでは、すべてのデータは**Vector**という保護膜に包まれています。
保護膜の中では何も実行されません（データ）。
保護膜から解放されると、初めてコードとして実行されます（開花）。

```forth
# Vectorの中 = データ
[ 1 2 + ]              # これは "1 2 +" という式のデータ

# BLOOMで開花 = 実行
[ 1 2 + ] BLOOM        # => 3
```

## LISPとの類似性

AjisaiはLISPの**同形性（Homoiconicity）** を持ちます。
コードとデータが同じ構造（Vector）で表現されます。

```forth
# データとして
[ 1 2 + ] LENGTH       # => 3（3要素のVector）
[ 1 2 + ] GET [ 1 ]    # => 2（2番目の要素）

# コードとして
[ 1 2 + ] BLOOM        # => 3（実行結果）
```

## ガード節

すべての構文はガード節です。
ガード節は条件と処理の組み合わせです。

```forth
# 基本形
条件1 : 処理1 :
条件2 : 処理2 :
デフォルト処理

# 例：符号判定
[ x ] 
  DUP 0 > : [ 'positive' PRINT ] :
  DUP 0 < : [ 'negative' PRINT ] :
  [ 'zero' PRINT ]

# デフォルト行のみ（通常のコード）
1 2 +                  # これもガード節
```

## REPLの挙動

REPLでは自動的に1層BLOOMします（ユーザーフレンドリー）。

```forth
# 入力
1 2 +

# 内部処理
# 1. [1 2 +] としてスタックに積む（保護）
# 2. 自動的にBLOOM
# 3. 実行される => 3

# 二重Vector
[ 1 2 + ]

# 内部処理
# 1. [[1 2 +]] としてスタックに積む
# 2. 自動的に1層BLOOM
# 3. [1 2 +] がスタックに残る（まだ保護されている）
```

## カスタムワード定義

```forth
# ワードを定義
[ 1 + ] 'INC' DEF

# 使用
[ 5 ] INC              # => 6
```

## 操作対象の指定

```forth
# STACKTOPモード（デフォルト）
STACKTOP [ 1 2 3 ] +   # Vector間の演算

# STACKモード
STACK 1 2 3 [ 3 ] +    # スタック上の3要素を畳み込み
```

## メタプログラミング

```forth
# コードをデータとして操作
[ 1 2 + ]              # コード
DUP                    # 複製
LENGTH                 # => 3（データとして扱う）
BLOOM                  # => 3（コードとして実行）

# コードを変換
[ 1 2 + ]
[ 10 ] CONCAT          # => [10 1 2 +]
BLOOM                  # => 13
```

## ファイル構造

```
rust/src/
├── lib.rs                    # ライブラリルート
├── main.rs                   # テスト用エントリーポイント（オプション）
├── types.rs                  # 基本型定義
├── types/
│   └── fraction.rs          # 分数型
├── tokenizer.rs             # トークナイザー
├── builtins.rs              # 組み込みワード定義
├── wasm_api.rs              # Wasm API
└── interpreter/
    ├── mod.rs               # インタープリターモジュール
    ├── error.rs             # エラー型
    ├── bloom.rs             # BLOOM実装
    ├── arithmetic.rs        # 算術演算
    ├── comparison.rs        # 比較・論理演算
    ├── vector_ops.rs        # Vector操作
    ├── higher_order.rs      # 高階関数
    ├── io.rs                # 入出力
    ├── dictionary.rs        # ワード管理
    ├── control.rs           # 制御構造
    └── audio.rs             # 音声生成
```

## ビルドとテスト

```bash
# Wasmビルド
wasm-pack build --target web

# ローカルテスト
cargo run

# テスト実行
cargo test
```

## 設計思想

1. **Vector = 保護膜**: すべてのデータはVectorで保護される
2. **BLOOM = 開花**: 保護膜から解放されて初めて実行される
3. **同形性**: コードとデータが同じ構造を持つ
4. **ガード節**: すべての構文は条件分岐として統一される
5. **操作対象の明示**: ユーザーが操作の適用先を選択できる

## 今後の拡張

- [ ] マクロシステム
- [ ] モジュールシステム
- [ ] 並行処理
- [ ] 型システム（オプション）
- [ ] デバッガー
