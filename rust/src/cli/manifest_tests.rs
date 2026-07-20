//! Tests for the `ajisai.toml` manifest parser (Phase 8B).

use super::manifest::{parse_manifest, Dependency};

const FULL: &str = r#"
# a project manifest
[project]
name = "example"
version = "0.1.0"
entry = "src/main.ajisai"
specification = "1.0"

[capabilities]
allow = ["effect", "clock"]

[dependencies]
util = { path = "../util" }
math = { path = "./vendor/math" }
"#;

#[test]
fn parses_a_full_manifest() {
    let m = parse_manifest(FULL).expect("valid manifest");
    assert_eq!(m.project.name, "example");
    assert_eq!(m.project.version, "0.1.0");
    assert_eq!(m.project.entry, "src/main.ajisai");
    assert_eq!(m.project.specification.as_deref(), Some("1.0"));
    // allow is sorted.
    assert_eq!(m.allow, vec!["clock".to_string(), "effect".to_string()]);
    assert_eq!(
        m.dependencies,
        vec![
            Dependency {
                name: "util".to_string(),
                path: "../util".to_string()
            },
            Dependency {
                name: "math".to_string(),
                path: "./vendor/math".to_string()
            },
        ]
    );
}

#[test]
fn minimal_manifest_defaults_are_empty() {
    let src = "[project]\nname = \"x\"\nversion = \"0.0.1\"\nentry = \"m.ajisai\"\n";
    let m = parse_manifest(src).expect("valid");
    assert_eq!(m.project.specification, None);
    assert!(m.allow.is_empty());
    assert!(m.dependencies.is_empty());
}

#[test]
fn empty_allow_array_is_accepted() {
    let src =
        "[project]\nname=\"x\"\nversion=\"1\"\nentry=\"m.ajisai\"\n[capabilities]\nallow = []\n";
    let m = parse_manifest(src).expect("valid");
    assert!(m.allow.is_empty());
}

#[test]
fn duplicate_capabilities_are_deduplicated() {
    let src = "[project]\nname=\"x\"\nversion=\"1\"\nentry=\"m.ajisai\"\n[capabilities]\nallow = [\"effect\", \"effect\"]\n";
    let m = parse_manifest(src).expect("valid");
    assert_eq!(m.allow, vec!["effect".to_string()]);
}

#[test]
fn hash_inside_a_string_is_not_a_comment() {
    let src = "[project]\nname = \"a#b\"\nversion = \"1\"\nentry = \"m.ajisai\"\n";
    let m = parse_manifest(src).expect("valid");
    assert_eq!(m.project.name, "a#b");
}

#[test]
fn missing_required_key_is_an_error() {
    let src = "[project]\nname = \"x\"\nentry = \"m.ajisai\"\n";
    let err = parse_manifest(src).unwrap_err();
    assert!(err.contains("version"), "unexpected error: {err}");
}

#[test]
fn unknown_section_is_an_error() {
    let src = "[project]\nname=\"x\"\nversion=\"1\"\nentry=\"m.ajisai\"\n[bogus]\nk = \"v\"\n";
    let err = parse_manifest(src).unwrap_err();
    assert!(err.contains("unknown section"), "unexpected error: {err}");
}

#[test]
fn unknown_project_key_is_an_error() {
    let src = "[project]\nname=\"x\"\nversion=\"1\"\nentry=\"m.ajisai\"\nbogus = \"v\"\n";
    let err = parse_manifest(src).unwrap_err();
    assert!(err.contains("unknown [project] key"), "unexpected: {err}");
}

#[test]
fn key_before_any_section_is_an_error() {
    let err = parse_manifest("name = \"x\"\n").unwrap_err();
    assert!(err.contains("before any"), "unexpected error: {err}");
}

#[test]
fn dependency_without_path_is_an_error() {
    let src =
        "[project]\nname=\"x\"\nversion=\"1\"\nentry=\"m.ajisai\"\n[dependencies]\nutil = { rev = \"1\" }\n";
    let err = parse_manifest(src).unwrap_err();
    assert!(err.contains("path"), "unexpected error: {err}");
}

#[test]
fn duplicate_dependency_is_an_error() {
    let src = "[project]\nname=\"x\"\nversion=\"1\"\nentry=\"m.ajisai\"\n[dependencies]\nutil = { path = \"a\" }\nutil = { path = \"b\" }\n";
    let err = parse_manifest(src).unwrap_err();
    assert!(err.contains("duplicate dependency"), "unexpected: {err}");
}

#[test]
fn unquoted_value_is_an_error() {
    let src = "[project]\nname = example\nversion = \"1\"\nentry = \"m.ajisai\"\n";
    let err = parse_manifest(src).unwrap_err();
    assert!(err.contains("double-quoted"), "unexpected error: {err}");
}
