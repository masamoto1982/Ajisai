//! # Ajisai Core
//!
//! Ajisai is a concatenative language in which exact values and vectors flow
//! left to right, and are observed, branched, and held at the points the
//! program names.
//!
//! The metaphor is load bearing, and every part of it cashes out in a rule:
//!
//! | Metaphor | Rule |
//! |---|---|
//! | the flow | evaluation order: values move left to right |
//! | the basin | the stack — the flow's current cross-section |
//! | `TOP` / `STAK` | where the next word draws from: the surface, or the whole standing flow |
//! | `EAT` / `KEEP` | whether the next word swallows what it drew, or leaves it standing and branches above it |
//! | `VENT` | release or block the next source unit, without evaluating it |
//! | `UNKNOWN` | the flow reached the gauge and did not settle |
//! | `NIL` | the flow arrived carrying no value |
//! | an error | the flow never formed |
//!
//! There is no word in the language whose only job is to sound like water.
//!
//! ## The shape of the crate
//!
//! * [`number`] — exact rationals. No floating point exists in the language.
//! * [`value`] — the six value shapes.
//! * [`role`] — the Semantic Plane, whose single canonical home is
//!   [`Value::role`](value::Value::role).
//! * [`k3`] — Strong Kleene logic.
//! * [`alias`] — the one alias table.
//! * [`mode`] — `TOP`/`STAK` × `EAT`/`KEEP`.
//! * [`syntax`] — source to program tree.
//! * [`contract`] — the machine-readable word contract.
//! * [`words`] — the vocabulary.
//! * [`interpreter`] — the one execution path, and the one place modes and
//!   `VENT` are implemented.
//! * [`lint`] — the contract lint, which reports and never proves.
//! * [`extension`] — the package surface. Ajisai Core knows of no package.
//!
//! ## Example
//!
//! ```
//! use ajisai_core::Interpreter;
//!
//! let mut ajisai = Interpreter::new();
//! ajisai.execute("1 3 DIV 3 MUL").unwrap();
//! assert_eq!(ajisai.stack()[0].to_string(), "1");
//! ```

pub mod alias;
pub mod contract;
pub mod error;
pub mod extension;
pub mod interpreter;
pub mod k3;
pub mod lint;
pub mod manifest;
pub mod mode;
pub mod number;
pub mod role;
pub mod syntax;
pub mod value;
pub mod words;

pub use error::{Error, Result};
pub use interpreter::Interpreter;
pub use k3::Truth;
pub use mode::{Mode, Retention, Selection};
pub use number::Number;
pub use role::Role;
pub use value::{Value, ValueData};

/// Render the flow's cross-section, bottom first — the canonical way to
/// observe a result.
pub fn render_stack(interpreter: &Interpreter) -> Vec<String> {
    interpreter
        .stack()
        .iter()
        .map(|value| value.to_string())
        .collect()
}
