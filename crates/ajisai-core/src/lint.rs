//! The contract lint.
//!
//! What this is: a check that walks a program with an abstract flow and
//! reports places where a declared stack effect or input type is obviously
//! contradicted.
//!
//! What this is not: a verifier. It does not decide whether a program
//! terminates, whether a division will be by zero, whether a vector index is
//! in range, or whether a program will succeed. It never blocks execution, and
//! it never reports "this program is safe" — the strongest thing it can say is
//! that it found nothing obviously wrong, which is a much weaker claim and is
//! worded that way everywhere it is surfaced.
//!
//! Where the abstract flow stops being knowable — a word with a dynamic stack
//! effect, a user definition, a mode that reshapes the whole flow, a vent that
//! may or may not have run — the lint goes opaque and stops reporting rather
//! than guessing. A false accusation is worse than silence.

use std::collections::BTreeSet;
use std::fmt;

use crate::contract::{Arity, TypeSpec, WordContract};
use crate::error::Result;
use crate::interpreter::{unit_len, Interpreter};
use crate::mode::Selection;
use crate::role::Role;
use crate::syntax::{self, Node};
use crate::value::ValueData;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// A contradiction between what the program does and what a contract
    /// declares. It will fail if reached.
    Error,
    /// Worth a look. It may be exactly what was meant.
    Advisory,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Severity::Error => "error",
            Severity::Advisory => "advisory",
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    pub message: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.severity, self.message)
    }
}

/// The kinds the abstract flow distinguishes. Deliberately coarse: the lint
/// only needs enough resolution to catch a definite contradiction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Number,
    Truth,
    /// A vector with no particular reading.
    Vector,
    /// A vector read as `TEXT`. Text is a vector, so it satisfies a `Vector`
    /// position; a bare vector does not satisfy a `Text` position, which is
    /// what lets the lint see `{ 1 } [ 88 ] DEF` before it runs.
    Text,
    Quote,
    /// Not narrowed. Never contradicts anything.
    Any,
}

#[derive(Clone, Copy, Debug)]
struct Slot {
    kind: Kind,
    may_be_nil: bool,
    may_be_unknown: bool,
}

impl Slot {
    fn of(kind: Kind) -> Self {
        Self {
            kind,
            may_be_nil: false,
            may_be_unknown: false,
        }
    }
}

/// The abstract flow: a known run of slots, or opaque.
enum Flow {
    Known(Vec<Slot>),
    Opaque,
}

/// Lint a source fragment against an interpreter's vocabulary.
///
/// Parsing errors are returned as errors; everything else comes back as
/// findings, because a lint that refuses to finish is not much of a lint.
pub fn lint(interpreter: &Interpreter, source: &str) -> Result<Vec<Finding>> {
    let program = syntax::parse(source)?;
    let mut findings = Vec::new();
    let mut flow = Flow::Known(Vec::new());
    // Names the source defines for itself. Without this the lint reports
    // `unknown word` for every word a program defines and then uses — a false
    // accusation on a correct program, and the loudest kind.
    let mut defined = BTreeSet::new();
    collect_definitions(&program, &mut defined);
    let context = Context {
        interpreter,
        defined: &defined,
    };
    walk(&context, &program, &mut flow, &mut findings);
    Ok(findings)
}

/// Gather the names `DEF` binds anywhere in the program.
///
/// Only the literal form `{ … } "NAME" DEF` is recognised, which is how a
/// definition is nearly always written. A name computed at run time is not
/// found, and the lint simply stays quiet about it rather than guessing.
fn collect_definitions(body: &[Node], into: &mut BTreeSet<String>) {
    for (index, node) in body.iter().enumerate() {
        match node {
            Node::Word(name) if name == "DEF" => {
                if let Some(Node::Literal(value)) = index.checked_sub(1).and_then(|i| body.get(i)) {
                    if value.role() == Role::Text {
                        if let Some(text) = value.as_text() {
                            into.insert(crate::alias::canonical(text.trim()));
                        }
                    }
                }
            }
            Node::Basin(inner) | Node::Quote(inner) => collect_definitions(inner, into),
            _ => {}
        }
    }
}

/// What the lint knows about the vocabulary while it walks.
struct Context<'a> {
    interpreter: &'a Interpreter,
    defined: &'a BTreeSet<String>,
}

impl Context<'_> {
    /// True when the name will resolve: a registered word, an existing user
    /// definition, or one this source binds.
    fn knows(&self, name: &str) -> bool {
        self.interpreter.word(name).is_some()
            || self.defined.contains(name)
            || self
                .interpreter
                .definitions()
                .iter()
                .any(|(defined, _)| *defined == name)
    }
}

fn walk(context: &Context<'_>, body: &[Node], flow: &mut Flow, findings: &mut Vec<Finding>) {
    let mut index = 0;
    let mut mode_armed = false;
    let mut armed_selection: Option<Selection> = None;
    while index < body.len() {
        match &body[index] {
            Node::Literal(value) => {
                push(
                    flow,
                    Slot::of(match value.data() {
                        ValueData::Number(_) => Kind::Number,
                        ValueData::Vector(_) if value.role() == Role::Text => Kind::Text,
                        ValueData::Vector(_) => Kind::Vector,
                        _ => Kind::Any,
                    }),
                );
                index += 1;
            }
            Node::Basin(inner) => {
                // A basin runs on its own flow, so its body is linted
                // independently and contributes exactly one vector here.
                let mut inner_flow = Flow::Known(Vec::new());
                walk(context, inner, &mut inner_flow, findings);
                push(flow, Slot::of(Kind::Vector));
                index += 1;
            }
            Node::Quote(inner) => {
                // A quote's body is linted on its own, with an opaque flow:
                // what it will be handed is not known here.
                let mut inner_flow = Flow::Opaque;
                walk(context, inner, &mut inner_flow, findings);
                push(flow, Slot::of(Kind::Quote));
                index += 1;
            }
            Node::Word(name) => {
                if name == "TOP" || name == "STAK" || name == "EAT" || name == "KEEP" {
                    // A mode reshapes how the next word draws and commits, in
                    // ways the abstract flow does not model. Stop guessing —
                    // but remember which selection was armed, because `STAK`
                    // over a vent is a definite error rather than an unknown.
                    mode_armed = true;
                    if name == "STAK" {
                        armed_selection = Some(Selection::Stak);
                    } else if name == "TOP" {
                        armed_selection = Some(Selection::Top);
                    }
                    *flow = Flow::Opaque;
                    index += 1;
                    continue;
                }
                if name == "VENT" {
                    mode_armed = false;
                    if armed_selection == Some(Selection::Stak) {
                        findings.push(Finding {
                            severity: Severity::Error,
                            message: "VENT ( truth -- ): does not accept mode STAK; a gate is \
                                      one value and a whole flow is not a reading of it"
                                .to_string(),
                        });
                    }
                    armed_selection = None;
                    // The gate is drawn before the unit is considered, so it
                    // can be checked even though the flow afterwards cannot.
                    check_vent_gate(context.interpreter, flow, findings);
                    match unit_len(body, index + 1) {
                        Ok(span) => {
                            // The unit may or may not run, so the flow after a
                            // vent is not knowable. Its body is still linted.
                            let unit = &body[index + 1..index + 1 + span];
                            let mut unit_flow = Flow::Opaque;
                            walk(context, unit, &mut unit_flow, findings);
                            *flow = Flow::Opaque;
                            index += 1 + span;
                        }
                        Err(_) => {
                            findings.push(Finding {
                                severity: Severity::Error,
                                message: "VENT: no source unit follows".to_string(),
                            });
                            index += 1;
                        }
                    }
                    continue;
                }
                mode_armed = false;
                armed_selection = None;
                check_word(context, name, flow, findings);
                index += 1;
            }
        }
    }
    if mode_armed {
        findings.push(Finding {
            severity: Severity::Error,
            message: "a flow mode was armed but no word consumed it".to_string(),
        });
    }
}

fn check_word(context: &Context<'_>, name: &str, flow: &mut Flow, findings: &mut Vec<Finding>) {
    let Some(word) = context.interpreter.word(name) else {
        if !context.knows(name) {
            findings.push(Finding {
                severity: Severity::Error,
                message: format!("unknown word: {name}"),
            });
        }
        // A user definition's effect is whatever its body does; the lint does
        // not infer it, and says nothing rather than guessing.
        *flow = Flow::Opaque;
        return;
    };
    let contract = &word.contract;
    let Some((inn, _out)) = contract.arity.fixed() else {
        *flow = Flow::Opaque;
        return;
    };
    let Flow::Known(slots) = flow else {
        return;
    };
    if slots.len() < inn {
        findings.push(Finding {
            severity: Severity::Error,
            message: format!(
                "{name} {}: needs {inn} value(s), the flow holds {}",
                contract.stack_effect,
                slots.len()
            ),
        });
        *flow = Flow::Opaque;
        return;
    }
    let base = slots.len() - inn;
    for (position, expected) in contract.input_types.iter().enumerate() {
        let Some(slot) = slots.get(base + position) else {
            break;
        };
        // A type contradiction is only an error when *every* value the slot
        // could hold contradicts the contract. A slot that may be UNKNOWN
        // reaching a word that propagates UNKNOWN is not a mistake: that is
        // what `UNKNOWN 1 ADD` is, and an earlier draft reported it, which
        // broke the lint's own rule that a false accusation costs more than a
        // missed one.
        if definitely_wrong(*expected, slot, contract) {
            findings.push(Finding {
                severity: Severity::Error,
                message: format!(
                    "{name} {}: operand {} is {}, expected {expected}",
                    contract.stack_effect,
                    position + 1,
                    describe(slot.kind)
                ),
            });
        }
        if slot.may_be_nil && contract.nil_policy.rejects {
            findings.push(Finding {
                severity: Severity::Advisory,
                message: format!(
                    "{name} {}: operand {} may be NIL, which {name} rejects",
                    contract.stack_effect,
                    position + 1
                ),
            });
        }
        if slot.may_be_unknown && contract.unknown_policy.rejects {
            findings.push(Finding {
                severity: Severity::Advisory,
                message: format!(
                    "{name} {}: operand {} may be UNKNOWN, which {name} rejects",
                    contract.stack_effect,
                    position + 1
                ),
            });
        }
    }
    slots.truncate(base);
    for output in outputs(contract) {
        slots.push(output);
    }
}

/// True when no value the slot could hold satisfies the contract.
///
/// The slot stands for a set of possibilities: a definite value of `slot.kind`,
/// and — when the flags say so — `NIL` or `UNKNOWN`. Each is tested against the
/// contract, and the finding is raised only if all of them fail.
fn definitely_wrong(expected: TypeSpec, slot: &Slot, contract: &WordContract) -> bool {
    if !contradicts(expected, slot.kind) {
        return false;
    }
    if slot.may_be_nil && !contract.nil_policy.rejects {
        return false;
    }
    if slot.may_be_unknown && !contract.unknown_policy.rejects {
        return false;
    }
    true
}

/// Report a `VENT` whose gate is definitely missing or definitely not a truth
/// value. Everything after the vent is unknowable, but the gate is drawn
/// before the unit is even considered, so it is checkable.
fn check_vent_gate(interpreter: &Interpreter, flow: &Flow, findings: &mut Vec<Finding>) {
    let Some(vent) = interpreter.word("VENT") else {
        return;
    };
    let Flow::Known(slots) = flow else {
        return;
    };
    let Some(gate) = slots.last() else {
        findings.push(Finding {
            severity: Severity::Error,
            message: "VENT ( truth -- ): needs 1 value(s), the flow holds 0".to_string(),
        });
        return;
    };
    if definitely_wrong(TypeSpec::TruthValue, gate, &vent.contract) {
        findings.push(Finding {
            severity: Severity::Error,
            message: format!(
                "VENT ( truth -- ): the gate is {}, expected truth value",
                describe(gate.kind)
            ),
        });
    }
    if gate.may_be_nil {
        findings.push(Finding {
            severity: Severity::Advisory,
            message: "VENT ( truth -- ): the gate may be NIL, which is not a truth value"
                .to_string(),
        });
    }
}

fn outputs(contract: &WordContract) -> Vec<Slot> {
    let count = match contract.arity {
        Arity::Fixed { out, .. } => out as usize,
        Arity::Dynamic => 0,
    };
    (0..count)
        .map(|position| {
            let kind = contract
                .output_types
                .get(position)
                .map(|spec| kind_of(*spec))
                .unwrap_or(Kind::Any);
            Slot {
                kind,
                may_be_nil: contract.nil_policy.may_produce,
                may_be_unknown: contract.unknown_policy.may_produce,
            }
        })
        .collect()
}

fn kind_of(spec: TypeSpec) -> Kind {
    match spec {
        TypeSpec::Number => Kind::Number,
        TypeSpec::Boolean | TypeSpec::TruthValue => Kind::Truth,
        TypeSpec::Vector => Kind::Vector,
        TypeSpec::Text => Kind::Text,
        TypeSpec::Quote => Kind::Quote,
        TypeSpec::Any => Kind::Any,
    }
}

/// Only definite contradictions count. `Any` on either side never conflicts.
///
/// Text is a vector, so it satisfies a `Vector` position. A bare vector is not
/// text, so it does not satisfy a `Text` position — that asymmetry is the
/// Semantic Plane showing up in the lint, and it is what catches
/// `{ 1 } [ 88 ] DEF`.
fn contradicts(expected: TypeSpec, actual: Kind) -> bool {
    if actual == Kind::Any || expected == TypeSpec::Any {
        return false;
    }
    if expected == TypeSpec::Vector && actual == Kind::Text {
        return false;
    }
    kind_of(expected) != actual
}

fn describe(kind: Kind) -> &'static str {
    match kind {
        Kind::Number => "a number",
        Kind::Truth => "a truth value",
        Kind::Vector => "a vector",
        Kind::Text => "text",
        Kind::Quote => "a quote",
        Kind::Any => "unnarrowed",
    }
}

fn push(flow: &mut Flow, slot: Slot) {
    if let Flow::Known(slots) = flow {
        slots.push(slot);
    }
}
