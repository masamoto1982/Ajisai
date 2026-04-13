/// Word-level execution tracer for the Elastic Engine.
///
/// # Enabling
/// **Native builds / tests** — set the environment variable before running:
/// ```text
/// AJISAI_TRACE=1 cargo test
/// ```
///
/// **WASM / programmatic** — call `set_enabled(true)` at runtime.
///
/// # Output format
/// All output goes to **stderr** so it does not pollute program output:
/// ```text
/// [trace] word=+          elapsed=0.012ms
/// [trace] word=MAP        elapsed=1.340ms
/// ...
/// === Elastic Tracer Report ===
///   +                    calls=42  avg=0.011ms  total=0.5ms
/// ```

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

// ── Global state ──────────────────────────────────────────────────────────────

static TRACE_ENABLED: AtomicBool = AtomicBool::new(false);

struct TraceData {
    call_counts: HashMap<String, u64>,
    total_nanos: HashMap<String, u64>,
}

impl Default for TraceData {
    fn default() -> Self {
        TraceData {
            call_counts: HashMap::new(),
            total_nanos: HashMap::new(),
        }
    }
}

// lazy_static is already a dependency of ajisai-core.
lazy_static::lazy_static! {
    static ref TRACE_DATA: Mutex<TraceData> = Mutex::new(TraceData::default());
}

// ── Initialisation (native only) ──────────────────────────────────────────────

/// Initialise tracer from environment on native targets.
/// Called once from `Interpreter::new()`.
pub fn init_from_env() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if std::env::var("AJISAI_TRACE").map(|v| v == "1").unwrap_or(false) {
            TRACE_ENABLED.store(true, Ordering::Relaxed);
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn is_enabled() -> bool {
    TRACE_ENABLED.load(Ordering::Relaxed)
}

pub fn set_enabled(enabled: bool) {
    TRACE_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Record one word invocation.
///
/// `elapsed_nanos` is 0 on WASM (monotonic clock unavailable).
pub fn record(word_name: &str, elapsed_nanos: u64) {
    if !is_enabled() {
        return;
    }

    {
        let mut data = TRACE_DATA.lock().unwrap();
        *data.call_counts.entry(word_name.to_string()).or_insert(0) += 1;
        *data.total_nanos.entry(word_name.to_string()).or_insert(0) += elapsed_nanos;
    }

    eprintln!(
        "[trace] word={:<16} elapsed={:.3}ms",
        word_name,
        elapsed_nanos as f64 / 1_000_000.0
    );
}

/// Print aggregated timing report to stderr.
pub fn report() {
    if !is_enabled() {
        return;
    }
    let data = TRACE_DATA.lock().unwrap();
    eprintln!("\n=== Elastic Tracer Report ===");

    let mut entries: Vec<(&String, u64)> =
        data.call_counts.iter().map(|(k, &v)| (k, v)).collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1));

    for (word, count) in entries.iter().take(20) {
        let total_ns  = data.total_nanos.get(*word).copied().unwrap_or(0);
        let avg_ms    = if *count > 0 { total_ns as f64 / *count as f64 / 1_000_000.0 } else { 0.0 };
        let total_ms  = total_ns as f64 / 1_000_000.0;
        eprintln!(
            "  {:<20} calls={:<6} avg={:.3}ms  total={:.1}ms",
            word, count, avg_ms, total_ms
        );
    }
}

/// Reset all accumulated trace data (useful between tests).
pub fn reset() {
    let mut data = TRACE_DATA.lock().unwrap();
    *data = TraceData::default();
}

/// Return the recorded call count for a word (0 if unseen).
pub fn call_count(word_name: &str) -> u64 {
    let data = TRACE_DATA.lock().unwrap();
    data.call_counts.get(word_name).copied().unwrap_or(0)
}

/// Return the total elapsed nanoseconds for a word (0 if unseen or WASM).
pub fn total_nanos(word_name: &str) -> u64 {
    let data = TRACE_DATA.lock().unwrap();
    data.total_nanos.get(word_name).copied().unwrap_or(0)
}
