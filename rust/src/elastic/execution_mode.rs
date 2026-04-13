/// Execution mode selector for the Ajisai interpreter (M5).
///
/// | Mode          | Behaviour |
/// |---------------|-----------|
/// | `Greedy`      | Sequential evaluation identical to all prior versions. Default. |
/// | `ElasticSafe` | Pure sub-expressions may be cached; impure words always fall back to greedy. |
/// | `ElasticForce`| Debug only — bypasses some safety gates. **Never use in production.** |

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ElasticMode {
    #[default]
    Greedy,
    ElasticSafe,
    ElasticForce,
}

impl ElasticMode {
    /// Parse from the CLI / WASM string representation.
    ///
    /// Unknown strings produce a stderr warning and fall back to `Greedy`.
    pub fn from_str(s: &str) -> Self {
        let normalized = s.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "elastic-safe"  => ElasticMode::ElasticSafe,
            "elastic_safe"  => ElasticMode::ElasticSafe,
            "elastic-force" => ElasticMode::ElasticForce,
            "elastic_force" => ElasticMode::ElasticForce,
            "greedy"        => ElasticMode::Greedy,
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
            ElasticMode::Greedy       => "greedy",
            ElasticMode::ElasticSafe  => "elastic-safe",
            ElasticMode::ElasticForce => "elastic-force",
        }
    }

    /// `true` for any elastic variant (safe or force).
    pub fn is_elastic(self) -> bool {
        matches!(self, ElasticMode::ElasticSafe | ElasticMode::ElasticForce)
    }
}

impl std::fmt::Display for ElasticMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
