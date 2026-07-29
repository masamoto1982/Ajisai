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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinExecutorKey {
    Abs,
    Neg,
    Sign,
    Min,
    Max,
    Sqrt,
    Sort,
    Unique,
    Contains,
    IndexOf,
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
    Map,
    Filter,
    Fold,
    Any,
    All,
    Get,
    Length,
    Concat,
    And,
    Or,
    Not,
    True,
    False,
    Nil,
    Exec,
    Cond,
    Def,
    Del,
    Lookup,
    Force,
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
    Fill,
    Floor,
    Ceil,
    Round,
    Mod,
    Str,
    Num,
    Chr,
    Chars,
    Join,
    Trim,
    Tokenize,
    Substitute,
    StartsWith,
    EndsWith,
    NilCheck,
    NilReason,
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
