//! `ajisai build` and `ajisai lock` — project-aware run and lockfile generation
//! (Phase 8B).
//!
//! A *project* is a directory with an `ajisai.toml` manifest (`manifest.rs`).
//! `build` resolves the manifest, composes the program from its local path
//! dependencies and entry source, runs it under a host confined to the
//! manifest's capability allow-list, and — if an `ajisai.lock` is present —
//! verifies the run's realized identities against it so execution is
//! reproducible. `lock` writes (or, with `--check`, verifies) that lockfile.
//!
//! The composition model is deliberately small (SPEC handoff §15.2, "start with
//! local path dependencies"): a dependency's `path` names an Ajisai *source
//! file* relative to the manifest directory. Dependency sources are executed
//! before the entry, in declared order, into one shared dictionary — a flat,
//! direct-dependency namespace. Transitive dependencies, directory/sub-manifest
//! dependencies, and remote registries are out of scope for this phase.
//!
//! This module adds no language semantics: it drives the same production Core
//! as `run` and only *observes* content identities and required capabilities
//! (via the Phase 6 receipt recorder). Capability confinement reuses the
//! existing capability gate — the project host simply reports a capability as
//! unavailable when the manifest does not allow it, so a disallowed Hosted word
//! fails through the ordinary structured missing-capability path.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::interpreter::{HostCapability, HostEnv, Interpreter};

use super::lockfile::{LockData, SourceEntry};
use super::manifest::{parse_manifest, Manifest, MANIFEST_SCHEMA_VERSION};
use super::run_render::render_completed_run;
use super::{block_on, print_payloads, Opts};

/// The manifest filename resolved inside a project directory.
const MANIFEST_FILE: &str = "ajisai.toml";
/// The generated lockfile filename.
const LOCK_FILE: &str = "ajisai.lock";

/// A host confined to a project's declared capability allow-list. A capability
/// is available only when the manifest allows it *and* the underlying terminal
/// actually provides it (the manifest can restrict, never conjure a device).
#[derive(Debug)]
struct ProjectHostEnv {
    allowed: BTreeSet<HostCapability>,
}

impl HostEnv for ProjectHostEnv {
    fn now_millis(&self) -> i64 {
        crate::interpreter::datetime::default_now_millis()
    }

    fn fill_random(&self, buf: &mut [u8]) -> std::result::Result<(), String> {
        crate::interpreter::random::default_fill_random(buf)
    }

    fn has_capability(&self, capability: HostCapability) -> bool {
        self.allowed.contains(&capability) && super::host::CliHostEnv.has_capability(capability)
    }
}

/// Map a manifest capability string to a modeled host capability. The manifest
/// vocabulary is exactly `HostCapability::as_protocol_str`, so the allow-list,
/// the runtime gate, and the receipt all speak the same names.
fn capability_from_protocol(name: &str) -> Option<HostCapability> {
    HostCapability::ALL
        .into_iter()
        .find(|cap| cap.as_protocol_str() == name)
}

/// One resolved source file that composes the project.
struct LoadedSource {
    /// Dependency name, or `None` for the entry.
    name: Option<String>,
    /// Path as declared in the manifest, relative to the manifest directory
    /// (recorded verbatim in the lockfile for a stable, portable identity).
    display_path: String,
    source: String,
}

/// A project resolved from its manifest, ready to run or lock.
struct LoadedProject {
    root: PathBuf,
    manifest: Manifest,
    allowed: BTreeSet<HostCapability>,
    /// Dependencies in declared order; the entry runs after them.
    dependencies: Vec<LoadedSource>,
    entry: LoadedSource,
}

/// Resolve a project from a path that is either its directory or its
/// `ajisai.toml` directly. Every failure here is a usage error (exit 2): a
/// missing manifest, a parse error, an unknown capability, or an unreadable
/// source is a problem with how the project is set up, not a language error.
fn load_project(path_arg: &str) -> Result<LoadedProject, String> {
    let path = Path::new(path_arg);
    let manifest_path = if path.is_dir() {
        path.join(MANIFEST_FILE)
    } else {
        path.to_path_buf()
    };
    let root = manifest_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let manifest_text = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("cannot read {}: {}", manifest_path.display(), e))?;
    let manifest = parse_manifest(&manifest_text)?;

    let mut allowed = BTreeSet::new();
    for name in &manifest.allow {
        match capability_from_protocol(name) {
            Some(cap) => {
                allowed.insert(cap);
            }
            None => return Err(format!("unknown capability `{}` in [capabilities]", name)),
        }
    }

    let dependencies = manifest
        .dependencies
        .iter()
        .map(|dep| load_source(&root, &dep.path, Some(dep.name.clone())))
        .collect::<Result<Vec<_>, _>>()?;
    let entry = load_source(&root, &manifest.project.entry, None)?;

    Ok(LoadedProject {
        root,
        manifest,
        allowed,
        dependencies,
        entry,
    })
}

/// Read one source file relative to the manifest directory, keeping the
/// declared (relative) path for the lockfile.
fn load_source(root: &Path, rel: &str, name: Option<String>) -> Result<LoadedSource, String> {
    let full = root.join(rel);
    let source = std::fs::read_to_string(&full)
        .map_err(|e| format!("cannot read {}: {}", full.display(), e))?;
    Ok(LoadedSource {
        name,
        display_path: rel.to_string(),
        source,
    })
}

/// Run the composed project (dependencies, then entry) through the confined
/// host. Returns the interpreter and the entry's result. A dependency that
/// fails short-circuits: its error is returned as the run's result.
fn run_project(loaded: &LoadedProject) -> (Interpreter, crate::error::Result<()>) {
    let host = ProjectHostEnv {
        allowed: loaded.allowed.clone(),
    };
    let mut interp = Interpreter::with_host(Arc::new(host));
    // Observe required capabilities and word identities without changing the
    // run (Phase 6 recording is purely observational).
    interp.set_receipt_recording(true);

    for dep in &loaded.dependencies {
        if let Err(err) = block_on(interp.execute(&dep.source)) {
            return (interp, Err(err));
        }
    }
    let result = block_on(interp.execute(&loaded.entry.source));
    (interp, result)
}

/// Build the realized lockfile data from a completed run.
fn build_lock_data(loaded: &LoadedProject, interp: &Interpreter) -> LockData {
    let mut sources: Vec<SourceEntry> = loaded
        .dependencies
        .iter()
        .map(|dep| {
            SourceEntry::new(
                "dependency",
                dep.name.clone(),
                dep.display_path.clone(),
                &dep.source,
            )
        })
        .collect();
    sources.push(SourceEntry::new(
        "entry",
        None,
        loaded.entry.display_path.clone(),
        &loaded.entry.source,
    ));

    // Every user word carries a §8.6 content identity keyed by its
    // fully-qualified name; that map is the project's public word surface.
    let mut public_words: Vec<(String, String)> = interp
        .word_identities
        .iter()
        .map(|(name, identity)| (name.clone(), identity.clone()))
        .collect();
    public_words.sort();

    let required: Vec<String> = interp
        .receipt_recorder()
        .required_capabilities()
        .iter()
        .map(|cap| cap.as_protocol_str().to_string())
        .collect();

    LockData {
        manifest_schema_version: MANIFEST_SCHEMA_VERSION,
        project_name: loaded.manifest.project.name.clone(),
        project_version: loaded.manifest.project.version.clone(),
        specification: loaded.manifest.project.specification.clone(),
        allowed: loaded.manifest.allow.clone(),
        required,
        sources,
        public_words,
    }
}

/// `ajisai build <dir-or-manifest>`: resolve the manifest, run the project
/// under its capability allow-list, and — when an `ajisai.lock` exists — verify
/// the run reproduces its pinned identities. Exit 0 on success, 1 on a language
/// error or a lock mismatch, 2 on a project setup error.
pub(crate) fn cmd_build(path_arg: &str, opts: &Opts) -> i32 {
    let loaded = match load_project(path_arg) {
        Ok(loaded) => loaded,
        Err(message) => {
            eprintln!("ajisai build: {}", message);
            return 2;
        }
    };

    let (mut interp, result) = run_project(&loaded);
    let trace = interp.drain_error_flow_trace();
    let output = print_payloads(&interp);
    let succeeded = result.is_ok();

    // Verify reproducibility only for a successful run: identities are
    // meaningless for a program that did not complete.
    let drift = succeeded.then(|| lock_drift(&loaded, &interp)).flatten();

    let code = render_completed_run(&interp, result, trace, output, None, opts);
    if code != 0 {
        return code;
    }
    match drift {
        Some(message) => {
            eprintln!("ajisai build: {}", message);
            1
        }
        None => 0,
    }
}

/// If an `ajisai.lock` exists, compare the freshly realized lock against it and
/// return a drift message when they differ. `None` means either no lockfile
/// (nothing to verify) or an exact match.
fn lock_drift(loaded: &LoadedProject, interp: &Interpreter) -> Option<String> {
    let lock_path = loaded.root.join(LOCK_FILE);
    let existing = std::fs::read_to_string(&lock_path).ok()?;
    let fresh = build_lock_data(loaded, interp).render();
    if existing == fresh {
        None
    } else {
        Some(format!(
            "{} is out of date; run `ajisai lock` to update it",
            lock_path.display()
        ))
    }
}

/// `ajisai lock <dir-or-manifest>`: run the project and write `ajisai.lock`
/// with its realized identities and required capabilities. With `--check`,
/// verify the lockfile is up to date instead of writing (exit 1 on drift).
pub(crate) fn cmd_lock(path_arg: &str, opts: &Opts) -> i32 {
    let loaded = match load_project(path_arg) {
        Ok(loaded) => loaded,
        Err(message) => {
            eprintln!("ajisai lock: {}", message);
            return 2;
        }
    };

    let (interp, result) = run_project(&loaded);
    if let Err(err) = result {
        eprintln!("ajisai lock: project failed to run: {}", err);
        return 1;
    }

    let lock = build_lock_data(&loaded, &interp);
    let text = lock.render();
    let lock_path = loaded.root.join(LOCK_FILE);

    if opts.fmt_check {
        match std::fs::read_to_string(&lock_path) {
            Ok(existing) if existing == text => 0,
            Ok(_) => {
                eprintln!(
                    "ajisai lock: {} is out of date; run `ajisai lock` to update it",
                    lock_path.display()
                );
                1
            }
            Err(_) => {
                eprintln!(
                    "ajisai lock: {} does not exist; run `ajisai lock` to create it",
                    lock_path.display()
                );
                1
            }
        }
    } else if let Err(e) = std::fs::write(&lock_path, &text) {
        eprintln!("ajisai lock: cannot write {}: {}", lock_path.display(), e);
        2
    } else {
        println!(
            "wrote {} ({} source{}, {} public word{})",
            lock_path.display(),
            lock.sources.len(),
            if lock.sources.len() == 1 { "" } else { "s" },
            lock.public_words.len(),
            if lock.public_words.len() == 1 {
                ""
            } else {
                "s"
            },
        );
        0
    }
}
