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
use crate::interpreter::{content_digest, IDENTITY_ALGORITHM};

/// Version of the lockfile document shape. Bumped only on a breaking change.
///
/// 2 — every identity in the file is a BLAKE3 digest. Version 1 recorded
/// polynomial-hash digests of the same shape, so the two are indistinguishable
/// by inspection; that is why a version-1 file is rejected outright rather than
/// compared (see [`identity_migration_error`]).
pub(crate) const LOCKFILE_VERSION: u64 = 2;

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
            "identityAlgorithm": IDENTITY_ALGORITHM,
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

/// Why an existing lockfile cannot be compared against a freshly realized one,
/// when the reason is that it predates the current identity algorithm.
///
/// A version-1 lockfile holds polynomial-hash digests in the same `#` + 64-hex
/// shape as a BLAKE3 digest, so a plain text comparison would report it as
/// ordinary drift and send the reader looking for a source change that never
/// happened. Returning `Some` here lets the caller say what actually changed.
/// `None` means the file is current (or unreadable as JSON, in which case the
/// normal drift path reports it).
pub(crate) fn identity_migration_error(existing: &str) -> Option<String> {
    let doc: serde_json::Value = serde_json::from_str(existing).ok()?;
    let version = doc.get("lockfileVersion")?.as_u64()?;
    if version >= LOCKFILE_VERSION {
        return None;
    }
    let recorded = doc
        .get("identityAlgorithm")
        .and_then(|v| v.as_str())
        .unwrap_or("a non-cryptographic polynomial hash");
    Some(format!(
        "this lockfile is version {version} and its identities were computed with \
         {recorded}; identities are now {IDENTITY_ALGORITHM} (lockfile version \
         {LOCKFILE_VERSION}). The recorded digests cannot be compared against \
         current ones — regenerate the file with `ajisai lock`"
    ))
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
    fn render_names_the_identity_algorithm() {
        let text = sample().render();
        assert!(text.contains("\"identityAlgorithm\": \"blake3\""), "{text}");
        assert!(text.contains("\"lockfileVersion\": 2"), "{text}");
    }

    #[test]
    fn a_current_lockfile_needs_no_migration() {
        assert_eq!(identity_migration_error(&sample().render()), None);
    }

    #[test]
    fn a_version_1_lockfile_reports_the_identity_migration() {
        // Shape-identical to a current file: same `#` + 64-hex identities, so
        // only the version distinguishes it.
        let old = r#"{"lockfileVersion": 1, "sources": []}"#;
        let message = identity_migration_error(old).expect("version 1 must be rejected");
        assert!(message.contains("blake3"), "{message}");
        assert!(message.contains("regenerate"), "{message}");
        assert!(message.contains("ajisai lock"), "{message}");
    }

    #[test]
    fn a_non_lockfile_falls_through_to_the_ordinary_drift_path() {
        // No JSON, and JSON without the version field, are both reported by the
        // caller's normal comparison rather than as a migration.
        assert_eq!(identity_migration_error("not json at all"), None);
        assert_eq!(identity_migration_error("{}"), None);
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
