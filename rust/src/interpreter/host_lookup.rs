// Looking a Word up is something the *host* does, not something a program does.
//
// This was `LOOKUP`, a Word: `'ADD' ?` ran through the interpreter, and the
// answer came back out of two fields on `Interpreter` that no evaluation rule
// ever read — the host drained them after the run and painted them somewhere.
// A Word whose entire result is a side channel to the editor is not part of the
// language, and its presence in the vocabulary made two claims that were not
// true: that a program can observe reference prose, and that the reference
// prose is a value.
//
// The names are the same, the spelling at the keyboard is the same, and the
// answer is the same text. What changed is who asks: the host parses
// `'ADD' ?` itself and calls in here, and the interpreter never sees it.

use crate::error::{AjisaiError, Result};
use crate::interpreter::Interpreter;

/// The text a lookup produced.
///
/// Both variants are prose to *read*, and a host shows them the same way, in
/// its output area (spec/gui-semantics.md, Lookup): the cursor can be anywhere
/// in a program still being written, so nothing about a lookup may overwrite
/// the editor — an earlier design loaded a definition back into it, and a
/// lookup then replaced a half-written program. They stay two variants because
/// they are two different texts, a reference entry and a reconstructed source,
/// and a host may present the source as source rather than as a reference page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostLookup {
    /// A Core Word's reference entry, to display.
    Documentation(String),
    /// A User Word's reconstructed `DEF` source, to display as read-only
    /// reference.
    Definition(String),
}

/// Resolve `name` against the dictionary the session currently holds.
///
/// Fails with `UnknownWord` for a name that is not defined — the host reports
/// that the same way it reports any other unknown name, so a typo at the
/// lookup prompt reads like a typo in a program.
pub fn resolve_host_lookup(interp: &Interpreter, name: &str) -> Result<HostLookup> {
    let canonical_name = crate::word_name::canonical_word_name(name);

    let Some(def) = interp.resolve_word(&canonical_name) else {
        return Err(AjisaiError::UnknownWord(name.to_string()));
    };

    if def.is_builtin {
        return Ok(HostLookup::Documentation(
            crate::builtins::lookup_builtin_detail(name),
        ));
    }

    let definition = interp
        .lookup_word_definition_tokens(&canonical_name)
        .unwrap_or_default();
    Ok(HostLookup::Definition(render_def_source(
        &definition,
        name,
        def.description.as_deref(),
    )))
}

/// Reconstruct the `DEF` source of a User Word so that running it again defines
/// the same word — the point of looking one up being to see an existing
/// definition as the program that would define it once more.
///
/// The body is wrapped in `[ ]`, the only bracket a program's code is written
/// in (`docs/dev/type-unification-work-order-2026-08.md`): `DEF` takes any
/// Vector as its body, and a bare name inside one is a Symbol — data until
/// something executes it — so a `[ ]`-wrapped body round-trips exactly like
/// the one that defined the word, whatever it called.
///
/// A multi-line body keeps its line structure. Where the break falls below the
/// body's own level it is presentation rather than meaning — but it is the
/// author's presentation, and a definition that comes back reformatted reads
/// as a definition that was changed.
///
/// A description is emitted as a leading `#` comment. `DEF` takes exactly two
/// positional arguments, so a third string on the line would be read as the
/// word's name; the comment carries the text without changing what runs.
fn render_def_source(definition: &str, name: &str, description: Option<&str>) -> String {
    let body = if definition.is_empty() {
        "[ NIL ]".to_string()
    } else if definition.contains('\n') {
        format!("[\n{}\n]", definition)
    } else {
        format!("[ {} ]", definition)
    };
    let source = format!("{} '{}' DEF", body, name);
    match description {
        Some(desc) if !desc.is_empty() => format!("# {}\n{}", desc.replace('\n', " "), source),
        _ => source,
    }
}
