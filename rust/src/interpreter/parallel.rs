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
//!
//! ## The one audited `unsafe` island (structural-memory-safety roadmap Phase 4)
//! The crate is `#![deny(unsafe_code)]` at the root; this module is its single
//! audited exception, re-permitting `unsafe` locally below. Elimination is not
//! free: the persistent pool exists precisely to avoid the ~270µs per-call cost
//! of `std::thread::scope` (measured, above), and the pool's `'static` worker
//! threads cannot name the caller-local lifetimes the per-call task borrows —
//! so the dispatch erases them through a raw pointer and re-establishes safety
//! by *joining every job before returning* (the pointee always outlives every
//! dereference). A fully safe rewrite would require either a scoped-pool
//! dependency (the module is deliberately zero-dependency) or accepting the
//! scope overhead that would erase the parallel win. The `unsafe` is therefore
//! kept, but pinned to this one file, minimized, and covered by two independent
//! nets: the differential proptests below (parallel == sequential, bit-exact,
//! swept across the chunk boundaries) and, at runtime, the shadow-validation
//! integrity check (`super::shadow_validation`,
//! `docs/dev/physical-resilience-design.md`), which re-runs the reference lane
//! and refuses a parallel result that disagrees.
#![allow(unsafe_code)]

use crate::types::fraction::Fraction;

#[cfg(not(target_arch = "wasm32"))]
use std::mem::MaybeUninit;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicBool, Ordering};
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

/// Runtime profitability floor for a *compute-bound* kernel.
///
/// Unlike element-wise `i64` arithmetic (memory-bandwidth-bound, floor
/// [`PARALLEL_DISPATCH_MIN`]), an exact-rational element op does real work per
/// lane — a num/den cross-multiplication followed by a gcd normalization — so a
/// single core no longer saturates the bus and the profitability crossover
/// drops by more than an order of magnitude. The robust, environment-
/// independent scaling target the roadmap calls out (手4「正しい戦場」) lives
/// here, not in the bandwidth-bound lane.
///
/// This floor is set conservatively (well above the measured crossover on the
/// throttled reference host) so Never-Slower holds absolutely; a low-wakeup-
/// latency multicore host could profitably go lower, but the constant only ever
/// errs toward staying sequential. Like [`PARALLEL_DISPATCH_MIN`] it governs
/// only production dispatch — the policy-free kernels below are exercised at any
/// size by the proptests, independent of this value.
pub const COMPUTE_BOUND_DISPATCH_MIN: usize = 32_768;

/// Coarse cost classification of a data-parallel kernel. The runtime profit
/// crossover depends on per-element work, so the dispatch floor is selected per
/// class rather than from a single global constant (手4: 演算特性別の閾値分離).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParallelOpClass {
    /// Memory-bandwidth-bound: one core already saturates much of the bus, so
    /// fan-out only amortizes at very large sizes (element-wise `i64` add/mul).
    BandwidthBound,
    /// Compute-bound: each lane does substantial arithmetic (exact-rational
    /// cross-multiply + gcd), so the crossover is far lower and scaling is
    /// closer to linear in core count.
    ComputeBound,
}

/// Minimum element count before the pool engages for a kernel of `class`.
pub const fn parallel_dispatch_min(class: ParallelOpClass) -> usize {
    match class {
        ParallelOpClass::BandwidthBound => PARALLEL_DISPATCH_MIN,
        ParallelOpClass::ComputeBound => COMPUTE_BOUND_DISPATCH_MIN,
    }
}

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

#[cfg(not(target_arch = "wasm32"))]
impl Pool {
    /// Run `task(chunk_index)` for every chunk on the pool and block until all
    /// have completed. `task` may borrow caller-local data because we join all
    /// jobs before returning.
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
fn fill_parallel<T, F>(n: usize, pool: &Pool, fill: F) -> Vec<T>
where
    T: Send,
    F: Fn(usize, &mut [MaybeUninit<T>]) + Sync,
{
    let mut out: Vec<T> = Vec::with_capacity(n);
    let (chunks, chunk) = chunk_plan(n, pool.workers);
    let base: SendMutPtr<MaybeUninit<T>> = SendMutPtr(out.spare_capacity_mut().as_mut_ptr());

    pool.for_each_chunk(chunks, |i| {
        // Clamp `start` to `n`: for a small `n` that is not a multiple of the
        // worker count, `chunk_plan`'s ceil-rounded `chunk` can push the last
        // chunk's `i * chunk` past `n` (e.g. n=5, workers=4 → chunk=2, i=3 →
        // start=6). Without the clamp `end - start` underflows `usize` and the
        // region length becomes enormous, so the worker writes far out of
        // bounds (UB / SIGSEGV). Clamping yields an empty trailing region; the
        // earlier chunks already cover every index because `chunk` is ceil-based.
        let start = (i * chunk).min(n);
        let end = (start + chunk).min(n);
        // SAFETY: chunk ranges are disjoint and within `0..n` (== reserved
        // capacity); this worker is the sole writer of `start..end`, and the
        // dispatcher joins all workers before `out` is read.
        let region: &mut [MaybeUninit<T>] =
            unsafe { std::slice::from_raw_parts_mut(base.ptr().add(start), end - start) };
        fill(start, region);
    });

    // SAFETY: every index in `0..n` was written exactly once above (the `fill`
    // contract requires every lane in each region to be initialized).
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

// ── Speculative (overflow-checked) parallel implementations ─────────────────
//
// The integer lane runs `i64` arithmetic speculatively: it is bit-identical to
// the exact rational result *iff* no lane overflows (handoff 奇策本命). These
// variants compute with `checked_*` ops and OR-aggregate an overflow flag
// across workers. On overflow the caller discards the result and recomputes on
// the exact `Fraction`/BigInt path, so the answer can never silently differ
// from the sequential exact value. The cheap overflow check (not a second full
// execution) is what keeps this on the Never-Slower side.

/// Multi-core overflow-checked element-wise binary op. Returns `None` if any
/// lane overflowed `i64`; otherwise the bit-exact result.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_parallel_binary_checked(
    a: &[i64],
    b: &[i64],
    op: fn(i64, i64) -> Option<i64>,
) -> Option<Vec<i64>> {
    debug_assert_eq!(a.len(), b.len());
    let n = a.len();
    let pool = pool();
    if pool.workers < 2 || n == 0 {
        let mut out = Vec::with_capacity(n);
        for (&x, &y) in a.iter().zip(b.iter()) {
            out.push(op(x, y)?);
        }
        return Some(out);
    }

    let overflow = Arc::new(AtomicBool::new(false));
    let result = fill_parallel(n, pool, |start, region| {
        let a_chunk = &a[start..start + region.len()];
        let b_chunk = &b[start..start + region.len()];
        for ((dst, &x), &y) in region.iter_mut().zip(a_chunk.iter()).zip(b_chunk.iter()) {
            match op(x, y) {
                Some(v) => {
                    dst.write(v);
                }
                None => {
                    overflow.store(true, Ordering::Relaxed);
                    // Keep the lane initialized (set_len safety); the whole
                    // buffer is discarded once the overflow flag is observed.
                    dst.write(0);
                }
            }
        }
    });

    if overflow.load(Ordering::Relaxed) {
        None
    } else {
        Some(result)
    }
}

/// Multi-core overflow-checked element-wise op between a lane and a scalar.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_parallel_scalar_checked(
    a: &[i64],
    scalar: i64,
    op: fn(i64, i64) -> Option<i64>,
) -> Option<Vec<i64>> {
    let n = a.len();
    let pool = pool();
    if pool.workers < 2 || n == 0 {
        let mut out = Vec::with_capacity(n);
        for &x in a.iter() {
            out.push(op(x, scalar)?);
        }
        return Some(out);
    }

    let overflow = Arc::new(AtomicBool::new(false));
    let result = fill_parallel(n, pool, |start, region| {
        let a_chunk = &a[start..start + region.len()];
        for (dst, &x) in region.iter_mut().zip(a_chunk.iter()) {
            match op(x, scalar) {
                Some(v) => {
                    dst.write(v);
                }
                None => {
                    overflow.store(true, Ordering::Relaxed);
                    dst.write(0);
                }
            }
        }
    });

    if overflow.load(Ordering::Relaxed) {
        None
    } else {
        Some(result)
    }
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

/// Overflow-checked counterpart of [`elementwise_binary`]. The result is `None`
/// when any lane overflowed `i64` (the caller must then recompute on the exact
/// path); the `bool` reports whether the multi-core kernel fired. Both `op` and
/// `lane` return `None` on overflow and MUST agree element-wise.
#[cfg(not(target_arch = "wasm32"))]
pub fn elementwise_binary_checked(
    word: &str,
    a: &[i64],
    b: &[i64],
    op: fn(i64, i64) -> Option<i64>,
    lane: fn(&[i64], &[i64]) -> Option<Vec<i64>>,
) -> (Option<Vec<i64>>, bool) {
    debug_assert_eq!(a.len(), b.len());
    let n = a.len();
    if pool().workers < 2 || n < PARALLEL_DISPATCH_MIN || !gate_allows(word, n) {
        return (lane(a, b), false);
    }
    let result = run_parallel_binary_checked(a, b, op);
    // Only the actual no-overflow success counts as a parallel firing; on
    // overflow the value is discarded and the exact path takes over.
    let fired = result.is_some();
    (result, fired)
}

#[cfg(target_arch = "wasm32")]
pub fn elementwise_binary_checked(
    _word: &str,
    a: &[i64],
    b: &[i64],
    _op: fn(i64, i64) -> Option<i64>,
    lane: fn(&[i64], &[i64]) -> Option<Vec<i64>>,
) -> (Option<Vec<i64>>, bool) {
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

/// Overflow-checked counterpart of [`elementwise_scalar`]. Same contract as
/// [`elementwise_binary_checked`].
#[cfg(not(target_arch = "wasm32"))]
pub fn elementwise_scalar_checked(
    word: &str,
    a: &[i64],
    scalar: i64,
    op: fn(i64, i64) -> Option<i64>,
    lane: fn(&[i64], i64) -> Option<Vec<i64>>,
) -> (Option<Vec<i64>>, bool) {
    let n = a.len();
    if pool().workers < 2 || n < PARALLEL_DISPATCH_MIN || !gate_allows(word, n) {
        return (lane(a, scalar), false);
    }
    let result = run_parallel_scalar_checked(a, scalar, op);
    let fired = result.is_some();
    (result, fired)
}

#[cfg(target_arch = "wasm32")]
pub fn elementwise_scalar_checked(
    _word: &str,
    a: &[i64],
    scalar: i64,
    _op: fn(i64, i64) -> Option<i64>,
    lane: fn(&[i64], i64) -> Option<Vec<i64>>,
) -> (Option<Vec<i64>>, bool) {
    (lane(a, scalar), false)
}

// ── Compute-bound exact-rational kernel (手4: 正しい戦場で並列化) ─────────────
//
// Element-wise exact arithmetic over `Fraction` lanes is the robust scaling
// target: each lane does a num/den cross-multiply plus a gcd normalization, so
// the work is compute-bound and fan-out scales near-linearly with cores. Unlike
// the speculative `i64` lane this never needs an overflow escape — `Fraction`
// is arbitrary precision, so the parallel result is *exactly* the sequential
// one. The caller assembles the result `Value` from the returned `Vec<Fraction>`
// through the identical constructor the sequential path uses, so Same Result is
// structural, not merely value-wise.

/// Policy-free compute-bound element-wise map producing `Fraction` lanes. Always
/// fans out when the pool has ≥2 workers; otherwise computes inline. `f(i)` must
/// be the same pure per-element op the sequential path runs, so the assembled
/// output is identical regardless of worker count.
///
/// On a lane error (e.g. division by zero) the parallel pass is discarded and
/// the map is recomputed sequentially, which surfaces the same first
/// (lowest-index) error the sequential path would — preserving error order.
#[cfg(not(target_arch = "wasm32"))]
pub fn parallel_try_elementwise<F>(n: usize, f: F) -> crate::error::Result<Vec<Fraction>>
where
    F: Fn(usize) -> crate::error::Result<Fraction> + Sync,
{
    let pool = pool();
    if pool.workers < 2 || n == 0 {
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            out.push(f(i)?);
        }
        return Ok(out);
    }

    let failed = AtomicBool::new(false);
    let buf = fill_parallel::<Fraction, _>(n, pool, |start, region| {
        for (k, slot) in region.iter_mut().enumerate() {
            match f(start + k) {
                Ok(v) => {
                    slot.write(v);
                }
                Err(_) => {
                    // Keep every lane initialized so `set_len` is sound and the
                    // discarded buffer drops cleanly; the placeholder value is
                    // never observed (the buffer is dropped below).
                    failed.store(true, Ordering::Relaxed);
                    slot.write(Fraction::nil());
                }
            }
        }
    });

    if failed.load(Ordering::Relaxed) {
        drop(buf);
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            out.push(f(i)?);
        }
        // Unreachable in practice: at least one lane errored above, so the loop
        // returns early. The `Ok` keeps the type-checker happy.
        Ok(out)
    } else {
        Ok(buf)
    }
}

/// Gated entry point for a compute-bound exact-rational element-wise op. Runs
/// the multi-core kernel only when the element count clears the compute-bound
/// floor and the pool has ≥2 workers; otherwise stays on the identical
/// sequential lane (Never Slower). The op is pure by type (`Fraction`-only), so
/// no purity gate is consulted — the caller guarantees `f` is the pure
/// per-element arithmetic the sequential path would run.
#[cfg(not(target_arch = "wasm32"))]
pub fn compute_bound_elementwise<F>(n: usize, f: F) -> crate::error::Result<Vec<Fraction>>
where
    F: Fn(usize) -> crate::error::Result<Fraction> + Sync,
{
    if pool().workers < 2 || n < parallel_dispatch_min(ParallelOpClass::ComputeBound) {
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            out.push(f(i)?);
        }
        return Ok(out);
    }
    parallel_try_elementwise(n, f)
}

/// On `wasm32` there is no native threading, so the compute-bound element-wise
/// op degrades to the sequential lane unchanged.
#[cfg(target_arch = "wasm32")]
pub fn compute_bound_elementwise<F>(n: usize, f: F) -> crate::error::Result<Vec<Fraction>>
where
    F: Fn(usize) -> crate::error::Result<Fraction>,
{
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(f(i)?);
    }
    Ok(out)
}

/// Policy-free compute-bound element-wise map producing arbitrary `Send` lanes.
///
/// Generalizes [`parallel_try_elementwise`] beyond `Fraction`: the per-lane op
/// returns any `T: Send`, so the caller can carry its own error encoding inside
/// `T` (e.g. `Option<ExactReal>` where `None` is a division-by-zero bubble)
/// rather than threading a `Result`. Used for irrational continued-fraction
/// lanes — each lane is an independent, pure, genuinely compute-bound Gosper
/// evaluation (not memory-bandwidth-bound), exactly the 戦局1 homogeneous case
/// the roadmap says to fan out across all cores. `Value` itself is not `Send`,
/// so the caller maps the `Send` lane results back into `Value`s on the calling
/// thread after this returns.
///
/// Always fans out when the pool has ≥2 workers; `f(i)` must be the same pure
/// per-element op the sequential path runs, so the assembled output is identical
/// regardless of worker count.
#[cfg(not(target_arch = "wasm32"))]
pub fn parallel_map<T, F>(n: usize, f: F) -> Vec<T>
where
    T: Send,
    F: Fn(usize) -> T + Sync,
{
    let pool = pool();
    if pool.workers < 2 || n == 0 {
        return (0..n).map(f).collect();
    }
    fill_parallel::<T, _>(n, pool, |start, region| {
        for (k, slot) in region.iter_mut().enumerate() {
            slot.write(f(start + k));
        }
    })
}

/// Gated compute-bound variant of [`parallel_map`]. Fans out only when the
/// element count clears the compute-bound floor and the pool has ≥2 workers;
/// otherwise stays on the identical sequential lane (Never Slower). Pure by type
/// (the lane op produces a value, no side effects), so no purity gate is
/// consulted — the caller guarantees `f` is the pure per-element op the
/// sequential path would run.
#[cfg(not(target_arch = "wasm32"))]
pub fn compute_bound_map<T, F>(n: usize, f: F) -> Vec<T>
where
    T: Send,
    F: Fn(usize) -> T + Sync,
{
    if pool().workers < 2 || n < parallel_dispatch_min(ParallelOpClass::ComputeBound) {
        return (0..n).map(f).collect();
    }
    parallel_map(n, f)
}

/// On `wasm32` there is no native threading, so the compute-bound map degrades
/// to the sequential lane unchanged.
#[cfg(target_arch = "wasm32")]
pub fn compute_bound_map<T, F>(n: usize, f: F) -> Vec<T>
where
    F: Fn(usize) -> T,
{
    (0..n).map(f).collect()
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
    fn tiny_input_below_worker_count_stays_sound() {
        // Soundness pin for the `fill_parallel` clamp: when `n` is smaller than
        // the worker count and not a multiple of it, the ceil-based chunk plan
        // can push a trailing chunk's start past `n`. The clamp turns that into
        // an empty region; without it `end - start` would underflow `usize` and
        // the worker would write wildly out of bounds. Drive the exact edge (a
        // handful of elements) and confirm bit-exact equality with sequential.
        for n in 1usize..=9 {
            let a: Vec<i64> = (0..n as i64).collect();
            let b: Vec<i64> = (0..n as i64).map(|x| x * 2).collect();
            let par = run_parallel_binary(&a, &b, |x, y| x + y);
            assert_eq!(par, seq_binary(&a, &b, |x, y| x + y), "mismatch at n={n}");
        }
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

    // ── Speculative (overflow-checked) kernel contracts ─────────────────────

    fn seq_binary_checked(
        a: &[i64],
        b: &[i64],
        op: fn(i64, i64) -> Option<i64>,
    ) -> Option<Vec<i64>> {
        a.iter().zip(b.iter()).map(|(&x, &y)| op(x, y)).collect()
    }

    #[test]
    fn checked_parallel_matches_sequential_when_no_overflow() {
        // Same Result: the overflow-checked multi-core kernel is bit-identical
        // to the sequential checked reference when nothing overflows.
        let n = 50_003;
        let a: Vec<i64> = (0..n as i64).collect();
        let b: Vec<i64> = (0..n as i64).map(|x| x - 7).collect();
        let par = run_parallel_binary_checked(&a, &b, |x, y| x.checked_add(y));
        let seq = seq_binary_checked(&a, &b, |x, y| x.checked_add(y));
        assert_eq!(par, seq, "checked parallel != checked sequential");
    }

    #[test]
    fn checked_parallel_detects_overflow_and_declines() {
        // A single overflowing lane anywhere in the buffer must make the whole
        // checked kernel decline (None), so the caller falls back to exact.
        let n = 100_003;
        let mut a: Vec<i64> = vec![1; n];
        let b: Vec<i64> = vec![1; n];
        a[n / 2] = i64::MAX; // one poisoned lane mid-buffer
        let par = run_parallel_binary_checked(&a, &b, |x, y| x.checked_add(y));
        assert!(par.is_none(), "any overflowing lane must decline the kernel");
    }

    proptest! {
        #[test]
        fn checked_parallel_equals_checked_sequential(
            len in 0usize..=20_000,
            seed in any::<i64>(),
        ) {
            // Values bounded so no overflow occurs; the checked parallel result
            // must equal the checked sequential one element for element.
            let a: Vec<i64> = (0..len as i64).map(|i| i.wrapping_mul(31).wrapping_add(seed) % 1000).collect();
            let b: Vec<i64> = (0..len as i64).map(|i| i.wrapping_mul(17).wrapping_sub(seed) % 1000).collect();
            let par = run_parallel_binary_checked(&a, &b, |x, y| x.checked_mul(y));
            let seq = seq_binary_checked(&a, &b, |x, y| x.checked_mul(y));
            prop_assert_eq!(par, seq);
        }
    }

    // ── Compute-bound exact-rational kernel contracts (手4) ──────────────────

    #[test]
    fn compute_bound_floor_is_below_bandwidth_floor() {
        // The whole point of per-class thresholds: a compute-bound kernel
        // becomes profitable far earlier than a bandwidth-bound one.
        assert!(
            parallel_dispatch_min(ParallelOpClass::ComputeBound)
                < parallel_dispatch_min(ParallelOpClass::BandwidthBound),
            "compute-bound floor must be lower than bandwidth-bound floor"
        );
        assert_eq!(
            parallel_dispatch_min(ParallelOpClass::BandwidthBound),
            PARALLEL_DISPATCH_MIN,
            "bandwidth-bound floor must stay the calibrated i64 constant"
        );
    }

    /// `i/3 + i/7` over a rational lane — exercises gcd normalization so the
    /// parallel write path carries real `Fraction` payloads (Small and, for some
    /// indices, denominators > 1), not just integers.
    fn rational_op(i: usize) -> crate::error::Result<Fraction> {
        let third = Fraction::from(i as i64).div(&Fraction::from(3));
        let seventh = Fraction::from(i as i64).div(&Fraction::from(7));
        Ok(third.add(&seventh))
    }

    fn seq_fraction_map<F>(n: usize, f: F) -> crate::error::Result<Vec<Fraction>>
    where
        F: Fn(usize) -> crate::error::Result<Fraction>,
    {
        (0..n).map(f).collect()
    }

    #[test]
    fn parallel_fraction_matches_sequential() {
        // Same Result: the policy-free compute-bound map is element-for-element
        // identical to the sequential reference. Size is multi-chunk and not a
        // multiple of any worker count so chunk boundaries are exercised.
        let n = 50_003;
        let par = parallel_try_elementwise(n, rational_op).unwrap();
        let seq = seq_fraction_map(n, rational_op).unwrap();
        assert_eq!(par, seq, "parallel rational map != sequential");
    }

    #[test]
    fn parallel_fraction_surfaces_first_error_like_sequential() {
        // A lane error must make the kernel decline with the same Err the
        // sequential lane raises (error order preserved). One poisoned index
        // mid-buffer stands in for e.g. a division by zero.
        let n = 40_000;
        let poison = n / 2;
        let f = |i: usize| -> crate::error::Result<Fraction> {
            if i == poison {
                Err(crate::error::AjisaiError::from("rational lane boom"))
            } else {
                Ok(Fraction::from(i as i64))
            }
        };
        assert!(
            parallel_try_elementwise(n, f).is_err(),
            "a poisoned lane must decline the parallel kernel"
        );
        assert!(seq_fraction_map(n, f).is_err(), "sequential must also error");
    }

    #[test]
    fn compute_bound_gate_matches_sequential_below_and_above_floor() {
        // The gated entry must be bit-identical to sequential both below the
        // floor (no fan-out) and above it (fan-out), so dispatch never changes
        // the answer.
        for n in [8usize, COMPUTE_BOUND_DISPATCH_MIN + 11] {
            let gated = compute_bound_elementwise(n, rational_op).unwrap();
            let seq = seq_fraction_map(n, rational_op).unwrap();
            assert_eq!(gated, seq, "gated compute-bound != sequential at n={n}");
        }
    }

    #[test]
    fn parallel_fraction_handles_chunk_overshoot_sizes() {
        // Regression: a small `n` that is not a multiple of the worker count
        // (e.g. n=5 with 4 workers) makes `chunk_plan`'s ceil-rounded chunk push
        // the last chunk's start past `n`. Before clamping `start`, the region
        // length underflowed and the worker wrote out of bounds (SIGSEGV). Every
        // tiny size must now produce exactly the sequential result. Both the i64
        // and Fraction lanes share `fill_parallel`, so cover both.
        for n in 0usize..=64 {
            let frac = parallel_try_elementwise(n, rational_op).unwrap();
            assert_eq!(frac, seq_fraction_map(n, rational_op).unwrap(), "fraction n={n}");

            let a: Vec<i64> = (0..n as i64).collect();
            let b: Vec<i64> = (0..n as i64).map(|x| x + 1).collect();
            let ints = run_parallel_binary(&a, &b, |x, y| x + y);
            assert_eq!(ints, seq_binary(&a, &b, |x, y| x + y), "i64 n={n}");
        }
    }

    proptest! {
        // Differential contract: the policy-free compute-bound rational map is
        // exactly the sequential one across sizes straddling chunk boundaries.
        #[test]
        fn parallel_fraction_equals_sequential(len in 0usize..=8_000) {
            let par = parallel_try_elementwise(len, rational_op).unwrap();
            let seq = seq_fraction_map(len, rational_op).unwrap();
            prop_assert_eq!(par, seq);
        }
    }

    // ── Generic Send-lane map (`parallel_map` / `compute_bound_map`) ─────────
    //
    // Backs the irrational continued-fraction broadcast: each lane is an
    // independent exact-real op whose `Send` result is rebuilt into a `Value`
    // on the calling thread. Drive it with `ExactReal` lanes (the production
    // payload), including a √2-bearing irrational so the Gosper path — not just
    // the rational fast lane — is exercised under fan-out.
    use crate::types::exact::ExactReal;

    /// Per-lane op mirroring `ExactArithmeticSchema::Add` over exact reals:
    /// `√2·i + (i+1)`, encoded as `Option` exactly like the broadcast's
    /// division-by-zero channel (always `Some` for add).
    fn exact_lane(i: usize) -> Option<ExactReal> {
        let sqrt2 = ExactReal::from_sqrt_rational(Fraction::from(2_i64)).unwrap();
        let scaled = sqrt2.mul(&ExactReal::from_integer(i as i64));
        Some(scaled.add(&ExactReal::from_integer(i as i64 + 1)))
    }

    fn seq_exact_map(n: usize) -> Vec<Option<ExactReal>> {
        (0..n).map(exact_lane).collect()
    }

    #[test]
    fn parallel_map_exact_matches_sequential_across_chunk_sizes() {
        // Same Result for the generic Send-lane kernel: the always-fan-out
        // `parallel_map` is element-identical to the sequential map at every
        // size, including the tiny chunk-overshoot sizes that share
        // `fill_parallel` with the i64/Fraction lanes.
        for n in 0usize..=64 {
            assert_eq!(parallel_map(n, exact_lane), seq_exact_map(n), "exact n={n}");
        }
        let n = 50_003; // multi-chunk, not a multiple of any worker count
        assert_eq!(parallel_map(n, exact_lane), seq_exact_map(n), "exact n={n}");
    }

    #[test]
    fn compute_bound_map_exact_matches_sequential_below_and_above_floor() {
        // The gated entry is identical to sequential both below the floor (no
        // fan-out) and above it (fan-out), so dispatch never changes the answer.
        for n in [8usize, COMPUTE_BOUND_DISPATCH_MIN + 7] {
            let gated = compute_bound_map(n, exact_lane);
            assert_eq!(
                gated,
                seq_exact_map(n),
                "gated exact map != sequential at n={n}"
            );
        }
    }

    proptest! {
        // Differential contract for the generic exact-real lane map across
        // sizes straddling chunk boundaries.
        #[test]
        fn parallel_map_exact_equals_sequential(len in 0usize..=4_000) {
            let par = parallel_map(len, exact_lane);
            prop_assert_eq!(par, seq_exact_map(len));
        }
    }
}
