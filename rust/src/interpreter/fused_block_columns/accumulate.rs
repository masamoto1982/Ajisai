//! The accumulating half of the column tier: a `FOLD` or `SCAN` fed a chunk
//! at a time, by a Vector's columns or by a `MAP` it never materialises.

use super::*;

/// A `FOLD` or `SCAN` fed its elements a chunk at a time: the accumulator in
/// register 0, the chunk's element in register 1. `accumulating_walk` feeds it a
/// Vector's columns; a typed chain (`run_chain`) feeds it the answers of a
/// `MAP` it never materialises.
pub(super) struct Accumulator<'b> {
    block: &'b FusedBlock,
    prepared: &'b Prepared,
    /// Registers for the instructions that run a column at a time; none
    /// when every one reads the accumulator, as in `[ ADD ]`.
    regs: Vec<Out>,
    column_wise: bool,
    scalars: smallvec::SmallVec<[Pair; 16]>,
    acc: Pair,
    scan: bool,
    visited: Vec<Value>,
}

impl<'b> Accumulator<'b> {
    pub(super) fn new(
        block: &'b FusedBlock,
        prepared: &'b Prepared,
        seed: Pair,
        scan: bool,
        n: usize,
    ) -> Self {
        let column_wise = prepared.reads_acc[2..].iter().any(|p| !p);
        Accumulator {
            block,
            prepared,
            regs: if column_wise {
                (0..block.instrs.len()).map(|_| Out::default()).collect()
            } else {
                Vec::new()
            },
            column_wise,
            scalars: smallvec::SmallVec::new(),
            acc: seed,
            scan,
            visited: Vec::with_capacity(if scan { n } else { 0 }),
        }
    }

    /// One chunk of `len` elements.
    pub(super) fn feed(&mut self, element: Lanes, len: usize) -> Option<()> {
        let Accumulator {
            block,
            prepared,
            regs,
            column_wise,
            scalars,
            acc,
            scan,
            visited,
        } = self;
        let (consts, out_ty, single) = (&prepared.consts[..], prepared.out_ty, prepared.single);
        let reads_acc = &prepared.reads_acc;
        let per_run = &reads_acc[2..];
        let inputs = [Lanes::Splat(0, 1), element];
        if *column_wise {
            run_chunk(block, consts, &inputs, regs, per_run, len)?;
        }
        if let Some((schema, flipped, x)) = single {
            let x = lanes_of(x, &inputs, regs, consts);
            if acc.1 == 1 && x.integer() {
                acc.0 = if *scan {
                    let each = |a| visited.push(plain(out_ty, (a, 1)).into_value());
                    fold_op(schema, flipped, acc.0, x, len, each)?
                } else {
                    fold_op(schema, flipped, acc.0, x, len, |_| {})?
                };
                return Some(());
            }
        }
        if scalars.is_empty() {
            scalars.resize(2 + block.instrs.len(), (0, 1));
        }
        for i in 0..len {
            scalars[0] = *acc;
            for (j, ins) in block.instrs.iter().enumerate() {
                if !per_run[j] {
                    continue;
                }
                let value = |s: Src| match s {
                    Src::Reg(r) if reads_acc[r] => scalars[r],
                    s => lanes_of(s, &inputs, regs, consts).at(i),
                };
                let arity = ins.op.arity();
                let a = value(ins.a);
                let b = if arity > 1 { value(ins.b) } else { a };
                let c = if arity > 2 { value(ins.c) } else { a };
                scalars[2 + j] = apply_pair(ins.op, a, b, c)?;
            }
            *acc = match block.out {
                Src::Reg(r) if reads_acc[r] => scalars[r],
                s => lanes_of(s, &inputs, regs, consts).at(i),
            };
            if *scan {
                visited.push(plain(out_ty, *acc).into_value());
            }
        }
        Some(())
    }

    pub(super) fn finish(self) -> Value {
        if self.scan {
            Value::from_vector(self.visited)
        } else {
            plain(self.prepared.out_ty, self.acc).into_value()
        }
    }
}

/// A `MAP` whose answers go straight into a `FOLD` or `SCAN`, the two walks
/// run together a chunk at a time so the mapped Vector is never built
/// (`engine`'s typed chain, LANG.CONTRACT.TYPES). Answers the value the
/// `FOLD` or `SCAN` leaves and the charges of the two walks — each exactly
/// what that walk alone charges on this tier — or `None` with nothing
/// touched whenever either walk alone would decline this tier.
pub(crate) fn run_chain(
    map: &FusedBlock,
    fold: &FusedBlock,
    interp: &Interpreter,
    target: &Value,
    seed: &Plain,
    scan: bool,
) -> Option<(Value, Charges)> {
    if interp.runtime_limits.max_bigint_bits < 64 {
        return None;
    }
    let columns = Columns::of(target)?;
    let n = columns.nums.len();
    if n == 0 {
        return None;
    }
    let mapped = map.preparations.get(map, &[columns.ty])?;
    // A mapped Vector of truths is built as a truth column, which the
    // column tier does not walk; such a chain runs as two Words.
    if mapped.out_ty != Ty::Num {
        return None;
    }
    let (seed_ty, seed) = pair_of(seed)?;
    let folded = fold.preparations.get(fold, &[seed_ty, mapped.out_ty])?;
    if folded.out_ty != seed_ty {
        return None;
    }
    let runs = n as u64;
    let steps = usize::try_from(
        runs.checked_mul(map.steps_per_run)?
            .checked_add(runs.checked_mul(fold.steps_per_run)?)?,
    )
    .ok()?;
    let work = runs.checked_mul(mapped.work_per_run.checked_add(folded.work_per_run)?)?;
    if interp
        .execution_step_count
        .checked_add(steps)?
        .checked_add(2)?
        > interp.max_execution_steps
        || interp.numeric_work_used.checked_add(work)? > interp.runtime_limits.max_numeric_work
    {
        return None;
    }
    let consts = &mapped.consts[..];
    let mut regs: Vec<Out> = (0..map.instrs.len()).map(|_| Out::default()).collect();
    let mut accumulator = Accumulator::new(fold, &folded, seed, scan, n);
    for (start, end) in chunks(n) {
        let len = end - start;
        let element = columns.chunk(start, end);
        run_chunk(map, consts, &[element], &mut regs, &[], len)?;
        let answer = lanes_of(map.out, &[element], &regs, consts);
        accumulator.feed(answer, len)?;
    }
    Some((accumulator.finish(), Charges { steps, work }))
}
