//! `RegProgram::run_accumulating`, the loop `FOLD` and `SCAN` run a block's
//! register code in: each run needs the last one's accumulator, so the block
//! is split, once when it is lowered, into what reads the accumulator and
//! what does not.

use super::{apply, exec_column, Instr, Kind, RegProgram, Src, CHUNK};
use crate::types::small_rational::overflowing_mul;

/// A two-input block split for `run_accumulating`.
#[derive(Debug, Clone)]
pub(super) struct AccPlan {
    /// The instructions that never read the accumulator, run a column at a
    /// time.
    free: Vec<Instr>,
    /// The rest, with operands resolved, run once per element.
    per_run: Vec<(Kind, usize, [Lane; 3])>,
    out: Lane,
    /// `acc = acc op x` (or `x op acc` when flipped), when that is all that
    /// reads the accumulator.
    single: Option<(Kind, bool, Lane)>,
}

/// `acc = op(acc, x)` for each lane, in a loop with nothing else in it.
#[inline(always)]
fn fold_with(
    acc: &mut i64,
    lanes: &[i64],
    each: &mut impl FnMut(i64),
    op: impl Fn(i64, i64) -> (i64, bool),
) -> bool {
    for &x in lanes {
        let (v, bad) = op(*acc, x);
        if bad {
            return false;
        }
        *acc = v;
        each(v);
    }
    true
}

/// The same against a constant operand, which is rare enough to dispatch
/// per lane.
fn fold_lanes(
    acc: &mut i64,
    lanes: impl Iterator<Item = i64>,
    kind: Kind,
    flipped: bool,
    each: &mut impl FnMut(i64),
) -> bool {
    for x in lanes {
        let (a, b) = if flipped { (x, *acc) } else { (*acc, x) };
        let (v, bad) = apply(kind, a, b, 0);
        if bad {
            return false;
        }
        *acc = v;
        each(v);
    }
    true
}

/// An operand of an instruction that reads the accumulator, resolved for
/// `run_accumulating`: a per-run scalar register, a lane of a column the
/// accumulator-free instructions filled, or a constant.
#[derive(Debug, Clone, Copy)]
enum Lane {
    Scalar(usize),
    Column(usize),
    Const(i64),
}

impl RegProgram {
    pub(super) fn accumulating_plan(&self) -> AccPlan {
        let mut reads_acc = vec![false; self.regs];
        reads_acc[0] = true;
        let reads = |reads_acc: &[bool], s: Src| matches!(s, Src::Reg(r) if reads_acc[r]);
        for ins in &self.instrs {
            reads_acc[ins.dst] =
                reads(&reads_acc, ins.a) || reads(&reads_acc, ins.b) || reads(&reads_acc, ins.c);
        }
        let (free, per_run): (Vec<Instr>, Vec<Instr>) =
            self.instrs.iter().partition(|ins| !reads_acc[ins.dst]);
        let lane = |s: Src| match s {
            Src::Reg(r) if reads_acc[r] => Lane::Scalar(r),
            Src::Reg(r) => Lane::Column(r),
            Src::Const(v) => Lane::Const(v),
        };
        let per_run: Vec<(Kind, usize, [Lane; 3])> = per_run
            .iter()
            .map(|ins| (ins.kind, ins.dst, [lane(ins.a), lane(ins.b), lane(ins.c)]))
            .collect();
        let out = lane(self.out);
        let single = match (per_run.as_slice(), out) {
            ([(kind @ (Kind::Add | Kind::Sub | Kind::Mul), dst, [a, b, _])], Lane::Scalar(o))
                if o == *dst =>
            {
                match (a, b) {
                    (Lane::Scalar(0), x @ (Lane::Column(_) | Lane::Const(_))) => {
                        Some((*kind, false, *x))
                    }
                    (x @ (Lane::Column(_) | Lane::Const(_)), Lane::Scalar(0)) => {
                        Some((*kind, true, *x))
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        AccPlan {
            free,
            per_run,
            out,
            single,
        }
    }

    /// Run a two-input block — accumulator in register 0, element in
    /// register 1 — over every element from `seed`, handing `each` every
    /// accumulator. Answers the last one, or `None` when a value leaves the
    /// tier.
    ///
    /// Each run needs the last one's accumulator, but usually only a little
    /// of the block does: in `[ 3 MUL ADD ]` the `3 MUL` reads the element
    /// alone. The instructions that never read the accumulator, however
    /// indirectly, run a column at a time as `run_columns` runs a whole
    /// block; only the rest runs per element. When that rest is one
    /// arithmetic instruction on the accumulator — `[ ADD ]`, `[ 3 MUL ADD ]`
    /// — it gets a loop of its own with nothing in it but the operation.
    pub(crate) fn run_accumulating(
        &self,
        elements: &[i64],
        seed: i64,
        mut each: impl FnMut(i64),
    ) -> Option<i64> {
        debug_assert_eq!(self.inputs, 2);
        let AccPlan {
            free,
            per_run,
            out,
            single,
        } = self.acc.as_ref()?;
        let (out, single) = (*out, *single);
        let per_run_reads_element = single.is_none();
        // A row is a chunk wide, or as wide as the walk when that is shorter:
        // a walk of three elements need not clear a 256-lane row per register.
        let width = elements.len().clamp(1, CHUNK);
        let mut regs = vec![0i64; self.regs * width];
        let mut scalars = vec![0i64; self.regs];
        let mut acc = seed;
        for chunk in elements.chunks(width) {
            let len = chunk.len();
            if !free.is_empty() || per_run_reads_element {
                regs[width..width + len].copy_from_slice(chunk);
            }
            for ins in free {
                if exec_column(ins, &mut regs, width, len) {
                    return None;
                }
            }
            let at = |regs: &[i64], scalars: &[i64], l: Lane, i: usize| match l {
                Lane::Scalar(r) => scalars[r],
                Lane::Column(r) => regs[r * width + i],
                Lane::Const(v) => v,
            };
            if let Some((kind, flipped, x)) = single {
                let column = |r: usize| {
                    if r == 1 {
                        chunk
                    } else {
                        &regs[r * width..r * width + len]
                    }
                };
                let lanes: &[i64] = match x {
                    Lane::Column(r) => column(r),
                    _ => &[],
                };
                let ok = match (x, kind, flipped) {
                    (Lane::Const(v), _, _) => fold_lanes(
                        &mut acc,
                        std::iter::repeat_n(v, len),
                        kind,
                        flipped,
                        &mut each,
                    ),
                    (_, Kind::Add, _) => {
                        fold_with(&mut acc, lanes, &mut each, i64::overflowing_add)
                    }
                    (_, Kind::Mul, _) => fold_with(&mut acc, lanes, &mut each, overflowing_mul),
                    (_, _, false) => fold_with(&mut acc, lanes, &mut each, i64::overflowing_sub),
                    (_, _, true) => {
                        fold_with(&mut acc, lanes, &mut each, |a, x| x.overflowing_sub(a))
                    }
                };
                if !ok {
                    return None;
                }
                continue;
            }
            for i in 0..len {
                scalars[0] = acc;
                for (kind, dst, [a, b, c]) in per_run {
                    let (v, bad) = apply(
                        *kind,
                        at(&regs, &scalars, *a, i),
                        at(&regs, &scalars, *b, i),
                        at(&regs, &scalars, *c, i),
                    );
                    if bad {
                        return None;
                    }
                    scalars[*dst] = v;
                }
                acc = at(&regs, &scalars, out, i);
                each(acc);
            }
        }
        Some(acc)
    }
}
