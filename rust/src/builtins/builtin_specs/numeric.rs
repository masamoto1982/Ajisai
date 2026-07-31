//! Arithmetic, comparison, and exact-math Word presentation metadata.
//!
//! Invariant: this table is prose/runtime-local classification only; canonical contracts come from `spec/words.json`.

use super::super::builtin_word_definitions::{BuiltinSpec, SPEC_DEFAULT};
use crate::coreword_registry::{Partiality, SafetyLevel};

pub(in crate::builtins) const ADD: BuiltinSpec = BuiltinSpec {
    name: "ADD",
    category: "arithmetic",
    hover_summary: "ADD — add values",
    hover_syntax: "1 2 +",
    summary: "Add two numeric values, element-wise with broadcasting.",
    role: "Numeric addition; one of the four arithmetic primitives.",

    stack_effect: "[ a ] [ b ] -> [ a + b ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const SUB: BuiltinSpec = BuiltinSpec {
    name: "SUB",
    category: "arithmetic",
    hover_summary: "SUB — subtract values",
    hover_syntax: "5 3 -",
    summary: "Subtract two numeric values, element-wise with broadcasting.",
    role: "Arithmetic primitive: Subtract two numeric values, element-wise with broadcasting.",

    stack_effect: "[ a ] [ b ] -> [ a - b ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const MUL: BuiltinSpec = BuiltinSpec {
    name: "MUL",
    category: "arithmetic",
    hover_summary: "MUL — multiply values",
    hover_syntax: "2 4 *",
    summary: "Multiply two numeric values, element-wise with broadcasting.",
    role: "Arithmetic primitive: Multiply two numeric values, element-wise with broadcasting.",

    stack_effect: "[ a ] [ b ] -> [ a * b ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const DIV: BuiltinSpec = BuiltinSpec {
    name: "DIV",
    category: "arithmetic",
    hover_summary: "DIV — divide values",
    hover_syntax: "10 2 /",
    summary: "Divide two numeric values exactly (fractional result).",
    role: "Arithmetic primitive: Divide two numeric values exactly (fractional result).",

    stack_effect: "[ a ] [ b ] -> [ a / b ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const EQ: BuiltinSpec = BuiltinSpec {
    name: "EQ",
    category: "comparison",
    hover_summary: "EQ — test equality",
    hover_syntax: "1 1 =",
    summary: "Test equality of two values.",
    role: "Comparison primitive: Test equality of two values.",

    stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const LT: BuiltinSpec = BuiltinSpec {
    name: "LT",
    category: "comparison",
    hover_summary: "LT — test less than",
    hover_syntax: "1 2 <",
    summary: "Test less-than comparison.",
    role: "Comparison primitive: Test less-than comparison.",

    stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const LTE: BuiltinSpec = BuiltinSpec {
    name: "LTE",
    category: "comparison",
    hover_summary: "LTE — test less than or equal",
    hover_syntax: "1 1 <=",
    summary: "Test less-than-or-equal comparison.",
    role: "Comparison primitive: Test less-than-or-equal comparison.",

    stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const GT: BuiltinSpec = BuiltinSpec {
    name: "GT",
    category: "comparison",
    hover_summary: "GT — test greater than",
    hover_syntax: "2 1 >",
    summary: "Test greater-than comparison.",
    role: "Comparison primitive: Test greater-than comparison.",

    stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const GTE: BuiltinSpec = BuiltinSpec {
    name: "GTE",
    category: "comparison",
    hover_summary: "GTE — test greater than or equal",
    hover_syntax: "1 1 >=",
    summary: "Test greater-than-or-equal comparison.",
    role: "Comparison primitive: Test greater-than-or-equal comparison.",

    stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const NEQ: BuiltinSpec = BuiltinSpec {
    name: "NEQ",
    category: "comparison",
    hover_summary: "NEQ — test inequality",
    hover_syntax: "1 2 <>",
    summary: "Test inequality of two values.",
    role: "Comparison primitive: Test inequality of two values.",

    stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const MOD: BuiltinSpec = BuiltinSpec {
    name: "MOD",
    category: "arithmetic",
    hover_summary: "MOD — modulo",
    hover_syntax: "7 3 %",
    summary: "Modulo (remainder) of two numeric values.",
    role: "Arithmetic primitive: Modulo (remainder) of two numeric values.",

    stack_effect: "[ a ] [ b ] -> [ a mod b ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const FLOOR: BuiltinSpec = BuiltinSpec {
    name: "FLOOR",
    category: "arithmetic",
    hover_summary: "FLOOR — round toward negative infinity",
    hover_syntax: "[ 7/3 ] FLOOR",
    summary: "Round toward negative infinity.",
    role: "Arithmetic primitive: Round toward negative infinity.",

    stack_effect: "[ x ] -> [ floor x ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const CEIL: BuiltinSpec = BuiltinSpec {
    name: "CEIL",
    category: "arithmetic",
    hover_summary: "CEIL — round toward positive infinity",
    hover_syntax: "[ 7/3 ] CEIL",
    summary: "Round toward positive infinity.",
    role: "Arithmetic primitive: Round toward positive infinity.",

    stack_effect: "[ x ] -> [ ceil x ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const ROUND: BuiltinSpec = BuiltinSpec {
    name: "ROUND",
    category: "arithmetic",
    hover_summary: "ROUND — round to nearest integer",
    hover_syntax: "[ 5/2 ] ROUND",
    summary: "Round to nearest integer (half-up).",
    role: "Arithmetic primitive: Round to nearest integer (half-up).",

    stack_effect: "[ x ] -> [ round x ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const SQRT: BuiltinSpec = BuiltinSpec {
    name: "SQRT",
    category: "math",
    hover_summary: "SQRT — exact square root",
    hover_syntax: "2 SQRT",
    summary: "Exact square root of a non-negative rational.",
    role: "The only Word that leaves the rationals: it produces the multiquadratic \u{221a}d.",
    stack_effect: "[ x ] -> [ sqrt(x) ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const ABS: BuiltinSpec = BuiltinSpec {
    name: "ABS",
    category: "math",
    hover_summary: "Absolute value of a number.",
    hover_syntax: "-2 ABS",
    summary: "Absolute value of a number.",
    role: "Magnitude without sign.",
    stack_effect: "[ x ] -> [ |x| ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const NEG: BuiltinSpec = BuiltinSpec {
    name: "NEG",
    category: "math",
    hover_summary: "Numeric negation.",
    hover_syntax: "2 NEG",
    summary: "Numeric negation.",
    role: "Additive inverse.",
    stack_effect: "[ x ] -> [ -x ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const SIGN: BuiltinSpec = BuiltinSpec {
    name: "SIGN",
    category: "math",
    hover_summary: "Sign of a number: -1, 0, or 1.",
    hover_syntax: "-2 SIGN",
    summary: "Sign of a number: -1, 0, or 1.",
    role: "Direction without magnitude.",
    stack_effect: "[ x ] -> [ sign ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const MIN: BuiltinSpec = BuiltinSpec {
    name: "MIN",
    category: "math",
    hover_summary: "Smaller of two numbers.",
    hover_syntax: "1 2 MIN",
    summary: "Smaller of two numbers.",
    role: "Comparison-selected operand; comparison is total, so it always decides.",
    stack_effect: "[ a ] [ b ] -> [ min ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const MAX: BuiltinSpec = BuiltinSpec {
    name: "MAX",
    category: "math",
    hover_summary: "Larger of two numbers.",
    hover_syntax: "1 2 MAX",
    summary: "Larger of two numbers.",
    role: "Comparison-selected operand; comparison is total, so it always decides.",
    stack_effect: "[ a ] [ b ] -> [ max ]",
    ..SPEC_DEFAULT
};
