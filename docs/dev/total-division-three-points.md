# Total division and the three points over zero

Status: non-canonical. 方針記録。正典は `spec/language-semantics.md` の
LANG.VALUES.EXACT と、`spec/words.json`・`spec/outcomes.json`・`spec/grammar.json`
のみ。本書は採用した判断とその理由を記録し、何も定義しない。

## 判断

数は**非負の分母を持つ既約な整数の組**である。除算は逆数との積であり、逆数は
すべての組に対して定義される（符号は分子へ運ぶ）。したがって除算は全域的で、
ゼロ除算は不在ではなく数を答える。`gcd(n, 0) = |n|` なので、0 の上の組は既約化
により `1/0`・`-1/0`・`0/0` の三つに尽きる。`100 0 DIV` が `1/0` なのは `4/2` が
`2/1` なのと同じ理由であり、`1/0` は `1/2` と同じくリテラルである。

四則は分数の公式をすべての組に適用して既約化したものと定める。公式から読み取れる
帰結（`x 0 DIV` は `x` の符号を 0 の上に置いた点、`1/0 1/0 ADD` は `0/0`、
`0 1/0 MUL` は `0/0`、`1 1/0 DIV` は `0`、`0/0` はすべてを吸収）を個別に決めない。
三つの点でだけ、0 に言及する体の法則（`0·x = 0`、`x − x = 0`、`x ÷ x = 1`、
分配法則）が成り立たなくなる。実装はこれを特別扱いで取り戻してはならない。

順序は `-1/0 < 体 < 1/0` で、`0/0` だけが順序を持たない。順序を問うワード
（LT GT MIN MAX SORT ORDER BSEARCH）は `0/0` に対して `domainMiss` を投影し、
真偽の位置ではそれが UNKNOWN として読まれる。等価は表示意味論で決まるので
`0/0 0/0 EQ` は TRUE である。

## 捨てたもの

- NIL 理由 `divisionByZero`（`spec/outcomes.json`）。ゼロ除算は数を答えるので、
  投影すべき不在がない。NIL の不在理由は `-1 SQRT` の `domainMiss` が代表する。
- 文法の `zeroDenominator` 規則。`1/0` はソースでもテキストでも数である。
- 密テンソルの不在マップ。密レーンは数だけを保持し、NIL はレーンに入らない。
  「分母 0 = 不在」という内部表現は、分母 0 が数になった時点で成立しない。
- `ErrorCategory`／`NilReason`／`AbsenceOrigin` の `DivisionByZero` 変種。

## なぜ三値へ寄せないか

ゼロ除算を NIL にする設計（旧）は、不在が算術を素通りする法則と、不在が
分子を覚えているという内部表現の二つを抱えていた。後者は観測できない構築履歴
であり、LANG.VALUES.DENOTATION に反する。前者は `0 x MUL` が 0 でなくなる代償を
払いながら、その代償を「不在」という別の層に隠していた。組の公式に戻せば代償は
同じ場所（三つの点）で同じ大きさのまま表に出て、不在は本来の意味（部分関数の
答えがないこと）だけを担う。三値論理には `0/0` の順序という一本の橋だけが残る。

## 実装の要点

- `rust/src/types/fraction_extended.rs`: `extended_add` / `extended_mul` /
  `reciprocal`、`positive_infinity` / `negative_infinity` / `nullity`、
  `is_finite` / `is_nullity` / `signum`。
- `rust/src/types/fraction_order.rs`: `Fraction::order` は `Option<Ordering>` を
  返し、`0/0` に対してのみ `None`。`PartialOrd` はこれを経由し、`Ord` は実装しない。
- `rust/src/interpreter/comparison.rs`: `unordered_projection()` が順序ワードの
  投影先。`three_way_compare` と `compare_scalar_pair` は `Option` を返す。
- 高速層（`fused_block*.rs`・`dense_kernels.rs`・`quickened.rs`・`simd_ops.rs`）
  は有限レーン以外を辞退し、組の公式を仮定する経路に 0 分母を通さない。
