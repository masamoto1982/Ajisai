//! The share of the run's `numericWork` a square root may spend factoring its
//! radicand (`types::exact::squarefree`).
//!
//! A radicand is reduced to its square-free part so that one number has one
//! normal form (LANG.VALUES.DENOTATION). That needs a factorization, whose cost
//! is not bounded by anything the operand's size alone predicts, so it is
//! metered on the same budget as the rest of the arithmetic: the root is taken
//! against what the run has left, the work actually spent is charged
//! afterwards, and a radicand the budget cannot factor is the same
//! `resourceLimitExceeded` any other exhausted work meter raises.

use std::cell::Cell;

use crate::error::{AjisaiError, Result};
use crate::interpreter::Interpreter;
use crate::types::exact::ExactReal;
use crate::types::fraction::Fraction;

pub(crate) struct RadicandBudget {
    limit: u64,
    start: u64,
    remaining: Cell<u64>,
    exhausted: Cell<bool>,
}

impl RadicandBudget {
    pub(crate) fn of(interp: &Interpreter) -> Self {
        let start = interp
            .runtime_limits
            .max_numeric_work
            .saturating_sub(interp.numeric_work_used);
        RadicandBudget {
            limit: interp.runtime_limits.max_numeric_work,
            start,
            remaining: Cell::new(start),
            exhausted: Cell::new(false),
        }
    }

    /// The budget left for one root, taken back by [`Self::spent`].
    pub(crate) fn take(&self) -> u64 {
        self.remaining.get()
    }

    /// Record what one root left of the budget it was given, and whether it
    /// ran out.
    pub(crate) fn spent(&self, left: u64, exhausted: bool) {
        self.remaining.set(left);
        if exhausted {
            self.exhausted.set(true);
        }
    }

    /// √`radicand` within the budget, or the exhausted meter's error.
    pub(crate) fn sqrt(&self, radicand: Fraction) -> Result<Option<ExactReal>> {
        let mut left = self.take();
        let root = ExactReal::try_sqrt_rational(radicand, &mut left);
        self.spent(left, root.is_err());
        root.map_err(|_| self.exhausted_error())
    }

    /// The error the work meter raises when this root's factorization used
    /// up what the run had left.
    pub(crate) fn exhausted_error(&self) -> AjisaiError {
        AjisaiError::ResourceLimitExceeded {
            resource: crate::error::ResourceLimit::NumericWork,
            limit: self.limit,
            observed: Some(self.limit.saturating_add(1)),
            progress: None,
        }
    }

    /// Charge the work spent to the run's meter; an exhausted budget charges
    /// everything that was left and one unit more.
    pub(crate) fn settle(&self, interp: &mut Interpreter) -> Result<()> {
        if self.exhausted.get() {
            return interp.charge_numeric_work(self.start.saturating_add(1));
        }
        interp.charge_numeric_work(self.start - self.remaining.get())
    }
}
