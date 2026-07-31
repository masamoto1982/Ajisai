//! Modifier, control, higher-order, I/O, and dictionary Word presentation metadata.
//!
//! Invariant: positional control directives declare their execution form explicitly instead of masquerading as stack Words.

use super::super::builtin_word_definitions::{BuiltinSpec, SPEC_DEFAULT};
use crate::coreword_registry::{ExecutionForm, Partiality, SafetyLevel};

pub(in crate::builtins) const EAT: BuiltinSpec = BuiltinSpec {
    name: "EAT",
    category: "modifier",
    hover_summary: "EAT — consume operands",
    hover_syntax: ", +",
    summary: "Set the consumption mode to consume operands.",
    role: "Modifier that switches the next word into operand-consuming mode.",

    stack_effect: "no values popped or pushed",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const KEEP: BuiltinSpec = BuiltinSpec {
    name: "KEEP",
    category: "modifier",
    hover_summary: "KEEP — keep operands and append result",
    hover_syntax: ",, +",
    summary: "Set the consumption mode to keep operands.",
    role: "Modifier that preserves operands while appending the next word's result.",

    stack_effect: "operands preserved; result pushed",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const COND: BuiltinSpec = BuiltinSpec {
    name: "COND",
    category: "control",
    hover_summary: "COND — evaluate guard/body clauses",
    hover_syntax: "1 { TRUE } { 'y' } { IDLE } { 'n' } COND",
    summary: "Evaluate guard/body clauses in order, executing the first match.",
    role: "General conditional dispatch with first-match semantics.",

    stack_effect: "value { ... } ... -> [ result ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const VENT: BuiltinSpec = BuiltinSpec {
    name: "VENT",
    category: "control-directive",
    hover_summary: "VENT — lazy NIL-coalescing fallback",
    hover_syntax: "NIL ^ [ 0 ]",
    summary: "Lazy NIL-coalescing control directive: keep a non-NIL top and skip \
             the following source unit; on a NIL top, discard it and evaluate \
             the following source unit as the fallback.",
    role: "Control directive that inspects the stack top. If the top is \
               non-NIL it is kept and the following source unit is skipped \
               UNEVALUATED. If the top is NIL it is discarded and the following \
               source unit is evaluated as the fallback. The fallback is the \
               source that follows the directive, not a value already on the \
               stack.",

    stack_effect: "top non-NIL: keeps top, skips next source unit unevaluated; \
                       top NIL: discards top, evaluates next source unit as fallback",
    execution_form: ExecutionForm::LazyNextUnitFallback,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const MAP: BuiltinSpec = BuiltinSpec {
    name: "MAP",
    category: "higher-order",
    hover_summary: "MAP — apply block to each element",
    hover_syntax: "[ 1 2 3 ] { [ 2 ] * } MAP",
    summary: "Apply a code block to each element of a vector.",
    role: "Higher-order primitive: Apply a code block to each element of a vector.",

    stack_effect: "[ vec ] { body } -> [ mapped ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const FILTER: BuiltinSpec = BuiltinSpec {
    name: "FILTER",
    category: "higher-order",
    hover_summary: "FILTER — keep elements matching predicate",
    hover_syntax: "[ 1 2 3 ] { [ 2 ] = } FILTER",
    summary: "Keep only the elements for which a predicate block returns TRUE.",
    role:
        "Higher-order primitive: Keep only the elements for which a predicate block returns TRUE.",

    stack_effect: "[ vec ] { pred } -> [ kept ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const FOLD: BuiltinSpec = BuiltinSpec {

        name: "FOLD",
        category: "higher-order",
        hover_summary: "FOLD — reduce with initial value",
        hover_syntax: "[ 1 2 3 ] [ 0 ] { + } FOLD",
        summary:
            "Reduce a vector to a single value using an initial accumulator and combiner block.",
        role: "Higher-order primitive: Reduce a vector to a single value using an initial accumulator and combiner block.",

        stack_effect: "[ vec ] [ init ] { combine } -> [ result ]",
        partiality: Partiality::Partial,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        };

pub(in crate::builtins) const ANY: BuiltinSpec = BuiltinSpec {
    name: "ANY",
    category: "higher-order",
    hover_summary: "ANY — true if any element matches",
    hover_syntax: "[ 1 2 3 ] { [ 2 ] = } ANY",
    summary: "TRUE if at least one element satisfies the predicate.",
    role: "Higher-order primitive: TRUE if at least one element satisfies the predicate.",

    stack_effect: "[ vec ] { pred } -> [ TRUE | FALSE ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const ALL: BuiltinSpec = BuiltinSpec {
    name: "ALL",
    category: "higher-order",
    hover_summary: "ALL — true if all elements match",
    hover_syntax: "[ 2 4 ] { [ 2 ] MOD [ 0 ] = } ALL",
    summary: "TRUE if every element satisfies the predicate.",
    role: "Higher-order primitive: TRUE if every element satisfies the predicate.",

    stack_effect: "[ vec ] { pred } -> [ TRUE | FALSE ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const PRINT: BuiltinSpec = BuiltinSpec {

        name: "PRINT",
        category: "io",
        hover_summary: "PRINT — output value to display",
        hover_syntax: "42 PRINT",
        summary: "Write the top stack value to the output stream, consuming it. A string is written as its raw text, without the quotes the stack shows ('TEST' prints as TEST); nested strings keep their quotes.",
        role: "Io primitive: output the top stack value at the output boundary, where a string is emitted as its raw character content (the stack's surrounding quotes are a display affordance only).",

        stack_effect: "[ x ] -> [ ]",
        stability: "experimental",
        safe_preview: false,
        partiality: Partiality::Partial,
        safety_level: SafetyLevel::D,
        ..SPEC_DEFAULT
        };

pub(in crate::builtins) const DEF: BuiltinSpec = BuiltinSpec {
    name: "DEF",
    category: "dictionary",
    hover_summary: "DEF — define user word",
    hover_syntax: "{ 2 * } 'DOUBLE' DEF",
    summary: "Define a user word from a body and a name.",
    role: "Dictionary primitive: Define a user word from a body and a name.",

    stack_effect: "{ body } [ name ] -> []",
    stability: "experimental",
    safe_preview: false,
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::D,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const DEL: BuiltinSpec = BuiltinSpec {
    name: "DEL",
    category: "dictionary",
    hover_summary: "DEL — delete user word",
    hover_syntax: "{ [ 1 ] } 'W' DEF 'W' DEL",
    summary: "Delete a user word from the dictionary.",
    role: "Dictionary primitive: Delete a user word from the dictionary.",

    stack_effect: "[ name ] -> []",
    stability: "experimental",
    safe_preview: false,
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::D,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const LOOKUP: BuiltinSpec = BuiltinSpec {
    name: "LOOKUP",
    category: "dictionary",
    hover_summary: "LOOKUP — show word documentation",
    hover_syntax: "'ADD' ?",
    summary: "Display the documentation for a named word.",
    role: "Provides word-level guidance from inside Ajisai.",

    stack_effect: "[ name ] -> []",
    stability: "experimental",
    safe_preview: false,
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::C,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const EXEC: BuiltinSpec = BuiltinSpec {
    name: "EXEC",
    category: "control",
    hover_summary: "EXEC — execute vector as code",
    hover_syntax: "[ 1 2 + ] EXEC",
    summary: "Execute a vector as Ajisai code.",
    role: "Control primitive: Execute a vector as Ajisai code.",

    stack_effect: "[ code ] -> [ result... ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};
