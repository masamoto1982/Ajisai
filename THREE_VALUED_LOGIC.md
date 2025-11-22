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

## NILと空Vector []の違い

NILと空Vector `[]` は**明確に異なる概念**です：

| 概念 | NIL | [] |
|------|-----|-----|
| **意味** | 値の不在、不明 | 要素数0の集合 |
| **型** | Nil型 | Vector型 |
| **条件分岐** | FALSE（falsy） | FALSE（空なのでfalsy） |
| **論理演算** | 不明として伝播 | 論理演算不可（型エラー） |
| **LENGTH** | エラー | `[0]` を返す |
| **CONCAT** | エラー | 単位元として機能 |
| **FILTER結果** | 該当なし（型が異なる） | 該当要素なし（正しい） |

### 使い分けの例

```forth
\ NIL: 「値が存在しない」
: FIND-USER ( id -- user|NIL )
  \ ユーザーが見つからない → NIL
;

\ []: 「要素が0個」
: FILTER-POSITIVE ( vector -- result )
  '[0 >]' FILTER  \ 正の値がない → []
;
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
- **明確性**: `[]` は空集合、`NIL` は値の不在と意味が分離

## まとめ

- **NIL** = 値の不在、不明状態を表す専用型
- **条件分岐**: NILは `FALSE` として評価（実用的）
- **論理演算**: NILは「不明」として伝播（厳密）
- **[]（空Vector）** = 要素数0の集合（NILとは別概念）

この設計により、日常的なコーディングでは簡潔に、論理的な推論では厳密に扱うことができます。
