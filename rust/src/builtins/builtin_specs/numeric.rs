//! Arithmetic, comparison, and exact-math Word presentation metadata.
//!
//! Invariant: this table is prose/runtime-local classification only; canonical contracts come from `spec/words.json`.

use super::super::builtin_word_definitions::{RuntimeSpec, SPEC_DEFAULT};
use crate::coreword_registry::{Partiality, SafetyLevel};

pub(in crate::builtins) const ADD: RuntimeSpec = RuntimeSpec {
    name: "ADD",
    category: "arithmetic",
    role: "Numeric addition; one of the four arithmetic primitives.",

    stack_effect: "[ a ] [ b ] -> [ a + b ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const SUB: RuntimeSpec = RuntimeSpec {
    name: "SUB",
    category: "arithmetic",
    role: "Arithmetic primitive: Subtract two numeric values, element-wise with broadcasting.",

    stack_effect: "[ a ] [ b ] -> [ a - b ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const MUL: RuntimeSpec = RuntimeSpec {
    name: "MUL",
    category: "arithmetic",
    role: "Arithmetic primitive: Multiply two numeric values, element-wise with broadcasting.",

    stack_effect: "[ a ] [ b ] -> [ a * b ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const DIV: RuntimeSpec = RuntimeSpec {
    name: "DIV",
    category: "arithmetic",
    role: "Arithmetic primitive: Divide two numeric values exactly (fractional result).",

    stack_effect: "[ a ] [ b ] -> [ a / b ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const EQ: RuntimeSpec = RuntimeSpec {
    name: "EQ",
    category: "comparison",
    role: "Comparison primitive: Test equality of two values.",

    stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const LT: RuntimeSpec = RuntimeSpec {
    name: "LT",
    category: "comparison",
    role: "Comparison primitive: Test less-than comparison.",

    stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const LTE: RuntimeSpec = RuntimeSpec {
    name: "LTE",
    category: "comparison",
    role: "Comparison primitive: Test less-than-or-equal comparison.",

    stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const GT: RuntimeSpec = RuntimeSpec {
    name: "GT",
    category: "comparison",
    role: "Comparison primitive: Test greater-than comparison.",

    stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const GTE: RuntimeSpec = RuntimeSpec {
    name: "GTE",
    category: "comparison",
    role: "Comparison primitive: Test greater-than-or-equal comparison.",

    stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const NEQ: RuntimeSpec = RuntimeSpec {
    name: "NEQ",
    category: "comparison",
    role: "Comparison primitive: Test inequality of two values.",

    stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const MOD: RuntimeSpec = RuntimeSpec {
    name: "MOD",
    category: "arithmetic",
    role: "Arithmetic primitive: Modulo (remainder) of two numeric values.",

    stack_effect: "[ a ] [ b ] -> [ a mod b ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const FLOOR: RuntimeSpec = RuntimeSpec {
    name: "FLOOR",
    category: "arithmetic",
    role: "Arithmetic primitive: Round toward negative infinity.",

    stack_effect: "[ x ] -> [ floor x ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const CEIL: RuntimeSpec = RuntimeSpec {
    name: "CEIL",
    category: "arithmetic",
    role: "Arithmetic primitive: Round toward positive infinity.",

    stack_effect: "[ x ] -> [ ceil x ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const ROUND: RuntimeSpec = RuntimeSpec {
    name: "ROUND",
    category: "arithmetic",
    role: "Arithmetic primitive: Round to nearest integer (half-up).",

    stack_effect: "[ x ] -> [ round x ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const SQRT: RuntimeSpec = RuntimeSpec {
    name: "SQRT",
    category: "math",
    role: "The only Word that leaves the rationals: it produces the multiquadratic \u{221a}d.",
    stack_effect: "[ x ] -> [ sqrt(x) ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const ABS: RuntimeSpec = RuntimeSpec {
    name: "ABS",
    category: "math",
    role: "Magnitude without sign.",
    stack_effect: "[ x ] -> [ |x| ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const NEG: RuntimeSpec = RuntimeSpec {
    name: "NEG",
    category: "math",
    role: "Additive inverse.",
    stack_effect: "[ x ] -> [ -x ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const SIGN: RuntimeSpec = RuntimeSpec {
    name: "SIGN",
    category: "math",
    role: "Direction without magnitude.",
    stack_effect: "[ x ] -> [ sign ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const MIN: RuntimeSpec = RuntimeSpec {
    name: "MIN",
    category: "math",
    role: "Comparison-selected operand; comparison is total, so it always decides.",
    stack_effect: "[ a ] [ b ] -> [ min ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const MAX: RuntimeSpec = RuntimeSpec {
    name: "MAX",
    category: "math",
    role: "Comparison-selected operand; comparison is total, so it always decides.",
    stack_effect: "[ a ] [ b ] -> [ max ]",
    ..SPEC_DEFAULT
};
