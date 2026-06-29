# Ajisai Authoring & Notation Style

The house style for **all authoritative writing about Ajisai** — the specification, the Reference, and any prose that refers to Ajisai or to the mathematics behind it. (Working name; this is the document previously sketched as 「仕様書の仕様」, broadened beyond the specification.)

## Authority

- **Non-canonical.** This document governs **how** Ajisai is written about, not **what** is true of it. It defines no language semantics.
- **Canonical source remains `SPECIFICATION.html`.** If this document ever appears to constrain meaning, the specification wins.
- **The authority model itself is specified.** Section 2 of the specification (Specification Authority) defines the responsibilities of, and the order of trust among, the mathematical formalization, the specification, the Reference, and this authoring discipline; this document supplies the notation rules that model requires.
- **Scope:** the specification, the README (`README.md`), the Reference (`public/docs/`), and any other authoritative text that names Ajisai words, symbols, or formulas.
- Sibling conventions: `docs/dev/reference-writing-style.md` (the Reference site and `?`/LOOKUP text) and `docs/dev/three-layer-documentation-model.md` (user-facing guidance structure). This document is the shared notation discipline both of those, and the specification, adhere to.

## 1. Why this style exists

The essence of Ajisai is not source code in any one language. It is the **written-down mathematics, and the specification that contains it**: implement faithfully from the specification and the same programming experience follows. The notation is therefore part of the artifact, not decoration around it. If a formula or a word is presented inconsistently — or, worse, ambiguously — the specification stops being a reliable blueprint.

Two consistency problems dominate, and the rest of this document addresses them:

1. **Ajisai symbols look like punctuation.** Ajisai source is UTF-8 text, and every symbol token that Ajisai defines is a word.
2. **Mathematics and Ajisai code share glyphs.** `/`, `+`, `(`, `)`, `>=` mean one thing in a formula and another as Ajisai sugar.

## 2. The symbol-as-word hazard

The tokenizer maps symbol forms to canonical English names; the master map is the surface-syntax table in Section 3 of the specification, and the modifiers are defined in Sections 6.1–6.2. The marks most easily mistaken for ordinary punctuation are the modifiers:

| Symbol | Canonical word | Kind | Role |
|--------|----------------|------|------|
| `.` | `TOP` | Target modifier | The word operates on the top value(s) of the stack (default) |
| `..` | `STAK` | Target modifier | The entire stack is treated as the operand |
| `,` | `EAT` | Consumption modifier | Operands are consumed after the operation (default) |
| `,,` | `KEEP` | Consumption modifier | Operands are retained; the result is also pushed |

So `.` is the sugar for the word that selects the **operation-target mode**, and `,` is the sugar for the word that selects **operand consumption**. Both look exactly like English punctuation. Other word-aliases (`+` `-` `*` `/` `%` `=` `<` `>` `&` `~` `^` `?` `!`) carry the same risk to a smaller degree. A casual comma-separated list written near Ajisai code invites the misreading "is that `,` part of the program?".

## 3. Primary technique: the gray code background, reserved for Ajisai

The existing writing already does the right thing: whenever it names a symbol meaningful to Ajisai, it wraps that symbol in inline code, which renders with a **light gray background**. That background is the signal — it says "this mark is a word, not punctuation." It is an excellent, low-cost device, and it is **mandatory**:

> Every token meaningful to Ajisai — every word, symbol, modifier, literal, or snippet — is wrapped in `` `…` `` (inline) or a fenced block when it appears in prose. A bare `,` or `[ 1 2 3 ]` never sits in running text.

Crucially, the gray background is **reserved for Ajisai**. Mathematics does not borrow it (Section 4). Keeping the channel exclusive is what makes a gray `/` unambiguously the `DIV` word rather than a division bar.

## 4. Mathematics is a separate channel

Because a formula and an Ajisai snippet share glyphs (`/` is both a division bar and `DIV`; `(` `)` are grouping and the `RESERVED-BEGIN`/`RESERVED-END` markers; `>=` is both ≥ and `GTE`), the two must travel in **visibly different channels**, and the channel decides the reading:

| Channel | How it is set | A `/` in it means |
|---------|---------------|-------------------|
| Ajisai code | inline `` `…` `` (gray) or a fenced Ajisai block | the `DIV` word |
| Mathematics | LaTeX typeset by KaTeX on the HTML surfaces (Section 4.1); Unicode text math on Markdown surfaces (Section 4.2) | the division operator |

Rules for the mathematics channel:

1. **Do not put mathematics in the gray Ajisai code span.** That background belongs to Ajisai tokens; sharing it destroys the signal of Section 3.
2. **Use the delimiters of the surface.** The HTML surfaces typeset LaTeX through KaTeX using backslash delimiters — `\(…\)` inline, `\[…\]` display (Section 4.1); `\` is not an Ajisai token, so those delimiters collide with nothing. Markdown surfaces may use the `$…$` / `$$…$$` delimiters their renderers support (Section 4.2): since the `COND` clause separator moved from `$` to `|`, `$` is no longer an Ajisai token and the historical collision is gone. (`|` **is** now an Ajisai token — see Sections 6 and 8.)
3. **Set display formulas off from prose** in their own block, the way the specification writes the continued-fraction forms.
4. **A glyph shared by both channels is disambiguated by channel alone.** When the same expression is shown both as mathematics and as the Ajisai that realizes it, present them as two distinct things — for example a math display beside an Ajisai snippet, or two columns — never blended into one run of text.

The goal is that a reader (human or machine) can always tell, from presentation alone, whether a `/` is the operator or the word.

### 4.1 LaTeX via KaTeX on the HTML surfaces

The specification (`SPECIFICATION.html`) and the Reference (`public/docs/`) typeset their mathematics channel as **LaTeX rendered by KaTeX**. The essence of Ajisai is the written-down mathematics (Section 1); presenting continued fractions, intervals, and partial-quotient formulas as flat Unicode text undersells exactly the part of the artifact that matters most. Rendered LaTeX also *strengthens* the channel separation of this section: a typeset formula is unmistakably not a gray Ajisai code span and not prose. Use it actively — whenever a formula, a variable, an interval, or a set would otherwise be hand-built from Unicode glyphs and `<em>` italics, write it as LaTeX instead.

| Aspect | Rule |
|--------|------|
| Delimiters | inline `\(…\)` · display `\[…\]` — nothing else |
| Dollar delimiters | not configured on the HTML surfaces; one delimiter pair per surface keeps scanning deterministic |
| Library | KaTeX with its auto-render extension, **self-hosted** at `public/vendor/katex/` — no CDN, matching the repository's no-external-services stance |
| Refreshing the vendored copy | `npm run vendor:katex` (copies from the `katex` devDependency; woff2 fonts only) |
| Ajisai channel exclusion | auto-render is configured with `ignoredTags` covering `code` and `pre`, so the gray Ajisai channel is structurally unreachable by math rendering — the channel separation is enforced by tooling, not just by discipline |
| Continued fractions | use `\cfrac` for the nested display forms; the NICF formula in specification Section 4.2.5 is the model |
| Variables and names | single letters italic by default (`\(a_0\)`, `\(b_i\)`, `\(\varepsilon_i\)`); multi-letter names as `\mathit{…}` (`\(\mathit{num}/\mathit{den}\)`) |
| No-JS degradation | without JavaScript the raw `\(…\)` LaTeX source remains visible; this is acceptable — LaTeX source is itself a precise, AI-readable notation |

The LaTeX source inside the HTML is part of the artifact: it must state exactly the mathematics intended, because machine readers consume the source, not the rendering.

### 4.2 Markdown surfaces use GitHub math

The README and the `docs/dev/` working documents are Markdown and get no KaTeX runtime, but GitHub renders `$…$` (inline) and `$$…$$` (display) LaTeX natively. Use it: with the `COND` clause separator moved from `$` to `|`, the dollar sign is no longer an Ajisai token and the historical collision is gone. Two cautions remain:

- **Code spans win.** An Ajisai snippet containing `|` (or any other token) is always inside `` `…` ``, where Markdown math does not reach — the channel rule of Section 3 already guarantees this.
- **Unicode text math is still acceptable** for a stray `≤` or `√` in running prose where a full math span would be noise. Heavy mathematics belongs on the HTML surfaces anyway (three-layer model); if a Markdown document accumulates formulas, that is a sign the content belongs in the specification or the Reference.

## 5. Tables for enumerable structure

A table boundary is structural, not textual: nothing inside a cell has to be re-parsed as a separator. Reach for a table when the content is a set of rows sharing one shape:

| Content shape | Columns |
|---------------|---------|
| Word and its sugar and role | `Canonical` then `Sugar` then `Role` |
| Property across an axis | `Subject` then `Value` then `Notes` |
| Category membership | `Category` then `Members` |
| State transition | `From` then `Event` then `To` |
| Mapping / correspondence | `Source` then `Target` |
| Worked example | `Sample code` then `Expected value` then `Notes` |

Keep paragraphs for the definition of a single concept, the rationale behind a rule, and any reasoning that does not decompose into uniform rows.

## 6. Rules

1. **Mark every Ajisai-meaningful token as code** so it carries the gray background (Section 3). Non-negotiable baseline.
2. **Keep mathematics in its own channel** (Section 4); never give it the gray Ajisai background. On the HTML surfaces, typeset it as LaTeX via KaTeX with `\(…\)` / `\[…\]` (Section 4.1); on Markdown surfaces, GitHub's `$…$` math is available (Section 4.2).
3. **Never use a bare Ajisai token as prose punctuation.** A symbol that is a word or sugar (`.` `..` `,` `,,` `+` `-` `*` `/` `%` `=` `<` `>` `<=` `>=` `<>` `&` `~` `^` `?` `!` `|`) appears only as marked-up code, never as the separator, bullet, or delimiter of running text.
4. **Promote an inline list of three or more code tokens to a table.**
5. **One concept axis per column.**
6. **Do not encode results with inline comment arrows.** `# → [ 1 ]` blurs code and result; use separate columns or blocks.
7. **A table or notation change preserves meaning exactly.** It is presentation only, never semantic.

## 7. Worked example

The hazard, then the fix.

**Before** — an inline list whose separators are themselves Ajisai words:

> The logic words are `AND`, `OR`, and `NOT`; the control-flow words are `COND`, `EXEC`, `MAP`, `FILTER`, `FOLD`, `UNFOLD`, `ANY`, `ALL`, `COUNT`, and `SCAN`.

Every separating comma there is a `,`, the sugar for `EAT`. The reader leans on the gray backgrounds of the words alone to know the commas between them are English.

**After** — the same content as structure:

| Category | Members |
|----------|---------|
| Logic | `AND` and `OR` and `NOT` |
| Control flow | `COND` `EXEC` `MAP` `FILTER` `FOLD` `UNFOLD` `ANY` `ALL` `COUNT` `SCAN` |

The cell boundaries carry the separation, and no comma is left adrift between two words.

## 8. In-cell separators

When a single cell must hold more than one token, separate them with something that is **not** Ajisai surface syntax:

- spelled-out **"and"** / **"or"** (`AND` and `OR`) — preferred, because it always renders; or
- a middle dot `·`, which is not an Ajisai token; or
- a single space between adjacent code spans (`` `MAP` `FILTER` ``).

Never separate in-cell tokens with `,`, `/`, or `|` — all three are Ajisai words or sugar (`|` is the `COND` clause separator), and `|` additionally collides with Markdown table syntax.

## 9. Surfaces and required formats: the specification is HTML, the README is Markdown

**The specification is mandatorily authored in HTML, not Markdown.** The canonical specification is `SPECIFICATION.html`; authoring it as Markdown is a style violation. The reason for the HTML requirement is **not** visual decoration: HTML makes the structural tools this style depends on — tables for enumerable structure (Section 5), and embedded diagram sources such as PlantUML — directly usable, where Markdown keeps them second-class. The pages stay plain and unadorned, in the same sober styling the Reference uses. GitHub's repository view shows HTML files as source, so the **reading surface for HTML documents is the Pages site** (the build copies `SPECIFICATION.html` into the deployed site next to the Playground).

**The README is the one mandatory Markdown surface.** `README.md` is the repository's front door, and GitHub renders only Markdown there — an HTML README would greet visitors with source code. The README therefore stays GitHub-flavored Markdown and acts as the entry point that links to the rendered documents: the specification (`https://masamoto1982.github.io/Ajisai/SPECIFICATION.html`), the Reference, and the Playground.

Each surface applies this style with its own tooling:

- **Specification** (`SPECIFICATION.html`) — hand-authored HTML. The gray background is supplied by the page's `code` styling; enumerable structure lives in `ref-table` tables; mathematics is LaTeX rendered by the self-hosted KaTeX (Section 4.1). When editing a section for any reason, promote its inline token lists to tables and its Unicode text math to LaTeX; do not renumber sections or restructure headings solely to insert a table.
- **README** (`README.md`) — GitHub-flavored Markdown. Inline code gives the gray background; tables are native. Worked examples live in sample tables (sample code, expected value, notes), never as code comments with inline result arrows. Tech-stack badges are homemade, uniform SVGs kept in the repository (`docs/assets/badges/`) — no borrowed third-party badge images.
- **Reference** (`public/docs/`) — hand-authored HTML. The gray background is supplied by the page's `code` styling, and examples already live in tables (sample code, expected value, notes); mathematics is LaTeX rendered by the self-hosted KaTeX (Section 4.1). New pages follow the same channels.

Representative specification candidates for table promotion (illustrative, not exhaustive): the Core-word category lists, the NIL-passthrough word enumerations in the Bubble Rule section, and any sentence that names four or more words in a row.

Legacy references to `SPECIFICATION.md` in older commits and archived notes are obsolete; they denote the same document, now `SPECIFICATION.html`. A short-lived `README.html` existed during the 2026-06 migration and is likewise obsolete; the README is `README.md`.

## 10. Relationship to the other style documents

| Document | Layer it governs |
|----------|------------------|
| This document | The shared notation discipline for all writing about Ajisai |
| `reference-writing-style.md` | Reference site and `?`/LOOKUP help text |
| `three-layer-documentation-model.md` | Structure of all user-facing guidance |

All of them share one root principle: **Ajisai code, the mathematics behind it, and the prose about both must be visually and structurally distinct, so that a symbol is never mistaken for punctuation and an operator is never mistaken for a word.** The gray code span marks each Ajisai token; a separate math channel carries the formulas; tables separate them in bulk.
