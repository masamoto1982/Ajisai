//! `ajisai.toml` project manifest (Phase 8B).
//!
//! The manifest declares a project's *intent*: its name and version, the entry
//! source, the host capabilities it is allowed to use, and its local path
//! dependencies. It is the human-authored half of the manifest/lockfile split
//! (SPEC handoff §15.2): the manifest states what the project may do; the
//! generated lockfile (`lockfile.rs`) records what it *is* — realized source
//! and word content identities, and the capabilities actually required.
//!
//! Only a small, fixed TOML subset is accepted — enough for the documented
//! shape and no more — parsed by hand so Core stays dependency-light (no `toml`
//! crate). The recognized shape is:
//!
//! ```toml
//! [project]
//! name = "example"
//! version = "0.1.0"
//! entry = "src/main.ajisai"
//! specification = "1.0"        # optional
//!
//! [capabilities]
//! allow = ["effect", "clock"]  # optional; default: none
//!
//! [dependencies]
//! util = { path = "../util" }  # local path dependencies only (§15.2)
//! ```

/// Schema version of the manifest/lockfile format. Bumped only on a breaking
/// change to the accepted shape; additive fields keep the same version.
pub(crate) const MANIFEST_SCHEMA_VERSION: u64 = 1;

/// The `[project]` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Project {
    pub name: String,
    pub version: String,
    pub entry: String,
    /// The specification version this project targets, if declared.
    pub specification: Option<String>,
}

/// A single local path dependency (`name = { path = "..." }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Dependency {
    pub name: String,
    pub path: String,
}

/// A parsed `ajisai.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Manifest {
    pub project: Project,
    /// Capability protocol strings the project is allowed to use (the same
    /// vocabulary as `HostCapability::as_protocol_str`). Sorted, de-duplicated.
    pub allow: Vec<String>,
    /// Local path dependencies, in declared order.
    pub dependencies: Vec<Dependency>,
}

/// Parse an `ajisai.toml` from its text, or return a human-readable error
/// naming the offending line. The parser is intentionally strict: an unknown
/// section, an unknown key, or a malformed value is an error, so a typo never
/// silently changes what a project is allowed to do.
#[derive(Clone, Copy)]
enum Section {
    Project,
    Capabilities,
    Dependencies,
}

pub(crate) fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let mut section: Option<Section> = None;
    let mut name = None;
    let mut version = None;
    let mut entry = None;
    let mut specification = None;
    let mut allow: Vec<String> = Vec::new();
    let mut dependencies: Vec<Dependency> = Vec::new();

    for (lineno, raw) in text.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        let at = || format!("line {}", lineno + 1);

        if let Some(header) = line.strip_prefix('[') {
            let header = header
                .strip_suffix(']')
                .ok_or_else(|| format!("{}: unterminated section header `{}`", at(), line))?;
            section = Some(match header.trim() {
                "project" => Section::Project,
                "capabilities" => Section::Capabilities,
                "dependencies" => Section::Dependencies,
                other => return Err(format!("{}: unknown section `[{}]`", at(), other)),
            });
            continue;
        }

        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("{}: expected `key = value`, found `{}`", at(), line))?;
        let key = key.trim();
        let value = value.trim();

        match section {
            Some(Section::Project) => match key {
                "name" => name = Some(parse_string(value).map_err(|e| format!("{}: {}", at(), e))?),
                "version" => {
                    version = Some(parse_string(value).map_err(|e| format!("{}: {}", at(), e))?)
                }
                "entry" => {
                    entry = Some(parse_string(value).map_err(|e| format!("{}: {}", at(), e))?)
                }
                "specification" => {
                    specification =
                        Some(parse_string(value).map_err(|e| format!("{}: {}", at(), e))?)
                }
                other => return Err(format!("{}: unknown [project] key `{}`", at(), other)),
            },
            Some(Section::Capabilities) => match key {
                "allow" => {
                    for cap in parse_string_array(value).map_err(|e| format!("{}: {}", at(), e))? {
                        // Validate at the parse boundary so a typo'd capability
                        // is rejected here rather than silently carried into the
                        // manifest — fulfilling this parser's stated principle
                        // that a typo never changes what a project is allowed to
                        // do. The vocabulary is exactly `HostCapability`.
                        if crate::interpreter::HostCapability::from_protocol_str(&cap).is_none() {
                            return Err(format!(
                                "{}: unknown capability `{}` in [capabilities] allow",
                                at(),
                                cap
                            ));
                        }
                        if !allow.contains(&cap) {
                            allow.push(cap);
                        }
                    }
                }
                other => return Err(format!("{}: unknown [capabilities] key `{}`", at(), other)),
            },
            Some(Section::Dependencies) => {
                let path = parse_dependency_path(value).map_err(|e| format!("{}: {}", at(), e))?;
                if dependencies.iter().any(|d| d.name == key) {
                    return Err(format!("{}: duplicate dependency `{}`", at(), key));
                }
                dependencies.push(Dependency {
                    name: key.to_string(),
                    path,
                });
            }
            None => {
                return Err(format!(
                    "{}: `{}` appears before any `[section]` header",
                    at(),
                    key
                ))
            }
        }
    }

    allow.sort();
    let project = Project {
        name: name.ok_or("missing required key `name` in [project]")?,
        version: version.ok_or("missing required key `version` in [project]")?,
        entry: entry.ok_or("missing required key `entry` in [project]")?,
        specification,
    };
    Ok(Manifest {
        project,
        allow,
        dependencies,
    })
}

/// Drop an unquoted trailing `#` comment, leaving text inside string literals
/// untouched (a `#` between quotes is data, not a comment).
fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    for (idx, ch) in line.char_indices() {
        match ch {
            '"' => in_string = !in_string,
            '#' if !in_string => return &line[..idx],
            _ => {}
        }
    }
    line
}

/// Parse a double-quoted string literal into its contents. No escape sequences
/// are recognized: project names, versions, and relative paths do not need
/// them, and rejecting `\` keeps the surface unambiguous.
fn parse_string(value: &str) -> Result<String, String> {
    let inner = value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .ok_or_else(|| format!("expected a double-quoted string, found `{}`", value))?;
    if inner.contains('"') || inner.contains('\\') {
        return Err(format!("string `{}` may not contain `\"` or `\\`", value));
    }
    Ok(inner.to_string())
}

/// Parse a `[ "a", "b" ]` array of double-quoted strings. An empty `[]` yields
/// an empty vector.
fn parse_string_array(value: &str) -> Result<Vec<String>, String> {
    let inner = value
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .ok_or_else(|| format!("expected an array `[...]`, found `{}`", value))?
        .trim();
    if inner.is_empty() {
        return Ok(Vec::new());
    }
    inner
        .split(',')
        .map(|item| parse_string(item.trim()))
        .collect()
}

/// Parse a dependency value `{ path = "..." }` into its path. Only the `path`
/// key is accepted (local path dependencies only, §15.2).
fn parse_dependency_path(value: &str) -> Result<String, String> {
    let inner = value
        .strip_prefix('{')
        .and_then(|v| v.strip_suffix('}'))
        .ok_or_else(|| format!("expected `{{ path = \"...\" }}`, found `{}`", value))?
        .trim();
    let (key, path) = inner
        .split_once('=')
        .ok_or_else(|| format!("expected `path = \"...\"` inside `{}`", value))?;
    match key.trim() {
        "path" => parse_string(path.trim()),
        other => Err(format!(
            "unknown dependency key `{}` (only `path` is supported)",
            other
        )),
    }
}
