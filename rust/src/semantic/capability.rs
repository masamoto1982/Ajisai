#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    Numeric,
    ExactNumeric,
    Iterable,
    Indexable,
    Callable,
    StackItem,
    NilPassthrough,
    Diagnosable,
    Serializable,
    Displayable,
    UserEditable,
    ModuleOwned,
    CoreOwned,
    AiExplainable,
    /// The value is a three-valued truth value (true / false / unknown,
    /// SPEC §7.5). Signals consumers to read the `truthValue` axis.
    TruthValued,
}
