//! Scalar, absence, cast, and logic Word presentation metadata.
//!
//! Invariant: diagnostic absence metadata remains distinct from logical truth metadata.

use super::super::builtin_word_definitions::{BuiltinSpec, SPEC_DEFAULT};
use crate::coreword_registry::{Partiality, SafetyLevel};

pub(in crate::builtins) const TRUE: BuiltinSpec = BuiltinSpec {
    name: "TRUE",
    category: "constant",
    hover_summary: "TRUE — push TRUE",
    hover_syntax: "TRUE",
    summary: "Push the boolean TRUE onto the stack.",
    role: "Constant primitive: Push the boolean TRUE onto the stack.",

    stack_effect: "-> [ TRUE ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const FALSE: BuiltinSpec = BuiltinSpec {
    name: "FALSE",
    category: "constant",
    hover_summary: "FALSE — push FALSE",
    hover_syntax: "FALSE",
    summary: "Push the boolean FALSE onto the stack.",
    role: "Constant primitive: Push the boolean FALSE onto the stack.",

    stack_effect: "-> [ FALSE ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const NIL: BuiltinSpec = BuiltinSpec {
    name: "NIL",
    category: "constant",
    hover_summary: "NIL — push NIL",
    hover_syntax: "NIL",
    summary: "Push the NIL value onto the stack.",
    role: "Represents the absence of a value or a recoverable failure.",

    stack_effect: "-> [ NIL ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const NIL_PREDICATE: BuiltinSpec = BuiltinSpec {

        name: "NIL?",
        category: "absence",
        hover_summary: "NIL? — test whether a value is absent",
        hover_syntax: "1 0 / NIL?",
        summary: "Test whether the top value is an operational NIL (absent).",
        role: "Diagnostic predicate: TRUE when the retained value is absent, FALSE otherwise. Never branches on the reason (SPEC §4.5.0).",

        stack_effect: "[ x ] -> [ x ] [ bool ]",
        ..SPEC_DEFAULT
        };

pub(in crate::builtins) const NIL_REASON: BuiltinSpec = BuiltinSpec {

        name: "NIL-REASON",
        category: "absence",
        hover_summary: "NIL-REASON — read the NIL reason protocol string",
        hover_syntax: "1 0 / NIL-REASON",
        summary: "Read the direct reason of an operational NIL as a protocol-string Text.",
        role: "Diagnostic accessor: the lowerCamelCase reason protocol string (SPEC §4.5.0), or NIL when there is no reason or the value is not an operational NIL.",

        stack_effect: "[ x ] -> [ x ] [ text|NIL ]",
        ..SPEC_DEFAULT
        };

pub(in crate::builtins) const CHARS: BuiltinSpec = BuiltinSpec {
    name: "CHARS",
    category: "cast",
    hover_summary: "CHARS — split string into characters",
    hover_syntax: "'hi' CHARS",
    summary: "Split a string into a vector of one-character strings.",
    role: "Cast primitive: Split a string into a vector of one-character strings.",

    stack_effect: "[ str ] -> [ chars ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const JOIN: BuiltinSpec = BuiltinSpec {
    name: "JOIN",
    category: "cast",
    hover_summary: "JOIN — join characters into string",
    hover_syntax: "[ 'h' 'i' ] JOIN",
    summary: "Join a vector of strings into a single string.",
    role: "Cast primitive: Join a vector of strings into a single string.",

    stack_effect: "[ chars ] -> [ str ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const TRIM: BuiltinSpec = BuiltinSpec {
    name: "TRIM",
    category: "cast",
    hover_summary: "TRIM — strip leading and trailing whitespace",
    hover_syntax: "'  hi  ' TRIM",
    summary: "Remove whitespace from both ends of a string.",
    role: "Cast primitive: Remove whitespace from both ends of a string.",

    stack_effect: "[ str ] -> [ str' ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const TOKENIZE: BuiltinSpec = BuiltinSpec {
    name: "TOKENIZE",
    category: "cast",
    hover_summary: "TOKENIZE — split string by separator",
    hover_syntax: "'a,b,c' ',' TOKENIZE",
    summary: "Split a string into a vector of substrings using a separator.",
    role: "Cast primitive: Split a string into a vector of substrings using a separator.",

    stack_effect: "[ str ] [ sep ] -> [ parts ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const SUBSTITUTE: BuiltinSpec = BuiltinSpec {
    name: "SUBSTITUTE",
    category: "cast",
    hover_summary: "SUBSTITUTE — replace substring occurrences",
    hover_syntax: "'hello' 'l' 'L' SUBSTITUTE",
    summary: "Replace every occurrence of a substring with another.",
    role: "Cast primitive: Replace every occurrence of a substring with another.",

    stack_effect: "[ str ] [ from ] [ to ] -> [ str' ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const STARTS_WITH: BuiltinSpec = BuiltinSpec {
    name: "STARTS-WITH?",
    category: "cast",
    hover_summary: "STARTS-WITH? — prefix predicate",
    hover_syntax: "'hello' 'he' STARTS-WITH?",
    summary: "Test whether a string begins with the given prefix.",
    role: "Cast primitive: Test whether a string begins with the given prefix.",

    stack_effect: "[ str ] [ prefix ] -> [ TRUE | FALSE ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const ENDS_WITH: BuiltinSpec = BuiltinSpec {
    name: "ENDS-WITH?",
    category: "cast",
    hover_summary: "ENDS-WITH? — suffix predicate",
    hover_syntax: "'hello' 'lo' ENDS-WITH?",
    summary: "Test whether a string ends with the given suffix.",
    role: "Cast primitive: Test whether a string ends with the given suffix.",

    stack_effect: "[ str ] [ suffix ] -> [ TRUE | FALSE ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const NUM: BuiltinSpec = BuiltinSpec {
    name: "NUM",
    category: "cast",
    hover_summary: "NUM — parse to number",
    hover_syntax: "'42' NUM",
    summary: "Parse text as a number; Bubble/NIL on parse failure.",
    role: "Cast primitive: Parse text as a number; Bubble/NIL on parse failure.",

    stack_effect: "[ x ] -> [ n | NIL ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const STR: BuiltinSpec = BuiltinSpec {
    name: "STR",
    category: "cast",
    hover_summary: "STR — convert to string",
    hover_syntax: "42 STR",
    summary: "Convert a value to its string representation.",
    role: "Cast primitive: Convert a value to its string representation.",

    stack_effect: "[ x ] -> [ str ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const CHR: BuiltinSpec = BuiltinSpec {
    name: "CHR",
    category: "cast",
    hover_summary: "CHR — make a character",
    hover_syntax: "65 CHR",
    summary: "Convert a numeric character code to a single-character string.",
    role: "Cast primitive: Convert a numeric character code to a single-character string.",

    stack_effect: "[ n ] -> [ char ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const AND: BuiltinSpec = BuiltinSpec {
    name: "AND",
    category: "logic",
    hover_summary: "AND — logical AND",
    hover_syntax: "TRUE TRUE &",
    summary: "Logical AND. A NIL operand passes through.",
    role: "Logic primitive: Logical AND with three-valued (Kleene) NIL handling.",

    stack_effect: "[ a ] [ b ] -> [ a AND b ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const OR: BuiltinSpec = BuiltinSpec {
    name: "OR",
    category: "logic",
    hover_summary: "OR — logical OR",
    hover_syntax: "TRUE FALSE OR",
    summary: "Logical OR. A NIL operand passes through.",
    role: "Logic primitive: Logical OR with three-valued (Kleene) NIL handling.",

    stack_effect: "[ a ] [ b ] -> [ a OR b ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const NOT: BuiltinSpec = BuiltinSpec {
    name: "NOT",
    category: "logic",
    hover_summary: "NOT — logical negation",
    hover_syntax: "TRUE NOT",
    summary: "Logical negation. A NIL operand passes through.",
    role: "Logic primitive: Logical negation.",

    stack_effect: "[ a ] -> [ NOT a ]",
    ..SPEC_DEFAULT
};
