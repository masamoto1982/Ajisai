use super::builtin_word_definitions::{lookup_builtin_spec, BuiltinSpec};
use super::builtin_word_lookup_docs::lookup_builtin_lookup_doc;
use crate::core_word_aliases::{lookup_core_word_alias, CoreWordAliasKind};
use crate::coreword_registry::{ExecutionForm, NilPolicy, Partiality};
use crate::kernel::generated::{generated_word, VocabularyTier};

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
    push_indented(&mut out, &derive_failure_text(spec, &canonical), "  ");
    if let Some(doc) = doc {
        if !doc.failure_note.is_empty() {
            push_indented(&mut out, doc.failure_note, "  ");
        }
    }

    out.push('\n');
    out.push_str("Side Effects:\n");
    push_indented(&mut out, &derive_side_effects_text(&canonical), "  ");

    if let Some(doc) = doc {
        if !doc.related.is_empty() {
            out.push('\n');
            out.push_str("Related:\n");
            push_indented(&mut out, &doc.related.join(", "), "  ");
        }
    }

    out.push('\n');
    out.push_str("Vocabulary:\n");
    push_indented(&mut out, &derive_vocabulary_text(&canonical), "  ");

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

/// Failure baseline derived from the §7.14 contract metadata. The wording
/// follows the Bubble Rule framing (three-layer model §2.3): well-formed
/// operations that cannot produce a value bubble as NIL with a reason,
/// while malformed usage raises an error.
///
/// The NIL sentence is derived from the *declared* policy in
/// `spec/words.json`, so what a reader is told about NIL and what the dispatch
/// guard enforces are the same fact read twice, not two claims that can drift.
fn derive_failure_text(spec: &BuiltinSpec, canonical: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();
    match spec.partiality {
        Partiality::Total => lines.push("Total: always produces a result."),
        Partiality::Projecting => lines.push(
            "Well-formed input that cannot produce a value yields a\nBubble/NIL with a reason; malformed usage raises an error.",
        ),
        Partiality::Partial => lines.push("Malformed or out-of-domain usage raises an error."),
    }
    if let Some(word) = generated_word(canonical) {
        match word.nil_policy {
            NilPolicy::Passthrough | NilPolicy::PassthroughThenProject => {
                lines.push("NIL operands pass through as NIL, keeping their reason.")
            }
            NilPolicy::CreatesNil => {}
            NilPolicy::RejectNil => lines.push("NIL operands are rejected with an error."),
            NilPolicy::ConsumeNil => lines.push("Accepts NIL operands as data."),
            NilPolicy::InspectNil => lines.push("Inspects whether its subject is NIL."),
            NilPolicy::PreserveReason => {
                lines.push("A NIL value keeps its reason through this word.")
            }
        }
    }
    lines.join("\n")
}

/// Side Effects derived from the §7.14 `effects` list declared in
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
        "dictionaryRead" => Some("Loads documentation into the editor."),
        _ => None,
    }
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
