use super::builtin_word_definitions::{lookup_builtin_spec, BuiltinSpec};
use super::builtin_word_lookup_docs::lookup_builtin_lookup_doc;
use crate::core_word_aliases::{lookup_core_word_alias, CoreWordAliasKind};
use crate::coreword_registry::{ExecutionForm, NilPolicy, Partiality};

/// Render the LOOKUP body for a built-in word: the four authored base
/// sections (Category / Summary / Role / Stack Effect), the authored
/// Layer 2 sections when `builtin_word_lookup_docs.rs` carries an entry
/// (Behavior / Examples / Failure note / Related), and the sections
/// derived from the §7.14 contract metadata (Failure baseline, Side
/// Effects, Stability) — derived so they can never drift from the
/// registry. See docs/dev/three-layer-documentation-model.md §3.
pub fn lookup_builtin_detail(name: &str) -> String {
    let canonical = crate::core_word_aliases::canonicalize_core_word_name(name);
    let alias_lead = build_alias_lead(name);

    let Some(spec) = lookup_builtin_spec(&canonical) else {
        if let Some(body) = crate::interpreter::modules::lookup_module_word_detail(&canonical) {
            return body;
        }
        return format!(
            "{}# {}\n\nNo documentation found for this word.\n",
            alias_lead, canonical
        );
    };

    let mut out = render_four_section(
        &alias_lead,
        spec.name,
        spec.stability,
        spec.category,
        spec.summary,
        spec.role,
        spec.stack_effect,
    );

    // Machine-readable execution form (SPEC §6.4): surface the control-directive
    // classification so LOOKUP states it explicitly rather than leaving it to
    // the prose. `RuntimeWord`s add nothing here.
    match spec.execution_form {
        ExecutionForm::LazyNextUnitFallback => {
            out.push('\n');
            out.push_str(
                "Form:\n  Lazy control directive (SPEC §6.4): inspects the stack top; a\n  \
                 non-NIL top is kept and the following source unit is skipped\n  \
                 unevaluated, a NIL top is discarded and the following unit is\n  \
                 evaluated as the fallback. Not a stack-consuming word.\n",
            );
        }
        ExecutionForm::NoOpControlDirective => {
            out.push('\n');
            out.push_str(
                "Form:\n  No-op control directive (SPEC §6.4): a positional marker with no\n  \
                 runtime effect.\n",
            );
        }
        ExecutionForm::RuntimeWord => {}
    }

    let doc = lookup_builtin_lookup_doc(spec.name);

    if let Some(doc) = doc {
        out.push('\n');
        out.push_str("Behavior:\n");
        push_indented(&mut out, doc.behavior, "  ");
    }

    out.push('\n');
    out.push_str("Examples:\n");
    match doc {
        Some(doc) if !doc.examples.is_empty() => {
            for example in doc.examples {
                push_indented(&mut out, example.code, "  ");
                if !example.result.is_empty() {
                    out.push('\n');
                    out.push_str("  Result:\n");
                    push_indented(&mut out, example.result, "    ");
                }
            }
        }
        _ => {
            // Every builtin carries a real invocation as its hover syntax
            // (three-layer model §4.3); reuse it when no authored example
            // exists yet.
            push_indented(&mut out, spec.hover_syntax, "  ");
        }
    }

    out.push('\n');
    out.push_str("Failure:\n");
    push_indented(&mut out, &derive_failure_text(spec), "  ");
    if let Some(doc) = doc {
        if !doc.failure_note.is_empty() {
            push_indented(&mut out, doc.failure_note, "  ");
        }
    }

    out.push('\n');
    out.push_str("Side Effects:\n");
    push_indented(&mut out, &derive_side_effects_text(spec), "  ");

    if let Some(doc) = doc {
        if !doc.related.is_empty() {
            out.push('\n');
            out.push_str("Related:\n");
            push_indented(&mut out, &doc.related.join(", "), "  ");
        }
    }

    out.push('\n');
    out.push_str("Stability:\n");
    push_indented(
        &mut out,
        if spec.stability.is_empty() {
            "stable"
        } else {
            spec.stability
        },
        "  ",
    );

    out
}

/// Failure baseline derived from the §7.14 contract metadata. The wording
/// follows the Bubble Rule framing (three-layer model §2.3): well-formed
/// operations that cannot produce a value bubble as NIL with a reason,
/// while malformed usage raises an error.
fn derive_failure_text(spec: &BuiltinSpec) -> String {
    let mut lines: Vec<&str> = Vec::new();
    match spec.partiality {
        Partiality::Total => lines.push("Total: always produces a result."),
        Partiality::Projecting => lines.push(
            "Well-formed input that cannot produce a value yields a\nBubble/NIL with a reason; malformed usage raises an error.",
        ),
        Partiality::Partial => lines.push("Malformed or out-of-domain usage raises an error."),
    }
    match spec.nil_policy {
        NilPolicy::Passthrough => lines.push("NIL operands pass through as NIL."),
        NilPolicy::CreatesNil => {}
        NilPolicy::RejectsNil => lines.push("NIL operands are rejected with an error."),
        NilPolicy::ConsumesNil => lines.push("Accepts NIL operands as data."),
        NilPolicy::PreservesReason => lines.push("A NIL value keeps its reason through this word."),
    }
    lines.join("\n")
}

/// Side Effects derived from the §7.14 `effects` list. The protocol names
/// form a small closed set; each maps to one user-facing sentence.
fn derive_side_effects_text(spec: &BuiltinSpec) -> String {
    if spec.effects.is_empty() {
        return "None.".to_string();
    }
    let mut sentences: Vec<&str> = Vec::new();
    for effect in spec.effects {
        let sentence = match *effect {
            "console-write" => "Writes to the output area.",
            "code-execution" => "Executes code supplied as data.",
            "dictionary-write" | "dictionary-register" => "Modifies the dictionary.",
            "dictionary-delete" => "Removes a word from the dictionary.",
            "dictionary-read" => "Loads documentation into the editor.",
            "dictionary-import"
            | "dictionary-import-only"
            | "dictionary-unimport"
            | "dictionary-unimport-only" => "Changes which module words are active.",
            "interpreter-mode-write" => "Changes the interpreter mode for the next word.",
            "runtime-control" => "Controls child runtime execution.",
            other => other,
        };
        if !sentences.contains(&sentence) {
            sentences.push(sentence);
        }
    }
    sentences.join("\n")
}

pub fn render_four_section(
    alias_lead: &str,
    name: &str,
    stability: &str,
    category: &str,
    summary: &str,
    role: &str,
    stack_effect: &str,
) -> String {
    let mut out = String::new();
    out.push_str(alias_lead);

    if stability.is_empty() || stability == "stable" {
        out.push_str(&format!("# {}\n\n", name));
    } else {
        out.push_str(&format!("# {}  ({})\n\n", name, stability));
    }

    out.push_str("Category:\n");
    push_indented(&mut out, category, "  ");
    out.push('\n');

    out.push_str("Summary:\n");
    push_indented(&mut out, summary, "  ");
    out.push('\n');

    out.push_str("Role:\n");
    push_indented(&mut out, role, "  ");
    out.push('\n');

    out.push_str("Stack Effect:\n");
    push_indented(&mut out, stack_effect, "  ");

    out
}

fn build_alias_lead(name: &str) -> String {
    lookup_core_word_alias(name)
        .and_then(|alias| {
            alias.canonical.map(|canonical_name| match alias.kind {
                CoreWordAliasKind::SymbolAlias => {
                    format!("{} is an alias of {}.\n\n", alias.alias, canonical_name)
                }
                CoreWordAliasKind::SyntaxSugar => {
                    format!(
                        "{} is syntax sugar for {}.\n\n",
                        alias.alias, canonical_name
                    )
                }
                CoreWordAliasKind::InputHelper => {
                    format!("{} is an input helper.\n\n", alias.alias)
                }
            })
        })
        .unwrap_or_default()
}

fn push_indented(out: &mut String, body: &str, indent: &str) {
    for line in body.split('\n') {
        out.push_str(indent);
        out.push_str(line);
        out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::super::builtin_word_definitions::{builtin_specs, lookup_builtin_spec};
    use super::super::builtin_word_lookup_docs::builtin_lookup_docs;
    use super::lookup_builtin_detail;

    const REQUIRED_SECTIONS: &[&str] = &["Category:", "Summary:", "Role:", "Stack Effect:"];

    /// Sections every builtin now renders, authored entry or not: the
    /// derived template (three-layer model §3.4) on top of the four base
    /// sections.
    const DERIVED_SECTIONS: &[&str] = &["Examples:", "Failure:", "Side Effects:", "Stability:"];

    #[test]
    fn every_builtin_renders_the_derived_sections() {
        for spec in builtin_specs() {
            let body = lookup_builtin_detail(spec.name);
            for section in REQUIRED_SECTIONS.iter().chain(DERIVED_SECTIONS) {
                assert!(
                    body.contains(section),
                    "{} LOOKUP body missing section {}: full body =\n{}",
                    spec.name,
                    section,
                    body
                );
            }
        }
    }

    #[test]
    fn every_authored_doc_entry_names_a_real_builtin() {
        for doc in builtin_lookup_docs() {
            assert!(
                lookup_builtin_spec(doc.word).is_some(),
                "authored LOOKUP doc for `{}` has no matching BuiltinSpec",
                doc.word
            );
            for related in doc.related {
                assert!(
                    lookup_builtin_spec(related).is_some(),
                    "`{}` lists unknown related word `{}`",
                    doc.word,
                    related
                );
            }
        }
    }

    #[test]
    fn authored_doc_entries_are_editor_safe_plain_text() {
        // §3.3: UTF-8 English plain text, ≤ 80 columns, no control
        // characters, no trailing whitespace — the LOOKUP body is loaded
        // into the code editor verbatim.
        for doc in builtin_lookup_docs() {
            assert!(
                !doc.behavior.is_empty(),
                "`{}` has an empty behavior",
                doc.word
            );
            let mut texts: Vec<&str> = vec![doc.behavior, doc.failure_note];
            for example in doc.examples {
                assert!(
                    !example.code.is_empty(),
                    "`{}` has an example with empty code",
                    doc.word
                );
                texts.push(example.code);
                texts.push(example.result);
            }
            for text in texts {
                for line in text.split('\n') {
                    assert!(
                        line.len() <= 80,
                        "`{}` has a line over 80 columns: {}",
                        doc.word,
                        line
                    );
                    assert!(
                        !line.chars().any(|c| c.is_control()),
                        "`{}` has a control character in: {}",
                        doc.word,
                        line
                    );
                    assert_eq!(
                        line,
                        line.trim_end(),
                        "`{}` has trailing whitespace in: {}",
                        doc.word,
                        line
                    );
                }
            }
        }
    }

    #[test]
    fn authored_entry_renders_behavior_examples_and_related() {
        let body = lookup_builtin_detail("GET");
        for section in ["Behavior:", "Related:", "Result:"] {
            assert!(
                body.contains(section),
                "GET LOOKUP body missing {}: full body =\n{}",
                section,
                body
            );
        }
        assert!(
            body.contains("indexOutOfBounds"),
            "GET Failure must name the Bubble reason:\n{}",
            body
        );
    }

    #[test]
    fn bubble_rule_words_describe_nil_not_only_errors() {
        // The three-layer model (§2.3) requires GET / DIV / NUM / CHR to
        // describe their Bubble/NIL cases separately from contract errors.
        for word in ["GET", "DIV", "NUM", "CHR"] {
            let body = lookup_builtin_detail(word);
            assert!(
                body.contains("Bubble/NIL"),
                "{} LOOKUP body must describe its Bubble/NIL case:\n{}",
                word,
                body
            );
        }
    }

    #[test]
    fn word_without_authored_entry_falls_back_to_hover_example() {
        let body = lookup_builtin_detail("SHAPE");
        let spec = lookup_builtin_spec("SHAPE").expect("SHAPE spec");
        assert!(
            body.contains(spec.hover_syntax),
            "SHAPE Examples should reuse hover_syntax until authored:\n{}",
            body
        );
    }

    #[test]
    fn lookup_for_add_contains_four_required_sections() {
        let body = lookup_builtin_detail("ADD");
        assert!(body.contains("# ADD"), "ADD header missing:\n{}", body);
        for section in REQUIRED_SECTIONS {
            assert!(
                body.contains(section),
                "ADD LOOKUP body missing section {}: full body =\n{}",
                section,
                body
            );
        }
    }

    #[test]
    fn lookup_for_alias_includes_alias_lead() {
        let body = lookup_builtin_detail("+");
        assert!(
            body.starts_with("+ is syntax sugar for ADD")
                || body.starts_with("+ is an alias of ADD"),
            "alias lead missing for '+'; got:\n{}",
            body
        );
        assert!(body.contains("# ADD"));
    }

    #[test]
    fn every_builtin_lookup_contains_all_four_sections() {
        for spec in crate::builtins::builtin_specs() {
            let body = lookup_builtin_detail(spec.name);
            for section in REQUIRED_SECTIONS {
                assert!(
                    body.contains(section),
                    "{} LOOKUP body missing section {}:\n{}",
                    spec.name,
                    section,
                    body
                );
            }
        }
    }

    #[test]
    fn experimental_word_header_shows_stability() {
        // SPAWN is marked experimental in BUILTIN_SPECS.
        let body = lookup_builtin_detail("SPAWN");
        assert!(
            body.contains("# SPAWN  (experimental)"),
            "SPAWN header must show '(experimental)':\n{}",
            body
        );
    }

    #[test]
    fn stable_word_header_omits_stability() {
        let body = lookup_builtin_detail("ADD");
        assert!(
            body.contains("# ADD\n"),
            "ADD (stable) header must be bare:\n{}",
            body
        );
        assert!(
            !body.contains("# ADD  (stable)"),
            "stable stability must NOT be shown in header:\n{}",
            body
        );
    }

    #[test]
    fn comparison_words_have_uniform_stack_effect() {
        // All six comparison primitives must use the same stack-effect
        // notation so the four-section template is consistent across the
        // comparison category.
        const EXPECTED: &str = "[ a ] [ b ] -> [ TRUE | FALSE ]";
        for name in &["EQ", "NEQ", "LT", "LTE", "GT", "GTE"] {
            let spec = crate::builtins::builtin_word_definitions::lookup_builtin_spec(name)
                .unwrap_or_else(|| panic!("{} must have a BuiltinSpec", name));
            assert_eq!(
                spec.stack_effect, EXPECTED,
                "{} stack_effect deviates from the comparison-word standard",
                name
            );
        }
    }

    #[test]
    fn lookup_output_is_utf8_plain_text() {
        for name in ["ADD", "MAP", "LOOKUP", "DEF", "VENT", "TOP", "PRINT"] {
            let body = lookup_builtin_detail(name);
            assert!(
                !body.chars().any(|c| c.is_control() && c != '\n'),
                "LOOKUP body for {} must be UTF-8 plain text without control characters:\n{}",
                name,
                body
            );
        }
    }
}
