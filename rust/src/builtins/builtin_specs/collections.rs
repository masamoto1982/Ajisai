//! Collection and tensor Word presentation metadata.
//!
//! Invariant: this module describes structure-oriented Words only; execution order is owned by the parent registry.

use super::super::builtin_word_definitions::{BuiltinSpec, SPEC_DEFAULT};
use crate::coreword_registry::{Partiality, SafetyLevel};

pub(in crate::builtins) const GET: BuiltinSpec = BuiltinSpec {
    name: "GET",
    category: "vector",
    hover_summary: "GET — extract element at index",
    hover_syntax: "[ 10 20 30 ] [ 0 ] GET",
    summary: "Extract one element of a vector by index.",
    role: "Random access into vectors and tensors.",

    stack_effect: "[ vec ] [ idx ] -> [ elem ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const INSERT: BuiltinSpec = BuiltinSpec {
    name: "INSERT",
    category: "vector",
    hover_summary: "INSERT — insert element at index",
    hover_syntax: "[ 1 3 ] [ 1 2 ] INSERT",
    summary: "Insert a value at a given index in a vector.",
    role: "Extends a vector by inserting an element at the indicated position.",

    stack_effect: "[ vec ] [ idx val ] -> [ vec' ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const REPLACE: BuiltinSpec = BuiltinSpec {
    name: "REPLACE",
    category: "vector",
    hover_summary: "REPLACE — replace element at index",
    hover_syntax: "[ 1 2 3 ] [ 0 9 ] REPLACE",
    summary: "Replace an element of a vector at a given index.",
    role: "In-place style update of a vector element.",

    stack_effect: "[ vec ] [ idx val ] -> [ vec' ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const REMOVE: BuiltinSpec = BuiltinSpec {
    name: "REMOVE",
    category: "vector",
    hover_summary: "REMOVE — remove element at index",
    hover_syntax: "[ 1 2 3 ] [ 0 ] REMOVE",
    summary: "Remove an element from a vector at a given index.",
    role: "Shrinks a vector by deleting one element.",

    stack_effect: "[ vec ] [ idx ] -> [ vec' ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const LENGTH: BuiltinSpec = BuiltinSpec {
    name: "LENGTH",
    category: "vector",
    hover_summary: "LENGTH — return element count",
    hover_syntax: "[ 1 2 3 ] LENGTH",
    summary: "Return the number of elements in a vector.",
    role: "Vector primitive: Return the number of elements in a vector.",

    stack_effect: "[ vec ] -> [ count ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const TAKE: BuiltinSpec = BuiltinSpec {
    name: "TAKE",
    category: "vector",
    hover_summary: "TAKE — take N elements from start or end",
    hover_syntax: "[ 1 2 3 4 5 ] [ 3 ] TAKE",
    summary: "Take the first N or last -N elements of a vector.",
    role: "Vector primitive: Take the first N or last -N elements of a vector.",

    stack_effect: "[ vec ] [ n ] -> [ prefix ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const SPLIT: BuiltinSpec = BuiltinSpec {
    name: "SPLIT",
    category: "vector",
    hover_summary: "SPLIT — split vector at sizes",
    hover_syntax: "[ 1 2 3 4 ] [ 2 2 ] SPLIT",
    summary: "Split a vector into chunks at the specified sizes.",
    role: "Vector primitive: Split a vector into chunks at the specified sizes.",

    stack_effect: "[ vec ] [ sizes ] -> [ chunks... ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const CONCAT: BuiltinSpec = BuiltinSpec {
    name: "CONCAT",
    category: "vector",
    hover_summary: "CONCAT — flatten and concatenate vectors",
    hover_syntax: "[ 1 2 ] [ 3 4 ] CONCAT",
    summary: "Flatten and concatenate two vectors.",
    role: "Vector primitive: Flatten and concatenate two vectors.",

    stack_effect: "[ a ] [ b ] -> [ a ++ b ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const REVERSE: BuiltinSpec = BuiltinSpec {
    name: "REVERSE",
    category: "vector",
    hover_summary: "REVERSE — reverse element order",
    hover_syntax: "[ 1 2 3 ] REVERSE",
    summary: "Reverse the order of vector elements.",
    role: "Vector primitive: Reverse the order of vector elements.",

    stack_effect: "[ vec ] -> [ reversed ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const RANGE: BuiltinSpec = BuiltinSpec {
    name: "RANGE",
    category: "vector",
    hover_summary: "RANGE — generate numeric sequence",
    hover_syntax: "[ 0 5 ] RANGE",
    summary: "Generate a numeric sequence from a [start, end] pair.",
    role: "Vector primitive: Generate a numeric sequence from a [start, end] pair.",

    stack_effect: "[ start end ] -> [ seq ]",
    // Projecting/CreatesNil for the space-budget miss: a well-formed but
    // over-budget range projects onto Bubble/NIL (SPEC §7.14, §11.2),
    // matching the DIV/GET/NUM/CHR family. Malformed ranges (zero step,
    // infinite direction) remain ordinary errors.
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const REORDER: BuiltinSpec = BuiltinSpec {
    name: "REORDER",
    category: "vector",
    hover_summary: "REORDER — reorder by index list",
    hover_syntax: "[ 'a' 'b' 'c' ] [ 2 0 1 ] REORDER",
    summary: "Reorder vector elements according to an index permutation.",
    role: "Vector primitive: Reorder vector elements according to an index permutation.",

    stack_effect: "[ vec ] [ indices ] -> [ permuted ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const COLLECT: BuiltinSpec = BuiltinSpec {
    name: "COLLECT",
    category: "vector",
    hover_summary: "COLLECT — collect N items into vector",
    hover_syntax: "1 2 3 3 COLLECT",
    summary: "Collect N items off the stack into a new vector.",
    role: "Vector primitive: Collect N items off the stack into a new vector.",

    stack_effect: "v1 ... vn n -> [ [ v1 ... vn ] ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const FILL: BuiltinSpec = BuiltinSpec {
    name: "FILL",
    category: "tensor",
    hover_summary: "FILL — fill shape with value",
    hover_syntax: "[ 2 2 0 ] FILL",
    summary: "Fill a target shape with a constant value.",
    role: "Tensor primitive: Fill a target shape with a constant value.",

    stack_effect: "[ shape... value ] -> [ filled ]",
    // Projecting/CreatesNil for the space-budget miss: a well-formed but
    // over-budget (or product-overflowing) shape projects onto Bubble/NIL
    // (SPEC §7.14, §11.2). A malformed shape remains an ordinary error.
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const SORT: BuiltinSpec = BuiltinSpec {
    name: "SORT",
    category: "vector",
    hover_summary: "Return a copy of a vector sorted in ascending order.",
    hover_syntax: "[ 3 1 2 ] SORT",
    summary: "Return a copy of a vector sorted in ascending order.",
    role: "Total ordering over exact scalars.",
    stack_effect: "[ vec ] -> [ sorted ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const UNIQUE: BuiltinSpec = BuiltinSpec {
    name: "UNIQUE",
    category: "vector",
    hover_summary: "UNIQUE — remove duplicates, keeping first-occurrence order",
    hover_syntax: "[ 1 2 1 ] UNIQUE",
    summary:
        "Return a copy of a vector with duplicates removed, preserving first-occurrence order.",
    role: "Set-like reduction that preserves order.",
    stack_effect: "[ vec ] -> [ vec ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const CONTAINS: BuiltinSpec = BuiltinSpec {
    name: "CONTAINS",
    category: "vector",
    hover_summary: "True if a vector contains an element equal to the given value.",
    hover_syntax: "[ 1 2 ] 2 CONTAINS",
    summary: "True if a vector contains an element equal to the given value.",
    role: "Membership as a definite truth value.",
    stack_effect: "[ vec ] [ x ] -> [ bool ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const INDEX_OF: BuiltinSpec = BuiltinSpec {
    name: "INDEX-OF",
    category: "vector",
    hover_summary: "Index of the first element equal to the value; Bubble/NIL if absent.",
    hover_syntax: "[ 1 2 ] 2 INDEX-OF",
    summary: "Index of the first element equal to the value; Bubble/NIL if absent.",
    role: "Search that projects to NIL when the value is absent.",
    stack_effect: "[ vec ] [ x ] -> [ idx ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};
