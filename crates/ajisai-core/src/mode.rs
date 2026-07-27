//! Flow modes: `TOP`/`STAK` crossed with `EAT`/`KEEP`.
//!
//! A mode is not sugar for a stack shuffle. It changes how the *next* word
//! draws from the flow and what it leaves behind, and it does so in one place
//! — [`crate::interpreter`]'s operand layer — so that no individual word
//! carries a copy of the same four-way branch.
//!
//! The two axes are independent:
//!
//! * **Selection** — where the word draws from.
//!   * `TOP` (`.`): the surface. The word takes exactly the operands its
//!     contract declares, from the top of the flow.
//!   * `STAK` (`:`): the whole standing flow. A one-in word is applied to
//!     every cell; a two-in one-out word is folded left across every cell.
//! * **Retention** — what happens to what it drew.
//!   * `EAT` (`!`): the operands are consumed.
//!   * `KEEP` (`&`): the operands stay, and results are laid above them, so
//!     the flow branches instead of being swallowed.
//!
//! The default is `TOP EAT`, the mode every ordinary concatenative word
//! already has. Arming a mode word applies it to the next word only; after
//! that word runs, the mode returns to the default. Modes compose rather than
//! override: `STAK KEEP ADD` arms both axes for the single `ADD`.

use std::fmt;

/// Where the next word draws from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Selection {
    /// The surface of the flow.
    #[default]
    Top,
    /// The whole standing flow.
    Stak,
}

/// What happens to the operands the next word drew.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Retention {
    /// Consume them.
    #[default]
    Eat,
    /// Leave them standing and lay the results above.
    Keep,
}

/// The two axes together.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Mode {
    pub selection: Selection,
    pub retention: Retention,
}

impl Mode {
    pub const DEFAULT: Mode = Mode {
        selection: Selection::Top,
        retention: Retention::Eat,
    };

    /// All four combinations, for exhaustiveness tests and the manifest.
    pub const ALL: [Mode; 4] = [
        Mode {
            selection: Selection::Top,
            retention: Retention::Eat,
        },
        Mode {
            selection: Selection::Top,
            retention: Retention::Keep,
        },
        Mode {
            selection: Selection::Stak,
            retention: Retention::Eat,
        },
        Mode {
            selection: Selection::Stak,
            retention: Retention::Keep,
        },
    ];

    pub fn is_default(self) -> bool {
        self == Mode::DEFAULT
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let selection = match self.selection {
            Selection::Top => "TOP",
            Selection::Stak => "STAK",
        };
        let retention = match self.retention {
            Retention::Eat => "EAT",
            Retention::Keep => "KEEP",
        };
        write!(f, "{selection} {retention}")
    }
}
