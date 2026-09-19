#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticKind {
    Number,
    Collection,
    Code,
    Absence,
    /// A keyed correspondence (LANG.RECORDS.STRUCTURE).
    Record,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueShape {
    Scalar,
    Vector,
    Tensor,
    CodeBlock,
    Absence,
    Record,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueOrigin {
    Literal,
    NilPropagation,
    HostEnvironment,
    Unknown,
}
