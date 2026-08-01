//! Scalar, absence, cast, and logic Word presentation metadata.
//!
//! Invariant: diagnostic absence metadata remains distinct from logical truth metadata.

use super::super::builtin_word_definitions::{RuntimeSpec, SPEC_DEFAULT};
use crate::coreword_registry::{Partiality, SafetyLevel};

pub(in crate::builtins) const TRUE: RuntimeSpec = RuntimeSpec {
    name: "TRUE",
    category: "constant",
    role: "Constant primitive: Push the boolean TRUE onto the stack.",

    stack_effect: "-> [ TRUE ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const FALSE: RuntimeSpec = RuntimeSpec {
    name: "FALSE",
    category: "constant",
    role: "Constant primitive: Push the boolean FALSE onto the stack.",

    stack_effect: "-> [ FALSE ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const NIL: RuntimeSpec = RuntimeSpec {
    name: "NIL",
    category: "constant",
    role: "Represents the absence of a value or a recoverable failure.",

    stack_effect: "-> [ NIL ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const NIL_PREDICATE: RuntimeSpec = RuntimeSpec {

        name: "NIL?",
        category: "absence",
        role: "Diagnostic predicate: TRUE when the retained value is absent, FALSE otherwise. Never branches on the reason (SPEC §4.5.0).",

        stack_effect: "[ x ] -> [ x ] [ bool ]",
        ..SPEC_DEFAULT
        };

pub(in crate::builtins) const NIL_REASON: RuntimeSpec = RuntimeSpec {

        name: "NIL-REASON",
        category: "absence",
        role: "Diagnostic accessor: the lowerCamelCase reason protocol string (SPEC §4.5.0), or NIL when there is no reason or the value is not an operational NIL.",

        stack_effect: "[ x ] -> [ x ] [ text|NIL ]",
        ..SPEC_DEFAULT
        };

pub(in crate::builtins) const CHARS: RuntimeSpec = RuntimeSpec {
    name: "CHARS",
    category: "cast",
    role: "Cast primitive: Split a string into a vector of one-character strings.",

    stack_effect: "[ str ] -> [ chars ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const JOIN: RuntimeSpec = RuntimeSpec {
    name: "JOIN",
    category: "cast",
    role: "Cast primitive: Join a vector of strings into a single string.",

    stack_effect: "[ chars ] -> [ str ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const TRIM: RuntimeSpec = RuntimeSpec {
    name: "TRIM",
    category: "cast",
    role: "Cast primitive: Remove whitespace from both ends of a string.",

    stack_effect: "[ str ] -> [ str' ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const TOKENIZE: RuntimeSpec = RuntimeSpec {
    name: "TOKENIZE",
    category: "cast",
    role: "Cast primitive: Split a string into a vector of substrings using a separator.",

    stack_effect: "[ str ] [ sep ] -> [ parts ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const SUBSTITUTE: RuntimeSpec = RuntimeSpec {
    name: "SUBSTITUTE",
    category: "cast",
    role: "Cast primitive: Replace every occurrence of a substring with another.",

    stack_effect: "[ str ] [ from ] [ to ] -> [ str' ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const STARTS_WITH: RuntimeSpec = RuntimeSpec {
    name: "STARTS-WITH?",
    category: "cast",
    role: "Cast primitive: Test whether a string begins with the given prefix.",

    stack_effect: "[ str ] [ prefix ] -> [ TRUE | FALSE ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const ENDS_WITH: RuntimeSpec = RuntimeSpec {
    name: "ENDS-WITH?",
    category: "cast",
    role: "Cast primitive: Test whether a string ends with the given suffix.",

    stack_effect: "[ str ] [ suffix ] -> [ TRUE | FALSE ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const NUM: RuntimeSpec = RuntimeSpec {
    name: "NUM",
    category: "cast",
    role: "Cast primitive: Parse text as a number; Bubble/NIL on parse failure.",

    stack_effect: "[ x ] -> [ n | NIL ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const STR: RuntimeSpec = RuntimeSpec {
    name: "STR",
    category: "cast",
    role: "Cast primitive: Convert a value to its string representation.",

    stack_effect: "[ x ] -> [ str ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const CHR: RuntimeSpec = RuntimeSpec {
    name: "CHR",
    category: "cast",
    role: "Cast primitive: Convert a numeric character code to a single-character string.",

    stack_effect: "[ n ] -> [ char ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const AND: RuntimeSpec = RuntimeSpec {
    name: "AND",
    category: "logic",
    role: "Logic primitive: Logical AND with three-valued (Kleene) NIL handling.",

    stack_effect: "[ a ] [ b ] -> [ a AND b ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const OR: RuntimeSpec = RuntimeSpec {
    name: "OR",
    category: "logic",
    role: "Logic primitive: Logical OR with three-valued (Kleene) NIL handling.",

    stack_effect: "[ a ] [ b ] -> [ a OR b ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const NOT: RuntimeSpec = RuntimeSpec {
    name: "NOT",
    category: "logic",
    role: "Logic primitive: Logical negation.",

    stack_effect: "[ a ] -> [ NOT a ]",
    ..SPEC_DEFAULT
};
