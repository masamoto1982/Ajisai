//! `ajisai new` — scaffold a new project (Phase 8A / 8B follow-up).
//!
//! Creates a directory with an `ajisai.toml` manifest (`manifest.rs`) and a
//! runnable entry source, so a fresh project is immediately usable with
//! `ajisai build` and `ajisai lock`. The generated manifest is a valid instance
//! of the Phase 8B format; the generated entry is a minimal program that runs
//! successfully under the manifest's declared capabilities.
//!
//! This adds no language semantics — it only writes template files.

use std::path::Path;

/// `ajisai new <path>`: scaffold a project at `<path>`. The project name is the
/// final path component. Exit 0 on success, 2 on a usage error (a name that is
/// not a safe identifier, a path that already exists, or an I/O failure).
pub(crate) fn cmd_new(path_arg: &str) -> i32 {
    let path = Path::new(path_arg);

    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name.to_string(),
        None => {
            eprintln!("ajisai new: `{}` has no project name component", path_arg);
            return 2;
        }
    };
    if let Err(message) = validate_name(&name) {
        eprintln!("ajisai new: {}", message);
        return 2;
    }

    if path.exists() {
        eprintln!("ajisai new: {} already exists", path_arg);
        return 2;
    }

    let src_dir = path.join("src");
    if let Err(e) = std::fs::create_dir_all(&src_dir) {
        eprintln!("ajisai new: cannot create {}: {}", src_dir.display(), e);
        return 2;
    }

    let files = [
        (path.join("ajisai.toml"), manifest_template(&name)),
        (src_dir.join("main.ajisai"), entry_template(&name)),
    ];
    for (file, contents) in files {
        if let Err(e) = std::fs::write(&file, contents) {
            eprintln!("ajisai new: cannot write {}: {}", file.display(), e);
            return 2;
        }
    }

    println!("Created project `{}` at {}", name, path.display());
    println!("  {}/ajisai.toml", path_arg);
    println!("  {}/src/main.ajisai", path_arg);
    println!();
    println!("Next steps:");
    println!("  ajisai build {}     # run it", path_arg);
    println!(
        "  ajisai lock {}      # pin its identities into ajisai.lock",
        path_arg
    );
    0
}

/// A project name must be a safe identifier: it becomes both a directory name
/// and a manifest string, so it may hold only `[A-Za-z0-9._-]`, must be
/// non-empty, and may not be `.` or `..`. This keeps the generated manifest
/// unambiguous (the manifest parser rejects `"` and `\` in strings anyway).
fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("the project name is empty".to_string());
    }
    if name == "." || name == ".." {
        return Err(format!("`{}` is not a valid project name", name));
    }
    if let Some(bad) = name
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')))
    {
        return Err(format!(
            "project name `{}` contains an unsupported character `{}` (use letters, digits, `.`, `_`, `-`)",
            name, bad
        ));
    }
    Ok(())
}

fn manifest_template(name: &str) -> String {
    format!(
        "[project]\n\
         name = \"{name}\"\n\
         version = \"0.1.0\"\n\
         entry = \"src/main.ajisai\"\n\
         \n\
         [capabilities]\n\
         # Host capabilities this project may use. `effect` covers PRINT output.\n\
         allow = [\"effect\"]\n\
         \n\
         [dependencies]\n\
         # Local path dependencies, e.g.:\n\
         # util = {{ path = \"lib/util.ajisai\" }}\n"
    )
}

fn entry_template(name: &str) -> String {
    format!(
        "# {name} — a new Ajisai project.\n\
         #\n\
         # Run it:   ajisai build .\n\
         # Pin it:   ajisai lock .\n\
         [ 'Hello from {name}!' ] PRINT\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::manifest::parse_manifest;
    use crate::cli::project::cmd_build;
    use crate::cli::{Lang, Opts};
    use std::sync::atomic::{AtomicU64, Ordering};

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

    fn temp_path(leaf: &str) -> std::path::PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!("ajisai-new-{}-{}", std::process::id(), id));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        base.join(leaf)
    }

    #[test]
    fn scaffolds_a_runnable_project() {
        let dir = temp_path("greeter");
        assert_eq!(cmd_new(dir.to_str().unwrap()), 0);

        // The manifest is a valid Phase 8B manifest naming the project.
        let manifest_text = std::fs::read_to_string(dir.join("ajisai.toml")).unwrap();
        let manifest = parse_manifest(&manifest_text).expect("generated manifest parses");
        assert_eq!(manifest.project.name, "greeter");
        assert_eq!(manifest.project.entry, "src/main.ajisai");
        assert_eq!(manifest.allow, vec!["effect".to_string()]);
        assert!(manifest.dependencies.is_empty());

        // The generated entry exists and the project runs.
        assert!(dir.join("src/main.ajisai").exists());
        assert_eq!(cmd_build(dir.to_str().unwrap(), &opts()), 0);
    }

    #[test]
    fn refuses_an_existing_directory() {
        let dir = temp_path("taken");
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(cmd_new(dir.to_str().unwrap()), 2);
    }

    #[test]
    fn rejects_unsafe_names() {
        assert!(validate_name("").is_err());
        assert!(validate_name(".").is_err());
        assert!(validate_name("..").is_err());
        assert!(validate_name("bad name").is_err());
        assert!(validate_name("bad\"name").is_err());
        assert!(validate_name("bad/name").is_err());
        // Safe identifiers pass.
        assert!(validate_name("ok-name_1.2").is_ok());
    }
}
