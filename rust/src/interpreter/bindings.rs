//! Local bindings — naming a value for the rest of the frame that made it.
//!
//! A stack gives a chain, and an expression worth writing down is usually a
//! DAG. A residual feeds both the squared error and the gradient; a mask feeds
//! both a count and a weighted count; a threshold feeds both the comparison and
//! the report. Every one of those needs a value more than once, and the stack
//! answers only for the value on top.
//!
//! `KEEP` duplicates, which sounds like enough and is not. Copy a value, derive
//! something from the upper copy, and the derived value lands *on* the lower
//! copy — the second use is now buried under the first result and cannot be
//! reached. Three copies do not help: consuming the first buries the second.
//! The workaround is to pack the values into a matrix and recover them with a
//! weighted fold, so `a - b` is written as
//! `CP DLS * 2 * 2 COLLECT [ [ -1 ] [ 1 ] ] * 0 { + } FOLD`. That is not a
//! program anyone can read, and it is what the third test report was reduced to.
//!
//! So a value gets a name. What makes this a dictionary operation rather than a
//! second name space is where the name lives and how long: a binding belongs to
//! the frame that made it, LANG.SOURCE.FRAME's frame and no other, and it is
//! gone when that frame is. Nothing outside can see it, so no Word's meaning
//! depends on its caller, and content identity is untouched because a binding
//! name is text in the body like every other name.

use crate::error::{AjisaiError, Result};
use crate::types::{Interpretation, Value};
use std::collections::HashMap;

use super::Interpreter;

/// One frame's local names.
///
/// `barrier` marks a scope that lookup does not search past: the frame of a
/// User Word call, or of the whole run. A block a Core Word evaluates —
/// `MAP`'s element block, a `COND` clause, `EXEC`'s block — opens a
/// transparent scope instead, so it reads the names of the frame it was
/// written in. That difference is the whole scoping rule: a name reaches into
/// the blocks written beneath it and never into a Word called from it.
#[derive(Debug, Default, Clone)]
pub(crate) struct BindingScope {
    names: HashMap<String, (Value, Interpretation)>,
    barrier: bool,
}

impl BindingScope {
    /// The scope a run itself owns. A barrier, like a Word call's: there is no
    /// frame outside the run for a name to come from.
    pub(crate) fn root() -> Self {
        Self {
            names: HashMap::new(),
            barrier: true,
        }
    }
}

impl Interpreter {
    /// Open a scope for a frame. `barrier` when the frame is a Word call or the
    /// run itself. Returns nothing: the matching [`Interpreter::close_binding_scope`]
    /// pops whatever this pushed, and the two are always paired within one
    /// function so no depth token is needed.
    pub(crate) fn open_binding_scope(&mut self, barrier: bool) {
        self.binding_scopes.push(BindingScope {
            names: HashMap::new(),
            barrier,
        });
    }

    pub(crate) fn close_binding_scope(&mut self) {
        self.binding_scopes.pop();
    }

    /// Discard every scope and open a fresh root. A run's bindings are the
    /// run's, so nothing survives into the next one — a binding that outlived
    /// its run would be a second, mutable name space competing with the
    /// dictionary, which is the thing this design exists not to be.
    pub(crate) fn reset_binding_scopes(&mut self) {
        self.binding_scopes.clear();
        self.binding_scopes.push(BindingScope::root());
    }

    /// Forget the names of the innermost scope without closing it. The tail-call
    /// trampoline re-runs a body as a backward jump rather than a call, so the
    /// scope stays put while the iteration it belongs to does not.
    pub(crate) fn clear_innermost_bindings(&mut self) {
        if let Some(scope) = self.binding_scopes.last_mut() {
            scope.names.clear();
        }
    }

    /// The value bound to `name` in the frames this one can see.
    pub(crate) fn lookup_binding(&self, name: &str) -> Option<(Value, Interpretation)> {
        for scope in self.binding_scopes.iter().rev() {
            if let Some(bound) = scope.names.get(name) {
                return Some(bound.clone());
            }
            if scope.barrier {
                break;
            }
        }
        None
    }

    /// Whether `name` is bound anywhere at all, including past a barrier. Used
    /// only to tell a reader why a name they can see in the source does not
    /// resolve where they used it.
    pub(crate) fn binding_exists_beyond_barrier(&self, name: &str) -> bool {
        self.binding_scopes
            .iter()
            .any(|scope| scope.names.contains_key(name))
    }

    /// Bind `name` in the innermost scope, replacing any binding it already
    /// holds there.
    ///
    /// Rebinding is deliberate. A block a higher-order Word applies opens a
    /// scope per evaluation, so a `BIND` written inside one is a fresh name
    /// every element rather than a collision on the second; but a body that
    /// binds the same scratch name twice is ordinary enough that refusing it
    /// would buy nothing. This is a local, not a definition: the rule that
    /// protects a referenced Word from being redefined is about names other
    /// code can reach, and nothing outside this frame can reach this one.
    pub(crate) fn bind_local(&mut self, name: String, value: Value, role: Interpretation) {
        if let Some(scope) = self.binding_scopes.last_mut() {
            scope.names.insert(name, (value, role));
        }
    }

    /// Reject a name a binding may not take.
    ///
    /// A binding is looked up before the dictionary, so a binding that took a
    /// Word's name would shadow it — and LANG.DICTIONARY.RESOLUTION makes Core
    /// unshadowable and resolution a function of the name and the dictionary
    /// alone. Refusing the overlap outright keeps that true and spares a reader
    /// the question of which one they got: within a program, a name is a Word
    /// or a binding and never both.
    pub(crate) fn check_bindable_name(&self, name: &str) -> Result<()> {
        if !crate::tokenizer::is_symbol_token_lexeme(name) {
            return Err(AjisaiError::from(format!(
                "BIND: '{}' is not a name. A binding is named like a Word.",
                name
            )));
        }
        if let Some(message) =
            crate::interpreter::naming_convention_checker::check_reserved_word_name(name)
        {
            return Err(AjisaiError::from(message.replace("define", "bind")));
        }
        let upper = name.to_uppercase();
        if self.core_vocabulary.contains_key(&upper) {
            return Err(AjisaiError::from(format!(
                "Cannot bind '{}': it is a Core Word, and a binding may not shadow one.",
                upper
            )));
        }
        if self.user_words.contains_key(&upper) {
            return Err(AjisaiError::from(format!(
                "Cannot bind '{}': it is a User Word. Delete it first, or bind another name.",
                upper
            )));
        }
        Ok(())
    }
}

/// `BIND ( x [ 'NAME' ] -> )`, and `BIND ( [ a b ] [ 'A' 'B' ] -> )`.
///
/// One name takes the whole value. Several names destructure a Vector of the
/// same length, position by position — the generalization `GET` makes for
/// reading, made once more for naming, because a state vector that arrives as
/// one value and wants to be read as its parts is the shape most of this comes
/// in.
///
/// Both operands are consumed and nothing is pushed. That is the point: the
/// value leaves the stack and becomes reachable by name for the rest of the
/// frame, however many times and in whatever order, which is exactly what a
/// stack cannot offer and what a DAG-shaped expression needs.
pub(crate) fn op_bind(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let (name_value, name_role) = interp.stack.pop_slot().ok_or(AjisaiError::StackUnderflow)?;
    let names = match binding_names(&name_value).and_then(|names| {
        for name in &names {
            interp.check_bindable_name(name)?;
        }
        Ok(names)
    }) {
        Ok(names) => names,
        Err(error) => {
            interp.stack.push_with_role(name_value, name_role);
            return Err(error);
        }
    };

    let (subject, role) = interp.stack.pop_slot().ok_or(AjisaiError::StackUnderflow)?;

    match names.as_slice() {
        [only] => interp.bind_local(only.to_uppercase(), subject, role),
        several => {
            // Destructuring is exact. A Vector longer than the name list would
            // otherwise drop its tail silently, and a shorter one would bind a
            // name to nothing; both are the caller having said something
            // different from what they meant.
            let width = if subject.is_vector() {
                subject.len()
            } else {
                0
            };
            if width != several.len() {
                interp.stack.push_with_role(subject, role);
                interp.stack.push_with_role(name_value, name_role);
                return Err(AjisaiError::create_structure_error(
                    &format!("vector of {} elements", several.len()),
                    &format!("{} of them", width),
                ));
            }
            for (position, name) in several.iter().enumerate() {
                let part = subject
                    .child(position)
                    .expect("the element count was checked against the name count");
                let part_role = part.hint;
                interp.bind_local(name.to_uppercase(), part, part_role);
            }
        }
    }
    Ok(())
}

/// The names an operand gives. A String is one name; a Vector of Strings is
/// that many, and a one-element Vector is the single-name case, so `[ 'W' ]`
/// and `'W'` mean the same thing.
fn binding_names(value: &Value) -> Result<Vec<String>> {
    use crate::interpreter::value_extraction_helpers::value_as_string;
    if value.is_text() {
        return Ok(vec![value_as_string(value).unwrap_or_default()]);
    }
    let Some(children) = value.as_vector() else {
        return Err(AjisaiError::create_structure_error(
            "name or vector of names",
            "other value",
        ));
    };
    children
        .iter()
        .map(|child| {
            if child.is_text() {
                Ok(value_as_string(child).unwrap_or_default())
            } else {
                Err(AjisaiError::create_structure_error("name", "other value"))
            }
        })
        .collect()
}
