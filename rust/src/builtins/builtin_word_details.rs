use super::builtin_word_definitions::{lookup_builtin_spec, BuiltinSpec};
use crate::core_word_aliases::{lookup_core_word_alias, CoreWordAliasKind};
use crate::coreword_registry::Partiality;
use crate::kernel::generated::{generated_word, OperandRole, VocabularyTier};

/// Render the LOOKUP body for a built-in word: the base sections (Family /
/// Summary / Stack Effect), the Word's one correct call (Examples), and the sections
/// derived from the LANG.CONTRACT.REGISTRY contract metadata (Failure baseline, Side
/// Effects, Vocabulary) — derived so they can never drift from the
/// registry. See docs/dev/three-layer-documentation-model.md §3.
pub fn lookup_builtin_detail(name: &str) -> String {
    let canonical = crate::core_word_aliases::canonicalize_core_word_name(name);
    let alias_lead = build_alias_lead(name);

    let Some(spec) = lookup_builtin_spec(&canonical) else {
        return format!(
            "{}# {}\n\nNo documentation found for this word.\n",
            alias_lead, canonical
        );
    };

    let mut out = render_sections(
        &alias_lead,
        spec.name,
        spec.family,
        spec.summary,
        spec.stack_effect,
    );

    // One source for what a Word does: the summary above, from
    // `spec/words.json`, carries the prose and the tested examples. The
    // example here is the one correct call a diagnosis quotes.
    out.push('\n');
    out.push_str("Examples:\n");
    push_indented(&mut out, spec.hover_syntax, "  ");

    out.push('\n');
    out.push_str("Failure:\n");
    push_indented(&mut out, &derive_failure_text(spec, &canonical), "  ");

    out.push('\n');
    out.push_str("Side Effects:\n");
    push_indented(&mut out, &derive_side_effects_text(&canonical), "  ");

    out.push('\n');
    out.push_str("Vocabulary:\n");
    push_indented(&mut out, &derive_vocabulary_text(&canonical), "  ");

    out
}

/// Where the Word sits in the public Core, read from the generated registry.
/// Core is one flat sealed dictionary, so this states a design classification
/// and never a namespace: a Standard Word is reached by its plain name exactly
/// as a Semantic Kernel Word is, and carries the same contract detail.
fn derive_vocabulary_text(canonical: &str) -> String {
    let Some(word) = generated_word(canonical) else {
        return "Core Word.".to_string();
    };
    match (word.vocabulary_tier, word.standard_kind) {
        (VocabularyTier::Kernel, _) => {
            "Core Word, Semantic Kernel: it builds or observes a value domain,\nor is the one explicit operation for its capability.".to_string()
        }
        (VocabularyTier::Standard, Some(kind)) => format!(
            "Core Word, Standard vocabulary ({kind}): one canonical contract for\na frequent concept, on the same terms as a Kernel Word."
        ),
        (VocabularyTier::Standard, None) => "Core Word, Standard vocabulary.".to_string(),
    }
}

/// Failure baseline derived from the LANG.CONTRACT.REGISTRY contract metadata. The wording
/// follows the NIL Projection Rule: well-formed operations that cannot
/// produce a value project onto NIL with a reason, while malformed usage
/// raises an error.
///
/// The NIL sentence is derived from the *declared* policy in
/// `spec/words.json`, so what a reader is told about NIL and what the dispatch
/// guard enforces are the same fact read twice, not two claims that can drift.
fn derive_failure_text(spec: &BuiltinSpec, canonical: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();
    match spec.partiality {
        Partiality::Total => lines.push(
            "Total: an operand of the kind it reads always produces a result;\nan operand of another kind raises the error its contract names.",
        ),
        Partiality::Projecting => lines.push(
            "Well-formed input that cannot produce a value yields a NIL\nwith a reason; an operand of the wrong kind raises an error.",
        ),
        Partiality::Partial => lines.push(
            "May raise even on operands of the right kind: the block it runs,\nor the dictionary it changes, can refuse.",
        ),
    }
    if let Some(word) = generated_word(canonical) {
        // One line per role the Word has (LANG.FAILURE.PASSTHROUGH), so the
        // hover says what a NIL does in each operand, not a single summary
        // that is true of some of them.
        let has = |role: OperandRole| word.operand_roles.contains(&role);
        if has(OperandRole::Data) || has(OperandRole::Leaf) {
            lines.push("A NIL data operand passes through as the result, keeping its reason.");
        }
        if has(OperandRole::Element) {
            lines.push(
                "A NIL it only carries (a stored, bound or compared value) is an ordinary value.",
            );
        }
        if has(OperandRole::Control) {
            lines.push("A NIL where a block, name or message belongs is an error.");
        }
        if has(OperandRole::Leaf) || has(OperandRole::Truth) {
            lines.push(
                "A Vector or Record where one value is read applies the word to each element.",
            );
        }
        if has(OperandRole::Truth) {
            lines.push(
                "A dominating definite operand (FALSE for AND) absorbs a NIL operand into that definite result; otherwise a NIL operand yields NIL as UNKNOWN.",
            );
        }
    }
    lines.join("\n")
}

/// Side Effects derived from the LANG.CONTRACT.REGISTRY `effects` list declared in
/// `spec/words.json`. Each declared effect maps to one user-facing sentence;
/// `effect_sentence` returns `None` for a name it does not know, which
/// `builtin_word_details_tests.rs` turns into a failure rather than letting the
/// raw protocol name reach a reader.
fn derive_side_effects_text(canonical: &str) -> String {
    let Some(word) = generated_word(canonical) else {
        return "None.".to_string();
    };
    if word.effects.is_empty() {
        return "None.".to_string();
    }
    let mut sentences: Vec<&str> = Vec::new();
    for effect in word.effects {
        let sentence = effect_sentence(effect).unwrap_or(effect);
        if !sentences.contains(&sentence) {
            sentences.push(sentence);
        }
    }
    sentences.join("\n")
}

/// The user-facing sentence for a declared effect name.
pub(super) fn effect_sentence(effect: &str) -> Option<&'static str> {
    match effect {
        "consoleWrite" => Some("Writes to the output area."),
        "dictionaryWrite" => Some("Modifies the dictionary."),
        "dictionaryDelete" => Some("Removes a word from the dictionary."),
        _ => None,
    }
}

pub fn render_sections(
    alias_lead: &str,
    name: &str,
    family: &str,
    summary: &str,
    stack_effect: &str,
) -> String {
    let mut out = String::new();
    out.push_str(alias_lead);

    out.push_str(&format!("# {}\n\n", name));

    out.push_str("Family:\n");
    push_indented(&mut out, family, "  ");
    out.push('\n');

    out.push_str("Summary:\n");
    push_indented(&mut out, summary, "  ");
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
mod failure_text_render_tests {
    use super::lookup_builtin_detail;

    /// The Failure section used to promise `SORT` "always produces a result",
    /// which is false of a vector holding a string. Totality is stated over
    /// the kind of operand the Word reads, which is what makes it true.
    #[test]
    fn totality_is_stated_over_the_kind_the_word_reads() {
        let text = lookup_builtin_detail("SORT");
        assert!(
            text.contains("Total: an operand of the kind it reads always produces a result"),
            "{text}"
        );
    }

    /// A raw Rust escape once reached a reader: SQRT's Role said
    /// `the multiquadratic \u{221a}d` verbatim.
    #[test]
    fn no_lookup_text_leaks_a_rust_unicode_escape() {
        for word in crate::builtins::builtin_specs() {
            let text = lookup_builtin_detail(word.name);
            assert!(
                !text.contains("\\u{"),
                "{}: LOOKUP text carries a raw escape:\n{text}",
                word.name
            );
        }
    }
}
