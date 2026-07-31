//! Modifier, control, higher-order, I/O, and dictionary Word presentation metadata.
//!
//! Invariant: positional control directives declare their execution form explicitly instead of masquerading as stack Words.

use super::super::builtin_word_definitions::{RuntimeSpec, SPEC_DEFAULT};
use crate::coreword_registry::{ExecutionForm, Partiality, SafetyLevel};

pub(in crate::builtins) const EAT: RuntimeSpec = RuntimeSpec {
    name: "EAT",
    category: "modifier",
    role: "Modifier that switches the next word into operand-consuming mode.",

    stack_effect: "no values popped or pushed",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const KEEP: RuntimeSpec = RuntimeSpec {
    name: "KEEP",
    category: "modifier",
    role: "Modifier that preserves operands while appending the next word's result.",

    stack_effect: "operands preserved; result pushed",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const COND: RuntimeSpec = RuntimeSpec {
    name: "COND",
    category: "control",
    role: "General conditional dispatch with first-match semantics.",

    stack_effect: "value { ... } ... -> [ result ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const VENT: RuntimeSpec = RuntimeSpec {
    name: "VENT",
    category: "control-directive",
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

pub(in crate::builtins) const MAP: RuntimeSpec = RuntimeSpec {
    name: "MAP",
    category: "higher-order",
    role: "Higher-order primitive: Apply a code block to each element of a vector.",

    stack_effect: "[ vec ] { body } -> [ mapped ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const FILTER: RuntimeSpec = RuntimeSpec {
    name: "FILTER",
    category: "higher-order",
    role:
        "Higher-order primitive: Keep only the elements for which a predicate block returns TRUE.",

    stack_effect: "[ vec ] { pred } -> [ kept ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const FOLD: RuntimeSpec = RuntimeSpec {

        name: "FOLD",
        category: "higher-order",
        role: "Higher-order primitive: Reduce a vector to a single value using an initial accumulator and combiner block.",

        stack_effect: "[ vec ] [ init ] { combine } -> [ result ]",
        partiality: Partiality::Partial,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        };

pub(in crate::builtins) const ANY: RuntimeSpec = RuntimeSpec {
    name: "ANY",
    category: "higher-order",
    role: "Higher-order primitive: TRUE if at least one element satisfies the predicate.",

    stack_effect: "[ vec ] { pred } -> [ TRUE | FALSE ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const ALL: RuntimeSpec = RuntimeSpec {
    name: "ALL",
    category: "higher-order",
    role: "Higher-order primitive: TRUE if every element satisfies the predicate.",

    stack_effect: "[ vec ] { pred } -> [ TRUE | FALSE ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const PRINT: RuntimeSpec = RuntimeSpec {

        name: "PRINT",
        category: "io",
        role: "Io primitive: output the top stack value at the output boundary, where a string is emitted as its raw character content (the stack's surrounding quotes are a display affordance only).",

        stack_effect: "[ x ] -> [ ]",
        stability: "experimental",
        safe_preview: false,
        partiality: Partiality::Partial,
        safety_level: SafetyLevel::D,
        ..SPEC_DEFAULT
        };

pub(in crate::builtins) const DEF: RuntimeSpec = RuntimeSpec {
    name: "DEF",
    category: "dictionary",
    role: "Dictionary primitive: Define a user word from a body and a name.",

    stack_effect: "{ body } [ name ] -> []",
    stability: "experimental",
    safe_preview: false,
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::D,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const DEL: RuntimeSpec = RuntimeSpec {
    name: "DEL",
    category: "dictionary",
    role: "Dictionary primitive: Delete a user word from the dictionary.",

    stack_effect: "[ name ] -> []",
    stability: "experimental",
    safe_preview: false,
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::D,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const LOOKUP: RuntimeSpec = RuntimeSpec {
    name: "LOOKUP",
    category: "dictionary",
    role: "Provides word-level guidance from inside Ajisai.",

    stack_effect: "[ name ] -> []",
    stability: "experimental",
    safe_preview: false,
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::C,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const EXEC: RuntimeSpec = RuntimeSpec {
    name: "EXEC",
    category: "control",
    role: "Control primitive: Execute a vector as Ajisai code.",

    stack_effect: "[ code ] -> [ result... ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};
