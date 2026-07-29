#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Rough cost class of evaluating a Word. Advisory metadata on the contract;
/// it is diagnostics, never a semantic discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalCost {
    /// Constant-time arithmetic / boolean ops.
    Trivial,
    /// Small fixed-overhead operations (casts, single-element lookups).
    Light,
    /// Linear collection traversals.
    Medium,
    /// Unbounded or recursive operations.
    Heavy,
}

pub enum BuiltinExecutorKey {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Lt,
    Le,
    Gt,
    Gte,
    Neq,
    CompareWithin,
    Map,
    Filter,
    Fold,
    Unfold,
    Any,
    All,
    Count,
    Scan,
    Get,
    Length,
    Concat,
    And,
    Or,
    Not,
    True,
    False,
    Nil,
    Idle,
    Exec,
    Eval,
    OrElse,
    Cond,
    Conserve,
    Def,
    Del,
    Lookup,
    Import,
    ImportOnly,
    Unimport,
    UnimportOnly,
    Force,
    ToCf,
    Print,
    Insert,
    Replace,
    Remove,
    Take,
    Split,
    Reverse,
    Range,
    Reorder,
    Collect,
    Shape,
    Rank,
    Reshape,
    Transpose,
    Fill,
    Floor,
    Ceil,
    Round,
    Quantize,
    QuantizeHalfAway,
    QuantizeFloor,
    QuantizeCeil,
    QuantizeTrunc,
    Mod,
    Str,
    Num,
    Bool,
    Chr,
    Chars,
    Join,
    Trim,
    TrimLeft,
    TrimRight,
    Tokenize,
    Substitute,
    StartsWith,
    EndsWith,
    Spawn,
    Await,
    Status,
    Kill,
    Monitor,
    Supervise,
    Precompute,
    NilCheck,
    NilReason,
    NilOrigin,
    NilRecoverable,
    NilDiagnosis,
}

// WordShape classifies how a word treats its data argument. Used by
// module words (see ModuleWord::word_shape) to feed future
// vector-pipeline planning. `Fold` and `Other` are not produced by
// any current module spec but are reserved for completeness of the
// classification and to keep planning code able to pattern-match all
// variants without `_ =>` catch-alls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum WordShape {
    Map,
    Form,
    Fold,
    Other,
}
