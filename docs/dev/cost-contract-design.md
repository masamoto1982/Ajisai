# Cost contract design (Phase 5)

Status: **non-canonical, design rationale** (`[設計根拠]`). Canonical source
remains `SPECIFICATION.html` / `spec/`; this document records why the `cost`
axis of `#:contract` is shaped the way it is. The space axis has no memo of its
own; its inference is documented in `rust/src/interpreter/word_space.rs`.

## 1. The three declarable axes

`cost` declares a bound on exactly the three fields of `ResourceUsage`
(`rust/src/interpreter/interpreter_core.rs`) — no more, no fewer:

| `#:contract cost` axis | `ResourceUsage` field | What it counts |
| --- | --- | --- |
| `steps` | `execution_steps` | Words dispatched (`executionSteps`) |
| `numeric` | `numeric_work` | Limb-multiply units charged by exact arithmetic (`numericWork`) |
| `collection` | `collection_work` | Element operations charged inside collection Words (`collectionWork`) |

`bigintBits` and `algebraicTerms` are deliberately not declarable axes:
`ResourceUsage` itself does not carry them (they are checked per-result, never
accumulated — see that struct's own doc comment: "inventing fields for them
would mean reporting a number nothing measured"). A declaration axis with no
runtime counter behind it would be exactly that defect moved into
`#:contract`.

## 2. The class lattice and its join rule

Four classes, ordered tightest to loosest: `Const` < `Linear` <
`Superlinear` < `Unbounded`. Identical in shape to `word_space.rs`'s
`SpaceClass` (Phase 2.2 of the structural-memory-safety roadmap), reused
rather than reinvented, because the two problems are the same shape: a sound
upper bound on *how a quantity grows as a function of input*, joined
monotonically across a word body's dependencies.

A `CostBound` carries `(class, exact)` per axis, exactly as `SpaceBound`
does: `exact = true` means some contribution *provably attains* that class
(licensing a declaration `Error`); `exact = false` means the class is a safe
upper bound with no proof of attainment (licensing only a `Note`, never an
`Error` — the "never a false error" invariant `word_space.rs`'s module
comment states, inherited verbatim here). Joining two bounds on one axis
takes the wider class; when both sides are already at the same class, `exact`
becomes the OR of the two — one attaining contribution is enough to prove the
whole join attains it.

## 3. What `exact` means and when a declaration can become `Error`

`check_one`'s cost check (`contract_decl.rs`) compares the inferred class
against the declared one per axis. A declared class *narrower* than the
inferred one is a mismatch:

- `exact == true` → the inferred class is provably attained → `Error`
  (a real violation — the same "declared X but inference proves worse than X"
  shape `#:contract`'s existing arity/purity/nil-free checks already use).
- `exact == false` → the inferred class is a sound bound with no proof of
  attainment → `Note`.

A witness survives to the join only when the operands that produced it are
genuinely traced *and* the class was not refined away beneath it (§6). A
witness is never carried on a class the operands do not actually justify —
that is what keeps the first bullet from firing on a true declaration.

This is the same two-outcome shape `word_space.rs` already established for
the space axis; `cost` inherits it rather than inventing a fourth kind of
disagreement. One real difference from arity/purity/nil-free: those three
checks gate `Note` vs. `Error` on the *word's* `ContractConfidence`
(`Conservative` vs. `Complete`), so their `Note` always carries a Phase 3 gap
id (`contract.gaps.first()`). A cost axis's `exact` bit is independent of
that flag — a builtin like `MAP` can be `Complete` (no unresolved word, no
recursion) while its `steps` axis is still `(Unbounded, false)` by the
classification's own honest design (§7's table), not because inference gave
up on the word. So a cost `Note` reuses the word's gap id only when the word
is itself `Conservative`; otherwise its `code` is `null`, and that is not a
bug — `docs/dev/agent-cli-output-contract.md`'s `cost` section states this
plainly for API consumers.

## 4. Why the class lattice, not degree-annotated polynomials

`3n^2 + 7n + 2` is the eventual target and is explicitly **not** built here.
Two reasons, not one:

- **The lattice's `join` is trivially monotone and sound**: taking the wider
  class of two bounds is correct by construction, and correctness of the
  `exact` OR-rule follows in one line. A polynomial join has to decide what
  happens to *coefficients* when two bounds combine — does `2n + 3n` become
  `5n`, and does that arithmetic itself need its own soundness argument for
  every combinator (sequential composition, the higher-order words' internal
  loop, `RANGE`'s value-driven count)? That is a real design surface with
  more than one defensible answer, and Phase 5 is not the place original
  judgment calls about it get made silently as a side effect of shipping a
  feature.
- **A coarse class already answers "will this blow the budget", which is the
  question `#:contract cost` exists to answer.** The `mcp.limits` ceilings
  a host declares are *fixed numbers* (`numericWork: 10,000,000`, etc.), not
  functions of input size the caller supplies at declaration time — so
  "this word is `Unbounded` in `numeric`" is already actionable (an agent
  composing calls knows not to feed it something whose size it cannot
  bound), while a fitted polynomial's extra precision buys nothing over the
  class answer unless a caller is doing algebra with the coefficients, which
  no current tool does.

## 5. What "machine-independent" means for this axis, precisely

`runtime_limits.rs`'s own module comment is explicit that `ALGEBRAIC_PAIR_UNITS`
(and by extension every unit this axis prices in) is **empirical**: measured
on one reference container, rounded to a power of two, right only to within
an order of magnitude. **The unit count a program is charged is a property of
the program** — the work meter charges before an operation runs, from static
facts about its operands (bit width, lane count, term count), not from a
clock — but **the wall-clock time one unit costs is a property of the
machine**, and the two must not be conflated. `#:contract cost steps=linear`
is a true, portable claim: *this word's charged unit count grows linearly
with its input*. "This word runs in linear wall-clock time" is a different,
weaker claim this axis does not make and must not be read as making — two
machines charged the identical unit count can still finish at different
wall-clock times, exactly as `ALGEBRAIC_PAIR_UNITS`'s own calibration record
already documents for the meter it feeds.

## 6. Operand-literal refinement: why it is *not* optional

`word_space.rs`'s `SpaceSim` tracks per-slot provenance (is an operand a
compile-time literal, and if so how large) so that `[ 0 10 ] RANGE` collapses
from `Unbounded` to `Const` — a real operand a materializer's class is a
function of. `CostSim` initially shipped **without** that refinement, on the
argument that omitting it was a safe simplification because "dropping a
refinement only ever makes a bound *looser*, which can only turn a
would-be-`Error` into a `Note` — never the reverse."

**That argument was wrong, and the omission was unsound in both directions.**
It is recorded here rather than deleted, because the failure mode is the
non-obvious part:

- Dropping the refinement loosens the *class* but leaves the `exact` witness
  attached to it. "Looser class + still exact" is precisely the combination
  that licenses a declaration `Error`. So `{ 1 2 ADD }` — a word that consumes
  nothing and charges a fixed constant (measured: `numericWork` 1) — reported
  a hard `error` against the *true* declaration `cost numeric=const`. A false
  error, the one outcome this model must never produce.
- In the other direction, the same missing refinement is what made it look
  acceptable to classify the value-driven materializers as `Linear`. They are
  not: `[ 0 10 ] RANGE` charges 187 and `[ 0 20000 ] RANGE` charges 340,017
  against the *same* 2-element operand, and `[ 300 300 0 ] FILL` charges
  1,530,000. Declaring `cost collection=linear` on `{ RANGE }` was reported as
  **verified**.

The two are one bug with one fix, and they had to be fixed together: correcting
`RANGE`/`FILL` to `Unbounded` *without* the refinement would have turned the
true declaration `cost collection=const` on a literal-driven range into a new
false error.

`CostSim` therefore refines exactly as `SpaceSim` does. It does **not** keep a
second slot stack: `SpaceSim::feed_word` returns an `OperandProfile` for each
dependency call and `CostSim` refines against that same reading. One slot
model, two bounds — the two walks cannot drift apart, and the provenance is
paid for once.

## 7. Summary of what ships

- `CostClass` / `CostBound`: the lattice and join, mirroring `SpaceClass` /
  `SpaceBound` (`word_cost.rs`).
- `builtin_cost(WordId) -> CostBound`: a per-builtin classification table
  covering all three axes, commented per word (`word_cost.rs`). Classified
  from direct evidence where it exists (the meter's own charging call sites,
  or `ajisai run --json`'s `resourceUsage` on a representative program) and
  from a plausible, never-looser upper bound with `exact = false` elsewhere —
  never a guess that could later prove too tight.
  The value-driven materializers (`RANGE`/`FILL`) are `Unbounded` on the
  collection axis for the same reason `word_space.rs` gives them `Unbounded`
  for space: their charge is set by an operand's *value*, not its size.
- `CostSim`: the single-pass token-stream accumulator, fed from the same
  `word_contract.rs` inference walk `SpaceSim` already rides along with — no
  second pass over a word's body — refining each dependency's classes against
  the `OperandProfile` that walk already computed (§6).
- `WordContract::cost: CostBound`, propagated the same way `space` is.
- `#:contract ... cost steps=<class> numeric=<class> collection=<class>`
  parsing (any subset of the three axes; an unknown axis or class name is a
  parse error, matching the existing `unknown term` behavior) and checking,
  split into `contract_cost.rs` alongside `contract_decl.rs` to keep the
  latter within the §14.1 file-size budget.

## Appendix A — SHA-256 → BLAKE3 in the MCP adapter: evaluated, not adopted

Moved verbatim from the cost-discoverability work order (2026-08, removed
from `docs/dev/` on 2026-09-30) so the decision and its reconsideration
condition stay findable. It is here because the question arrived alongside
the cost-discoverability work and concerns the same digest machinery that
`#:contract`'s auditability claims rest on.

### A.1 前提の訂正

> Ajisai に採用されているハッシュ技術は SHA-256 だと思いますが

実測の結果、**この前提は成り立たない**。

- **Rust コア（言語処理系・エージェント境界）は既に全面 BLAKE3。**
  `rust/Cargo.toml` は `blake3 = { version = "1", ... }` を宣言し、
  `Cargo.lock` の解決済みバージョンは **1.8.5**。
  語の同一性（`word_identity.rs`）も観測ダイジェスト（`observation_digest.rs`）も BLAKE3 である。
- **Rust 側に SHA-256 は1箇所も無い**（`sha2` クレートへの依存も無い）。
- SHA-256 が使われているのは **Node 側の `tools/mcp-server/` 内の4箇所のみ**。

### A.2 その4箇所の実態

| 箇所 | 用途 | 突き合わせ相手 |
| --- | --- | --- |
| `capture-traces.js:54` | トレース記録の短縮ID（先頭16桁） | 無し（同一プロセス内の同定のみ） |
| `capture-repairs.js:36` | 同上 | 無し |
| `sync-assets.js:19` | 同梱レジストリの `registryDigest` を**書く** | `index.js` |
| `index.js:300` | 同梱レジストリの digest を**照合する** | `sync-assets.js` |

後半2つは「JS が書いて JS が検証する」自己完結した破損検知であり、
**Rust 側に対応する digest は存在しない**（`registryDigest` 相当の実装は Rust に0件）。
前半2つは短縮IDで、暗号学的性質を要求していない。

### A.3 置換しない理由

1. **シナジーが発生する接点が存在しない。** シナジーが生まれるのは
   「Rust が作った digest を JS が独立に再計算して検証する」場合だが、その要件は現在無い。
   4箇所はいずれも言語をまたがない。
2. **依存が増える。** Node には BLAKE3 が組み込まれていない。現在の4箇所は
   標準の `crypto.createHash` で**依存ゼロ**である。`ajisai-mcp-server` は npm に公開する
   パッケージで、実行時依存は現在2つしかない。内部識別子のために
   サプライチェーン依存（多くは wasm もしくはネイティブビルドを伴う）を足すのは割に合わない。
3. **`1.0.0` への固定は後退になる。** 現行の解決版は 1.8.5 であり、
   1.0.0 を指定すると Rust 側が8マイナーバージョン分の後退となる。
   BLAKE3 のバージョンを動かす積極的な理由は見当たらない。

### A.4 再検討の条件（この条件が満たされたら A.3 は無効になる）

**JS が Rust の作った digest を独立に検証する要件が生まれたとき。**
例えば観測ダイジェストを MCP アダプタ側で再計算して突き合わせたい、という要求である。

ただしその場合でも、ハッシュ関数の選択は問題の**小さい方**であることに注意する。
観測ダイジェストは `AJISAI-OBS-1` のバイト文法（値の符号化・代数的数の包囲キー・
辞書の正規化順序）の上に載っており、JS で再現すべき本体はそちらである。
ハッシュを揃えるのは、その文法を移植し終えた後の最後の1行に過ぎない。
「まずハッシュを統一する」順序で着手すると、労力の大半を占める部分が手つかずのまま残る。
