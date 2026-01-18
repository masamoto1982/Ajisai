# Ajisaiにおける三値論理とNIL

## 概要

Ajisaiは **NIL** を使用した三値論理（Three-Valued Logic）をサポートしています。これにより、真（TRUE）、偽（FALSE）に加えて、「不明」「未定義」「値の不在」を表現できます。

## NILの二つの側面

NILは文脈によって異なる振る舞いをします：

### 1. 条件分岐での扱い（実用的側面）

**NILは `FALSE` として評価されます**

```forth
NIL IF
  "実行されない"
ELSE
  "実行される"    \ ← これが実行される
END
```

これにより、NILチェックを簡潔に記述できます：

```forth
: PROCESS-RESULT ( value -- )
  DUP NIL = IF
    DROP "結果なし"
  ELSE
    \ 通常の処理
  END
;
```

### 2. 論理演算での扱い（厳密な三値論理）

**NILは「不明（unknown）」として伝播します**

論理演算（AND, OR, NOT）では、Kleene の三値論理（K3）に従います。

## 三値論理の真理値表

### AND演算

| A     | B     | A AND B |
|-------|-------|---------|
| TRUE  | TRUE  | TRUE    |
| TRUE  | FALSE | FALSE   |
| TRUE  | NIL   | **NIL** |
| FALSE | TRUE  | FALSE   |
| FALSE | FALSE | FALSE   |
| FALSE | NIL   | FALSE   |
| NIL   | TRUE  | **NIL** |
| NIL   | FALSE | FALSE   |
| NIL   | NIL   | **NIL** |

**ポイント**：
- `FALSE AND x` は常に `FALSE`（xに関わらず）
- `TRUE AND NIL` は `NIL`（不明が伝播）

### OR演算

| A     | B     | A OR B  |
|-------|-------|---------|
| TRUE  | TRUE  | TRUE    |
| TRUE  | FALSE | TRUE    |
| TRUE  | NIL   | TRUE    |
| FALSE | TRUE  | TRUE    |
| FALSE | FALSE | FALSE   |
| FALSE | NIL   | **NIL** |
| NIL   | TRUE  | TRUE    |
| NIL   | FALSE | **NIL** |
| NIL   | NIL   | **NIL** |

**ポイント**：
- `TRUE OR x` は常に `TRUE`（xに関わらず）
- `FALSE OR NIL` は `NIL`（不明が伝播）

### NOT演算

| A     | NOT A   |
|-------|---------|
| TRUE  | FALSE   |
| FALSE | TRUE    |
| NIL   | **NIL** |

**ポイント**：
- `NOT NIL` は `NIL`（不明の否定は不明）

## 使用例

### 例1: 不明値の伝播

```forth
\ データベース検索で値が見つからない場合
: LOOKUP ( key -- value|NIL )
  \ ... 検索処理 ...
  \ 見つからない場合は NIL を返す
;

\ 複数条件の組み合わせ
'key1' LOOKUP    \ NIL (見つからない)
'key2' LOOKUP    \ TRUE (見つかった)
AND              \ NIL (片方が不明なので結果も不明)
```

### 例2: 条件分岐での判定

```forth
\ NILチェックを含む処理
: VALIDATE ( value -- result )
  DUP NIL = IF
    DROP "値が設定されていません"
  ELSE
    DUP [0] > IF
      "正の値です"
    ELSE
      "0以下の値です"
    END
  END
;
```

### 例3: 三値論理の実用例

```forth
\ 複数の条件が全て真でなければならない場合
condition1   \ TRUE
condition2   \ NIL (不明)
condition3   \ TRUE
[3] STACK AND  \ NIL (一つでも不明があれば全体が不明)

IF
  "全条件が真です"
ELSE
  "条件が偽、または不明です"  \ ← NILはfalsyなのでこちら
END
```

## NILの内部表現

**NILはセンチネル分数（0/0）を持つスカラー値として表現されます。**

```ajisai
NIL        # → センチネルNIL（data: [0/0], shape: []）
```

| 概念 | 内部表現 | 意味 |
|------|----------|------|
| **NIL** | `data: [Fraction(0/0)]`, `shape: []` | 値の不在、不明状態 |

### 重要な設計判断

- **空ブラケット `[ ]` はエラーになります**（空Vectorは作成不可）
- NILが必要な場合は明示的に `NIL` キーワードを使用してください
- NILはVector内に要素として格納可能です（例：`[ 1 NIL 3 ]`）

### NIL判定

- `is_nil()` メソッドでセンチネルNIL（0/0）を判定
- 空の `Vec<Fraction>` は NIL ではありません

### 使用例

```ajisai
# NILの明示的な使用
NIL LENGTH    # → [ 0 ]（NILの長さは0）

# FILTERで該当なしの場合
[ 1 2 3 ] '[ 10 ] >' FILTER    # → NIL（該当なし）

# Vector内のNIL
[ 1 NIL 3 ]   # → NILを要素として持つVector
```

## 設計の意図

### 実用性と厳密性のバランス

1. **条件分岐での falsy 扱い**：
   - Pythonの `None`、JavaScriptの `null` と同様
   - `if value then ... end` のような簡潔な記述が可能
   - 実用的なコードが書きやすい

2. **論理演算での不明伝播**：
   - Kleene論理に準拠
   - データベースのNULL、形式論理の三値論理と一貫
   - 厳密な論理推論が可能

### この設計の利点

- **簡潔性**: NILチェックが `IF` 一つで可能
- **安全性**: 不明値が計算を通じて明示的に伝播
- **統一性**: 統一分数アーキテクチャにより、NIL = [] = 空のVec<Fraction>

## まとめ

- **NIL** = 値の不在、不明状態（センチネル分数 `0/0` として表現）
- **条件分岐**: NILは `FALSE` として評価（実用的）
- **論理演算**: NILは「不明」として伝播（厳密）
- **空ブラケット `[ ]`**: エラー（NILではない。NILが必要な場合は `NIL` キーワードを使用）
- **Vector内のNIL**: `[ 1 NIL 3 ]` のようにVector内にNILを格納可能

この設計により、日常的なコーディングでは簡潔に、論理的な推論では厳密に扱うことができます。
