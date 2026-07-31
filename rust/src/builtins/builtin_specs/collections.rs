//! Collection and tensor Word presentation metadata.
//!
//! Invariant: this module describes structure-oriented Words only; execution order is owned by the parent registry.

use super::super::builtin_word_definitions::{RuntimeSpec, SPEC_DEFAULT};
use crate::coreword_registry::{Partiality, SafetyLevel};

pub(in crate::builtins) const GET: RuntimeSpec = RuntimeSpec {
    name: "GET",
    category: "vector",
    role: "Random access into vectors and tensors.",

    stack_effect: "[ vec ] [ idx ] -> [ elem ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const INSERT: RuntimeSpec = RuntimeSpec {
    name: "INSERT",
    category: "vector",
    role: "Extends a vector by inserting an element at the indicated position.",

    stack_effect: "[ vec ] [ idx val ] -> [ vec' ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const REPLACE: RuntimeSpec = RuntimeSpec {
    name: "REPLACE",
    category: "vector",
    role: "In-place style update of a vector element.",

    stack_effect: "[ vec ] [ idx val ] -> [ vec' ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const REMOVE: RuntimeSpec = RuntimeSpec {
    name: "REMOVE",
    category: "vector",
    role: "Shrinks a vector by deleting one element.",

    stack_effect: "[ vec ] [ idx ] -> [ vec' ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const LENGTH: RuntimeSpec = RuntimeSpec {
    name: "LENGTH",
    category: "vector",
    role: "Vector primitive: Return the number of elements in a vector.",

    stack_effect: "[ vec ] -> [ count ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const TAKE: RuntimeSpec = RuntimeSpec {
    name: "TAKE",
    category: "vector",
    role: "Vector primitive: Take the first N or last -N elements of a vector.",

    stack_effect: "[ vec ] [ n ] -> [ prefix ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const SPLIT: RuntimeSpec = RuntimeSpec {
    name: "SPLIT",
    category: "vector",
    role: "Vector primitive: Split a vector into chunks at the specified sizes.",

    stack_effect: "[ vec ] [ sizes ] -> [ chunks... ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const CONCAT: RuntimeSpec = RuntimeSpec {
    name: "CONCAT",
    category: "vector",
    role: "Vector primitive: Flatten and concatenate two vectors.",

    stack_effect: "[ a ] [ b ] -> [ a ++ b ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const REVERSE: RuntimeSpec = RuntimeSpec {
    name: "REVERSE",
    category: "vector",
    role: "Vector primitive: Reverse the order of vector elements.",

    stack_effect: "[ vec ] -> [ reversed ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const RANGE: RuntimeSpec = RuntimeSpec {
    name: "RANGE",
    category: "vector",
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

pub(in crate::builtins) const REORDER: RuntimeSpec = RuntimeSpec {
    name: "REORDER",
    category: "vector",
    role: "Vector primitive: Reorder vector elements according to an index permutation.",

    stack_effect: "[ vec ] [ indices ] -> [ permuted ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const COLLECT: RuntimeSpec = RuntimeSpec {
    name: "COLLECT",
    category: "vector",
    role: "Vector primitive: Collect N items off the stack into a new vector.",

    stack_effect: "v1 ... vn n -> [ [ v1 ... vn ] ]",
    partiality: Partiality::Partial,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const FILL: RuntimeSpec = RuntimeSpec {
    name: "FILL",
    category: "tensor",
    role: "Tensor primitive: Fill a target shape with a constant value.",

    stack_effect: "[ shape... value ] -> [ filled ]",
    // Projecting/CreatesNil for the space-budget miss: a well-formed but
    // over-budget (or product-overflowing) shape projects onto Bubble/NIL
    // (SPEC §7.14, §11.2). A malformed shape remains an ordinary error.
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const SORT: RuntimeSpec = RuntimeSpec {
    name: "SORT",
    category: "vector",
    role: "Total ordering over exact scalars.",
    stack_effect: "[ vec ] -> [ sorted ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const UNIQUE: RuntimeSpec = RuntimeSpec {
    name: "UNIQUE",
    category: "vector",
    role: "Set-like reduction that preserves order.",
    stack_effect: "[ vec ] -> [ vec ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const CONTAINS: RuntimeSpec = RuntimeSpec {
    name: "CONTAINS",
    category: "vector",
    role: "Membership as a definite truth value.",
    stack_effect: "[ vec ] [ x ] -> [ bool ]",
    ..SPEC_DEFAULT
};

pub(in crate::builtins) const INDEX_OF: RuntimeSpec = RuntimeSpec {
    name: "INDEX-OF",
    category: "vector",
    role: "Search that projects to NIL when the value is absent.",
    stack_effect: "[ vec ] [ x ] -> [ idx ]",
    partiality: Partiality::Projecting,
    safety_level: SafetyLevel::B,
    ..SPEC_DEFAULT
};
