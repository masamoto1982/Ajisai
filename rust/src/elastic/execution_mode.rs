/// Execution mode selector for the Ajisai interpreter (M5).
///
/// | Mode          | Behaviour |
/// |---------------|-----------|
/// | `Greedy`      | Sequential evaluation identical to all prior versions. Default. |
/// | `ElasticSafe` | Pure sub-expressions may be cached; impure words always fall back to greedy. |
/// | `HedgedSafe`  | Safe hedged race for allowlisted pure paths. |
/// | `HedgedTrace` | Same as `HedgedSafe` plus verbose race tracing. |
/// | `FastGuarded` | Guarded fast path (no plain race); guard miss falls back safely. |
/// | `ElasticForce`| Debug only — bypasses some safety gates. **Never use in production.** |

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ElasticMode {
    #[default]
    Greedy,
    ElasticSafe,
    HedgedSafe,
    HedgedTrace,
    FastGuarded,
    ElasticForce,
}

impl ElasticMode {
    /// Parse from the CLI / WASM string representation.
    ///
    /// Unknown strings produce a stderr warning and fall back to `Greedy`.
    pub fn from_str(s: &str) -> Self {
        let normalized = s.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "elastic-safe" => ElasticMode::ElasticSafe,
            "elastic_safe" => ElasticMode::ElasticSafe,
            "elastic-force" => ElasticMode::ElasticForce,
            "elastic_force" => ElasticMode::ElasticForce,
            "hedged-safe" => ElasticMode::HedgedSafe,
            "hedged_safe" => ElasticMode::HedgedSafe,
            "hedged-trace" => ElasticMode::HedgedTrace,
            "hedged_trace" => ElasticMode::HedgedTrace,
            "fast-guarded" => ElasticMode::FastGuarded,
            "fast_guarded" => ElasticMode::FastGuarded,
            "greedy" => ElasticMode::Greedy,
            _ => {
                eprintln!(
                    "[warn] Unknown execution mode '{}'. Falling back to greedy.",
                    s
                );
                ElasticMode::Greedy
            }
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ElasticMode::Greedy => "greedy",
            ElasticMode::ElasticSafe => "elastic-safe",
            ElasticMode::HedgedSafe => "hedged-safe",
            ElasticMode::HedgedTrace => "hedged-trace",
            ElasticMode::FastGuarded => "fast-guarded",
            ElasticMode::ElasticForce => "elastic-force",
        }
    }

    /// `true` for any elastic variant (safe or force).
    pub fn is_elastic(self) -> bool {
        matches!(
            self,
            ElasticMode::ElasticSafe
                | ElasticMode::HedgedSafe
                | ElasticMode::HedgedTrace
                | ElasticMode::FastGuarded
                | ElasticMode::ElasticForce
        )
    }
}

impl std::fmt::Display for ElasticMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
