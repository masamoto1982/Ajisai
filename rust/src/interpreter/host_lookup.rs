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

/// What a host should do with the text a lookup produced.
///
/// The two destinations are opposite, which is why they are separate variants
/// rather than one string. Reference text is *read*, so it belongs in the output
/// area, where reading it costs the reader nothing. A reconstructed definition
/// is *edited*, so it belongs in the editor — that is the whole point of looking
/// a User Word up. Collapsing them meant `'ADD' ?` replaced a half-written
/// program with several screens of prose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostLookup {
    /// A Core Word's reference entry, to display.
    Documentation(String),
    /// A User Word's reconstructed `DEF` source, to load into the editor.
    Definition(String),
}

/// Resolve `name` against the dictionary the session currently holds.
///
/// Fails with `UnknownWord` for a name that is not defined — the host reports
/// that the same way it reports any other unknown name, so a typo at the
/// lookup prompt reads like a typo in a program.
pub fn resolve_host_lookup(interp: &Interpreter, name: &str) -> Result<HostLookup> {
    let canonical_name = crate::core_word_aliases::canonicalize_core_word_name(name);

    let Some(def) = interp.resolve_word(&canonical_name) else {
        return Err(AjisaiError::UnknownWord(name.to_string()));
    };

    if def.is_builtin {
        return Ok(HostLookup::Documentation(
            crate::builtins::lookup_builtin_detail(name),
        ));
    }

    if let Some(original_source) = &def.original_source {
        return Ok(HostLookup::Definition(original_source.clone()));
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
/// the same word — the point of looking one up being to load an existing
/// definition, edit it, and define it once more.
///
/// The body is wrapped in a **code block**. It used to be wrapped in `[ ]`, and
/// the result did not round-trip in either direction: `DEF` refuses a Vector
/// body ("DEF requires a code block { ... } as the definition body"), and even
/// as a value the meaning differed, since LANG.VALUES.VECTOR makes a bare name
/// inside `[ ]` its own text rather than a call. For a word whose body holds
/// `|` clauses the loaded text did not even tokenize, because `|` is COND-clause
/// sugar and is meaningless inside a Vector.
///
/// A multi-line body keeps its line structure. COND no longer requires one `|`
/// clause per line, so this is presentation rather than meaning — but it is the
/// presentation the formatter treats as canonical, and a definition that comes
/// back reformatted reads as a definition that was changed.
///
/// A description is emitted as a leading `#` comment. `DEF` takes exactly two
/// positional arguments, so a third string on the line would be read as the
/// word's name; the comment carries the text without changing what runs.
fn render_def_source(definition: &str, name: &str, description: Option<&str>) -> String {
    let body = if definition.is_empty() {
        "{ NIL }".to_string()
    } else if definition.contains('\n') {
        format!("{{\n{}\n}}", definition)
    } else {
        format!("{{ {} }}", definition)
    };
    let source = format!("{} '{}' DEF", body, name);
    match description {
        Some(desc) if !desc.is_empty() => format!("# {}\n{}", desc.replace('\n', " "), source),
        _ => source,
    }
}
