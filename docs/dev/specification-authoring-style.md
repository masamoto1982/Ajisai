# Ajisai Specification Authoring Style

「仕様書の仕様」 — the authoring convention for `SPECIFICATION.md`.

## Authority

- **Non-canonical.** This document governs **how** `SPECIFICATION.md` is written, not **what** it says. It defines no language semantics.
- **Canonical source remains `SPECIFICATION.md`.** If this document ever appears to constrain meaning, the specification wins.
- Sibling conventions: `docs/dev/reference-writing-style.md` (the Reference site and `?`/LOOKUP text) and `docs/dev/three-layer-documentation-model.md` (user-facing guidance structure). This document is the same kind of rule, applied one layer up — to the specification itself.

## 1. Why the specification needs a style

Ajisai's surface syntax is unusually dense in ASCII punctuation, and **almost every punctuation mark is a word**. The tokenizer maps symbol forms to canonical English names; the master map is the surface-syntax table in Section 3, and the modifiers are defined in Sections 6.1–6.2.

The marks most easily mistaken for ordinary punctuation are the modifiers:

| Symbol | Canonical word | Kind | Role |
|--------|----------------|------|------|
| `.` | `TOP` | Target modifier | The word operates on the top value(s) of the stack (default) |
| `..` | `STAK` | Target modifier | The entire stack is treated as the operand |
| `,` | `EAT` | Consumption modifier | Operands are consumed after the operation (default) |
| `,,` | `KEEP` | Consumption modifier | Operands are retained; the result is also pushed |

So `.` is the sugar for the word that selects the **operation-target mode**, and `,` is the sugar for the word that selects **operand consumption**. Both look exactly like English punctuation. Other word-aliases (`+` `-` `*` `/` `%` `=` `<` `>` `&` `==` `=>` `?` `!` …, Section 3) carry the same risk to a smaller degree.

The hazard this creates: when prose uses one of these marks as **ordinary punctuation**, a reader cannot always tell whether they are looking at English punctuation or at Ajisai code. The comma is the sharpest case — `,` is the sugar for `EAT`, so a casual comma-separated list written *near* Ajisai code invites exactly the misreading "is that comma part of the program?".

## 2. The primary technique: render every Ajisai symbol as code

The specification already does the right thing in its prose: whenever it names a symbol that is meaningful to Ajisai, it wraps that symbol in inline code, which renders with a **light gray background**. That background is the signal — it tells the reader "this mark is a word, not punctuation." It is an excellent, low-cost device and it is **mandatory**:

> Every token meaningful to Ajisai — every word, symbol, modifier, literal, or snippet — is wrapped in `` `…` `` (inline) or a fenced block when it appears in prose. A bare `,` or `[ 1 2 3 ]` never sits in running text.

This single rule resolves most ambiguity on its own: a gray-backgrounded `,` is unmistakably the `EAT` word being discussed, while an un-backgrounded comma is plain English punctuation. Tables (Section 3 below) are the second, structural line of defense for the cases where even marked-up tokens accumulate into a hard-to-read list.

## 3. Tables for enumerable structure

A table boundary is structural, not textual: nothing inside a cell has to be re-parsed as a separator. This is why the Reference site leans on tables, and the same reasoning applies to the specification.

Reach for a table when the content is a set of rows sharing the same shape:

| Content shape | Columns |
|---------------|---------|
| Word and its sugar and role | `Canonical` then `Sugar` then `Role` |
| Property across an axis | `Subject` then `Value` then `Notes` |
| Category membership | `Category` then `Members` |
| State transition | `From` then `Event` then `To` |
| Mapping / correspondence | `Source` then `Target` |
| Worked example | `Sample code` then `Expected value` then `Notes` |

Keep paragraphs for: the definition of a single concept, the rationale behind a rule, and any reasoning that does not decompose into uniform rows. Do not flatten an argument into a table merely to avoid commas — Section 2 (mark the tokens as code) already covers prose; tabularize only genuinely enumerable structure.

## 4. Rules

1. **Mark every Ajisai-meaningful token as code** so it carries the gray background (Section 2). This is the non-negotiable baseline.
2. **Never use a bare Ajisai token as prose punctuation.** A symbol that is a word (`.` `..` `,` `,,` `+` `-` `*` `/` `%` `=` `<` `>` `<=` `>=` `<>` `&` `==` `=>` `?` `!`) must appear only as marked-up code, never as the separator, bullet, or delimiter of running text.
3. **Promote an inline list of three or more code tokens to a table.** A sentence that enumerates several words or symbols is a table in disguise.
4. **One concept axis per column.** When the enumeration carries structure — a word and its sugar, a property and its values, a state and its transition — give each axis its own column.
5. **Do not encode results with inline comment arrows.** `# → [ 1 ]` style annotations blur code and result. Put the input in one column or block and the expected value in another. (Mirrors `reference-writing-style.md`.)
6. **A table conversion preserves meaning exactly.** It is a presentation change, never a semantic one.

## 5. Worked example

The hazard, then the fix.

**Before** — an inline list whose separators are themselves Ajisai words:

> The logic words are `AND`, `OR`, and `NOT`; the control-flow words are `COND`, `EXEC`, `MAP`, `FILTER`, `FOLD`, `UNFOLD`, `ANY`, `ALL`, `COUNT`, and `SCAN`.

Every separating comma there is a `,`, which is the sugar for `EAT`. The reader must rely on the backgrounds of the words alone to know the commas between them are English.

**After** — the same content as structure:

| Category | Members |
|----------|---------|
| Logic | `AND` and `OR` and `NOT` |
| Control flow | `COND` `EXEC` `MAP` `FILTER` `FOLD` `UNFOLD` `ANY` `ALL` `COUNT` `SCAN` |

The cell boundaries carry the separation, and no comma is left adrift between two words.

## 6. In-cell separators

When a single cell must hold more than one token, separate them with something that is **not** Ajisai surface syntax:

- spelled-out **"and"** / **"or"** (`AND` and `OR`) — preferred, because it always renders; or
- a middle dot `·`, which is not an Ajisai token.

Never separate in-cell tokens with `,`, `/`, or `|` — the first two are words, and `|` collides with Markdown table syntax. Where the tokens are simply a set, a single space between code spans (`` `MAP` `FILTER` ``) also reads cleanly.

## 7. Migration is incremental

`SPECIFICATION.md` already uses tables heavily (the metadata firewall, the surface-syntax map, the contract registry). The gap is the remaining **inline enumerations** scattered through the prose. They are converted opportunistically, not in one sweep:

- When editing a section for any reason, promote its inline token lists to tables.
- Do not renumber sections or restructure headings solely to insert a table.
- Preserve meaning exactly; the conversion is presentation only.

Representative candidates (illustrative, not exhaustive): the Core-word category lists, the NIL-passthrough word enumerations in the Bubble Rule section, and any sentence that names four or more words in a row.

## 8. Relationship to the other style documents

| Document | Layer it governs |
|----------|------------------|
| This document | `SPECIFICATION.md` (the canonical spec) |
| `reference-writing-style.md` | Reference site and `?`/LOOKUP help text |
| `three-layer-documentation-model.md` | Structure of all user-facing guidance |

All three share one root principle: **Ajisai code and the prose about it must be visually and structurally distinct, so that a symbol is never mistaken for punctuation.** The gray-background code span marks each token; tables separate them in bulk. Each layer uses both deliberately.
