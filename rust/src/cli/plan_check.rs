//! Light contract / flow-mass pre-check of an Ajisai plan — the "approach 2,
//! light version" of the natural-language surface design note
//! (`docs/dev/natural-language-surface-design.md` §4): use the *existing*
//! checking machinery to reject or flag a malformed plan **before execution**,
//! without becoming a contract-driven search. It reuses
//! [`mass_conservation::analyze_source`](crate::interpreter::mass_conservation)
//! (SPEC §13.1) for flow-mass accounting and the §7.14 `nil_policy` contract
//! for NIL-flow accounting. It executes nothing and defines no semantics
//! (canonical source: `SPECIFICATION.html`).
//!
//! The findings are the signals approach 4 (clarification / UNKNOWN) will later
//! consume: an over-consuming flow is a malformed plan (a Channel-error-shaped
//! verdict), and a NIL source with no fallback is the `handleUnknownOrNil`
//! prompt rendered ahead of time.

use std::collections::HashSet;

use super::explain::Lang;
use crate::coreword_registry::{get_coreword_metadata, NilPolicy};
use crate::interpreter::mass_conservation::analyze_source;
use crate::interpreter::Interpreter;
use crate::types::Token;

/// Result of the light, execution-free plan check.
pub(crate) struct PlanCheck {
    /// The flow reads more operands than it provides over the statically known
    /// prefix (mass `min_depth < 0`, SPEC §13.1): a malformed plan.
    pub over_consumes: bool,
    /// Lowest abstract stack depth from an empty start (negative ⇒ over-consume).
    pub min_depth: i64,
    /// Net stack-depth change over the known prefix.
    pub net_mass: i64,
    /// `false` once a `Dynamic`-arity word froze the static analysis (the check
    /// is then only valid for the prefix before that word).
    pub mass_known: bool,
    /// Words whose `nil_policy = CreatesNil` (SPEC §7.14): they project a
    /// well-formed domain miss onto NIL (e.g. `DIV` `GET` `NUM`). First
    /// appearance order, deduplicated.
    pub may_bubble: Vec<String>,
    /// A `VENT` (`^`) appears in the flow, i.e. an explicit NIL fallback.
    pub has_fallback: bool,
    /// Words whose `nil_policy = RejectsNil` (they raise on a NIL operand).
    pub rejects_nil: Vec<String>,
    /// Flow-sensitive NIL producers whose result may still be NIL after local fallbacks.
    pub unguarded_nil: Vec<String>,
    /// RejectsNil words reached by a maybe-NIL abstract operand.
    pub rejects_nil_flows: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AbstractSlot {
    maybe_nil: bool,
    sources: Vec<String>,
}

impl AbstractSlot {
    fn present() -> Self {
        Self {
            maybe_nil: false,
            sources: Vec::new(),
        }
    }

    fn maybe_nil(source: String) -> Self {
        Self {
            maybe_nil: true,
            sources: vec![source],
        }
    }
}

fn flow_sensitive_nil(tokens: &[Token]) -> (Vec<String>, Vec<String>) {
    let mut stack: Vec<AbstractSlot> = Vec::new();
    let mut unguarded_seen = HashSet::new();
    let mut reject_seen = HashSet::new();
    let mut rejects_nil_flows = Vec::new();

    for token in tokens {
        match token {
            Token::Number(_) | Token::String(_) => stack.push(AbstractSlot::present()),
            Token::NilCoalesce => {
                if let Some(top) = stack.last_mut() {
                    top.maybe_nil = false;
                    top.sources.clear();
                }
            }
            Token::Symbol(symbol) => {
                let normalized = super::normalize_word(symbol);
                let canonical = crate::core_word_aliases::canonicalize_core_word_name(&normalized);
                // `VENT` (both `^` and the spelled-out name) tokenizes as
                // `Token::NilCoalesce` and is handled by that arm above; it never
                // reaches here as a `Symbol`.
                let Some(meta) = get_coreword_metadata(&canonical) else {
                    continue;
                };
                let Some((consumes, produces)) = meta.mass.fixed() else {
                    break;
                };
                let mut operands = Vec::new();
                for _ in 0..consumes {
                    operands.push(stack.pop().unwrap_or_else(AbstractSlot::present));
                }
                operands.reverse();

                let mut input_sources = Vec::new();
                for operand in &operands {
                    if operand.maybe_nil {
                        input_sources.extend(operand.sources.clone());
                    }
                }
                if meta.nil_policy == NilPolicy::RejectsNil && !input_sources.is_empty() {
                    let sink = canonical.to_string();
                    if reject_seen.insert(sink.clone()) {
                        rejects_nil_flows.push(format!("{} -> {}", input_sources.join(", "), sink));
                    }
                }

                let output = match meta.nil_policy {
                    NilPolicy::CreatesNil => AbstractSlot::maybe_nil(canonical.to_string()),
                    NilPolicy::Passthrough | NilPolicy::PreservesReason => {
                        if input_sources.is_empty() {
                            AbstractSlot::present()
                        } else {
                            AbstractSlot {
                                maybe_nil: true,
                                sources: input_sources,
                            }
                        }
                    }
                    NilPolicy::RejectsNil | NilPolicy::ConsumesNil => AbstractSlot::present(),
                };
                for _ in 0..produces {
                    stack.push(output.clone());
                }
            }
            Token::VectorStart
            | Token::VectorEnd
            | Token::BlockStart
            | Token::BlockEnd
            | Token::Pipeline
            | Token::CondClauseSep
            | Token::LineBreak => {}
        }
    }

    let mut unguarded_nil = Vec::new();
    for slot in stack {
        if slot.maybe_nil {
            for source in slot.sources {
                if unguarded_seen.insert(source.clone()) {
                    unguarded_nil.push(source);
                }
            }
        }
    }
    (unguarded_nil, rejects_nil_flows)
}

/// Tokenize, compile and statically check `src` for flow-mass conservation and
/// NIL-flow hygiene. Never executes. `Err` only for a lexical failure.
pub(crate) fn check_plan(interp: &Interpreter, src: &str) -> Result<PlanCheck, String> {
    let mass = analyze_source(interp, src)?;
    let tokens = crate::tokenizer::tokenize(src)?;

    let mut may_bubble: Vec<String> = Vec::new();
    let mut rejects_nil: Vec<String> = Vec::new();
    let mut has_fallback = false;
    let mut seen_bubble: HashSet<String> = HashSet::new();
    let mut seen_reject: HashSet<String> = HashSet::new();

    for token in &tokens {
        // `VENT` — both the `^` sugar and the spelled-out canonical name —
        // tokenizes as `NilCoalesce` (SPEC §6.4), so a single match on the token
        // covers both spellings of the fallback. `OR-NIL` / `=>` are historical
        // names/forms the current tokenizer does not produce.
        if matches!(token, Token::NilCoalesce) {
            has_fallback = true;
            continue;
        }
        let Token::Symbol(symbol) = token else {
            continue;
        };
        let normalized = super::normalize_word(symbol);
        let canonical = crate::core_word_aliases::canonicalize_core_word_name(&normalized);
        let Some(meta) = get_coreword_metadata(&canonical) else {
            continue;
        };
        // `CreatesNil` is the precise "can bubble to NIL" signal. A `Projecting`
        // comparison (LT/SORT/…) projects to logical U, not NIL, so it is
        // deliberately not flagged here.
        match meta.nil_policy {
            NilPolicy::CreatesNil => {
                if seen_bubble.insert(canonical.to_string()) {
                    may_bubble.push(canonical.into_owned());
                }
            }
            NilPolicy::RejectsNil => {
                if seen_reject.insert(canonical.to_string()) {
                    rejects_nil.push(canonical.into_owned());
                }
            }
            _ => {}
        }
    }

    let (unguarded_nil, rejects_nil_flows) = flow_sensitive_nil(&tokens);

    Ok(PlanCheck {
        over_consumes: mass.over_consumes_from_empty(),
        min_depth: mass.min_depth,
        net_mass: mass.net_mass,
        mass_known: mass.all_known,
        may_bubble,
        has_fallback,
        rejects_nil,
        unguarded_nil,
        rejects_nil_flows,
    })
}

/// Severity of a finding, so a consumer (and approach 4) can tell a malformed
/// plan apart from advice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Severity {
    /// A malformed plan: it cannot run as written.
    Error,
    /// Well-formed but worth surfacing (e.g. an unguarded NIL source).
    Advisory,
    /// Informational only (e.g. the static check stopped early).
    Note,
}

impl Severity {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Advisory => "advisory",
            Severity::Note => "note",
        }
    }
}

/// One plain-language finding (the L0 surface, design note §2). Mechanism terms
/// (`nil_policy`, mass, `VENT`) are not exposed; the `^` sugar is named because
/// it is the literal fix the user types.
pub(crate) struct Finding {
    pub severity: Severity,
    pub message: String,
}

impl PlanCheck {
    /// Render the findings in `lang`, most severe first. An empty result means
    /// the plan is clean over the statically known prefix.
    pub(crate) fn findings(&self, lang: Lang) -> Vec<Finding> {
        let mut findings = Vec::new();

        if self.over_consumes {
            findings.push(Finding {
                severity: Severity::Error,
                message: match lang {
                    Lang::Ja => format!(
                        "この手順は与えられるより多くの値を読み取ります（最小深さ {}）。語の並びを見直してください。",
                        self.min_depth
                    ),
                    Lang::En => format!(
                        "This plan reads more values than it provides (min depth {}). Check the words and their order.",
                        self.min_depth
                    ),
                },
            });
        }

        if !self.unguarded_nil.is_empty() {
            let words = self.unguarded_nil.join(", ");
            findings.push(Finding {
                severity: Severity::Advisory,
                message: match lang {
                    Lang::Ja => format!(
                        "次の語は値を生めないこと（NIL）があります: {}。`^` で既定値を決めるか、分岐を足してください。",
                        words
                    ),
                    Lang::En => format!(
                        "These words can fail to produce a value (NIL): {}. Supply a fallback with `^`, or add a branch.",
                        words
                    ),
                },
            });
        }

        if !self.rejects_nil_flows.is_empty() {
            let sinks = self.rejects_nil_flows.join(", ");
            findings.push(Finding {
                severity: Severity::Advisory,
                message: match lang {
                    Lang::Ja => format!(
                        "次の語は NIL を受け取れません: {}。NIL が届く前に解消してください。",
                        sinks
                    ),
                    Lang::En => format!(
                        "These words reject a NIL operand: {}. Resolve the NIL before it reaches them.",
                        sinks
                    ),
                },
            });
        }

        if !self.mass_known {
            findings.push(Finding {
                severity: Severity::Note,
                message: match lang {
                    Lang::Ja => {
                        "動的なアリティの語を含むため、静的な流量チェックは途中で停止しました。"
                            .to_string()
                    }
                    Lang::En => {
                        "Contains a dynamic-arity word, so the static flow check stopped early."
                            .to_string()
                    }
                },
            });
        }

        findings
    }
}
