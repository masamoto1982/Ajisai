//! `ajisai.lock` — the generated reproducibility record (Phase 8B).
//!
//! Where the manifest (`manifest.rs`) declares intent, the lockfile records
//! realized fact: the content identity of every source that makes up the
//! project, the content identity of each public word it defines, the host
//! capabilities the run actually required, the specification version it
//! targets, and the manifest schema version. Package identity is thus tied to
//! *content*, never to a name and version alone (SPEC handoff §15.2 / §15.4).
//!
//! The lockfile is machine-owned and emitted as canonical JSON (object keys are
//! sorted by `serde_json`'s map, arrays are sorted here) so regenerating it from
//! unchanged inputs is byte-for-byte stable — that stability is what lets
//! `build` and `lock --check` detect drift by comparison.

use serde_json::json;

use super::pretty;
use crate::interpreter::content_digest;

/// Version of the lockfile document shape. Bumped only on a breaking change.
pub(crate) const LOCKFILE_VERSION: u64 = 1;

/// One source file that composes the project.
pub(crate) struct SourceEntry {
    /// `"entry"` or `"dependency"`.
    pub role: &'static str,
    /// The dependency's manifest name, or `None` for the entry.
    pub name: Option<String>,
    /// The source path as it appears relative to the manifest directory.
    pub path: String,
    /// `content_digest` of the source bytes.
    pub identity: String,
}

impl SourceEntry {
    /// Build an entry, computing the content identity from the raw source.
    pub fn new(role: &'static str, name: Option<String>, path: String, source: &str) -> Self {
        SourceEntry {
            role,
            name,
            path,
            identity: content_digest(source.as_bytes()),
        }
    }
}

/// The full realized picture of a project, ready to serialize as `ajisai.lock`.
pub(crate) struct LockData {
    pub manifest_schema_version: u64,
    pub project_name: String,
    pub project_version: String,
    pub specification: Option<String>,
    /// Capability protocol strings the manifest allows (sorted).
    pub allowed: Vec<String>,
    /// Capability protocol strings the run actually required (sorted).
    pub required: Vec<String>,
    /// Sources in composition order (dependencies first, then entry).
    pub sources: Vec<SourceEntry>,
    /// Public words as `(name, contentIdentity)`, sorted by name.
    pub public_words: Vec<(String, String)>,
}

impl LockData {
    fn to_json(&self) -> serde_json::Value {
        let sources: Vec<serde_json::Value> = self
            .sources
            .iter()
            .map(|s| {
                let mut obj = serde_json::Map::new();
                obj.insert("role".into(), json!(s.role));
                if let Some(name) = &s.name {
                    obj.insert("name".into(), json!(name));
                }
                obj.insert("path".into(), json!(s.path));
                obj.insert("sourceIdentity".into(), json!(s.identity));
                serde_json::Value::Object(obj)
            })
            .collect();
        let public_words: Vec<serde_json::Value> = self
            .public_words
            .iter()
            .map(|(name, identity)| json!({ "name": name, "contentIdentity": identity }))
            .collect();
        json!({
            "lockfileVersion": LOCKFILE_VERSION,
            "manifestSchemaVersion": self.manifest_schema_version,
            "project": {
                "name": self.project_name,
                "version": self.project_version,
            },
            "specification": self.specification,
            "capabilities": {
                "allowed": self.allowed,
                "required": self.required,
            },
            "sources": sources,
            "publicWords": public_words,
        })
    }

    /// Canonical lockfile text: pretty-printed JSON plus a single trailing
    /// newline (the file convention shared with the rest of the CLI).
    pub fn render(&self) -> String {
        format!("{}\n", pretty(&self.to_json()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> LockData {
        LockData {
            manifest_schema_version: 1,
            project_name: "example".to_string(),
            project_version: "0.1.0".to_string(),
            specification: Some("1.0".to_string()),
            allowed: vec!["clock".to_string(), "effect".to_string()],
            required: vec!["effect".to_string()],
            sources: vec![
                SourceEntry::new(
                    "dependency",
                    Some("util".to_string()),
                    "../util/src/main.ajisai".to_string(),
                    "[ 2 ] DEF DOUBLE",
                ),
                SourceEntry::new("entry", None, "src/main.ajisai".to_string(), "[ 3 ]"),
            ],
            public_words: vec![("DOUBLE".to_string(), "sha256:abc".to_string())],
        }
    }

    #[test]
    fn render_is_deterministic_and_newline_terminated() {
        let a = sample().render();
        let b = sample().render();
        assert_eq!(a, b);
        assert!(a.ends_with("}\n"));
    }

    #[test]
    fn render_carries_the_required_lock_fields() {
        let text = sample().render();
        for needle in [
            "\"lockfileVersion\"",
            "\"manifestSchemaVersion\"",
            "\"specification\"",
            "\"allowed\"",
            "\"required\"",
            "\"sourceIdentity\"",
            "\"contentIdentity\"",
        ] {
            assert!(text.contains(needle), "missing {needle} in:\n{text}");
        }
    }

    #[test]
    fn source_identity_is_content_addressed() {
        let a = SourceEntry::new("entry", None, "m.ajisai".to_string(), "[ 1 ]");
        let b = SourceEntry::new("entry", None, "other.ajisai".to_string(), "[ 1 ]");
        let c = SourceEntry::new("entry", None, "m.ajisai".to_string(), "[ 2 ]");
        // Identity depends on content, not on the path.
        assert_eq!(a.identity, b.identity);
        assert_ne!(a.identity, c.identity);
    }
}
