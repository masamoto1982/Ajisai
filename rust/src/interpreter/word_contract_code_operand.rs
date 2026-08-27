//! Resolves a `[ ... ]` code operand's Symbols into the enclosing Word's
//! contract, split from `word_contract.rs` to keep that file under the
//! per-file line budget (§14.1). The algorithm itself is unchanged: this is
//! the same resolve-then-widen step `infer_word_contract_inner` already runs
//! for an ordinary body-level dependency, applied instead to a Symbol found
//! inside a literal that `word_contract_widen.rs` classified as
//! `LiteralContext::Code`.

use std::collections::HashSet;
use std::sync::Arc;

use crate::agent::contract_gap::GapCode;

use super::word_contract::{static_word_contract, AccumulatedContract, WordContract};
use super::Interpreter;

impl Interpreter {
    /// Resolve `symbol` and widen `acc` with its contract, for a Symbol found
    /// inside a `[ ... ]` classified `LiteralContext::Code` (`word_contract_
    /// widen.rs`) — a fixed-position code operand the enclosing call (`MAP`,
    /// `EXEC`, ...) will actually run, unlike an ordinary data literal. Only
    /// `acc` is touched: arity/space/cost already treated the whole literal
    /// as one opaque value (the caller's `flow`/`sim`/`cost_sim.feed_literal`
    /// calls), and stay attributed at that Word's own call site rather than
    /// unrolled here, exactly as for a ordinary body-level dependency's own
    /// internal cost.
    pub(crate) fn widen_with_code_operand_symbol(
        &mut self,
        symbol: &str,
        visiting: &mut HashSet<String>,
        acc: &mut AccumulatedContract,
        complete: &mut bool,
    ) {
        let canonical = crate::core_word_aliases::canonicalize_core_word_name(symbol);
        let Some((dep_name, dep_def)) = self.resolve_word_entry(&canonical) else {
            *complete = false;
            acc.gaps.push(GapCode::UnresolvedWord);
            return;
        };
        let dep_contract = if dep_def.is_builtin {
            Arc::new(static_word_contract(&dep_name, &dep_def))
        } else if visiting.contains(&dep_name) {
            *complete = false;
            acc.gaps.push(GapCode::RecursiveDependency);
            let mut placeholder =
                WordContract::conservative(self.contract_cache_key(&dep_name, &dep_def));
            placeholder.gaps.clear();
            Arc::new(placeholder)
        } else {
            match self.infer_word_contract_inner(&dep_name, &dep_def, visiting) {
                Some(contract) => contract,
                None => {
                    *complete = false;
                    acc.gaps.push(GapCode::DependencyUnknown);
                    return;
                }
            }
        };
        acc.widen_with(&dep_contract);
    }
}
