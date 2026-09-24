//! The reflection Words: `DIGEST`, `CONTRACT`
//! (LANG.DICTIONARY.RESOLUTION, LANG.DICTIONARY.MUTATION, LANG.CONTRACT.REGISTRY,
//! LANG.CONTRACT.CHECK).
//!
//! Each reads what the machine already knows about its dictionary from inside
//! the language — the resolution execution itself performs, the content
//! identity the dictionary already keeps, the contract record the registry
//! already holds — and none of them runs anything. Their operand is a Symbol,
//! a bare name written inside `[ ]`, never a String: a Word that looked a
//! name up from text would turn text into a call, and the acyclicity check
//! (LANG.DICTIONARY.ACYCLIC) is complete only because no Word does. So a
//! String is malformed use, `notASymbol`, rather than a lookup that failed.
//! `CONTRACT` also takes a block, whose contract it infers the way
//! `ajisai check --contract` does — the pre-execution check reached from
//! inside the language (LANG.CONTRACT.CHECK) — never evaluating it, so the
//! call carries none of the block's own effects.

use super::ordering_ops::{restore, take_operand};
use crate::agent::observation_digest::value_digest;
use crate::builtins::lookup_builtin_spec;
use crate::core_word_aliases::canonicalize_core_word_name;
use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::contract_record::{inferred_contract_record, registered_contract_record};
use crate::interpreter::word_identity::content_digest;
use crate::interpreter::Interpreter;
use crate::kernel::generated::generated_word;
use crate::semantic::Recoverability;
use crate::types::{Value, ValueData};

/// Version tag for a sealed Core Word's identity. Core Words have no body to
/// normalize, so their identity is the digest of the canonical name under a
/// tag of its own, distinct from every User identity and every value digest.
const CORE_WORD_IDENTITY_TAG: &[u8] = b"AJISAI-CORE-WORD-1";

fn not_a_symbol(word: &str, got: &str) -> AjisaiError {
    let accepted = if word == "CONTRACT" {
        "a Symbol naming a Word or a block"
    } else {
        "a Symbol naming a Word"
    };
    AjisaiError::declared(
        "notASymbol",
        format!("{word}: expected {accepted}, got {got}; a String is text, not a name"),
    )
}

fn describe(value: &Value) -> &'static str {
    match &value.data {
        ValueData::Text(_) => "a String",
        ValueData::Nil => "NIL",
        ValueData::Boolean(_) => "a Boolean",
        ValueData::Scalar(_) | ValueData::ExactScalar(_) => "a number",
        ValueData::Vector(_) | ValueData::Tensor { .. } => "a Vector",
        ValueData::Record(_) => "a Record",
        ValueData::Symbol(_) => "a Symbol",
    }
}

/// The name a Symbol resolves under: aliases folded, case folded.
fn canonical_name(symbol: &str) -> String {
    canonicalize_core_word_name(symbol).into_owned()
}

/// Which tier a canonical name lives in, if any.
enum Resolved {
    Core,
    User,
}

fn resolve(interp: &Interpreter, canonical: &str) -> Option<Resolved> {
    if lookup_builtin_spec(canonical).is_some() {
        Some(Resolved::Core)
    } else if interp.user_words.contains_key(canonical) {
        Some(Resolved::User)
    } else {
        None
    }
}

fn symbol_name(value: &Value) -> Option<String> {
    match &value.data {
        ValueData::Symbol(name) => Some(name.to_string()),
        _ => None,
    }
}

/// `DIGEST ( [ x ] -> [ digest ] )`: a Word's content identity for a Symbol
/// naming one, the denotation digest for every other value.
pub(crate) fn op_digest(interp: &mut Interpreter) -> Result<()> {
    let operand = take_operand(interp)?;
    let word_identity = symbol_name(&operand).and_then(|name| {
        let canonical = canonical_name(&name);
        match resolve(interp, &canonical)? {
            Resolved::Core => {
                let mut bytes = CORE_WORD_IDENTITY_TAG.to_vec();
                bytes.extend_from_slice(canonical.as_bytes());
                Some(content_digest(&bytes))
            }
            Resolved::User => interp.word_identity(&canonical).cloned(),
        }
    });
    let digest = word_identity.or_else(|| value_digest(&operand));
    match digest {
        Some(digest) => interp.stack.push(Value::from_string(&digest)),
        // A computable real has no finite canonical form to digest: the
        // same outcome its comparison reaches when refinement runs out.
        None => interp.stack.push(Value::nil_with_reason(
            NilReason::Undecidable,
            Recoverability::Retryable,
        )),
    }
    Ok(())
}

/// `CONTRACT ( [ symbol | code ] -> [ record ] )`: the registered contract of
/// a Core Word, the inferred contract of a User Word or of a block (never
/// evaluated); `missingField` for a Symbol naming neither.
pub(crate) fn op_contract(interp: &mut Interpreter) -> Result<()> {
    let operand = take_operand(interp)?;
    if let Some(elements) = operand.as_vector_view() {
        // A block: the same inference `ajisai check --contract` runs, over
        // the block's tokens, without running one of them.
        let tokens = match crate::interpreter::value_as_code::value_elements_to_tokens(&elements) {
            Ok(tokens) => tokens,
            Err(e) => {
                restore(interp, operand);
                return Err(e);
            }
        };
        let contract = interp.infer_contract_for_block(&tokens);
        interp.stack.push(inferred_contract_record(&contract));
        return Ok(());
    }
    let Some(name) = symbol_name(&operand) else {
        let got = describe(&operand);
        restore(interp, operand);
        return Err(not_a_symbol("CONTRACT", got));
    };
    let canonical = canonical_name(&name);
    let answer = match resolve(interp, &canonical) {
        Some(Resolved::Core) => generated_word(&canonical).map(registered_contract_record),
        Some(Resolved::User) => interp
            .infer_word_contract(&canonical)
            .map(|contract| inferred_contract_record(&contract)),
        None => None,
    };
    match answer {
        Some(record) => interp.stack.push(record),
        None => interp.stack.push(Value::nil_with_reason(
            NilReason::MissingField,
            Recoverability::Recoverable,
        )),
    }
    Ok(())
}
