//! Authored LOOKUP-body content for core builtin words: the Behavior /
//! Examples / Failure-note / Related sections of the three-layer model's
//! Layer 2 template (docs/dev/three-layer-documentation-model.md §3.4).
//!
//! Only content that must be *authored* lives here. Everything derivable
//! from `BuiltinSpec` — Failure baseline (partiality / nil_policy), Side
//! Effects (effects), Stability — is derived at render time in
//! `builtin_word_details.rs`, so it can never drift from the §7.14
//! contract metadata. Words without an entry here still render the full
//! derived template; an entry adds the authored depth on top.
//!
//! Authoring rules (§3.3): UTF-8 English plain text, lines ≤ 80 columns,
//! no control characters. `behavior` is the mechanical effect on inputs
//! and runtime state (§3.5), never design history.

/// One authored example: the canonical invocation and an optional result
/// note (empty string = no result line).
#[derive(Clone, Copy)]
pub struct BuiltinExampleDoc {
    pub code: &'static str,
    pub result: &'static str,
}

#[derive(Clone, Copy)]
pub struct BuiltinLookupDoc {
    /// Canonical word name (must match a `BuiltinSpec.name`).
    pub word: &'static str,
    /// Mechanical effect on inputs and runtime state (§3.5 Behavior).
    pub behavior: &'static str,
    /// Authored examples; empty slice = derive one from `hover_syntax`.
    pub examples: &'static [BuiltinExampleDoc],
    /// Authored addition to the derived Failure baseline ("" = none).
    pub failure_note: &'static str,
    /// Related words ("" slice = section omitted).
    pub related: &'static [&'static str],
}

pub fn lookup_builtin_lookup_doc(word: &str) -> Option<&'static BuiltinLookupDoc> {
    BUILTIN_LOOKUP_DOCS.iter().find(|d| d.word == word)
}

#[cfg(test)]
pub(crate) fn builtin_lookup_docs() -> &'static [BuiltinLookupDoc] {
    BUILTIN_LOOKUP_DOCS
}

const BUILTIN_LOOKUP_DOCS: &[BuiltinLookupDoc] = &[
    // ── Vector access and editing ─────────────────────────────────────────
    BuiltinLookupDoc {
        word: "GET",
        behavior: "Pops the index vector, then the target vector, and pushes\nthe element at that zero-based index.",
        examples: &[BuiltinExampleDoc {
            code: "[ 10 20 30 ] [ 0 ] GET",
            result: "Pushes the first element, 10.",
        }],
        failure_note: "An out-of-range index yields a Bubble/NIL with reason\nindexOutOfBounds.",
        related: &["INSERT", "REPLACE", "REMOVE", "LENGTH", "TAKE"],
    },
    BuiltinLookupDoc {
        word: "INSERT",
        behavior: "Pops an [ index value ] pair, then the target vector, and\npushes a new vector with the value inserted at that index.",
        examples: &[BuiltinExampleDoc {
            code: "[ 1 3 ] [ 1 2 ] INSERT",
            result: "Pushes [ 1 2 3 ].",
        }],
        failure_note: "",
        related: &["GET", "REPLACE", "REMOVE", "CONCAT"],
    },
    BuiltinLookupDoc {
        word: "REPLACE",
        behavior: "Pops an [ index value ] pair, then the target vector, and\npushes a new vector with the element at that index replaced.",
        examples: &[BuiltinExampleDoc {
            code: "[ 1 2 3 ] [ 0 9 ] REPLACE",
            result: "Pushes [ 9 2 3 ].",
        }],
        failure_note: "",
        related: &["GET", "INSERT", "REMOVE"],
    },
    BuiltinLookupDoc {
        word: "REMOVE",
        behavior: "Pops the index vector, then the target vector, and pushes a\nnew vector without the element at that index.",
        examples: &[BuiltinExampleDoc {
            code: "[ 1 2 3 ] [ 0 ] REMOVE",
            result: "Pushes [ 2 3 ].",
        }],
        failure_note: "",
        related: &["GET", "INSERT", "REPLACE"],
    },
    BuiltinLookupDoc {
        word: "LENGTH",
        behavior: "Pops a vector and pushes the number of its elements.",
        examples: &[BuiltinExampleDoc {
            code: "[ 1 2 3 ] LENGTH",
            result: "Pushes 3.",
        }],
        failure_note: "",
        related: &["GET", "TAKE", "SPLIT"],
    },
    BuiltinLookupDoc {
        word: "CONCAT",
        behavior: "Pops two vectors and pushes their concatenation.",
        examples: &[BuiltinExampleDoc {
            code: "[ 1 2 ] [ 3 4 ] CONCAT",
            result: "Pushes [ 1 2 3 4 ].",
        }],
        failure_note: "",
        related: &["INSERT", "SPLIT", "REVERSE"],
    },
    BuiltinLookupDoc {
        word: "REVERSE",
        behavior: "Pops a vector and pushes it with the element order reversed.",
        examples: &[BuiltinExampleDoc {
            code: "[ 1 2 3 ] REVERSE",
            result: "Pushes [ 3 2 1 ].",
        }],
        failure_note: "",
        related: &["CONCAT", "REORDER"],
    },
    BuiltinLookupDoc {
        word: "TAKE",
        behavior: "Pops the count vector, then the target vector, and pushes\nthe first N elements (or the last N for a negative count).",
        examples: &[BuiltinExampleDoc {
            code: "[ 1 2 3 4 5 ] [ 3 ] TAKE",
            result: "Pushes [ 1 2 3 ].",
        }],
        failure_note: "",
        related: &["SPLIT", "GET", "LENGTH"],
    },
    BuiltinLookupDoc {
        word: "RANGE",
        behavior: "Pops a [ start end ] pair and pushes the numeric sequence\nit spans.",
        examples: &[BuiltinExampleDoc {
            code: "[ 0 5 ] RANGE",
            result: "Pushes the sequence from 0 to 5.",
        }],
        failure_note: "",
        related: &["TAKE", "SPLIT"],
    },
    // ── Arithmetic ────────────────────────────────────────────────────────
    BuiltinLookupDoc {
        word: "ADD",
        behavior: "Pops two numeric values and pushes their exact sum.",
        examples: &[BuiltinExampleDoc {
            code: "1 2 +",
            result: "Pushes 3.",
        }],
        failure_note: "",
        related: &["SUB", "MUL", "DIV", "MOD"],
    },
    BuiltinLookupDoc {
        word: "SUB",
        behavior: "Pops two numeric values and pushes their exact difference.",
        examples: &[BuiltinExampleDoc {
            code: "5 3 -",
            result: "Pushes 2.",
        }],
        failure_note: "",
        related: &["ADD", "MUL", "DIV", "MOD"],
    },
    BuiltinLookupDoc {
        word: "MUL",
        behavior: "Pops two numeric values and pushes their exact product.",
        examples: &[BuiltinExampleDoc {
            code: "2 4 *",
            result: "Pushes 8.",
        }],
        failure_note: "",
        related: &["ADD", "SUB", "DIV", "MOD"],
    },
    BuiltinLookupDoc {
        word: "DIV",
        behavior: "Pops two numeric values and pushes their exact quotient as\na reduced fraction. Nothing is rounded.",
        examples: &[BuiltinExampleDoc {
            code: "10 2 /",
            result: "Pushes 5.",
        }],
        failure_note: "Division by zero yields a Bubble/NIL with reason\ndivisionByZero.",
        related: &["ADD", "SUB", "MUL", "MOD"],
    },
    BuiltinLookupDoc {
        word: "MOD",
        behavior: "Pops two numeric values and pushes the remainder of their\ndivision.",
        examples: &[BuiltinExampleDoc {
            code: "7 3 %",
            result: "Pushes 1.",
        }],
        failure_note: "",
        related: &["DIV", "FLOOR", "CEIL", "ROUND"],
    },
    // ── Comparison ────────────────────────────────────────────────────────
    BuiltinLookupDoc {
        word: "EQ",
        behavior: "Pops two values and pushes TRUE when they are equal,\nFALSE otherwise.",
        examples: &[BuiltinExampleDoc {
            code: "1 1 =",
            result: "Pushes TRUE.",
        }],
        failure_note: "",
        related: &["NEQ", "LT", "LTE", "GT", "GTE", "COMPARE-WITHIN"],
    },
    BuiltinLookupDoc {
        word: "LT",
        behavior: "Pops two values and pushes TRUE when the first is less\nthan the second, FALSE otherwise. The decision is exact.",
        examples: &[BuiltinExampleDoc {
            code: "1 2 <",
            result: "Pushes TRUE.",
        }],
        failure_note: "",
        related: &["LTE", "GT", "GTE", "EQ", "COMPARE-WITHIN"],
    },
    BuiltinLookupDoc {
        word: "COMPARE-WITHIN",
        behavior: "Pops the budget, then two values, and pushes -1, 0, or 1\nfor their ordering. Every value the current vocabulary can\nconstruct decides regardless of the budget; the budget bounds\nthe refinement of future general computable reals, whose\nexhaustion yields UNKNOWN.",
        examples: &[BuiltinExampleDoc {
            code: "1/3 1/2 64 COMPARE-WITHIN",
            result: "Pushes -1, 0, 1, or UNKNOWN.",
        }],
        failure_note: "UNKNOWN is a value, not an error: it reports that the\nrequested depth was reached before the order was decided.",
        related: &["EQ", "LT", "GT"],
    },
    // ── Casts and text ────────────────────────────────────────────────────
    BuiltinLookupDoc {
        word: "NUM",
        behavior: "Pops a text value and pushes the number it spells.",
        examples: &[BuiltinExampleDoc {
            code: "'42' NUM",
            result: "Pushes 42.",
        }],
        failure_note: "Text with no numeric reading yields a Bubble/NIL.",
        related: &["STR", "BOOL", "CHR"],
    },
    BuiltinLookupDoc {
        word: "STR",
        behavior: "Pops a value and pushes its text representation.",
        examples: &[BuiltinExampleDoc {
            code: "42 STR",
            result: "Pushes '42'.",
        }],
        failure_note: "",
        related: &["NUM", "BOOL", "CHARS", "JOIN"],
    },
    BuiltinLookupDoc {
        word: "CHR",
        behavior: "Pops a code-point number and pushes the one-character text\nit denotes.",
        examples: &[BuiltinExampleDoc {
            code: "65 CHR",
            result: "Pushes 'A'.",
        }],
        failure_note: "A number that is not a valid code point yields a\nBubble/NIL.",
        related: &["NUM", "CHARS", "JOIN"],
    },
    BuiltinLookupDoc {
        word: "CHARS",
        behavior: "Pops a text value and pushes a vector of its one-character\ntexts.",
        examples: &[BuiltinExampleDoc {
            code: "'hi' CHARS",
            result: "Pushes [ 'h' 'i' ].",
        }],
        failure_note: "",
        related: &["JOIN", "CHR", "STR"],
    },
    BuiltinLookupDoc {
        word: "JOIN",
        behavior: "Pops a vector of texts and pushes their concatenation as\none text.",
        examples: &[BuiltinExampleDoc {
            code: "[ 'h' 'i' ] JOIN",
            result: "Pushes 'hi'.",
        }],
        failure_note: "",
        related: &["CHARS", "CONCAT", "STR"],
    },
    // ── Control and higher-order words ────────────────────────────────────
    BuiltinLookupDoc {
        word: "COND",
        behavior: "Reads guard/body clause pairs in order. The first guard\nthat holds selects its body; IDLE marks the else clause.\nIn a tail position the selected body continues the loop\nwithout growing the stack.",
        examples: &[BuiltinExampleDoc {
            code: "1 { TRUE } { 'y' } { IDLE } { 'n' } COND",
            result: "Pushes 'y'.",
        }],
        failure_note: "When every guard fails and no else clause exists, COND\nraises an error.",
        related: &["IDLE", "MAP", "EXEC"],
    },
    BuiltinLookupDoc {
        word: "MAP",
        behavior: "Pops the code block, then the target vector, applies the\nblock to each element, and pushes the vector of results.",
        examples: &[BuiltinExampleDoc {
            code: "[ 1 2 3 ] { [ 2 ] * } MAP",
            result: "Pushes [ 2 4 6 ].",
        }],
        failure_note: "An element the block cannot produce a value for follows\nthe Bubble Rule: that lane becomes a Bubble/NIL, e.g.\ndividing by zero maps the element to NIL.",
        related: &["FILTER", "FOLD", "SCAN", "COUNT"],
    },
    BuiltinLookupDoc {
        word: "FILTER",
        behavior: "Pops the predicate block, then the target vector, and\npushes the vector of elements for which the predicate\nholds.",
        examples: &[BuiltinExampleDoc {
            code: "[ 1 2 3 ] { [ 2 ] = } FILTER",
            result: "Pushes [ 2 ].",
        }],
        failure_note: "",
        related: &["MAP", "FOLD", "ANY", "ALL", "COUNT"],
    },
    BuiltinLookupDoc {
        word: "FOLD",
        behavior: "Pops the combining block, then the initial value, then the\ntarget vector, and combines the elements left to right\ninto a single result, starting from the initial value.",
        examples: &[BuiltinExampleDoc {
            code: "[ 1 2 3 ] [ 0 ] { + } FOLD",
            result: "Pushes [ 6 ].",
        }],
        failure_note: "",
        related: &["MAP", "FILTER", "SCAN", "UNFOLD"],
    },
    // ── Dictionary words ──────────────────────────────────────────────────
    BuiltinLookupDoc {
        word: "DEF",
        behavior: "Pops the name, then the body block, and defines a user\nword under that name in the dictionary.",
        examples: &[BuiltinExampleDoc {
            code: "{ 2 * } 'DOUBLE' DEF",
            result: "Defines DOUBLE; 5 DOUBLE then pushes 10.",
        }],
        failure_note: "Redefining a built-in word is refused.",
        related: &["DEL", "LOOKUP", "FORC"],
    },
    BuiltinLookupDoc {
        word: "DEL",
        behavior: "Pops the name and deletes that user word from the\ndictionary.",
        examples: &[BuiltinExampleDoc {
            code: "{ [ 1 ] } 'W' DEF 'W' DEL",
            result: "Defines a word, then removes it from the dictionary.",
        }],
        failure_note: "Deleting a built-in word is refused. Deleting a word other\nwords depend on requires FORC.",
        related: &["DEF", "FORC", "LOOKUP"],
    },
    BuiltinLookupDoc {
        word: "LOOKUP",
        behavior: "Pops the word name and loads its documentation into the\neditor. For a user word, the original defining source is\nloaded instead.",
        examples: &[BuiltinExampleDoc {
            code: "'ADD' ?",
            result: "Loads the documentation for ADD into the editor.",
        }],
        failure_note: "Unknown words raise an error.",
        related: &["DEF", "DEL"],
    },
    // ── Output and evaluation ─────────────────────────────────────────────
    BuiltinLookupDoc {
        word: "PRINT",
        behavior: "Writes the top stack value to the output area and leaves\nthe value on the stack. Text prints as its raw characters,\nwithout the quotes the stack shows.",
        examples: &[BuiltinExampleDoc {
            code: "42 PRINT",
            result: "Writes 42 to the output.",
        }],
        failure_note: "",
        related: &["STR", "EVAL"],
    },
    BuiltinLookupDoc {
        word: "EVAL",
        behavior: "Pops a text value, parses it as Ajisai source, and\nexecutes it in the current context.",
        examples: &[BuiltinExampleDoc {
            code: "'1 2 +' EVAL",
            result: "Pushes 3.",
        }],
        failure_note: "",
        related: &["EXEC", "PRINT"],
    },
    BuiltinLookupDoc {
        word: "NIL-REASON",
        behavior: "Reads the top value without consuming it and pushes the\ndirect reason of an operational NIL as a protocol-string\ntext, or NIL when the value carries no reason.",
        examples: &[BuiltinExampleDoc {
            code: "1 0 / NIL-REASON",
            result: "Pushes 'divisionByZero'.",
        }],
        failure_note: "",
        related: &["NIL?", "NIL-ORIGIN", "NIL-RECOVERABLE?", "NIL-DIAGNOSIS"],
    },
];
