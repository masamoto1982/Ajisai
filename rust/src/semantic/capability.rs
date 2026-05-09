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
}
