# Ajisai の数学的定式化 — denotational / algebraic semantics

> Status: **Non-canonical / descriptive.** 正典は `SPECIFICATION.html` のみ。
> 本書は仕様に対する**第二の権威ではなく**、仕様が定める現象を数学の言葉で
> 記述するモデルである(§16.1「第二の設計権威を導入しない」を尊重する)。
> Active coverage tracking lives in `docs/formalization-coverage.json`; current language observations live in `tests/conformance/index.html`.
> 仕様と食い違う場合は仕様が優先する。本書の目的は、Ajisai を特定の実装言語に
> 依存させず、**数式そのもの**として定義しうることを示し、そこから何が得られるかを
> 明らかにすることにある。

## 0. 動機 — 外延的同一性から内包的同一性へ

`PORTABILITY.md` と `SPECIFICATION.html`「Conformance and Identity」は、
Ajisai の同一性を **conformance suite が固定する入出力対応**で定義する。
これは数学的には、意味関数

```
⟦·⟧ : Program → (Σ ⇀ Σ)
```

の **グラフ上の有限個の標本点** にすぎない(現在 53 点)。標本は外延的で
あり、標本に載らない入力では実装の自由が残る(所見 A、前回レビュー参照)。

「いかなるプログラミング言語にも依存しない」の究極形は、参照実装でも有限の
テスト集合でもなく、**意味関数 ⟦·⟧ 自身を数式で与えること**である。これは
*内包的*・*完全*な定義であり、conformance suite を特殊例として包摂する。
本書はその ⟦·⟧ を構成する。

---

## 1. 意味領域 (semantic domains)

### 1.1 値空間 V

値空間を直和(coproduct)として定める:

```
V  =  𝔸  ⊎  K3  ⊎  V*  ⊎  (Name ⇀ V)  ⊎  ⊥_R  ⊎  Blk  ⊎  H
```

| 構成子 | 数学的対象 | 仕様 |
|---|---|---|
| `𝔸` | 連分数で表現可能な厳密実数(後述 §3) | §4.2 Scalar |
| `K3` | Kleene 3 値真偽領域 `{T,F,U}`(後述 §4) | §7.5 |
| `V*` | V の有限列(ベクトル/テンソル) | §4.3 |
| `Name ⇀ V` | 有限部分写像、挿入順を保持(Record) | §4.4 |
| `⊥_R` | 理由 r∈R∞ を帯びる吸収元(NIL/Bubble) | §4.5 |
| `Blk` | トークン列(コードブロック、第一級) | §4.6 |
| `H` | 子ランタイム/スーパバイザのハンドル | §4.7 |

**重要**: `⊎` は **直和**であり、各単射像は互いに素でなければならない。とくに
`K3` の真偽値 `{T,F,U}`(§4)は `𝔸` とは別の単射で V に入り、
Boolean/TruthValue と Number/ExactReal は観測上も混同されない。

### 1.2 配置(状態)Σ

観測可能な機械配置を

```
Σ  =  Stack × Dict × Eff
   =  V*  ×  (Name ⇀ Blk)  ×  E*
```

とする。`Stack` は値の列(右端が頂上)、`Dict` はユーザ辞書、`Eff` は
**構造化ホスト効果**の順序列(§5.2, conformance の観測対象)。
予算・ステップ計数・意味プレーンのロール等は観測同値(§9)に影響する範囲でのみ
別途扱い、ここでは省略する。

### 1.3 誤差層

誤差(§11.1)は配置の外にある吸収状態 `Error` であり、捕捉子を持たず評価を
停止させる(§11.4)。したがって意味関数の余域は `Σ + Error` である。
**NIL(⊥_R)と Error は層が異なる**:前者は V の内部にある捕捉可能な値、後者は
評価そのものを止める捕捉不能な例外。この二層構造が「できなかった→泡 /
使い方が違う→エラー」(§11.2)の数学的内容である。

---

## 2. 中心定理 — プログラムは状態変換子のモノイド準同型

### 2.1 フレーズと生成元

ソースを字句解析・構文解析すると、トップレベルは **フレーズ**の列になる。
フレーズとは「1 個の被評価単位」であり、次のいずれか:

- 押下リテラル(数、文字列、ベクトル `[…]`、ブロック `{…}`)
- 修飾子接頭辞付きの語適用 `μ·w`(μ は修飾子の組、§7)
- 中置糖衣 `^`(VENT)等は、後続フレーズを第二被演算子として束縛する
  二項適用へ脱糖される(§6.3)

各フレーズ φ は配置変換子 `⟦φ⟧ : Σ ⇀ (Σ + Error)` を表す。

### 2.2 準同型

プログラム p = φ₁ φ₂ … φₙ に対し、その意味は **Kleisli 合成**(誤差を伝播させる
合成 •)で与えられる:

```
⟦φ₁ φ₂ … φₙ⟧  =  ⟦φₙ⟧ • … • ⟦φ₂⟧ • ⟦φ₁⟧
⟦ε⟧           =  id_Σ
```

すなわち写像

```
⟦·⟧ : (Phrase*, 連結, ε)  ⟶  (StateTransformer, •, id)
```

は **モノイド準同型**である。これが「Ajisai を数式として表す」ことの核心:
**プログラムとはモノイドの積**であり、言語の定義は

1. 生成元(各フレーズ ⟦φ⟧)の定義、と
2. モノイド演算(誤差伝播合成 •)

の二つに完全に分解される。`IDLE`(§7.7 no-op)は単位元 id_Σ であり、
`p IDLE ≡ p`(実測 §10 で HOLDS)。

> 連接言語(concatenative language)が合成のモノイドであることは一般に知られる。
> Ajisai 固有の注意点は、修飾子と中置糖衣のために**生成元は「裸の語」ではなく
> フレーズ**であること。脱糖を構文段で済ませれば準同型は厳密に成り立つ。

---

## 3. 数の層 — 連分数と (双)一次変換

### 3.1 数は行列の積である

正則連分数 `[a₀; a₁, a₂, …]` は GL₂(ℤ) の行列積として表せる:

```
[a₀; a₁, …, aₙ]  =  ( ∏_{i=0}^{n} [ aᵢ 1 ; 1 0 ] ) ▷ ∞
```

ここで `▷` は ℝ∪{∞} 上の一次分数変換 `[a b; c d] ▷ x = (a x + b)/(c x + d)`。
収束分子・分母は積行列の列に現れる。無理数は無限積(遅延ストリーム)。
すなわち **Ajisai の数とは、整数行列の(有限または遅延無限の)積**である。
これは特定言語の浮動小数点や有理数型に依存しない、純粋に代数的な対象。

### 3.2 算術は 8 個の整数

二項算術は **双一次変換**(bihomographic)

```
z(x,y) = (a·xy + b·x + c·y + d) / (e·xy + f·x + g·y + h)
```

の正規化(=結果の連分数桁を確定次第放出する Gosper 法、§7.3)である。
四則は係数 `(a b c d ; e f g h)` の定数だけで尽きる:

| 語 | (a b c d ; e f g h) | 変換 |
|---|---|---|
| `ADD` | (0 1 1 0 ; 0 0 0 1) | x+y |
| `SUB` | (0 1 −1 0 ; 0 0 0 1) | x−y |
| `MUL` | (1 0 0 0 ; 0 0 0 1) | x·y |
| `DIV` | (0 1 0 0 ; 0 0 1 0) | x/y |

単項(`FLOOR`/`CEIL`/`ROUND` や合成途中)は一次変換 `[a b; c d] ∈ GL₂(ℤ)`。
**算術全体が「8 整数のテンソルを CF ストリームへ作用させ、正規化する」一つの
スキーマ**に還元される。係数は常に BigInt(§4.2.2, §16.11)。

### 3.3 比較は予算付き観測

順序は 𝔸 ⊂ ℝ の全順序。実装可能な順序は、両オペランドの(最近接整数)CF 桁を
予算 β まで並走させ最初の相違で符号を決める **予算付き近似**:

```
cmp_β : 𝔸 × 𝔸 → {<, =, >, U}
```

β→∞ で相異なる実数は必ず決し、等しい無理数は決して相違しない。
この **U の出現は設計の気まぐれではなく計算理論の帰結**である:実数の同値は
半決定的(Π⁰₁)で決定不能だから、順序を*全域*にするには第三の値 U を付け加える
ほかない。`agreedPrefix` は「一致した桁数」= 近似精度の下界(§7.4.1)。
`COMPARE-WITHIN` は β を第一級にする唯一の語(§7.4.2)。

---

## 4. 真偽値 — Kleene 3 値代数 K3

真偽領域を鎖 `F < U < T` 上の束とし、

```
a ∧ b = min(a,b),   a ∨ b = max(a,b),   ¬ = 鎖を反転する対合(T↔F, U固定)
```

と定める。これは §7.5 の strong Kleene 真理表と一致する(min(T,U)=U,
min(F,U)=F, max(T,U)=T, max(F,U)=U, ¬U=U)。`(K3, ∧, ∨, ¬, F, T)` は
**De Morgan 束(Kleene 代数)**であり、De Morgan 則・二重否定・分配・吸収を
満たす(実測 §10 で HOLDS)。吸収元規則(F が ∧ を、T が ∨ を支配)は
min/max から自動的に従う。

NIL との相互作用(§4.5.2, §7.5)は、K3 を ⊥_R で拡張した代数上で「吸収元が
先に決する場合を除き ⊥ が U に優先する」規則として書ける。

---

## 5. 部分性 — Bubble モナドと orelse

### 5.1 NIL モナド

理由集合を R∞ = R ∪ {none} とし、左偏(first-write-wins)モノイドを入れる。
関手

```
M(X) = X  +  (⊥ × R∞)
```

は **例外/Maybe モナド**で、単位 η(x)=x、伝播は「いずれかの引数が ⊥ なら
結果は ⊥、理由は最左の理由を持つ ⊥」(§4.5.1)。NIL-passthrough 語(§7.12)は
n 項関数 f を Kleene 拡張 f* へ持ち上げたものに等しい。

### 5.2 VENT は handler

`A ^ B`(中置糖衣)は脱糖して二項 `vent(A, B)`:

```
vent(⊥_r, b) = b          vent(a, b) = a   (a が純粋値)
```

これはモナドの除去子(catch)であり、Bubble 層の唯一の handler。
誤差層(§1.3)には handler が無く、伝播して停止する(§11.4)。

> 注意:`^` は**後続フレーズ B を第二被演算子として束縛する中置糖衣**であって、
> 単純な後置スタック語ではない(実測:`5 ^ 99 → 5`、`1 0 / ^ 99 → 99`)。
> この被演算子順序の曖昧さこそ、数式化が除去すべき対象であり、形式化の効用の
> 具体例である。

---

## 6. 修飾子 — 変換子上のコンビネータ

修飾子(§6)は語の基底変換子に作用する高階作用素である:

```
TOP, STAK : Region 選択   (頂上 n 個 / スタック全体を被演算域に)
EAT, KEEP : Consumption   (消費 / 複製して結果も積む)
```

修飾語フレーズは `⟦μ·w⟧ = κ_consume( δ_region( base(w) ) )`。
`KEEP` は分岐(bifurcation, §13.2):被演算子を残しつつ結果を積む
(実測 `3 4 KEEP ADD → 3 4 7`)。`TOP/STAK × EAT/KEEP` の 4 組は
変換子→変換子の関手として閉じる。修飾子は新しい語ではなく**合成のコンビネータ**
であるため、§2 の準同型を保つ(生成元をフレーズに取る理由)。

---

## 7. ベクトル/テンソル — 添字函手とブロードキャスト

形 s=(d₁,…,dₖ) のテンソルは添字集合 ∏ᵢ[dᵢ] → V の写像(行優先で V* と同型)。
要素ごと算術は、スカラ演算をこの添字函手上へ持ち上げた **applicative**(zip)であり、
長さ 1 の次元は任意の幅へ写る(broadcast、実測 `[1 2 3] [10] * → [10 20 30]`)。
段階パイプライン「平坦化→形/ストライド→添字変換→再構築」(§7.2)は、
同型 `Tensor ≅ (data: V*, shape)` 上で reshape 群が添字に作用する図式に等しい。
密/入れ子の二表現(§4.3.1)は同一の観測意味を持つ(同型)。

---

## 8. 観測関数と同一性

観測可能量を

```
observe(p)  =  ( render( π_Stack( ⟦p⟧(σ₀) ) ),  π_Eff( ⟦p⟧(σ₀) ) )
```

と定める。`render` は **(data, role) の純関数**(§12.1)。二実装が Ajisai として
同一であるとは、すべての p で observe が一致すること。conformance suite は
この等式の有限標本にすぎない。**完全な同一性は ⟦·⟧ の等しさ**であり、本書の
⟦·⟧ がその基準を与える。

---

## 9. 得られるもの

### 9.1 完全・内包的定義(所見 A への根本解)

53 個の標本でなく、関数 ⟦·⟧ を与えることで「suite に載らない入力」の自由が
消える。conformance は ⟦·⟧ の検証用標本に格下げされ、未被覆領域(修飾子・
テンソル・子ランタイム等)も定義済みになる。

### 9.2 代数法則 ⇒ 性質ベース conformance(最大の実益)

準同型と生成元の定義から、**全入力で成り立つべき等式**が導かれる。これは
有限個の方程式に**無限個のテストケース**を圧縮したもので、53 個の列挙より
桁違いに強い移植性契約である。例(実測検証は §10):

- 合成/単位: `p IDLE ≡ p`、`(p q) r ≡ p (q r)`
- 体の法則(𝔸 上): ADD 可換・結合、MUL 分配、`x 0 ADD ≡ x`、`x 1 MUL ≡ x`、
  `x x SUB ≡ 0`、そして **`x a ADD a SUB ≡ x`**
- Kleene 則: De Morgan、二重否定 `a NOT NOT ≡ a`、吸収・冪等
- NIL モナド則: `⊥_r ^ v ≡ v`、passthrough 自然性 `f(⊥_r) ≡ ⊥_r`
- 比較予算: `agreedPrefix` は β に対し単調、有理数の確定順序は β 非依存

### 9.3 数式は乖離をその場で暴く(オラクルとしての形式化)

実装を ⟦·⟧ と突き合わせると、**「実装が Ajisai でない」点が等式の破れとして
即座に可視化**される。本作業で検出した二件(いずれも前回レビューの所見を
法則レベルで裏づける)は、**いずれも本ブランチで解消済み**である:

- **所見 B(真偽値=数値)— 解消済み:** §1.1 は V を直和とし {T,F,U} と 𝔸 を
  素にする。旧実装は `TRUE 1 EQ → TRUE` で **T = 1 ∈ 𝔸** と単射が衝突していた。
  真偽値を**データ面で独立した値種 `Boolean`** として追加し(仕様 §4.1 改訂)、
  `TRUE 1 EQ → FALSE` となった。比較・論理・リテラルの全経路が一貫して
  `TRUE`/`FALSE`/`UNKNOWN` を観測表層に出す(B1)。
- **所見 C(無理数の表示)— 解消済み:** 算術自体は §3.2 の双一次変換(Gosper)で
  **元から厳密**(等しい無理数の比較は予算内で決せず `UNKNOWN`=§7.4.1準拠)で
  あり、`~` 近似は**表示層のみ**の現象だった。表示を §4.2.3 の入れ子連分数形
  へ統一し、`√2 → ( 1 ( 2 ( 2 ...) ) )`、`√2+1 → ( 2 ( 2 ...) )` を出すように
  した。法則 `x a ADD a SUB ≡ x` は値として元から成立しており(比較は U を返す)、
  表示の乖離が除去された。

### 9.4 検証・自己ホストへの道

⟦·⟧ が数式であれば、証明支援系へ機械化し、参照実装がそれを refine することを
**証明**できる(53 ケースのテストではなく)。短期的には §9.2 の法則を性質
ベーステストとして実行し、実装の適合を連続的に監視できる。

---

## 9-bis. 拡張定式化 I — 構文・高階語・構造データ(2026-06 改修)

本節は改修ロードマップ
(`docs/dev/ajisai-formalization-expansion-roadmap.md`)Phase 2/4/5 の成果を
形式化本体へ取り込む。いずれも §2 の準同型・§5 のモナド・§7 の添字函手の上に
立つ。各小節の法則は実行可能な性質ベーステストとして常時検証される
(末尾の「テスト」欄)。

### A. 構文層 — 脱糖は観測透明(Phase 2)

字句・構文・脱糖を全域関数の合成として与える:

```
tokenize : Source ⇀ Token*      parse : Token* ⇀ Phrase*      desugar : Phrase* → Phrase*
```

`desugar` は §3.9 / §7.0 の表に従い、記号糖衣を英語正準語へ書き換える。本書 §2 の
準同型はこの脱糖後のフレーズ列に対して厳密に閉じる。脱糖の**健全性**は

```
⟦desugar(s)⟧ = ⟦s⟧
```

であり、観測レベルでは「糖衣形と正準形が同一に render される」こととして現れる。
語名は大文字へ正規化される(§3.8)ので `add ≡ Add ≡ ADD`。`FLOW`(`~`)は
単位変換子 id(視覚的分離子、§6.4)、`TOP-EAT`(`;`)は `. ,` の合成、すなわち
脱糖は**新しい意味を加えない構文層の恒等**である。`(`/`)` は字句段で拒否され、
`>CF` 等の conversion word は単一トークン化される(§3.9)。

**テスト**: `rust/tests/desugar_laws.rs` — 四則・比較の記号≡英語、`~` no-op、
大小文字正規化、`;`≡`. ,`、`&`≡`AND` を {T,F,U} 上で(計 6 法則群)。

### B. 高階・制御語 — 再帰スキーム(Phase 4)

§7.7 の制御語を、そのモデルの代数法則で特徴づける。被演算ブロックは第一級
`Blk`(§4.6)であり、`EXEC` が脱参照適用、`EVAL` が文字列ソースに対する `⟦·⟧` の
reflection。

| 語 | モデル | 中心法則 |
|---|---|---|
| `MAP` | 添字函手上の関手持ち上げ | 恒等 `MAP id = id`、融合 `MAP g ∘ MAP f = MAP (g∘f)` |
| `FOLD` | catamorphism(モノイド畳み込み) | `[a b c] 0 {ADD} FOLD = a+b+c`、`1 {MUL} FOLD = a·b·c` |
| `SCAN` | catamorphism の中間累算列 | prefix-sums: `[1 2 3 4] 0 {ADD} SCAN = [1 3 6 10]` |
| `FILTER` | 述語による部分列制限 | 冪等 `FILTER p ∘ FILTER p = FILTER p`、可換 `FILTER p ∘ FILTER q = FILTER q ∘ FILTER p` |
| `ANY`/`ALL` | 存在/全称量化 | De Morgan 双対 `ALL p = ¬ ANY ¬p`(K3、§4) |
| `EXEC` | ブロック脱参照 | `a b {ADD} EXEC = a b ADD` |
| `EVAL` | `⟦·⟧` の reflection | `⟦EVAL(STR p)⟧ = ⟦p⟧` |
| `COND` | K3 ガード付き case | **U ガードは不発火**:U は `false` と同様に次節へ落ち、definite `true` のみ発火(§7.4.3) |

`COND` の U-不発火は K3 への忠実性そのもの:ガードが「真と確立できない」(U)とき
その節は選ばれず、しかし値はスタック上で `truthValue=unknown` として観測可能。これは
比較の半決定性(§3.3)が制御フローへ波及する地点であり、`MIN`/`MAX`/`SORT` の
U 伝播(§7.4.3)と同じ規律に属する。

**テスト**: `rust/tests/higher_order_laws.rs` — 上記 11 法則群(map 恒等・融合、
fold 加法/乗法、scan、filter 冪等/可換、any/all 双対、exec/eval reflection、
cond U-不発火)。

### C. 構造データ — 自由モノイドと reshape 群(Phase 5)

ベクトル語(§7.1)を `V*` 上の**自由モノイド**として:`CONCAT` が結合的な積、
空でない列が要素(空列は NIL、§4.5)、`REVERSE` が反同型の対合

```
REVERSE ∘ REVERSE = id        REVERSE(a ++ b) = REVERSE(b) ++ REVERSE(a)
```

テンソル(§7.2/§4.3)は同型 `Tensor ≅ (data: V*, shape)` 上で reshape 群が添字に
作用する対象。`TRANSPOSE` は 2 階テンソルの対合、`RESHAPE` は総要素数を保つ往復、
`SHAPE`/`RANK` は添字構造の読み出し、`FILL` は所与の形のテンソル構成
(`SHAPE ∘ FILL = id` on shape)。`SPLIT` と `CONCAT` は逆操作
(`split` してから `concat` で復元)。`REORDER` は添字写像で、恒等添字 `[0 1 …]` は
id、反転添字は `REVERSE`。`SORT`(正準ホーム `ALGO`、§9.1)は決定可能な有理部分域で
冪等かつ置換不変(§7.4.3 の決定ケース)。

**テスト**: `rust/tests/structural_laws.rs` — reverse 対合・反同型、concat 結合、
split↔concat 往復、transpose 対合、reshape 往復、shape/rank/fill/range、take 全長恒等、
reorder 恒等/反転、sort 冪等/置換不変(計 10 法則群)。

---

## 9-ter. 拡張定式化 II — 観測基盤(Phase 1, 2026-06 改修)

本節は改修ロードマップ Phase 1(観測基盤と観測代数の確定)の成果を取り込む。
§8 の観測関数 `observe` を SPEC §2.3 の semantic axes で再定義し、§8 が用いる
表示函数 `render` を **全ロールについて純関数**として特徴づける。以後の全フェーズは
本節が固定する観測基盤(protocol 軸 + 純 `render`)の上に立つ。

### D. 観測代数 — 軸射影と純 `render`(Phase 1)

#### D.1 観測の二層分解

観測を **データ面の軸射影**と **表示**に分ける。値 `v ∈ V` の観測可能面は、
SPEC §2.3 の semantic axes への射影で与えられる:

```
obs_axes(v) = ( semanticKind(v), shape(v), capabilities(v),
                truthValue(v),   origin(v), absence(v) )
```

各成分は protocol 文字列(lower camelCase)であり、Rust enum 名・`Debug`・表示
テキスト・GUI 配色には一切分岐しない(semantic firewall)。プログラム全体の観測は

```
observe(p) = ( render*(π_Stack ⟦p⟧ σ₀), π_Eff ⟦p⟧ σ₀ )
```

で、`render*` はスタック各値への `render` の写像。**`π_Eff`(効果列)は Phase 7**
の効果代数で与え、本節はデータ面 `render*` と軸射影を確定する。

#### D.2 `render` は `(data, role)` の純関数

表示函数を

```
render : ValueData × Role → Display
```

とする(SPEC §12.1)。`Role` は SPEC §12.2 の解釈ロール 8 種
`{Unassigned, RawNumber, ContinuedFraction, Interval, Text, TruthValue,
Timestamp, Nil}`。`render` は **全ロールで定義された全域関数**であり、値が
内部に持つ既定ロール(hint)には依存しない。観測上の中心法則:

| 法則 | 内容 |
|---|---|
| 全域性・決定性 | `render(d, r)` は 8 ロール全てで定義(網羅)・決定的(SPEC §12.2) |
| ロール純粋性 | `render(d, r)` は `(data, role)` のみに依存し、担体値の hint に依存しない。すなわち **データとロールが等しければ表示は等しい**(SPEC §12.2 末尾) |
| 既定観測の分解 | 既定の表示 `to_string(v)` は `render(v, hint(v))`。`observe` の表示半は `render` を経由する |
| U の表示吸収 | 論理 Unknown は **全ロールで `UNKNOWN`** に表示され、`NIL` や数値形へ漏れない(SPEC §2.3, §7.5) |

ロール純粋性は「表示は意味プレーン(ロール割当)だけの関数であり、データ面を
変えない」という二面分離(SPEC §5.2, §12.1)の表示側の内容である。

#### D.3 semantic firewall — 構造軸はロール直交

データ面の **構造軸** `semanticKind` / `shape` / `origin` は値の data と absence
だけを読む。ゆえにロール(意味プレーン)の割当は構造軸を変えない:

```
obs_struct(v) = obs_struct(v with role := r)   (∀ r)
```

これは「意味面の変更が計算・データ面に非干渉」(SPEC §5.2)の軸レベルの言明。
真偽軸 `truthValue` と `truthValued` capability は **意図的にロール結合**で、
SPEC §2.3 が言う「`TruthValue` ロールを担う値にのみ truthValue 軸が現れる」を
反映する(構造軸とは別扱い)。

#### D.4 軸整合(runtime-produced 値)

参照実装が生成する値の上で、真偽軸と capability は整合する:

```
truthValue(v) ≠ ⊥  ⟺  truthValued ∈ capabilities(v)
```

また全値は基底 capability `{stackItem, serializable, displayable}` を備える。

#### D.5 所見(finding、descriptive)

- **所見 D1(真偽 capability のロール結合):** `truthValued` capability は
  `hint = TruthValue` を鍵に算出される一方、`truthValue` 軸は **Boolean を
  本質的に真偽値**として扱う(ロール非依存に `true`/`false` を返す)。両者は
  **runtime-produced 値では整合**するが、Boolean の担体を `TruthValue` 以外の
  ロールへ人為的に付け替えると軸=Some・capability=なしと **乖離する**。
  これは SPEC §2.3 が要求する「truthValue 軸を持つ値は truthValued capability も
  持つ」の境界事例であり、実行時に Boolean は常に `TruthValue` ロールを担うため
  実害はない。**モデル/実装の不変条件として追跡**(到達可能状態では HOLDS、
  人為再ロールでは BREAKS)。仕様は正典(§16.1)であり、本所見は記述に留める。
- **所見 D2(完全平方の√は有理に縮退):** `√4`・`√9` 等は有理数へ collapse し、
  等値比較が `U` でなく定値 `true` を返す。U を得る生成器は **非完全平方の
  radicand** に限定する必要がある(モデリング上の注意。仕様上の乖離ではない)。
- **所見 D3(空ブロックの表示は空文字列):** `{ }` は空文字列に表示される。
  したがって `render` の **非空性は法則ではない**;全域性・決定性のみが法則。

**テスト**: `rust/tests/observation_laws.rs` — render 全域/決定性、ロール純粋性、
既定観測=`render(·,hint)`、U の表示吸収、構造軸のロール直交、真偽軸=capability
整合、基底 capability、真偽値≠数値(所見 B の観測層再確認)、protocol 文字列の
lower camelCase(計 10 法則群)。生成器は `rust/tests/test_support/generators.rs`
(意味領域別)、観測は `rust/tests/test_support/observe.rs`(軸射影 + 純 `render`)。
以後のフェーズは generator を足すだけで法則を追加できる。

---

## 9-quater. 拡張定式化 III — 修飾子・契約・質量保存(Phase 3 ⭐, 2026-06 改修)

本節は改修ロードマップ Phase 3(修飾子と質量保存の型理論=契約)の成果を取り込む。
§2 の準同型(生成元はフレーズ)・§6 の修飾子コンビネータ・SPEC §7.14 の Coreword
契約・SPEC §13 の質量保存を、**実装に依らず検証可能な型システム**として与える。
本節は SPEC §6/§7.14/§13 の現象を記述するモデルであり、第二権威化しない(§16.1)。

### E. 修飾子コンビネータ・契約格子・線形質量(Phase 3)

#### E.1 修飾子は変換子コンビネータ

語の基底変換子 `base(w) : Σ ⇀ Σ+Error` に、修飾子が高階作用素として作用する
(§6)。被演算域 `δ_region` と消費規律 `κ_consume` の二軸:

```
δ : {TOP, STAK}   region 選択(頂上 arity 個 / 頂上 count 個の畳み込み)
κ : {EAT, KEEP}   consumption(消費 / 複製して結果も積む)
⟦μ·w⟧ = κ_consume ∘ δ_region ∘ base(w)
```

`TOP` と `EAT` は **既定の恒等**:`base(w) = ⟦TOP·w⟧ = ⟦EAT·w⟧ = ⟦TOP EAT·w⟧`。
糖衣は `.`≡TOP, `,`≡EAT, `..`≡STAK, `,,`≡KEEP(SPEC §6.1/§6.2)。4 組
`{TOP,STAK}×{EAT,KEEP}` は変換子→変換子の関手として閉じ、生成元をフレーズに取る
ため §2 の準同型を保つ。

`STAK·w` は **頂上の count を引数に取る左畳み込み**(probe で確定):スタック頂上の
整数 `n` を消費し、続く `n` 個を `w` で左結合的に畳む。`count∈{0,1}` は no-op
(count を戻す)。例 `x₁ … xₙ n STAK ADD = (((x₁ x₂ ADD) x₃ ADD) … xₙ ADD)`。

#### E.2 KEEP は分岐(bifurcation)= 質量の複製

`KEEP·w` は被演算子を残しつつ結果も積む(§13.2)。二項語 `w` の観測:

```
obs(a b KEEP w) = obs(a b) ⧺ obs(a b w)        (= [a, b, w(a,b)])
```

すなわち `EAT` 経路の上に、消費したはずの被演算子のコピーを保持する。

#### E.3 質量保存は線形(資源)不変条件(§13)

語の契約は arity・consumption・production・bifurcation を宣言する(SPEC §13.1)。
質量保存を **スタック深さ(資源)の差分**として観測的に定式化する。二項純粋語
`w`(arity 2)で:

```
depth(EAT·w 適用後) − depth(適用前) = production − consumption = 1 − 2 = −1
depth(KEEP·w 適用後) − depth(適用前) = production = +1
⟹ depth(KEEP·w) − depth(EAT·w) = consumption = arity = 2
```

`KEEP` と `EAT` の深さ差が **ちょうど arity** に等しいことが、分岐が「被演算子を
複製して保存する」という質量関係(§13.2)の観測内容である。過消費・未消費漏れ・
分岐比違反は、この資源等式の破れとして現れる。最適化経路は契約検証後にのみ進入
可能(§13.1)で、本モデルは資源等式が合成で閉じることを要求する。

#### E.4 Coreword 契約は Hoare 契約 + 格子(§7.14)

各 Coreword は機械可読契約 `{requires} w {ensures}` を持ち、分類は三つの格子で
型付けされる:

| 軸 | 領域(格子) | 観測的内容 |
|---|---|---|
| `partiality` | `Total` ⊑ `Projecting` / `Partial` | `Total`/`Projecting` は整形入力で**エラーを発生させない**(`Projecting` は領域失敗を NIL へ射影=total-by-projection);`Partial` のみ整形入力でエラーを起こしうる |
| `nil_policy` | `Passthrough`/`CreatesNil`/`RejectsNil`/`ConsumesNil`/`PreservesReason` | NIL への反応・生成規律(§4.5.1, §11.2) |
| `safety_level` | `A` ⊏ `B` ⊏ `C` ⊏ `D` ⊏ `Quarantined` | 保証の強さ。`A`=total・pure・deterministic |

契約欠落=型不在=適合違反(SPEC §7.14)。観測される契約↔挙動の対応(全 174 語で
HOLDS):

- **partiality↔挙動**: `Total` 語は整形入力で常に値を残す(`ensures` の充足);
  `Projecting`/`CreatesNil` 語は領域失敗(0 除算・範囲外 GET)を **NIL へ射影**し
  停止しない(§11.2)。
- **safety 格子の単調性**: `A ⟹ pure ∧ deterministic`;`effects ≠ ∅ ⟹ safety ∉
  {A,B}`;`Effectful ⟹ safety ∈ {C,D,Quarantined}`(反例 0)。
- **契約合成**: Kleisli 合成 `•`(§2.2)下で、上流の `ensures` が下流の
  `requires` を含意するとき合成は安全。partiality は `Partial` が伝播で支配的
  (一方が Partial なら合成も Partial 以上)であり、`Total/Projecting` は閉じる。

#### E.5 所見(finding)

- **所見 E1(質量フィールドの未実装)— 解消済み:** かつて SPEC §13.1 が宣言すると
  記す `arity / consumption / production / bifurcation` が `CorewordMetadata` に
  **無かった**。本改修で機械可読な質量契約 `mass : Fixed{consumes,produces} |
  Dynamic` を §7.14 契約フィールドへ追加(arity/consumption/production を担い、
  bifurcation は KEEP 修飾子=§13.2、NIL 射影は nil_policy が担う)。コンパイル済み
  プラン解析器(`quantized_block::builtin_arity`)と契約レジストリは同一の
  `coreword_registry::mass_contract` を読み、乖離しない。静的検証器
  `interpreter::mass_conservation::validate_mass_conservation` が CompiledPlan を
  抽象解釈し、過消費(空スタックからの深さが負)を **診断として**報告する
  (§13.1「reported by ... developer diagnostics」。最適化経路の gating は変更せず
  実行時は per-value FlowToken を持たない)。SPEC §7.14/§13.1 を相応に改訂。
- **所見 E2(safety A と partiality の不整合)— 解消済み:** SPEC §7.14 は safety `A`
  を「total・pure・deterministic」と定義する。`GCD`/`LCM` は非整数入力で実際に
  エラーを出す真の `Partial` なので、契約を `A`→`B`(「partial but explicit error
  categories」)へ修正(`Projecting` の `POW` 等は total-by-projection で `A` 据置)。
  以後「safety A ⟹ partiality∈{Total,Projecting}」が反例なく成立し、テストで不変
  条件として固定。仕様は正典のまま実装を仕様へ一致させた。
- **所見 E3(GET は非消費):** `[v…] i GET` は **元ベクタを保持**し要素(範囲外は
  NIL)を積む(`obs([1 2 3] 9 GET) = [[1 2 3], NIL]`)。GET の consumption profile は
  (vector 保持, index 消費, element 生成)であり、素朴な arity 計算と異なる
  (質量モデリング上の注意)。
- **所見 E4(契約は正準名で索引):** 糖衣 `/` は契約を持たず(canonical `DIV` で
  解決)、`DUP`/`DROP` 等は語として存在しない(Ajisai は修飾子で代替)。これは
  正準名索引の帰結であり、仕様上の乖離ではない。

**テスト**: `rust/tests/contract_modifier_laws.rs` — 既定修飾子の恒等、KEEP 分岐、
KEEP−EAT=arity(質量)、STAK 畳み込み・`..`≡STAK、STAK KEEP の保持、Total 無エラー、
Projecting の NIL 射影、契約の全域性(全語が到達可能契約を持つ)、safety 格子の
単調性(A⟹{Total,Projecting})、所見 E2 不変条件(A+Partial 皆無・GCD/LCM=B)、
§7.14 アンカー契約(ADD/DIV/EQ/LT/AND/OR/NOT)(計 11 法則群)。
`rust/tests/mass_conservation_laws.rs` — 質量契約の §7.14 露出、静的純 net mass=実行時
深さの健全性、KEEP の net mass 増分=arity、過消費⇔実行時 underflow、Dynamic で abstain
(計 5 法則群)。

---

## 9-quinquies. 拡張定式化 IV — 名前解決と辞書(Phase 6, 2026-06 改修)

本節は改修ロードマップ Phase 6(名前解決と辞書=モジュール・DEF)の成果を取り込む。
語名はもはや「裸の文字列」ではなく、**辞書状態 `Σ_dict` に相対的な束縛**である。
SPEC §7.8/§8(ユーザ辞書・DEF/DEL)・§7.10/§9(モジュール・IMPORT)・§7.14
(`canonical_home`/`listed_*` 契約フィールド)の現象を、決定的な解決関数と
可視性格子上の単調作用素として記述する。本節は仕様を追い越さない(§16.1)。

### F. 辞書・決定的 `resolve`・可視性格子(Phase 6)

#### F.1 辞書と解決の対象領域

辞書を三層の有限写像とする(SPEC §4.6・§7.8・§9):

```
Core : Name ⇀ Blk              (Core Words、不変)
Mod  : Module → (Name ⇀ Blk)   (Module Words、公式準組み込み語)
Usr  : Dict   → (Name ⇀ Blk)   (User Words、既定 DICT=EXAMPLE; Example Words はこの一群)
```

可視性状態 `Vis` は import 表 `Module ⇀ {all | W⊆Name}`(全公開取込か明示部分取込)と
ユーザ辞書からなる。語名は §3.8 で大文字へ正規化される(`sqrt ≡ SQRT`、`%≡MOD`,
`&≡AND`)ので、`resolve` の定義域は正規化済み `Name`。

#### F.2 `resolve` は決定的な可視性相対関数

解決を

```
resolve : Name × Vis ⇀ Blk + Unknown
```

とし、裸名の解決順は **Core Words → 取込済み Module Words → User Words**
で固定する(参照実装 `resolve_short_name`):

```
resolve(w, Vis) =
  Core[w]                          if w ∈ dom Core            -- (1) 核が最優先
  Mod[m][m@w]   (m: w を取込済みの最初のモジュール)            -- (2) 取込 Module Words
  最小 registration_order の一致    if User Words に一致       -- (3) 残りは登録順で決定的
  Unknown                          otherwise
```

修飾名は層で解決する:`CORE@w` は核へ、`m@w` は
モジュール `m` の取込済み Module Words へ、`USER@d@w`・`DICT@…` は各辞書へ(`split_path`)。
**中心性質——決定性**: `resolve` は評価履歴に依らず `(Name, Vis)` のみの関数であり、
同じ状態で同じ名は常に同じ束縛へ解決する。

#### F.3 DEF/DEL は辞書状態変換子(依存グラフ `FORC`)

`DEF`/`DEL` を `Σ_dict` 上の変換子として与える(SPEC §8):

```
⟦{b} 'w' DEF⟧ : Usr[EXAMPLE][w] := b      (核語は不可=「built-in 再定義不可」)
⟦'w' DEL⟧     : Usr[EXAMPLE] ∖ w
```

定義時に本体トークンを `resolve` して**依存グラフ** `dependents : Name → 2^Name` を
構成する(`rebuild_dependencies`)。被参照語の再定義・削除は **`FORC` ガード**で
保護され、力修飾子 `!` なしでは拒否される(`{requires: deps=∅ ∨ forced}`、§8.2)。
解決面での法則:`DEF` は名を可視化し定義本体の変換子を束ね、`DEL` はそれを左から
打ち消す——新鮮名 `w` について `resolve(w) = Unknown`、`DEF` 後 `= b`、`DEL` 後
再び `Unknown`(可視性に対する **DEF/DEL の往復恒等**)。

#### F.4 IMPORT/UNIMPORT は可視性格子の単調作用素

import 状態を包含で順序づけた格子 `(Vis, ⊑)` 上で(SPEC §9.2):

| 作用素 | 効果 | 法則 |
|---|---|---|
| `IMPORT` | `Vis[m] := all` | **冪等** `IMPORT_m ∘ IMPORT_m = IMPORT_m`(単調・上昇) |
| `IMPORT-ONLY S` | `Vis[m] := Vis[m] ∪ S` | 選択的:`S` のみ可視化、兄弟語は不可視のまま |
| `UNIMPORT` | 被参照語を保つ最小可視へ縮小 | **参照保存**:ユーザ語が指す `m@w` は残し残余を隠す |
| `UNIMPORT-ONLY S` | `S` を個別に隠す | 被参照セレクタは**拒否**(辞書 UNIMPORT を要求) |

`IMPORT` と `UNIMPORT` は、被参照語が無いとき**互いに逆**:`IMPORT_m` 後に
`UNIMPORT_m` すると裸名・修飾名の双方が import 前の `Unknown` へ戻る。境界語の
**`bare ≡ m@w`**:`m` 取込後、正準モジュール語は裸名でも `m@w` でも同一に観測される
(SPEC §7.14)。`core-listed` セレクタ(モジュール view に列挙されるが正準は核の語、
例 `IO` の `PRINT`)の `IMPORT-ONLY`/`UNIMPORT-ONLY` は **no-op**(既に核で可用、
警告のみ)。`canonical_home`/`listed_in_*`(§7.14)はこの作用素群の静的台帳であり、
裸名が複数の正準ホームを持つとき(例 核 `GET` と `JSON@GET`)契約照会は**核を優先**
する(実行時解決順に一致)。

#### F.5 所見(finding)

- **所見 F1(修飾解決は import ゲート付き)— 追跡中:** 静的契約レジストリ
  `get_coreword_metadata("MATH@SQRT")` は **常に**モジュール項へ到達する(SPEC §7.14
  「`MODULE@WORD` always reaches the module entry」)が、**実行時**の `m@w` 解決は
  `IMPORT` を要する(`4 MATH@SQRT` は未取込で `Unknown`)。すなわち §7.14 の到達性
  言明は**静的台帳の性質**であり、実行時可視性はそれに import ゲートを重ねる。
  乖離ではなく層の分離だが、両者を混同しないようガード付きオラクルで固定。
- **所見 F2(取込モジュール語は同名ユーザ語を遮蔽)— 追跡中:** 解決順が
  Core→取込モジュール→ユーザであるため、ユーザが先に `SQRT` を定義していても
  `'math' IMPORT` 後の裸 `SQRT` は **MATH@SQRT** に解決する(ユーザ語ではない)。
  自分の定義が勝つという素朴な期待と異なる決定的順序であり、オラクルで固定。
  到達可能状態では一貫(曖昧は §F.2(3) で `Unknown` 化)。

**テスト**: `rust/tests/naming_resolution_laws.rs` — 境界語 `bare≡m@w`、IMPORT 冪等、
解決決定性、核語は import で遮蔽されない、F2 遮蔽、DEF 本体インライン、DEF/DEL 往復、
`FORC` ガード(再定義・削除の `!`)、built-in 再定義不可、UNIMPORT が import を逆転、
UNIMPORT の参照保存、UNIMPORT-ONLY の被参照拒否、IMPORT-ONLY の選択性・core-listed
no-op、レジストリの canonical_home/核優先・修飾到達・F1 import ゲート(計 18 法則群)。
生成器は `rust/tests/test_support/generators.rs` に `module_word_call`/`user_word_name`
/`user_word_body` を追加。

---

## 9-sexies. 拡張定式化 V — 効果と観測(Phase 7, 2026-06 改修)

本節は改修ロードマップ Phase 7(効果と観測=ホスト効果・IO・意味プレーン)の成果を
取り込む。§9-ter D が与えた観測 `observe(p) = (render*(π_Stack ⟦p⟧ σ₀),
π_Eff ⟦p⟧ σ₀)` の **効果半 `π_Eff`** をここで確定する。SPEC §5.2(二面構造)・
§7.9(IO)・§9.4(SERIAL 受信)・§12(意味プレーン)の現象を、順序付き効果の代数と
移植性プロファイルの分離として記述する。本節は仕様を追い越さない(§16.1)。

### G. 効果代数 `π_Eff`・非決定性分離・移植性プロファイル(Phase 7)

#### G.1 `π_Eff` は送出ホスト効果の順序付き自由モノイド

実行が外界へ送り出す効果を、列(自由モノイド)として観測する:

```
Eff = (HostEffect*, ⧺, ε)       π_Eff : Program → Eff
```

`HostEffect` は安定なプロトコルタグ `kind ∈ {print, serial, audio, config,
effect, json_export, diagnostic}` と `payload`(文字列)を持つ(参照実装
`host.rs`、conformance の `data-kind`/`data-payload`)。観測は §2.3 firewall に従い
`kind`/`payload` のみを読み、Rust enum 名・`Debug` には分岐しない。**中心法則——
順序保存準同型**:

```
π_Eff(p ⧺ q) = π_Eff(p) ⧺ π_Eff(q)        π_Eff(ε) = ε
```

効果は実行順に追記され(`emit_host_effect` の push)、純粋文脈は列を乱さない
(効果は効果語のみに依存)。`PRINT` の payload は **意味境界での純 `render`** に
等しい(`x PRINT` ⟼ `(print, render(x))`);すなわち意味プレーン(ロール)は
π_Eff へ render を通じてのみ流入する(§12、§9-ter D.2)。

#### G.2 二面の独立(データ面 ⫫ 効果面)

スタック観測 `render*(π_Stack)` と効果列 `π_Eff` は **独立した観測チャネル**で
ある(SPEC §5.2)。純粋・内部Σ計算(算術・論理・構造・辞書・import)は
`π_Eff = ε` を出しつつスタックには値を残す。意味面(ロール割当)の変更は
データ面の構造軸を変えず(§9-ter D.3)、render は効果を出さない。

#### G.3 SERIAL 受信:`READ` の射影と run 内決定性(§9.4)

SERIAL の inbound は run 前に host が注入する受信バッファ(`serial_inbox`)で、
`READ` がこれを drain する event-poll モデル。観測契約:

```
READ(空 inbox)        = Bubble(noData)           (§11.2 射影、効果列に非追加)
READ(disconnected)    = Bubble(portDisconnected)
```

`READ` は領域失敗をエラーにせず NIL へ射影する(`Projecting`/`CreatesNil`)
total-by-projection であり、データ面の drain なので送出効果を出さない。outbound
(`OPEN`/`WRITE`/`CLOSE` …)は各々 `serial` 効果を実行順に 1 つ送出し、Phase 6 の
境界律が効果面にも及ぶ:**`bare ≡ SERIAL@w`** は同一効果列を生む。run 内は注入
入力が固定なので決定的(`serial_inbox` は `pub(crate)` 境界のため、注入を要する
完全 drain 律は crate 内 `serial/serial_command_tests.rs` が担い、本節は観測可能な
no-data 射影と run 内決定性を固定する)。

#### G.4 非決定性の分離と移植性プロファイル(§5.2 Portability Profiles)

非決定性は **効果ラベルで分離**され、host に隔離される。固定 host
(`DeterministicHostEnv`)の下では `NOW`/`CSPRNG` は再現的:**⟦·⟧ modulo host は
決定的**。レジストリの静的分類が不純を漏れなくラベル付けする:

```
purity = Pure        ⟹ deterministic ∧ effects = ∅
¬deterministic       ⟹ effects ≠ ∅                  (不純は必ずラベル付き)
profile = Hosted     ⟺ required_capability = Some    (host 依存の唯一の指標)
```

`Pure`(`ADD`/`HASH`)・`Observable`(host 読み取り:`NOW`/`CSPRNG`)・`Effectful`
(送出/内部変更:`PRINT`/`DEF`)の三層。Core プロファイルは **host 非依存**を意味し、
host capability を要しない範囲で `Effectful` な内部Σ語(`DEF`/`IMPORT`)を含む。

#### G.5 所見(finding)

- **所見 G1(π_Eff は送出効果のみ)— 追跡中:** 効果列 `π_Eff`(HostEffect ログ)は
  **外界へ送り出す効果**だけを記録する。host *読み取り*(`NOW`/`CSPRNG`、
  `Observable`、ラベル `time-read`/`random-read`)も、**内部Σ変更**(`DEF`/`IMPORT`、
  `Effectful` だが `Core`、ラベル `dictionary-*`)も、**ログには出さない**。すなわち
  レジストリ `effects` ラベル集合 ⊋ π_Eff 生成語。π_Eff は不純全体の **送出射影**で
  あり、非決定性の完全な記録ではない(host 入力は別チャネル=固定 host で決定化)。
  乖離ではなくチャネルの役割分担だが、混同しないようガード付きオラクルで固定。
- **所見 G2(`Core ⊋ Pure`)— 追跡中:** `WordProfile::Core` は「host 非依存」を
  意味し(`required_capability = None`)、`DEF`/`IMPORT`/`SPAWN` 等の **`Effectful`
  だが host 不要**な内部状態語を含む。ロードマップの「Core profile では純粋部分のみ」
  は不正確で、正しくは「Core = host に依存しない部分」(辞書/ランタイム変更を含む)。
  host 依存の指標は純度ではなく `Hosted ⟺ required_capability = Some`。

**テスト**: `rust/tests/effect_observation_laws.rs` — 効果列の連結準同型、PRINT の順序
=render payload、純/内部Σは ε、純文脈は効果不変、SERIAL outbound 順序・`bare≡SERIAL@w`、
READ no-data 射影、固定 host 下の NOW/CSPRNG 決定性、G1(送出のみログ)、純度分類の
整合(Pure⟹det∧ε・¬det⟹ラベル)、profile/capability 分離、G2 アンカー(計 12 法則群)。
生成器は `effect_free_src`/`serial_outbound_call` を追加。

---

## 9-septies. 拡張定式化 VI — 並行・子ランタイム(Phase 8, 2026-06 改修)

本節は改修ロードマップ Phase 8(並行性=子ランタイム)の成果を取り込む。
`SPAWN`/`AWAIT`/`STATUS`/`KILL`/`MONITOR`/`SUPERVISE`(SPEC §10)。ロードマップが
明示するとおり、完全な denotational 化は探索課題とし、本節は **観測契約の固定**を
優先する(隔離・状態機械・`AWAIT` を子の最終配置の射影として)。本節は仕様を
追い越さない(§16.1)。

### H. 子ランタイムの観測契約(Phase 8)

#### H.1 spawn 隔離と ProcessHandle

`SPAWN` はコードブロックを取り、**spawn 時に親辞書のスナップショット**(user/module
辞書・import 表・依存の deep copy)を子へ渡し、`ProcessHandle` を積む(SPEC §10.1)。
親子は stack・dict を**非共有**:子は空 stack で始まり、子の `DEF` は親に伝播しない。
ProcessHandle は §2.3 軸で観測可能:

```
semanticKind(handle) = process     shape(handle) = handle
```

#### H.2 子は AWAIT で同期実行され、最終配置を射影する

参照実装では `SPAWN` は子を**記録するのみ**で、実行は `AWAIT` 時に同期的に起こる
(所見 H1)。`AWAIT` は子を完了まで走らせ、結果タプルを積む:

```
AWAIT(handle) = [ status , result-stack ]
status ∈ {completed, failed, killed, timeout}     (Text)
result-stack = 子の最終 stack(値の順序付きベクタ;空なら NIL)
```

**中心法則——最終配置の射影**:整形ブロック `b` について

```
AWAIT(SPAWN{b}) = [ 'completed', ⟦b⟧∅ ]
```

ここで `⟦b⟧∅` は **空配置から `b` を走らせた最終 stack**。すなわち子は隔離下で同じ
denotation を計算する。`result-stack ≡ b の単独実行`(観測一致)。

#### H.3 状態機械(§10.2)

状態 `running → {completed, failed, killed, timeout}`:

| 観測 | 結果 |
|---|---|
| `SPAWN STATUS` | `running`(まだ AWAIT されていない、H1) |
| `SPAWN KILL` | `killed` |
| 整形ブロックの `AWAIT` | `completed` |
| 実エラー(未知語・underflow)の `AWAIT` | `failed`(失敗点までの部分 stack を保持) |

`KILL` は子を `killed` へ、`MONITOR` は **観測上恒等**(同一タプルを返す)、
`SUPERVISE{b}[n]` は完了ブロックで `AWAIT` と同一の `[completed, ⟦b⟧∅]` を返す。

#### H.4 隔離・再現性(§10.1)

- **親⫫子 stack 隔離**:`SPAWN` 直下の親値は子に不可視で、result-stack を変えない;
  親値は結果タプルの下に残る。
- **親⫫子 dict 隔離(双方向)**:子内 `DEF` は親で `Unknown`;spawn 前に親で `DEF`
  した語は子のスナップショットに入り可視。
- **再現性**:決定的ブロックの `SPAWN AWAIT` は二度走らせても同一タプル(子は
  決定的)。

#### H.5 所見(finding)

- **所見 H1(子は AWAIT で走る)— 追跡中:** `SPAWN` は子を記録するだけで実行しない;
  実行は `AWAIT` がプルした時に同期的に起こる。`SPAWN STATUS` は `running` を返す。
  SPEC §10.4「AWAIT … Blocks until the child finishes」と整合(本実装は遅延起動+
  同期実行で、観測上は単一 run 内で決定的)。真の並行(スレッド)ではない。
- **所見 H2(ドメイン失敗は子を失敗させない)— 追跡中:** `x 0 /` は Bubble 則で NIL へ
  射影(§11.2)するため、子は `failed` でなく **`completed`**(result `[NIL]`)で終わる。
  `failed` になるのは **実エラー**(未知語・stack underflow 等)に限る。これは §9-quater
  E.4 の partiality 格子(`Projecting` は total-by-projection)が子の終了状態に波及した
  もので、乖離ではない。ガード付きオラクルで固定。

**テスト**: `rust/tests/child_runtime_laws.rs` — ProcessHandle 軸、AWAIT=最終配置射影、
親子 stack 隔離、再現性、MONITOR 恒等、SUPERVISE=AWAIT 一致、状態機械(running/killed/
completed/failed)、実エラーで failed、dict スナップショット隔離(双方向)、H1 遅延実行、
H2 ドメイン失敗は completed(計 11 法則群)。生成器は `completing_block_body`/
`failing_block_body` を追加。

---

## 9-octies. 拡張定式化 VII — 統合:レコード・文字列・被覆閉包(Phase 9, 2026-06 改修)

本節は改修ロードマップ Phase 9(統合・検証・自己ホストへの道)の成果を取り込む。
残る `Absent` 領域(レコード §4.4・文字列 §7.6)を Defined 化して被覆行列を閉じ、
conformance を `⟦·⟧` の標本として位置づけ直し、証明支援系機械化の実現可能性を評価し、
法則スイートをオラクルとして総括する。本節は仕様を追い越さない(§16.1)。

### I. レコード・文字列の代数と被覆閉包(Phase 9)

#### I.1 レコードは挿入順保存の有限写像(§4.4)

レコードを `Record ≅ (Name ⇀ V, 挿入順)` として与える。構成・操作は `JSON`
モジュール経由(レコードリテラル構文は無い=所見 I1):

```
JSON@PARSE : Text ⇀ Record        JSON@GET : Record × Name ⇀ V + NIL
JSON@SET   : Record × Name × V → Record'   (点別更新)
JSON@KEYS / JSON@VALUES : Record → V*   (挿入順)
JSON@MERGE : Record × Record → Record   (右優先)
```

観測:`semanticKind = record`, `shape = record`。法則:`GET` は写像の定義性
(`{a:vₐ…} 'a' GET = vₐ`)、`SET` 後 `GET` 同鍵 = 新値・他鍵不変(点別)、`KEYS` の
挿入順は既存鍵 `SET` で不変、`VALUES` は鍵順、`MERGE` は重複鍵で右優先、欠落鍵の
`GET` は NIL 射影(`Projecting`、§11.2)。

#### I.2 文字列は符号点列(§7.6)

文字列リテラル `'abc'` は **`Text` ヒント付き符号点ベクタ**(空列は NIL=
`EmptySequence`、§4.5)。テキスト語(`STR`/`NUM`/`BOOL`/`CHR`/`CHARS`/`JOIN`/
`TRIM*`/`TOKENIZE`/`SUBSTITUTE`/`STARTS-WITH?`/`ENDS-WITH?`)は核(境界列挙 `TEXT`)。
法則:`CHARS∘JOIN = id`(符号点列の分解/再結合)、`TRIM` 冪等、`STR∘NUM = id`(整数を
テキスト経由で値保存)、`STR` は正準十進、`SUBSTITUTE` の自己置換恒等、自己 prefix/
suffix(`STARTS/ENDS-WITH?` 反射)、`TOKENIZE` 片の `JOIN` 復元。非数値 `NUM` は NIL
射影(`Projecting`)。

#### I.3 被覆行列の閉包

Phase 1–9 で `⟦·⟧` の被覆は仕様の全主要節に及んだ。残る Sketched 行は **核挙動は
Defined で、Sketch 部分は最適化・予算層の精緻化**であり、核 `⟦·⟧` の意味としては
**明示的 Out-of-scope**(refinement であって未定義意味ではない):

| 仕様節 | 状態 | 扱い |
|---|---|---|
| §4.4 レコード | **Defined**(Phase 9) | I.1 |
| §7.6 文字列 | **Defined**(Phase 9) | I.2 |
| §4.2.5/§7.4.1.1 NICF 比較 | Defined(核)/ Out-of-scope(予算単位の精緻化) | `cmp_β` は §3.3 で Defined;NICF 加速は実装最適化 |
| §4.5 absence metadata | Defined(核)/ Out-of-scope(診断メタの内部表現) | Bubble モナドは Defined;reason 文字列の網羅は §11 conformance |
| §7.4.2/§7.4.3 予算比較・U 伝播 | Defined(MIN/MAX/SORT/COND は Phase 4/5)/ Out-of-scope(`COMPARE-WITHIN` の予算単位) | U 伝播は Defined;明示予算 API は最適化層 |
| §11 誤差述語 | Defined(Bubble Rule)/ Out-of-scope(整形/不整形述語の網羅形式化) | 二層 `Σ+Error` は Defined |

すなわち成功基準「全節が Defined か明示的 Out-of-scope」(ロードマップ §5-1)を満たす。

#### I.4 conformance は `⟦·⟧` の標本

参照 conformance スイート(`tests/conformance/index.html`、45 ケース、
`rust/src/conformance_tests.rs` が実行)は、各ケースが `(source, expect-result,
expect-effects)` を与える**言語非依存**の標本である。本形式化はこれを `⟦·⟧` の
有限標本として包摂する:法則スイート(計 10 ファイル・130 超の法則群)は conformance
が点検する個別等式を **内包的全称**へ格上げする(ロードマップ §5-3)。

#### I.5 証明支援系機械化の実現可能性(評価)

核(数=GL₂(ℤ)行列積、K3=Kleene 束、NIL=Bubble モナド、モノイド準同型、契約格子)は
Lean/Coq/Agda への機械化が **実現可能**と評価する:いずれも有限公理化された代数構造で、
本書の等式は型付き等式として転記できる。妥当な順序は (1) K3 束と De Morgan、(2) 有理
GL₂(ℤ) 算術と単位/逆元、(3) Bubble モナド則、(4) 契約格子の単調性。一方、効果列
(Phase 7)・子ランタイム(Phase 8)・予算付き観測(§3.3)は **co-inductive/効果系**を
要し投資が大きいため、核の機械化を先行し、参照実装が `⟦·⟧` を refine することの証明は
長期目標に留める(ロードマップ §5-5)。

#### I.6 オラクルとしての法則スイート

「法則破れ = 実装が Ajisai でない」を CI で自動可視化する装置は、**性質ベース法則
スイートそれ自体**である(`rust/tests/*_laws.rs`)。各法則は §2.3 firewall を通した
観測等式であり、破れは即 CI 赤として現れる。追跡中の乖離(所見 B/C/D1/E1/E2/F1/F2/
G1/G2/H1/H2/I1/I2)はすべてガード付きオラクルで固定され、ドリフトは検知される
(ロードマップ §5-4)。

#### I.7 所見(finding)

- **所見 I1(レコードリテラル構文は無い)— 記述:** レコードは `JSON@PARSE` 等
  モジュール経由でのみ構成され、`{ }`(コードブロック)・`[ ]`(ベクタ)に相当する
  レコードリテラルは無い。仕様 §4.4 と整合(乖離ではない)。
- **所見 I2(CONCAT は単一要素 top でアンダーフロー)— 追跡中:** `CONCAT` の **頂上
  オペランドが単一要素ベクタ**だと `[ 1 ] [ 2 ] CONCAT` は `StackUnderflow` になる
  (単一要素ベクタ頂上が特別扱い=spread される)。多要素頂上 `[ 1 ] [ 2 3 ] CONCAT`
  は正常。文字列の `JOIN∘CONCAT` 則は ≥2 文字語に制約して回避し、本挙動はガード付き
  オラクルで固定。実装/仕様判断が要る候補(§7.1 の CONCAT arity)。

**テスト**: `rust/tests/record_laws.rs` — レコード観測軸、`GET`=写像、get-after-set、
点別 SET、KEYS 順序安定、VALUES 鍵順、MERGE 右優先、欠落鍵 NIL 射影、PARSE/STRINGIFY
往復(計 9 法則群)。`rust/tests/string_laws.rs` — リテラル恒等、`CHARS∘JOIN`、TRIM
冪等、`STR∘NUM`、`STR` 正準十進、SUBSTITUTE 自己置換、自己 prefix/suffix、JOIN/CONCAT
連結、TOKENIZE 復元、空列 NIL、非数値 NUM の NIL 射影、CHR/BOOL アンカー、所見 I2
(計 13 法則群)。生成器は `ascii_word`/`record_abc` を追加。

---

## 9-novies. 拡張定式化 VIII — 観測面と表示プロファイル(2026-06 改修)

本節は SPEC §12.3「Observation surfaces」と Portability Profiles「Presentation
Profile」の現象を、**観測面の射影**と**可視性の遷移系**として記述する。§9-ter D
の観測代数(軸射影と純 `render`)と §9-sexies G の効果代数 `π_Eff` の上に立ち、
GUI を「現行の数式に次ぐ本質」として ⟦·⟧ の周辺に位置づける。本節は仕様を追い越さ
ない記述モデルである(§16.1)。

### J. 観測面の射影と Presentation Profile LTS(Phase 10)

#### J.1 四つの観測面は状態の全域射影

観測可能な機械状態を、§1.2 の配置 Σ に **未評価の編集バッファ** `B`(まだ評価さ
れていないソーステキスト、§2.1 のフレーズ列の素)を添えて

```
R  =  B × Σ  =  B × (Stack × Dict × Eff)
```

とする。Ajisai は `R` を **ちょうど四つの観測面**で露出する。各面は `R` 上の
**全域かつ純粋な射影**であり、三つは既出の射影そのものである:

```
π_Input(R) = B                              (編集バッファ、新規命名)
π_Stack(R) = render*(Stack)                 (§9-ter D.1, §9-sexies G.2)
π_Output(R) = π_Eff(R) = Eff                (§9-sexies G.1、送出ホスト効果列)
π_Dict(R) = resolve-view(Dict)              (§9-quinquies F、可視語彙)
```

| 法則 | 内容 | 由来 |
|---|---|---|
| 全域性 | 各 `π` は到達可能な全 `R` で定義(空 `B`/空 `Stack`/空 `Eff`/Core のみの `Dict` は通常値) | SPEC §12.3 |
| 純粋性 | 各 `π` は `R` のみに依存。`R` が等しければ四面は同一(§9-ter D.2「data と role が等しければ表示は等しい」のマシン全体版) | SPEC §12.2, §12.3 |
| デバイス非依存 | 四面は **何が観測可能か**を定め、空間配置・パネル化・配色(非観測、SPEC §2.3)は含まない | SPEC §12.3 |

画面を持たないホストでも四射影は定義され、`π_Output` は `PRINT`/`IO`/`SERIAL`
(§9-sexies G)、`π_Dict` は `LOOKUP`(§9-quinquies F)等の host 語で露出する。
**射影は本質(常に定義)であり、可視化は次段のプロファイル**である。

#### J.2 Presentation Profile は可視性のラベル付き遷移系

四面のうち**どれを同時に見せ、ユーザ操作でどう遷移するか**を、面集合
`A = {Input, Output, Stack, Dictionary}` 上の **LTS** として与える:

```
M  =  (C, Σ_op, →, c₀)
   C ⊆ 𝒫(A)            可視構成 c ⊆ A の到達集合
   Σ_op                抽象操作の語(show(a), run, advance, retreat, …)
   → ⊆ C × Σ_op × C    遷移関係
   c₀ ∈ C              初期構成
```

具体的な `C`・`Σ_op`・幾何は実装自由。次の **不変量**が規範であり、表示プロファイル
が適合する ⟺ これら全てを満たす LTS のモデルであること:

| # | 不変量 | 内容 |
|---|---|---|
| 1 | Partition | 各 `c` で各面は可視/隠匿のいずれか、`c ⊎ (A∖c) = A`。隠匿は破棄ではない |
| 2 | Reachability | 任意の `a∈A`・`c∈C` から有限列で `a` を露出可能。永久不可達な面はない |
| 3 | Non-emptiness | 全 `c∈C` で `c ≠ ∅`。ユーザに「何も無い」を見せない |
| 4 | Determinism | `→` は部分関数。同一構成・同一操作は同一構成へ |
| 5 | Idempotent selection | 可視面の再選択は no-op:`a∈c ⟹ c −show(a)→ c` |
| 6 | Semantic coupling | (i) 編集可能な構成は `Input∈c`;(ii) `run` の後状態は `Stack∈c`;(iii) 語挿入は `B` へ書き、`π_Input` を到達可能に保つ |

不変量 1〜6 は **Ajisai の編集体験を定義する挙動**を捉え、デバイス的偶有
(タイル/単一面の別、切替ブレークポイント、ジェスチャ閾値、タップ数)は §5.3 の
ステップ上限と同格の調律として自由に残す。

#### J.3 結合律が冪等部分空間を彫り出す(所見 J1)

デスクトップ二列モデルは状態 `(left, right) ∈ {Input,Output}×{Stack,Dictionary}`、
可視構成 `c = {left, right}`。結合規則(不変量 6:Output⇒右=Stack、Dictionary⇒
左=Input)は、**意図の衝突する構成 `{Output, Dictionary}` を到達集合 C から排除**
する。ゆえに `C = {{Input,Stack}, {Output,Stack}, {Input,Dictionary}}` の三点に
閉じ、この **到達部分空間上でのみ**不変量 5(可視面の再選択 no-op)が成立する
(到達不能な `{Output,Dictionary}` では `show(Output)` が右を Stack へ動かし
no-op でない)。すなわち **意味結合(6)が冪等選択(5)の閉性を生む**。これは
「不変量は到達集合上で量化する」ことの具体例であり、検証は BFS で C を構成して行う。

単一面(モバイル)モデルは状態 `s∈A`、`c={s}`、`advance/retreat` が
`VIEW_ORDER = [Input,Output,Stack,Dictionary]` を巡回。巡回は全面を辿るので
到達性(2)を、`{s}` は常に単元なので非空性(3)を、`run⟼Stack`(三連タップ)は
実行可観測(6.ii)を満たす。デスクトップとモバイルは **同一の抽象 LTS の二つの
モデル**であり、同じ言語を提示する(SPEC §12.3 デバイス非依存)。

**テスト**: `src/gui/layout/presentation-profile.test.ts` — 実装の遷移コア
(`updateDesktopModes`・`resolveNextViewMode`)を直接駆動し、デスクトップ/モバイル
両モデルで不変量 1〜6 を到達集合上で検証、所見 J1(`{Output,Dictionary}` 不可達と
三点 C)をアンカー(計 20 ケース)。射影命名 `π_Input` は本節が新規、`π_Stack`/
`π_Output=π_Eff`/`π_Dict` は既出射影への結線。

---

## 10. 付録 — 経験的法則チェック(参照実装 2026-06)

参照実装に直接プログラムを流して §9.2 の法則を確認した結果。
`HOLDS` = 両辺の観測が一致、`BREAKS` = 不一致。

| 法則 | 結果 | 備考 |
|---|---|---|
| `5 IDLE ≡ 5`(単位元) | HOLDS | |
| `2 3 ADD ≡ 3 2 ADD`(可換) | HOLDS | |
| `1 2 ADD 3 ADD ≡ 1 2 3 ADD ADD`(結合) | HOLDS | |
| MUL 分配 | HOLDS | |
| `7 0 ADD ≡ 7`, `7 1 MUL ≡ 7`, `7 7 SUB ≡ 0` | HOLDS | 体の単位/逆元 |
| De Morgan, 二重否定 | HOLDS | K3 健全 |
| `1 0 / 1 ADD ≡ NIL`(passthrough) | HOLDS | Bubble モナド |
| K3 De Morgan / 結合 / 冪等(T,F,U 全網羅) | HOLDS | `tests/algebraic_laws.rs` で常時検証 |
| `TRUE 1 EQ ≡ FALSE`(B2 区別) | HOLDS(修正後) | **所見 B 解消**: T≠1 |
| 真偽値の一貫表示 `TRUE`/`FALSE`/`UNKNOWN` | HOLDS(修正後) | **所見 B1 解消** |
| 無理数の入れ子CF表示 §4.2.3 | HOLDS(修正後) | **所見 C 解消** |
| 脱糖透明 `a b + ≡ a b ADD` ほか(§9-bis A) | HOLDS | `tests/desugar_laws.rs` |
| `;` ≡ `. ,`、`~` no-op、大小文字正規化 | HOLDS | `tests/desugar_laws.rs` |
| MAP 恒等/融合、FOLD/SCAN cata(§9-bis B) | HOLDS | `tests/higher_order_laws.rs` |
| FILTER 冪等/可換、`ALL p ≡ ¬ANY¬p` | HOLDS | `tests/higher_order_laws.rs` |
| EXEC inline、`EVAL(STR p) ≡ p`、COND の U 不発火 | HOLDS | `tests/higher_order_laws.rs` |
| REVERSE 対合/反同型、CONCAT 結合、SPLIT↔CONCAT | HOLDS | `tests/structural_laws.rs` |
| TRANSPOSE 対合、RESHAPE 往復、SORT 冪等/置換不変 | HOLDS | `tests/structural_laws.rs` |
| render 全域/決定性・ロール純粋・既定観測=`render(·,hint)`(§9-ter D) | HOLDS | `tests/observation_laws.rs` |
| U の表示吸収・構造軸のロール直交・真偽軸=capability 整合 | HOLDS | `tests/observation_laws.rs` |
| 真偽値≠数値(所見 B 観測層)・protocol 文字列 lower camelCase | HOLDS | `tests/observation_laws.rs` |
| 所見 D1: 真偽 capability のロール結合(人為再ロールで乖離) | BREAKS(人為) / HOLDS(到達可能) | §9-ter D.5、追跡中 |
| 既定修飾子の恒等・KEEP 分岐・KEEP−EAT=arity・STAK 畳み込み(§9-quater E) | HOLDS | `tests/contract_modifier_laws.rs` |
| Total 無エラー・Projecting の NIL 射影・契約全域性・safety 格子単調 | HOLDS | `tests/contract_modifier_laws.rs` |
| §7.14 アンカー契約(ADD/DIV/EQ/LT/AND/OR/NOT) | HOLDS | `tests/contract_modifier_laws.rs` |
| 所見 E1: 質量契約 `mass` を §7.14 へ露出・静的検証器を実装 | HOLDS(解消) | §9-quater E.5、`tests/mass_conservation_laws.rs` |
| 静的 net mass=実行時深さ・過消費⇔underflow・KEEP増分=arity | HOLDS | `tests/mass_conservation_laws.rs` |
| 所見 E2: safety A⟹{Total,Projecting}(GCD/LCM を B へ修正) | HOLDS(解消) | §9-quater E.5、`tests/contract_modifier_laws.rs` |
| 境界語 `bare≡m@w`・IMPORT 冪等・解決決定性・核語は import で非遮蔽(§9-quinquies F) | HOLDS | `tests/naming_resolution_laws.rs` |
| DEF 本体インライン・DEF/DEL 往復・`FORC` ガード(`!`)・built-in 再定義不可 | HOLDS | `tests/naming_resolution_laws.rs` |
| UNIMPORT が import を逆転・参照保存・UNIMPORT-ONLY 被参照拒否・IMPORT-ONLY 選択/core-listed no-op | HOLDS | `tests/naming_resolution_laws.rs` |
| 所見 F1: 修飾解決は import ゲート(静的台帳は常に到達) | HOLDS(層分離) | §9-quinquies F.5、追跡中 |
| 所見 F2: 取込モジュール語が同名ユーザ語を遮蔽(Core→mod→user) | HOLDS(到達可能) | §9-quinquies F.5、追跡中 |
| π_Eff 連結準同型・PRINT 順序=render payload・純/内部Σは ε(§9-sexies G) | HOLDS | `tests/effect_observation_laws.rs` |
| SERIAL outbound 順序・`bare≡SERIAL@w`・READ no-data 射影・固定host下 NOW/CSPRNG 決定 | HOLDS | `tests/effect_observation_laws.rs` |
| 純度分類整合(Pure⟹det∧ε・¬det⟹ラベル)・profile/capability 分離 | HOLDS | `tests/effect_observation_laws.rs` |
| 所見 G1: π_Eff は送出効果のみ(host読取・内部Σ変更はログ非追加) | HOLDS(チャネル分担) | §9-sexies G.5、追跡中 |
| 所見 G2: `Core ⊋ Pure`(Core=host非依存、Effectful内部Σ語を含む) | HOLDS | §9-sexies G.5、追跡中 |
| ProcessHandle 軸・AWAIT=最終配置射影・親子stack隔離・再現性(§9-septies H) | HOLDS | `tests/child_runtime_laws.rs` |
| 状態機械(running/killed/completed/failed)・MONITOR恒等・SUPERVISE=AWAIT一致 | HOLDS | `tests/child_runtime_laws.rs` |
| dict スナップショット隔離(双方向) | HOLDS | `tests/child_runtime_laws.rs` |
| 所見 H1: 子は AWAIT で同期実行(SPAWN は記録のみ、STATUS=running) | HOLDS | §9-septies H.5、追跡中 |
| 所見 H2: ドメイン失敗(0除算→NIL)は completed、実エラーのみ failed | HOLDS | §9-septies H.5、追跡中 |
| レコード観測・GET=写像・get-after-set・点別SET・KEYS順序・MERGE右優先(§9-octies I.1) | HOLDS | `tests/record_laws.rs` |
| 文字列 CHARS∘JOIN・TRIM冪等・STR∘NUM・自己prefix/suffix・空列NIL(§9-octies I.2) | HOLDS | `tests/string_laws.rs` |
| 所見 I1: レコードリテラル構文は無い(JSON 経由構成) | HOLDS(記述) | §9-octies I.7 |
| 所見 I2: CONCAT は単一要素 top で underflow | HOLDS | §9-octies I.7、`tests/string_laws.rs`、追跡中 |

健全な核(有理算術・K3 論理・NIL モナド・モノイド合成)は法則を満たす。
かつて §9.3 にあった二つの破れ(B: T=1 の型混同、C: 無理数の近似表示)は
**本ブランチで解消**され、K3 法則 14 件は `tests/algebraic_laws.rs` の
性質ベーステストとして常時グリーンである。

---

## 11. 結論

Ajisai は **「整数行列積としての数」+「予算付き観測としての比較」+
「Kleene 代数としての真偽」+「Bubble モナドとしての部分性」+
「状態変換子のモノイド準同型としてのプログラム」** という、特定の実装言語に
一切依存しない数学的対象として定式化できる。この定式化は (1) conformance を
内包する完全な同一性基準を与え、(2) 有限の等式で無限のテストを表す強い移植性
契約を生み、(3) 実装の乖離(所見 B・C)を法則の破れとして自動的に暴く。

これは前回レビューの「P0: conformance を仕様各節へ拡張」の上位互換である。
§9.2 の法則は **`rust/tests/algebraic_laws.rs` の性質ベーステストとして導入済み**で、
参照実装の適合を等式レベルで連続検証する。所見 B(真偽値とデータ面での数値の
分離)と所見 C(無理数の入れ子CF表示)も本ブランチで解消され、対応する仕様改訂
(§4.1, §12.2)と conformance ケース(`core-boolean-is-not-a-number` ほか)を伴う。
