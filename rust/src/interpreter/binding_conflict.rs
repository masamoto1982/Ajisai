//! The name rule between bindings and Words across the dictionary
//! (LANG.SOURCE.FRAME, LANG.DICTIONARY.RESOLUTION): a name is a Word or a
//! binding, never both, so `DEF` refuses a name some User Word's body binds
//! and a body that binds the name of a User Word.

use super::word_contract_flow::{note_bound_names, BoundNames};
use super::Interpreter;
use crate::types::Token;
use crate::word_name::canonical_word_name;

impl Interpreter {
    /// Why `DEF` of `name` (uppercased) with `body` would let a binding and
    /// a Word share a name, if it would: the name bound in a live frame, or
    /// bound by another Word's body, or a name this body binds that is
    /// already a User Word.
    pub(crate) fn binding_conflict(&self, name: &str, body: &[Token]) -> Option<String> {
        const RULE: &str = "A binding and a Word may not share a name.";
        if self.binding_exists_beyond_barrier(name) {
            return Some(format!(
                "Cannot define '{name}': the name is bound in a live frame. {RULE}"
            ));
        }
        if let Some(binder) = self.user_word_binding(name) {
            return Some(format!(
                "Cannot define '{name}': {binder} binds it as a local name. {RULE} Bind another name in {binder}, or name this Word otherwise."
            ));
        }
        let mut taken: Vec<String> = bound_names(body)
            .into_iter()
            .filter(|word| word != name && self.user_words.contains_key(word))
            .collect();
        taken.sort();
        taken.first().map(|word| {
            format!("Cannot define '{name}': its body binds '{word}', which is a User Word. {RULE}")
        })
    }

    /// The first User Word, by name, whose body binds `name` as a local: a
    /// String spelling it followed by `BIND`, alone (`'K' BIND`) or among the
    /// names of a destructuring (`[ 'J' 'K' ] BIND`). `DEF` of that name is
    /// refused, the static half of the rule `check_bindable_name` keeps at
    /// run time: without it, the Word would be defined, the body that binds
    /// the name would stop at its `BIND`, and the body's own mention of the
    /// name would hold the Word against `DEL`.
    pub(crate) fn user_word_binding(&self, name: &str) -> Option<String> {
        let mut binders: Vec<&String> = self
            .user_words
            .iter()
            .filter(|(word, def)| word.as_str() != name && bound_names(&def.body).contains(name))
            .map(|(word, _)| word)
            .collect();
        binders.sort();
        binders.first().map(|word| (*word).clone())
    }
}

/// The names `body` binds as locals, uppercased: the String written right
/// before a `BIND` (`'K' BIND`), or each String of the Vector literal written
/// right before it (`[ 'J' 'K' ] BIND`).
pub(crate) fn bound_names(body: &[Token]) -> BoundNames {
    let mut bound = BoundNames::new();
    for (at, token) in body.iter().enumerate() {
        if let Token::Symbol(word) = token {
            note_bound_names(&mut bound, &canonical_word_name(word.as_str()), body, at);
        }
    }
    bound
}
