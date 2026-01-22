# Ajisai 基本操作サンプル / Basic Operations Sample

このサンプルでは、Ajisaiの基本的な操作を紹介します。

---

## Vector の作成 / Creating Vectors

### 1次元 Vector

- 1
- 2
- 3
- 4
- 5

結果: `{ 1 2 3 4 5 }`

### 2次元 Vector (テーブル形式)

| 1 | 2 | 3 |
|---|---|---|
| 4 | 5 | 6 |
| 7 | 8 | 9 |

結果: `{ ( 4 5 6 ) ( 7 8 9 ) }`

---

## 算術演算 / Arithmetic Operations

### 加算

- 10
- 20
- 30

---

```ajisai
[ 5 ] +
```

結果: `{ 15 25 35 }`

### 乗算

- 1
- 2
- 3

---

```ajisai
[ 2 ] *
```

結果: `{ 2 4 6 }`

---

## 高階関数 / Higher-Order Functions

# DOUBLE

値を2倍にする

```ajisai
[ 2 ] *
```

# SQUARE

値を自乗する

```ajisai
DUP *
```

# main

- 1
- 2
- 3
- 4
- 5

---

```ajisai
'SQUARE' MAP
```

結果: `{ 1 4 9 16 25 }`
