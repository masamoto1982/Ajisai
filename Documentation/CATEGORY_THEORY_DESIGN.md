# Ajisai 圏論設計ドキュメント

## 基本概念

### Tensor圏 (Monoidal Category) としてのAjisai

Ajisaiは以下の圏構造を基盤とする：

- **対象 (Objects)**: 自然数 n ∈ ℕ（ベクトル空間の次元 ℝⁿ を表す）
- **射 (Morphisms)**: テンソル（行列）hom(m, n) = n×m行列
- **合成 (Composition)**: 行列積 (matmul)
- **恒等射 (Identity)**: 単位行列 (eye)
- **テンソル積 (⊗)**: クロネッカー積 (kron)
- **単位対象 (I)**: 1（スカラー、ℝ¹）

### 弦図式 (String Diagrams) = Tensor Network

圏論の弦図式とTensor Networkは数学的に同一。
Ajisaiではテンソル縮約を弦の接続として表現。

## 記法

### 射の表現
```ajisai
# n×m 行列は m → n の射
[ [ 1 2 ] [ 3 4 ] [ 5 6 ] ]  # 3×2行列 = R² → R³ の射
```

### 合成
```ajisai
# g ∘ f （数学的順序）
f g COMPOSE

# f ; g （図式的順序、Forth風）
f g THEN
```

### テンソル積
```ajisai
# f ⊗ g
f g TENSOR

# または
f g KRON
```

## 圏論的法則

### 恒等律
```
id_n ∘ f = f = f ∘ id_m
```

Ajisaiでの検証:
```ajisai
# f: R² → R³
[ [ 1 2 ] [ 3 4 ] [ 5 6 ] ]  # f

# id_2 ∘ f = f ?
DUP [ 2 ] ID COMPOSE =
# → [ TRUE ]

# f ∘ id_3 = f ?
DUP [ 3 ] ID SWAP COMPOSE =
# → [ TRUE ]
```

### 結合律
```
(h ∘ g) ∘ f = h ∘ (g ∘ f)
```

Ajisaiでの検証:
```ajisai
# f, g, h を定義
[ [ 1 2 ] [ 3 4 ] ] 'F' DEF     # f: R² → R²
[ [ 5 6 ] [ 7 8 ] ] 'G' DEF     # g: R² → R²
[ [ 9 10 ] [ 11 12 ] ] 'H' DEF  # h: R² → R²

# (h ∘ g) ∘ f
G H COMPOSE F COMPOSE

# h ∘ (g ∘ f)
F G COMPOSE H COMPOSE

# 等しいか?
=
# → [ TRUE ]
```

### モノイダル構造の法則

#### 関手性
```
(f ⊗ g) ∘ (f' ⊗ g') = (f ∘ f') ⊗ (g ∘ g')
```

#### 単位対象の性質
```
I ⊗ f = f = f ⊗ I
```

## 実装マッピング

### Vect圏からAjisaiへ

| 圏論 | Ajisai | 説明 |
|------|--------|------|
| 対象 n | 自然数スカラー `[ n ]` | ℝⁿ の次元 |
| 射 f: m → n | n×m 行列 Tensor | 線形写像 |
| g ∘ f | `f g COMPOSE` | 行列積 g @ f |
| id_n | `[ n ] ID` | n×n 単位行列 |
| f ⊗ g | `f g KRON` | クロネッカー積 |
| dom(f) | `f DOM` | 定義域の次元 |
| cod(f) | `f COD` | 値域の次元 |

### 弦図式からテンソルネットワークへ

弦図式表記:
```
  ┌───┐
──┤ f ├──
  └───┘
```

Ajisai表記:
```ajisai
[ [ ... ] ] 'F' DEF  # 射 f の定義
```

並列合成 (f ⊗ g):
```
  ┌───┐
──┤ f ├──
  └───┘
  ┌───┐
──┤ g ├──
  └───┘
```

Ajisai:
```ajisai
F G KRON
```

直列合成 (g ∘ f):
```
  ┌───┐   ┌───┐
──┤ f ├───┤ g ├──
  └───┘   └───┘
```

Ajisai:
```ajisai
F G COMPOSE
```

## 応用ドメイン

### 1. 量子計算

量子計算は**有限次元ヒルベルト空間の compact closed category**として表現できる。

- 対象: 量子ビット数 n（2ⁿ次元複素ベクトル空間）
- 射: ユニタリ行列（2ⁿ × 2ⁿ）
- テンソル積: 複合系

例: ベル状態の生成
```ajisai
# |00⟩ 初期状態
[ 2 ] QUBIT

# Hadamard ⊗ I
H [ 1 ] ID KRON

# CNOT
CNOT COMPOSE

# → (|00⟩ + |11⟩)/√2 (ベル状態)
```

### 2. 確率モデリング (Markov圏)

確率過程を圏論的に扱う。

- 対象: 有限集合のサイズ
- 射: 確率行列（各列の和が1）
- 合成: 行列積
- テンソル積: 独立な確率分布の積

例: マルコフ連鎖
```ajisai
# 状態遷移行列 (3状態)
[ [ 0.7 0.2 0.1 ]
  [ 0.2 0.6 0.2 ]
  [ 0.1 0.2 0.7 ] ] 'P' DEF

# 初期分布
[ 1 0 0 ]

# n ステップ後の分布
P P COMPOSE P COMPOSE  # P³
SWAP MATMUL
```

### 3. 機械学習

ニューラルネットワークの層を射として扱う。

- 対象: 特徴空間の次元
- 射: 重み行列
- 合成: 層の積み重ね

例: 2層ニューラルネットワーク
```ajisai
# Layer 1: R⁴ → R³
[ [ w11 w12 w13 w14 ]
  [ w21 w22 w23 w24 ]
  [ w31 w32 w33 w34 ] ] 'L1' DEF

# Layer 2: R³ → R²
[ [ w11 w12 w13 ]
  [ w21 w22 w23 ] ] 'L2' DEF

# 合成ネットワーク: R⁴ → R²
L1 L2 COMPOSE 'NET' DEF

# 入力
[ 1 2 3 4 ]

# 推論
NET SWAP MATMUL
```

### 4. 微分可能プログラミング (微分圏)

各射が順方向計算と逆方向勾配を持つ。

- 対象: ベクトル空間
- 射: 微分可能写像
- 追加構造: ヤコビアン

```ajisai
# 関数 f: R² → R²
[ [ ... ] ] 'F' DEF

# ヤコビアン計算
F JACOBIAN

# 勾配降下
# ∇f を計算し、パラメータ更新
```

## 型安全性と次元チェック

Ajisaiは実行時に射の合成が型安全であることを検証する。

```ajisai
# f: R² → R³ (3×2行列)
[ [ 1 2 ] [ 3 4 ] [ 5 6 ] ]

# g: R⁴ → R² (2×4行列)
[ [ 1 2 3 4 ] [ 5 6 7 8 ] ]

# 合成を試みる
COMPOSE
# → Error: COMPOSE: codomain of f (3) != domain of g (4)
```

正しい合成:
```ajisai
# f: R² → R³ (3×2行列)
[ [ 1 2 ] [ 3 4 ] [ 5 6 ] ]

# g: R³ → R² (2×3行列)
[ [ 1 2 3 ] [ 4 5 6 ] ]

# 合成成功: g ∘ f: R² → R²
COMPOSE
# → [ [ 9 12 ] [ 24 33 ] ]
```

## 設計原則

1. **明示的な型チェック**: すべての射の合成で次元を検証
2. **不変条件の保持**: 圏論の法則（恒等律、結合律）が常に成立
3. **実用性優先**: 理論的純粋性より、機械学習・量子計算への応用を重視
4. **段階的型付け**: 実行時チェックから静的型チェックへの移行を視野に
5. **可読性**: 数学的記法とForth風記法のバランス

## 拡張可能性

### Phase 1（基本）
- ✓ 圏論基本ワード (COMPOSE, ID, KRON, etc.)
- ✓ 次元チェック

### Phase 2（関手とモナド）
- □ FMAP（関手的写像）
- □ RETURN, BIND（モナド）
- □ 自然変換

### Phase 3（弦図式）
- □ ASCII 弦図式パーサー
- □ テンソルネットワーク評価

### Phase 4（応用）
- □ 量子ゲート（H, CNOT, etc.）
- □ 確率行列操作
- □ 自動微分（GRAD, JACOBIAN）

### Phase 5（最適化）
- □ テンソル縮約の最適化
- □ 遅延評価
- □ 型推論システム

## 参考文献

1. **Seven Sketches in Compositionality** (Fong & Spivak, 2018)
   - 応用圏論の基礎

2. **Categories for the Working Mathematician** (Mac Lane, 1971)
   - 圏論の古典的教科書

3. **Tensor Network Contractions** (arXiv:1708.00006)
   - テンソルネットワークの計算理論

4. **Categorical Quantum Mechanics** (Abramsky & Coecke, 2004)
   - 量子計算の圏論的基礎

5. **Backprop as Functor** (Fong et al., 2019)
   - 逆伝播の圏論的理解

---

**作成日**: 2025-12-07
**バージョン**: 1.0
**作成者**: Claude (Anthropic)
