//! End-to-end tests for `ajisai build` / `ajisai lock` (Phase 8B).
//!
//! These drive the public commands over real temp-dir projects and assert on
//! exit codes and the written `ajisai.lock`, exercising manifest resolution,
//! capability confinement, dependency composition, and reproducibility drift.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::project::{cmd_build, cmd_lock};
use super::{Lang, Opts};

fn opts() -> Opts {
    Opts {
        json: false,
        explain: false,
        contract: false,
        receipt: false,
        fmt_check: false,
        fmt_write: false,
        lang: Lang::Ja,
        step_limit: None,
    }
}

fn check_opts() -> Opts {
    Opts {
        fmt_check: true,
        ..opts()
    }
}

/// Create a fresh temp project directory populated with the given files.
fn temp_project(files: &[(&str, &str)]) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("ajisai-8b-{}-{}", std::process::id(), id));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (rel, content) in files {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }
    dir
}

fn dir_arg(dir: &Path) -> String {
    dir.to_str().unwrap().to_string()
}

fn read_lock(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("ajisai.lock")).unwrap()
}

const DEF_ONLY: &str = "[project]\nname = \"lib\"\nversion = \"0.1.0\"\nentry = \"main.ajisai\"\n";

#[test]
fn lock_writes_a_content_addressed_lockfile() {
    let dir = temp_project(&[
        ("ajisai.toml", DEF_ONLY),
        ("main.ajisai", "{ [ 2 ] * } 'DOUBLE' DEF"),
    ]);
    assert_eq!(cmd_lock(&dir_arg(&dir), &opts()), 0);
    let lock = read_lock(&dir);
    assert!(lock.contains("\"lockfileVersion\""));
    assert!(lock.contains("DOUBLE"));
    assert!(lock.contains("\"contentIdentity\""));
    assert!(lock.contains("\"sourceIdentity\""));
}

#[test]
fn lock_check_passes_when_current_and_fails_when_stale() {
    let dir = temp_project(&[
        ("ajisai.toml", DEF_ONLY),
        ("main.ajisai", "{ [ 2 ] * } 'DOUBLE' DEF"),
    ]);
    assert_eq!(cmd_lock(&dir_arg(&dir), &opts()), 0);
    // Freshly written: --check agrees.
    assert_eq!(cmd_lock(&dir_arg(&dir), &check_opts()), 0);
    // Change the source: the pinned word identity no longer matches.
    std::fs::write(dir.join("main.ajisai"), "{ [ 3 ] * } 'DOUBLE' DEF").unwrap();
    assert_eq!(cmd_lock(&dir_arg(&dir), &check_opts()), 1);
}

#[test]
fn lock_check_fails_when_no_lockfile_exists() {
    let dir = temp_project(&[
        ("ajisai.toml", DEF_ONLY),
        ("main.ajisai", "{ [ 2 ] * } 'DOUBLE' DEF"),
    ]);
    assert_eq!(cmd_lock(&dir_arg(&dir), &check_opts()), 1);
}

#[test]
fn build_runs_a_project_with_an_allowed_capability() {
    let toml = "[project]\nname=\"app\"\nversion=\"0.1.0\"\nentry=\"main.ajisai\"\n[capabilities]\nallow=[\"effect\"]\n";
    let dir = temp_project(&[("ajisai.toml", toml), ("main.ajisai", "[ 'HI' ] PRINT")]);
    assert_eq!(cmd_build(&dir_arg(&dir), &opts()), 0);
}

#[test]
fn build_fails_when_a_capability_is_not_allowed() {
    // PRINT needs the `effect` capability; the manifest allows none.
    let toml = "[project]\nname=\"app\"\nversion=\"0.1.0\"\nentry=\"main.ajisai\"\n[capabilities]\nallow=[]\n";
    let dir = temp_project(&[("ajisai.toml", toml), ("main.ajisai", "[ 'HI' ] PRINT")]);
    assert_eq!(cmd_build(&dir_arg(&dir), &opts()), 1);
}

#[test]
fn build_composes_a_dependency_before_the_entry() {
    let toml = "[project]\nname=\"app\"\nversion=\"0.1.0\"\nentry=\"main.ajisai\"\n[capabilities]\nallow=[\"effect\"]\n[dependencies]\nutil = { path = \"util.ajisai\" }\n";
    let dir = temp_project(&[
        ("ajisai.toml", toml),
        ("util.ajisai", "{ [ 2 ] * } 'DOUBLE' DEF"),
        // Entry uses a word the dependency defines.
        ("main.ajisai", "[ 5 ] DOUBLE PRINT"),
    ]);
    assert_eq!(cmd_build(&dir_arg(&dir), &opts()), 0);
    // The dependency source and its public word are recorded in the lock.
    assert_eq!(cmd_lock(&dir_arg(&dir), &opts()), 0);
    let lock = read_lock(&dir);
    assert!(lock.contains("\"dependency\""));
    assert!(lock.contains("util.ajisai"));
    assert!(lock.contains("DOUBLE"));
}

#[test]
fn build_verifies_the_lockfile_and_detects_drift() {
    let dir = temp_project(&[
        ("ajisai.toml", DEF_ONLY),
        ("main.ajisai", "{ [ 2 ] * } 'DOUBLE' DEF"),
    ]);
    // Pin, then a matching build passes.
    assert_eq!(cmd_lock(&dir_arg(&dir), &opts()), 0);
    assert_eq!(cmd_build(&dir_arg(&dir), &opts()), 0);
    // Change the source without re-locking: build refuses (not reproducible).
    std::fs::write(dir.join("main.ajisai"), "{ [ 9 ] * } 'DOUBLE' DEF").unwrap();
    assert_eq!(cmd_build(&dir_arg(&dir), &opts()), 1);
}

#[test]
fn build_without_a_lockfile_still_runs() {
    let dir = temp_project(&[
        ("ajisai.toml", DEF_ONLY),
        ("main.ajisai", "{ [ 2 ] * } 'DOUBLE' DEF"),
    ]);
    assert_eq!(cmd_build(&dir_arg(&dir), &opts()), 0);
}

#[test]
fn missing_manifest_is_a_usage_error() {
    let dir = temp_project(&[]);
    assert_eq!(cmd_build(&dir_arg(&dir), &opts()), 2);
    assert_eq!(cmd_lock(&dir_arg(&dir), &opts()), 2);
}

#[test]
fn unknown_capability_is_a_usage_error() {
    let toml = "[project]\nname=\"app\"\nversion=\"0.1.0\"\nentry=\"main.ajisai\"\n[capabilities]\nallow=[\"bogus\"]\n";
    let dir = temp_project(&[("ajisai.toml", toml), ("main.ajisai", "[ 1 ]")]);
    assert_eq!(cmd_build(&dir_arg(&dir), &opts()), 2);
}

#[test]
fn build_reports_a_language_error_from_the_entry() {
    let dir = temp_project(&[("ajisai.toml", DEF_ONLY), ("main.ajisai", "NOSUCHWORD")]);
    assert_eq!(cmd_build(&dir_arg(&dir), &opts()), 1);
}
