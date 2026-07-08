# Benchmark results — 2026-07-08 — Claude tier sweep (Haiku/Sonnet/Opus) — json-parser, condition B (with SKILL.md)

- Date (UTC): 2026-07-08
- Model / version: Claude Haiku 4.5 (T1), Claude Sonnet 4.5 (T2), Claude Opus 4.8 (T3)
- Model capability tier: T1 / T2 / T3 (protocol §1.1) — one row per tier below
- Condition: **B = with SKILL.md** (SPECIFICATION.html + task .md + repo-root SKILL.md)
- ajisai commit: d03da63
- TRANSITION_METRICS_VERSION: 1 (design memo §6)
- Notes:
  - Direct A/B pair with `2026-07-08-claude-tier-sweep-json-parser-A.md`. The
    **only** difference from that condition-A run is the presence of
    `SKILL.md`: same task text, same tiers, same rules, same fresh-context
    independent-session setup, same barred files (grading harness, `examples/`,
    Reference). This isolates SKILL.md as the single manipulated variable
    (protocol §1).
  - `passed` / `casesPassing` re-verified by the recorder via `verify.sh`.
    **Effort** (tool-uses / combined tokens / duration) is harness-measured.
    Combined tokens in condition B include the one-time ~33 KB SKILL.md read
    (~8–9k tokens), so **tool-uses and duration are the cleaner effort proxies**
    for the A/B delta; the token column is muddied by that fixed read cost.
  - Tiers are relative within the Claude family (memo §1.1), spanning the
    4.5/4.8 generation boundary (same caveat as prior sweeps).

## Results (condition B)

| tier | task | passed | casesPassing | onePass | finalLines | effort (toolUses / tokens / sec) | contractCov | selfResolved | errorQuality(1-5) | energyProxyScore |
|---|---|---|---|---|---|---|---|---|---|---|
| T1 (Haiku) | json-parser | ✅ | 13/13 | ✅ (self-report; 20 toolUses) | 5 | **20 / 63.0k / 146** | 13/13 | ✅ | n/a (no errors) | n/a |
| T2 (Sonnet) | json-parser | ✅ | 13/13 | ✅ | 5 | **9 / 53.1k / 72** | 13/13 | ✅ | n/a (no errors) | n/a |
| T3 (Opus) | json-parser | ✅ | 13/13 | ✅ | 5 | **6 / 44.6k / 54** | 12/12 | ✅ | n/a (no errors) | n/a |

Both Haiku and Sonnet independently added a redundant `STR` after `JSON@HAS`
in condition B (raising their contractCov to 13/13); `JSON@HAS` already
returns the string `'TRUE'`/`'FALSE'`, so the `STR` is harmless and all cases
still pass. In condition A, Sonnet used bare `JSON@HAS`. Minor artifact: the
guide nudged toward an explicit boolean→string conversion. Solutions off-tree.

## A/B comparison (the point of this run)

Harness-measured effort, condition A → condition B:

| tier | toolUses A→B | Δ toolUses | duration A→B (s) | tokens A→B |
|---|---|---|---|---|
| T1 Haiku | 34 → **20** | −14 (−41%) | 198 → 146 | 75.3k → **63.0k** (down) |
| T2 Sonnet | 23 → **9** | −14 (−61%) | 99 → 72 | 47.9k → 53.1k (up) |
| T3 Opus | 6 → **6** | 0 (floor) | 71 → 54 | 35.7k → 44.6k (up) |

Across-tier tool-use **spread halved**: condition A ranged 34→6 (span 28);
condition B ranges 20→6 (span 14).

## Observations

- **First direct H1 evidence in the whole thread.** The design memo's core
  hypothesis (H1, memo §1) is: reduce the *ambiguity denominator* and a given
  tier needs less "work." Condition B does exactly that — SKILL.md documents
  the `JSON@` operand order that condition-A agents had to discover by probing
  — and the weaker tiers' effort dropped sharply (Haiku −41%, Sonnet −61%
  tool-uses) while the frontier tier, already at the effort floor, was
  unchanged. The across-tier effort **gradient compressed** (span halved). The
  binary metric stays saturated (13/13 in both conditions), so this is,
  consistent with every prior sweep, a **continuous-metric** effect — but this
  time it is the *intervention* moving it in the predicted direction, not just
  a difficulty gradient. This is the cleanest support for H1 so far.

- **The SKILL.md read cost pays off only for the tier that was thrashing.**
  Combined tokens went *down* for Haiku (75.3k → 63.0k) but *up* for Sonnet and
  Opus (they paid the ~8–9k fixed SKILL.md read without enough dynamic probing
  to recoup it). So the guide is a net token win exactly where the ambiguity
  was biting (the small tier) and a net token cost where it was not. This is a
  concrete, honest cost/benefit picture for the "AI-first guide" investment:
  its value is tier-dependent and concentrated at the weak end.

- **On the soliton / grokking framing — this run leans against a
  non-dispersing reading, with one honest caveat.** The question posed was
  whether reducing ambiguity makes the effort signal *shrink* (dissipate) or
  *relocate intact* (soliton-like). For Haiku the signal genuinely **shrank**:
  fewer tool-uses, less wall-clock, *and* fewer total tokens — less work
  overall, not the same work moved elsewhere. That is dissipation, and it is
  what H1 predicts; it is not the conserved, non-dispersing behavior a soliton
  analogy would want. **Caveat (the partial relocation):** for the *strong*
  tiers the information cost did not vanish, it **moved** — from dynamic
  probing (condition A) to a fixed upfront read (condition B) — leaving their
  total token work flat-to-higher. So "cost conserved but relocated" holds for
  tiers already above the ambiguity floor, while "cost dissipated" holds for
  the tier below it. Neither is a phase transition (the binary never moved).
  Held as metaphor, not mechanism (memo §5 discipline): a genuine
  grokking/soliton test needs a metric that is *conserved* across the
  manipulation, and effort here is not conserved for the tier that mattered.

- **Cross-condition arc (four sweeps).** exact-rational (both metrics floor) →
  energy-refactor A− (binary floors, continuous = judgment artifact) →
  json-parser A (binary floors, continuous = effort gradient) → **json-parser B
  (same binary floor, the effort gradient *compresses under the SKILL.md
  intervention*)**. The through-line: the binary output has been saturated the
  entire time, and every bit of real signal — difficulty and now
  *intervention* — lives in the continuous effort metrics. To move the binary
  (a real transition), the next probe still needs a task a small tier cannot
  complete at all; but for H1 specifically, this A/B pair already shows the
  mechanism the hypothesis predicts, on the continuous axis.
