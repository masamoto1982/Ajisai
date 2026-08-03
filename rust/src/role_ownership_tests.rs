//! CS3 (ownership): the `Stack` is the sole authority for top-level roles, and
//! every save/restore boundary carries roles with values in lockstep.
//!
//! These drive the interpreter and observe through the shared `(value, role)`
//! rendering (SPEC §12), so they exercise the real save/restore paths that were
//! migrated from the parallel `SemanticStack` snapshot onto a `Stack` clone: a
//! interpretation role applied to a slot *below* an isolated-stack word must
//! survive that word with its role intact.
