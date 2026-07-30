#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticKind {
    Number,
    Collection,
    Code,
    Absence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueShape {
    Scalar,
    Vector,
    Tensor,
    CodeBlock,
    Absence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueOrigin {
    Literal,
    NilPropagation,
    HostEnvironment,
    Unknown,
}
