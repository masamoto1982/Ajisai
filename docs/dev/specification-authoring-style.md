# Ajisai Specification Authoring Style

「仕様書の仕様」 — the authoring convention for `SPECIFICATION.md`.

## Authority

- **Non-canonical.** This document governs **how** `SPECIFICATION.md` is written, not **what** it says. It defines no language semantics.
- **Canonical source remains `SPECIFICATION.md`.** If this document ever appears to constrain meaning, the specification wins.
- Sibling conventions: `docs/dev/reference-writing-style.md` (the Reference site and `?`/LOOKUP text) and `docs/dev/three-layer-documentation-model.md` (user-facing guidance structure). This document is the same kind of rule, applied one layer up — to the specification itself.

## 1. Why the specification needs a style

Ajisai's surface syntax is unusually dense in ASCII punctuation, and **almost every punctuation mark is a word**. The tokenizer maps symbol forms to canonical English names (Section 6.5):

| Symbol | Canonical word | Role |
|--------|----------------|------|
| `,` | `EAT` | Stack modifier: operands are consumed after the operation (the default) |
| `,,` | `KEEP` | Stack modifier: operands are retained |
| `.` | `TOP` | Stack modifier: act on the top element only |
| `..` | `STAK` | Stack modifier: act on the whole stack |
| `=>` | `OR-NIL` | NIL coalescing |
| `==` | `PIPE` | Visual pipeline marker |

The hazard this creates: when prose uses one of these marks as **ordinary punctuation**, a reader cannot always tell whether they are looking at English punctuation or at Ajisai code. The comma is the sharpest case — `,` is the sugar for `EAT`, so a casual comma-separated list written *near* Ajisai code invites exactly the misreading "is that comma part of the program?".

Tables remove the ambiguity. A cell boundary is structural, not textual: nothing inside a cell has to be re-parsed as a separator. This is why the Reference site leans on tables, and the same reasoning applies to the specification.

## 2. Rules

1. **Never use a bare Ajisai token as prose punctuation.** A symbol that is a word (`,` `,,` `.` `..` `+` `-` `*` `/` `%` `=` `<` `>` `<=` `>=` `<>` `&` `==` `=>` `?` `!` `~`) must appear only as marked-up code (inline code or a fenced block), never as the separator, bullet, or delimiter of running text.

2. **Prefer a table to an inline enumeration of three or more code tokens.** A sentence that lists several words, symbols, or literals separated by commas is a table in disguise. Promote it.

3. **One concept axis per column.** When the enumeration carries structure — a word and its sugar, a property and its values, a state and its transition — give each axis its own column rather than packing it into one cell with internal punctuation.

4. **Keep Ajisai code inside code spans.** Every word, symbol, literal, or snippet is wrapped in `` `…` `` (inline) or a fenced block. Bare `[ 1 2 3 ]` never appears in the running text of the specification. (This mirrors `reference-writing-style.md` rule 5.)

5. **Do not encode results with inline comment arrows.** `# → [ 1 ]` style annotations blur code and result. Put the input in one column or block and the expected value in another. (Mirrors `reference-writing-style.md` rule 6.)

6. **Tables are for enumerable structure, not for prose.** Definitions, rationale, and multi-sentence reasoning stay as paragraphs. Do not flatten an argument into a table merely to avoid commas; rule 1 is satisfied by marking up the tokens, not by tabularizing the explanation.

## 3. When a table is the right tool

Reach for a table when the content is a set of rows sharing the same shape:

| Content shape | Columns |
|---------------|---------|
| Word ↔ sugar ↔ role | `Canonical` · `Sugar` · `Role` |
| Property across an axis | `Subject` · `Value` · `Notes` |
| Category membership | `Category` · `Members` |
| State transition | `From` · `Event` · `To` |
| Mapping / correspondence | `Source` · `Target` |
| Worked example | `Sample code` · `Expected value` · `Notes` |

Keep paragraphs for: definitions of a single concept, the rationale behind a rule, and any reasoning that does not decompose into uniform rows.

## 4. Worked example

The hazard, then the fix.

**Before** — an inline list whose commas sit beside Ajisai words:

> The logic words are `AND`, `OR`, and `NOT`; the control-flow words are `COND`, `EXEC`, `MAP`, `FILTER`, `FOLD`, `UNFOLD`, `ANY`, `ALL`, `COUNT`, and `SCAN`.

Every separator there is a `,`, which is the sugar for `EAT`. The reader must rely on the backticks alone to know the commas are English.

**After** — the same content as structure:

| Category | Members |
|----------|---------|
| Logic | `AND` · `OR` · `NOT` |
| Control flow | `COND` · `EXEC` · `MAP` · `FILTER` · `FOLD` · `UNFOLD` · `ANY` · `ALL` · `COUNT` · `SCAN` |

The cell boundaries carry the separation; where an in-cell separator is still needed, a middle dot (`·`) is used because it is not an Ajisai token. (A plain English "and" is also fine: "`AND` and `OR`".)

## 5. In-cell separators

When a single cell must hold more than one token, separate them with a character that is **not** Ajisai surface syntax:

- a middle dot `·`, or
- the word "and"/"or" spelled out.

Do **not** separate in-cell tokens with `,`, `/`, or `|` — the first two are words, and `|` collides with Markdown table syntax.

## 6. Migration is incremental

`SPECIFICATION.md` already uses tables heavily (the metadata firewall, the sugar map, the contract registry). The gap is the remaining **inline enumerations** scattered through the prose. They are converted opportunistically, not in one sweep:

- When editing a section for any reason, promote its inline token lists to tables.
- Do not renumber sections or restructure headings solely to insert a table.
- A table conversion must preserve meaning exactly; it is a presentation change, never a semantic one.

Representative candidates (illustrative, not exhaustive): the category lists in the Core-word sections, the NIL-passthrough word enumerations in the Bubble Rule section, and any sentence that names four or more words in a row.

## 7. Relationship to the other style documents

| Document | Layer it governs |
|----------|------------------|
| This document | `SPECIFICATION.md` (the canonical spec) |
| `reference-writing-style.md` | Reference site and `?`/LOOKUP help text |
| `three-layer-documentation-model.md` | Structure of all user-facing guidance |

All three share one root principle: **Ajisai code and the prose about it must be visually and structurally distinct, so that a symbol is never mistaken for punctuation.** Tables are the strongest tool for that, which is why each layer uses them deliberately.
