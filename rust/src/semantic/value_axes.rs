#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticKind {
    Number,
    Collection,
    Record,
    Code,
    Process,
    Supervisor,
    Absence,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueShape {
    Scalar,
    Vector,
    Tensor,
    Record,
    CodeBlock,
    Handle,
    Absence,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueOrigin {
    Literal,
    Computed,
    CoreWord,
    BuiltinWord,
    ModuleWord { module: Option<String> },
    UserWord,
    SafeProjection,
    NilPropagation,
    HostEnvironment,
    Optimizer,
    Unknown,
}
