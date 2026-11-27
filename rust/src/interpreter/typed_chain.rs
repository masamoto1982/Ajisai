//! A typed Word's `MAP` feeding a `FOLD` or `SCAN`, run as one walk
//! (LANG.CONTRACT.TYPES).
//!
//! In the body of a Word with a declared signature, the five ops of
//! `[ f ] MAP seed [ g ] FOLD` (or `SCAN`) are compiled to one [`Chain`].
//! When both blocks are in the fused subset and the column tier takes both
//! walks, the chain runs them together a chunk at a time, so the mapped
//! Vector is never built; it charges the two Words' steps and exactly what
//! the two walks would. Whenever either walk alone would decline — a value
//! past 64 bits part way, a block outside the subset, a limit — nothing has
//! been touched, and the five ops run as written. So which route ran is
//! unobservable (LANG.AUTHORITY.FREEDOM), and erasing the signature, which
//! leaves the five ops, answers the same values and meters.

use super::engine::Op;
use super::fused_block::Plain;
use super::higher_order::extract_executable_code;
use super::Interpreter;
use crate::kernel::generated::WordId;

#[derive(Debug, Clone)]
pub(crate) struct Chain {
    /// `[ f ]`, `MAP`, the seed, `[ g ]`, and `FOLD` or `SCAN`, as compiled.
    pub(crate) ops: [Op; 5],
    scan: bool,
}

/// Whether `ops` starts with a chain, and if so whether it ends in `SCAN`.
fn chain_at(ops: &[Option<Op>]) -> Option<bool> {
    let word = |k: usize| match ops.get(k) {
        Some(Some(Op::Word(word))) => Some(word.id),
        _ => None,
    };
    let bracket = |k: usize| matches!(ops.get(k), Some(Some(Op::Bracket { .. })));
    let push = matches!(ops.get(2), Some(Some(Op::Push(_))));
    if !(bracket(0) && word(1) == Some(WordId::Map) && push && bracket(3)) {
        return None;
    }
    match word(4)? {
        WordId::Fold => Some(false),
        WordId::Scan => Some(true),
        _ => None,
    }
}

/// A typed body's ops with each chain folded into one op.
pub(crate) fn chain_walks(ops: Vec<Op>) -> Vec<Op> {
    let mut slots: Vec<Option<Op>> = ops.into_iter().map(Some).collect();
    let mut out = Vec::with_capacity(slots.len());
    let mut i = 0;
    while i < slots.len() {
        match chain_at(&slots[i..]) {
            Some(scan) => {
                let mut take = |k: usize| slots[i + k].take().expect("matched above");
                let ops = [take(0), take(1), take(2), take(3), take(4)];
                out.push(Op::Chain(Box::new(Chain { ops, scan })));
                i += 5;
            }
            None => {
                out.push(slots[i].take().expect("each op is taken once"));
                i += 1;
            }
        }
    }
    out
}

impl Interpreter {
    /// Run `chain` as one walk, as the five ops would leave the stack and
    /// meters, or answer `false` with nothing touched.
    pub(crate) fn chain_in_place(&mut self, chain: &Chain) -> bool {
        if !self.scalar_fastpath_enabled || !self.fused_block_enabled {
            return false;
        }
        let (
            Op::Bracket {
                value: map_block,
                names_words: map_names,
                ..
            },
            Op::Push(seed),
            Op::Bracket {
                value: fold_block,
                names_words: fold_names,
                ..
            },
        ) = (&chain.ops[0], &chain.ops[2], &chain.ops[3])
        else {
            return false;
        };
        let Some(seed) = Plain::of(seed) else {
            return false;
        };
        let (Ok(map_code), Ok(fold_code)) = (
            extract_executable_code(self, map_block),
            extract_executable_code(self, fold_block),
        ) else {
            return false;
        };
        let (Some(map), Some(fold)) = (map_code.fused(self, 1), fold_code.fused(self, 2)) else {
            return false;
        };
        if map.has_lanes() || fold.has_lanes() {
            return false;
        }
        let Some(target) = self.stack.last() else {
            return false;
        };
        let Some((value, charges)) =
            super::fused_block_columns::run_chain(&map, &fold, self, target, &seed, chain.scan)
        else {
            return false;
        };
        // The two Words, then their walks.
        self.execution_step_count += charges.steps + 2;
        self.numeric_work_used += charges.work;
        self.stack.pop();
        self.stack.push(value);
        for (block, names) in [(map_block, map_names), (fold_block, fold_names)] {
            if *names {
                self.code_literals.push(block.clone());
            }
        }
        self.pending_def_body_tokens = None;
        #[cfg(test)]
        CHAIN_RUNS.with(|c| c.set(c.get() + 1));
        true
    }
}

#[cfg(test)]
thread_local! {
    static CHAIN_RUNS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Chains run as one walk on this thread, for tests that pin the route.
#[cfg(test)]
pub(crate) fn chain_runs_on_this_thread() -> u64 {
    CHAIN_RUNS.with(|c| c.get())
}
