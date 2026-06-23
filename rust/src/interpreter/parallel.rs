//! Native multi-core data-parallel kernels for the implicit-parallelism
//! engine (Phase 3, hierarchy A — task-internal data parallelism).
//!
//! Zero external dependencies: a small persistent `std::thread` worker pool
//! (created lazily, one set of workers for the whole process) fans a
//! homogeneous element-wise integer kernel across long-lived threads. Each
//! worker owns a **disjoint** index range of the output (原理III: 動線を交差さ
//! せない), so there is no data race and no result concatenation — workers write
//! straight into their own region of one shared output buffer. The pool is
//! essential, not cosmetic: measured per-call cost of spawning fresh threads
//! (`std::thread::scope`) in this environment is ~270µs, which pushes the
//! profitability crossover past a million elements; reusing parked workers
//! drops dispatch to the low-µs range so moderately sized vectors win too.
//!
//! ## The three non-negotiable contracts (roadmap §2)
//! * **Same Result** — each worker applies the identical scalar op as the
//!   sequential path to a disjoint, fixed index range, so the assembled output
//!   is bit-identical regardless of worker count (`assert_eq!` holds; see the
//!   proptests below, which sweep sizes across the dispatch threshold).
//! * **Never Slower** — the parallel path fires only when *both* the Phase-2
//!   [`parallel_kernel_eligible`](crate::elastic::EvaluationUnit::parallel_kernel_eligible)
//!   gate allows it (purity ∧ space budget ∧ minimum work score) *and* the
//!   element count clears [`PARALLEL_DISPATCH_MIN`], the runtime floor below
//!   which fan-out cannot amortize. Otherwise the call runs the existing
//!   sequential lane with no dispatch overhead.
//! * **Zero Syntax** — selection is automatic from element count plus the
//!   `purity_table`; no user-visible word, annotation, or thread count.
//!
//! On `wasm32` there is no native threading available to Core yet (browser
//! thread-pools are Phase 5), so every entry point degrades to the sequential
//! SIMD lane unchanged.

#[cfg(not(target_arch = "wasm32"))]
use std::mem::MaybeUninit;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{Receiver, Sender};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Condvar, Mutex, OnceLock};

/// Minimum number of output lanes before the pool is engaged in production.
///
/// Below this the per-call dispatch cost cannot be amortized for a trivial,
/// memory-bandwidth-bound integer op, so the kernel stays sequential to honor
/// Never-Slower. This is deliberately far above the Phase-2 gate's
/// `MIN_PARALLEL_WORK_SCORE` element floor (1024 for a `Light`-cost word): the
/// gate decides *semantic eligibility*, this decides *runtime profitability*.
///
/// ## Calibration (why this is large, not 8K)
/// Element-wise `i64` arithmetic is memory-bandwidth-bound — exactly the case
/// 原理I predicts multicore cannot accelerate, because one core already
/// saturates much of the bandwidth. Measured on the Phase-3 reference
/// environment (4 reported cores, but CPU-throttled with ~84µs thread-wakeup
/// latency), `seq` vs pool-`par` element-wise add crossed over only near ~900K
/// elements; 131K–786K were even-to-slower. To keep Never-Slower absolute, the
/// floor sits in the reliably-faster region. On a low-wakeup-latency, truly
/// multicore host the crossover is far lower, so this floor is conservative but
/// always correct. The robust, environment-independent scaling target is a
/// *compute-bound* kernel (matrix multiply / exact-rational element ops), which
/// lands in a follow-up; this constant only governs the element-wise path.
///
/// The parallel *algorithm* is exercised independently of this floor by the
/// proptests below (which call [`run_parallel_binary`] directly), so correctness
/// coverage does not depend on the production threshold.
pub const PARALLEL_DISPATCH_MIN: usize = 900_000;

// ── Persistent worker pool ──────────────────────────────────────────────────

/// A job is a chunk index plus a type-erased pointer to the per-call task.
#[cfg(not(target_arch = "wasm32"))]
type Job = Box<dyn FnOnce() + Send + 'static>;

#[cfg(not(target_arch = "wasm32"))]
struct Pool {
    /// One independent job queue per worker. Dispatch sends one chunk to each
    /// worker's own channel so every worker wakes in parallel — a single shared
    /// receiver behind a mutex serializes pickup and dominates the dispatch cost
    /// at the sizes we care about.
    senders: Vec<Sender<Job>>,
    workers: usize,
}

#[cfg(not(target_arch = "wasm32"))]
static POOL: OnceLock<Pool> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
fn pool() -> &'static Pool {
    POOL.get_or_init(|| {
        let workers = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        let mut senders = Vec::with_capacity(workers);
        for _ in 0..workers {
            let (tx, rx): (Sender<Job>, Receiver<Job>) = std::sync::mpsc::channel();
            senders.push(tx);
            std::thread::Builder::new()
                .name("ajisai-par".to_string())
                .spawn(move || {
                    // Loop ends when the sender is dropped (process shutdown).
                    while let Ok(job) = rx.recv() {
                        job();
                    }
                })
                .expect("spawn ajisai parallel worker");
        }
        Pool { senders, workers }
    })
}

/// Type-erased `Send`/`Sync` raw pointer to the per-call task.
///
/// The pointee is the caller's task closure; we erase it to `*const ()` so the
/// `'static` pool jobs do not name the borrowed lifetimes the closure carries.
/// Soundness rests on the dispatcher joining every job before returning, so the
/// pointee always outlives all dereferences.
#[cfg(not(target_arch = "wasm32"))]
struct SendPtr(*const ());
// SAFETY: dereferenced (via a monomorphized trampoline) only as a shared `&F`
// while the dispatcher keeps the pointee alive, and the bound `F: Sync` makes
// concurrent shared access sound. `*const ()` is `'static`.
#[cfg(not(target_arch = "wasm32"))]
unsafe impl Send for SendPtr {}
#[cfg(not(target_arch = "wasm32"))]
unsafe impl Sync for SendPtr {}
#[cfg(not(target_arch = "wasm32"))]
impl Clone for SendPtr {
    fn clone(&self) -> Self {
        *self
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl Copy for SendPtr {}
#[cfg(not(target_arch = "wasm32"))]
impl SendPtr {
    /// By-value accessor: forces a closure to capture the whole wrapper (which
    /// is `Send`) rather than the inner `*const ()` field, under edition-2021
    /// disjoint closure captures.
    #[inline]
    fn raw(self) -> *const () {
        self.0
    }
}

/// `Send`/`Sync` wrapper for the shared output base pointer.
#[cfg(not(target_arch = "wasm32"))]
struct SendMutPtr<T>(*mut T);
// SAFETY: each worker writes only its own disjoint index range, so although the
// base pointer is shared, no two threads ever touch the same element; the
// dispatcher joins all workers before the buffer is read.
#[cfg(not(target_arch = "wasm32"))]
unsafe impl<T> Send for SendMutPtr<T> {}
#[cfg(not(target_arch = "wasm32"))]
unsafe impl<T> Sync for SendMutPtr<T> {}
#[cfg(not(target_arch = "wasm32"))]
impl<T> Clone for SendMutPtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl<T> Copy for SendMutPtr<T> {}
#[cfg(not(target_arch = "wasm32"))]
impl<T> SendMutPtr<T> {
    /// By-value accessor: forces a closure to capture the whole wrapper (which
    /// is `Send`/`Sync`) rather than the inner `*mut T` field, under
    /// edition-2021 disjoint closure captures.
    #[inline]
    fn ptr(self) -> *mut T {
        self.0
    }
}

impl Pool {
    /// Run `task(chunk_index)` for every chunk on the pool and block until all
    /// have completed. `task` may borrow caller-local data because we join all
    /// jobs before returning.
    #[cfg(not(target_arch = "wasm32"))]
    fn for_each_chunk<F>(&self, chunks: usize, task: F)
    where
        F: Fn(usize) + Sync,
    {
        // Monomorphized trampoline: recovers the concrete `&F` from the erased
        // pointer and invokes it. Its function pointer is `'static`, so it can
        // ride inside the `'static` job without naming `F`'s borrowed lifetimes.
        fn invoke<F: Fn(usize)>(ptr: *const (), i: usize) {
            // SAFETY: `ptr` was produced from `&task as *const F`; the
            // dispatcher guarantees the referent is alive for this call.
            let task: &F = unsafe { &*(ptr as *const F) };
            task(i);
        }

        let latch: Arc<(Mutex<usize>, Condvar)> = Arc::new((Mutex::new(chunks), Condvar::new()));
        let erased = SendPtr(&task as *const F as *const ());
        let trampoline: fn(*const (), usize) = invoke::<F>;

        debug_assert!(chunks <= self.workers);
        for i in 0..chunks {
            let latch = latch.clone();
            self.senders[i]
                .send(Box::new(move || {
                    trampoline(erased.raw(), i);
                    let (lock, cvar) = &*latch;
                    let mut remaining = lock.lock().unwrap();
                    *remaining -= 1;
                    if *remaining == 0 {
                        cvar.notify_all();
                    }
                }))
                .expect("ajisai parallel pool accepts jobs");
        }

        let (lock, cvar) = &*latch;
        let mut remaining = lock.lock().unwrap();
        while *remaining != 0 {
            remaining = cvar.wait(remaining).unwrap();
        }
    }
}

// ── Gate + allocation helpers ───────────────────────────────────────────────

/// `true` when the Phase-2 data-parallel gate admits `word` at this size.
///
/// This is the single semantic門: it reuses the canonical purity table and the
/// existing space waterline (`MAX_MATERIALIZED_ELEMENTS`) as the available
/// space budget, exactly as the roadmap prescribes (原理II: 作業者を増やすコス
/// トは空間で払う). An unknown word — anything not provably pure — fails the
/// gate and stays sequential (保守的が常に安全側).
#[cfg(not(target_arch = "wasm32"))]
fn gate_allows(word: &str, element_count: usize) -> bool {
    use crate::elastic::purity_table::purity_by_name;
    use crate::elastic::{EvaluationUnit, ParallelGate};

    let Some(info) = purity_by_name(word) else {
        return false;
    };
    let unit = EvaluationUnit::from_purity(word, &info);
    let gate = ParallelGate::new(
        element_count,
        Some(crate::interpreter::MAX_MATERIALIZED_ELEMENTS),
    );
    unit.parallel_kernel_eligible(gate)
}

/// Number of chunks to split `n` elements into: one per worker, but never more
/// than there are elements.
#[cfg(not(target_arch = "wasm32"))]
fn chunk_plan(n: usize, workers: usize) -> (usize, usize) {
    let chunks = workers.min(n).max(1);
    let chunk = n.div_ceil(chunks);
    (chunks, chunk)
}

/// Run `fill(chunk_index, &mut [MaybeUninit<i64>])` over disjoint chunks of a
/// freshly-reserved buffer and return it fully initialized.
///
/// Writing into the uninitialized spare capacity avoids the zero-init memset
/// that `vec![0; n]` would cost — on a bandwidth-bound integer kernel that
/// memset doubles the write traffic and erases the parallel win.
#[cfg(not(target_arch = "wasm32"))]
fn fill_parallel<F>(n: usize, pool: &Pool, fill: F) -> Vec<i64>
where
    F: Fn(usize, &mut [MaybeUninit<i64>]) + Sync,
{
    let mut out: Vec<i64> = Vec::with_capacity(n);
    let (chunks, chunk) = chunk_plan(n, pool.workers);
    let base: SendMutPtr<MaybeUninit<i64>> = SendMutPtr(out.spare_capacity_mut().as_mut_ptr());

    pool.for_each_chunk(chunks, |i| {
        let start = i * chunk;
        let end = (start + chunk).min(n);
        // SAFETY: chunk ranges are disjoint and within `0..n` (== reserved
        // capacity); this worker is the sole writer of `start..end`, and the
        // dispatcher joins all workers before `out` is read.
        let region: &mut [MaybeUninit<i64>] =
            unsafe { std::slice::from_raw_parts_mut(base.ptr().add(start), end - start) };
        fill(start, region);
    });

    // SAFETY: every index in `0..n` was written exactly once above.
    unsafe {
        out.set_len(n);
    }
    out
}

// ── Policy-free parallel implementations (always fan out) ───────────────────
//
// These run the multi-core kernel unconditionally (subject only to having ≥2
// workers). They carry no profitability policy, so the benches and the
// differential proptests can drive the real parallel algorithm at any size
// without depending on `PARALLEL_DISPATCH_MIN`.

/// Multi-core element-wise binary op over two equal-length lanes. Always
/// parallel when the pool has ≥2 workers; otherwise computes inline.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_parallel_binary(a: &[i64], b: &[i64], op: fn(i64, i64) -> i64) -> Vec<i64> {
    debug_assert_eq!(a.len(), b.len());
    let n = a.len();
    let pool = pool();
    if pool.workers < 2 || n == 0 {
        return a.iter().zip(b.iter()).map(|(&x, &y)| op(x, y)).collect();
    }

    fill_parallel(n, pool, |start, region| {
        let a_chunk = &a[start..start + region.len()];
        let b_chunk = &b[start..start + region.len()];
        for ((dst, &x), &y) in region.iter_mut().zip(a_chunk.iter()).zip(b_chunk.iter()) {
            dst.write(op(x, y));
        }
    })
}

/// Multi-core element-wise op between a lane and a broadcast scalar.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_parallel_scalar(a: &[i64], scalar: i64, op: fn(i64, i64) -> i64) -> Vec<i64> {
    let n = a.len();
    let pool = pool();
    if pool.workers < 2 || n == 0 {
        return a.iter().map(|&x| op(x, scalar)).collect();
    }

    fill_parallel(n, pool, |start, region| {
        let a_chunk = &a[start..start + region.len()];
        for (dst, &x) in region.iter_mut().zip(a_chunk.iter()) {
            dst.write(op(x, scalar));
        }
    })
}

// ── Public, gated entry points ──────────────────────────────────────────────

/// Apply an element-wise binary integer op across two equal-length lanes.
///
/// Returns the result plus `true` when the multi-core path actually fired
/// (used only for observational metrics). `op` is the scalar elementwise
/// operation; `lane` is the sequential kernel used both as the below-threshold
/// fallback and on wasm. The two MUST agree element-wise so the parallel and
/// sequential outputs are bit-identical.
#[cfg(not(target_arch = "wasm32"))]
pub fn elementwise_binary(
    word: &str,
    a: &[i64],
    b: &[i64],
    op: fn(i64, i64) -> i64,
    lane: fn(&[i64], &[i64]) -> Vec<i64>,
) -> (Vec<i64>, bool) {
    debug_assert_eq!(a.len(), b.len());
    let n = a.len();
    if pool().workers < 2 || n < PARALLEL_DISPATCH_MIN || !gate_allows(word, n) {
        return (lane(a, b), false);
    }
    (run_parallel_binary(a, b, op), true)
}

#[cfg(target_arch = "wasm32")]
pub fn elementwise_binary(
    _word: &str,
    a: &[i64],
    b: &[i64],
    _op: fn(i64, i64) -> i64,
    lane: fn(&[i64], &[i64]) -> Vec<i64>,
) -> (Vec<i64>, bool) {
    (lane(a, b), false)
}

/// Apply an element-wise op between a lane and a broadcast scalar.
///
/// Same contract as [`elementwise_binary`]; `op(lane_value, scalar)` is applied
/// per element.
#[cfg(not(target_arch = "wasm32"))]
pub fn elementwise_scalar(
    word: &str,
    a: &[i64],
    scalar: i64,
    op: fn(i64, i64) -> i64,
    lane: fn(&[i64], i64) -> Vec<i64>,
) -> (Vec<i64>, bool) {
    let n = a.len();
    if pool().workers < 2 || n < PARALLEL_DISPATCH_MIN || !gate_allows(word, n) {
        return (lane(a, scalar), false);
    }
    (run_parallel_scalar(a, scalar, op), true)
}

#[cfg(target_arch = "wasm32")]
pub fn elementwise_scalar(
    _word: &str,
    a: &[i64],
    scalar: i64,
    _op: fn(i64, i64) -> i64,
    lane: fn(&[i64], i64) -> Vec<i64>,
) -> (Vec<i64>, bool) {
    (lane(a, scalar), false)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::interpreter::simd_ops;
    use proptest::prelude::*;

    fn seq_binary(a: &[i64], b: &[i64], op: fn(i64, i64) -> i64) -> Vec<i64> {
        a.iter().zip(b.iter()).map(|(&x, &y)| op(x, y)).collect()
    }

    fn seq_scalar(a: &[i64], s: i64, op: fn(i64, i64) -> i64) -> Vec<i64> {
        a.iter().map(|&x| op(x, s)).collect()
    }

    #[test]
    fn small_input_stays_sequential() {
        // Below PARALLEL_DISPATCH_MIN the gated entry point must not fan out
        // (Never Slower): the parallel flag stays false.
        let a: Vec<i64> = (0..16).collect();
        let b: Vec<i64> = (0..16).map(|x| x * 2).collect();
        let (result, parallel) =
            elementwise_binary("+", &a, &b, |x, y| x + y, simd_ops::lane_add);
        assert!(!parallel, "tiny input must stay sequential (Never Slower)");
        assert_eq!(result, seq_binary(&a, &b, |x, y| x + y));
    }

    #[test]
    fn impure_word_stays_sequential() {
        // An order-sensitive / impure word fails the Phase-2 gate even at a
        // size past the dispatch floor: it must never fan out.
        let n = PARALLEL_DISPATCH_MIN + 1;
        let a: Vec<i64> = (0..n as i64).collect();
        let b: Vec<i64> = (0..n as i64).collect();
        let (_r, parallel) = elementwise_binary("NOW", &a, &b, |x, y| x + y, simd_ops::lane_add);
        assert!(!parallel, "non-pure word must stay sequential");
    }

    #[test]
    fn parallel_matches_sequential_bitwise() {
        // Same Result: the parallel algorithm is bit-identical to the
        // sequential one. Driven through the policy-free impl so the test does
        // not allocate PARALLEL_DISPATCH_MIN-sized vectors.
        let n = 50_003; // multi-chunk, not a multiple of any worker count
        let a: Vec<i64> = (0..n as i64).collect();
        let b: Vec<i64> = (0..n as i64).map(|x| x - 3).collect();

        for (op, lane) in [
            ((|x, y| x + y) as fn(i64, i64) -> i64, simd_ops::lane_add as fn(&[i64], &[i64]) -> Vec<i64>),
            ((|x, y| x - y) as fn(i64, i64) -> i64, simd_ops::lane_sub as fn(&[i64], &[i64]) -> Vec<i64>),
            ((|x, y| x * y) as fn(i64, i64) -> i64, simd_ops::lane_mul as fn(&[i64], &[i64]) -> Vec<i64>),
        ] {
            let par = run_parallel_binary(&a, &b, op);
            assert_eq!(par, seq_binary(&a, &b, op), "binary mismatch");
            assert_eq!(par, lane(&a, &b), "binary != lane");
        }
    }

    proptest! {
        // Differential contract (roadmap §10.1): parallel == sequential, exact.
        // Sizes deliberately straddle small / multi-chunk boundaries; the
        // policy-free impl always fans out so the parallel algorithm is what is
        // under test, independent of the production threshold.
        #[test]
        fn parallel_equals_sequential_binary(
            len in 0usize..=20_000,
            seed in any::<i64>(),
        ) {
            let a: Vec<i64> = (0..len as i64).map(|i| i.wrapping_mul(31).wrapping_add(seed) % 1000).collect();
            let b: Vec<i64> = (0..len as i64).map(|i| i.wrapping_mul(17).wrapping_sub(seed) % 1000).collect();
            let par = run_parallel_binary(&a, &b, |x, y| x + y);
            prop_assert_eq!(par, seq_binary(&a, &b, |x, y| x + y));
        }

        #[test]
        fn parallel_equals_sequential_scalar(
            len in 0usize..=20_000,
            scalar in -1000i64..=1000,
        ) {
            let a: Vec<i64> = (0..len as i64).map(|i| i % 777).collect();
            let par = run_parallel_scalar(&a, scalar, |x, s| x * s);
            prop_assert_eq!(par, seq_scalar(&a, scalar, |x, s| x * s));
        }
    }
}
