//! The interpreter: the one execution path.
//!
//! ```text
//! parse -> normalize -> interpret -> result
//! ```
//!
//! There is no second backend, no plan cache, no speculative path, no
//! policy switch, and no feature flag that changes what a program means.
//! Ajisai Core has exactly one evaluator, and its behaviour is the language's
//! behaviour.
//!
//! Two things live here rather than in individual words, because both of them
//! are properties of *how a word is fed* rather than of what a word computes:
//!
//! * [`Interpreter::apply_op`] — the operand layer. It implements
//!   `TOP`/`STAK` and `EAT`/`KEEP` once, for every word with a fixed stack
//!   effect. No word implements a mode.
//! * [`Interpreter::run_vent`] — `VENT`. It decides whether the next source
//!   unit is evaluated at all, so a blocked unit is never run, never errors,
//!   and never has an effect.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::contract::{Arity, Body, StakSupport, Word};
use crate::error::{Error, Result};
use crate::k3::Truth;
use crate::mode::{Mode, Retention, Selection};
use crate::syntax::{self, Node};
use crate::value::Value;
use crate::words;

/// The nesting budget for basins, quotes, and user words. A budget, not a
/// semantics: reaching it is an error, never a silent truncation.
pub const DEPTH_LIMIT: usize = 512;

/// The Ajisai Core interpreter.
pub struct Interpreter {
    stack: Vec<Value>,
    dictionary: BTreeMap<String, Arc<Vec<Node>>>,
    registry: BTreeMap<&'static str, Word>,
    mode: Mode,
    depth: usize,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        let mut registry = BTreeMap::new();
        for word in words::core_words() {
            registry.insert(word.contract.name, word);
        }
        Self {
            stack: Vec::new(),
            dictionary: BTreeMap::new(),
            registry,
            mode: Mode::DEFAULT,
            depth: 0,
        }
    }

    // ---------------------------------------------------------------- state

    /// The flow's current cross-section, bottom first.
    pub fn stack(&self) -> &[Value] {
        &self.stack
    }

    /// The names of every word the interpreter knows: Ajisai Core, registered
    /// packages, and user definitions.
    pub fn vocabulary(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.registry.keys().copied().collect();
        names.extend(self.dictionary.keys().map(String::as_str));
        names.sort_unstable();
        names
    }

    /// Every registered word's contract, in name order.
    pub fn contracts(&self) -> Vec<&Word> {
        self.registry.values().collect()
    }

    pub fn word(&self, name: &str) -> Option<&Word> {
        self.registry.get(name)
    }

    /// The user definitions, in name order.
    pub fn definitions(&self) -> Vec<(&str, &Arc<Vec<Node>>)> {
        self.dictionary
            .iter()
            .map(|(name, body)| (name.as_str(), body))
            .collect()
    }

    // ------------------------------------------------------------ execution

    /// Parse and run a source fragment against the current flow.
    ///
    /// The flow persists between calls, which is what makes a session or a
    /// REPL possible. The mode always starts at `TOP EAT`: an error leaves no
    /// armed mode behind for the next fragment to inherit.
    pub fn execute(&mut self, source: &str) -> Result<()> {
        let program = syntax::parse(source)?;
        self.mode = Mode::DEFAULT;
        self.depth = 0;
        let result = self.eval_body(&program);
        if result.is_err() {
            self.mode = Mode::DEFAULT;
        }
        result
    }

    /// Evaluate a body in its own mode scope.
    ///
    /// A quote boundary saves the surrounding mode, starts the body at the
    /// default, and restores the surrounding mode on the way out. A mode that
    /// is armed but never consumed by the end of a body is an error, so a
    /// stray `KEEP` is reported where it was written instead of silently
    /// attaching to whatever comes next.
    pub fn eval_body(&mut self, body: &[Node]) -> Result<()> {
        if self.depth >= DEPTH_LIMIT {
            return Err(Error::DepthLimitExceeded { limit: DEPTH_LIMIT });
        }
        self.depth += 1;
        let outer = std::mem::replace(&mut self.mode, Mode::DEFAULT);
        let mut result = self.eval_sequence(body);
        if result.is_ok() && !self.mode.is_default() {
            result = Err(Error::DanglingMode { mode: self.mode });
        }
        self.mode = outer;
        self.depth -= 1;
        result
    }

    fn eval_sequence(&mut self, body: &[Node]) -> Result<()> {
        let mut index = 0;
        while index < body.len() {
            if let Node::Word(name) = &body[index] {
                if let Some(selection) = selection_word(name) {
                    self.mode.selection = selection;
                    index += 1;
                    continue;
                }
                if let Some(retention) = retention_word(name) {
                    self.mode.retention = retention;
                    index += 1;
                    continue;
                }
                if name == "VENT" {
                    let span = unit_len(body, index + 1)?;
                    let mode = std::mem::replace(&mut self.mode, Mode::DEFAULT);
                    self.run_vent(&body[index + 1..index + 1 + span], mode)?;
                    index += 1 + span;
                    continue;
                }
            }
            self.eval_node(&body[index])?;
            index += 1;
        }
        Ok(())
    }

    fn eval_node(&mut self, node: &Node) -> Result<()> {
        match node {
            // Literals, basins, and quotes flow onto the stack without
            // consuming an armed mode: a mode is a statement about the next
            // *word*, and discarding it silently on an intervening literal
            // would make `KEEP [ 1 2 ] ADD` mean something other than it reads.
            Node::Literal(value) => {
                self.stack.push(value.clone());
                Ok(())
            }
            Node::Quote(body) => {
                self.stack.push(Value::quote(Arc::clone(body)));
                Ok(())
            }
            Node::Basin(body) => {
                let produced = self.run_in_basin(body, Vec::new())?;
                self.stack.push(Value::vector(produced));
                Ok(())
            }
            Node::Word(name) => self.invoke(name),
        }
    }

    /// Invoke a word, atomically with respect to the flow.
    ///
    /// `SPECIFICATION.md` §5.7 promises that a word that fails leaves the flow
    /// exactly as it found it. Words dispatched through the operand layer get
    /// that from the shape of [`Interpreter::apply_op`], which runs the word
    /// before it touches the stack. Every other word — one that needs the
    /// interpreter, and every user definition — draws its own operands and can
    /// fail after doing so, so this is where they are wrapped: the flow is
    /// saved, and restored if the word fails.
    ///
    /// The dictionary is deliberately *not* rolled back; §5.7 is a promise
    /// about the flow. A quote that defines a word and then fails leaves the
    /// definition behind, and the specification says so.
    fn invoke(&mut self, name: &str) -> Result<()> {
        let mode = std::mem::replace(&mut self.mode, Mode::DEFAULT);

        if let Some(body) = self.dictionary.get(name).cloned() {
            // A user word's stack effect is whatever its body does, so there
            // is no operand region for the mode layer to select. Rejecting the
            // mode is honest; silently ignoring it would not be.
            if !mode.is_default() {
                return Err(Error::ModeUnsupported {
                    word: name.to_string(),
                    mode,
                });
            }
            return self.atomically(|interpreter| interpreter.eval_body(&body));
        }

        let Some(word) = self.registry.get(name) else {
            return Err(Error::UnknownWord(name.to_string()));
        };
        let arity = word.contract.arity;
        let stak = word.contract.stak;
        match word.body {
            Body::Op(op) => self.apply_op(name, op, arity, stak, mode),
            Body::Full(run) => self.apply_full(name, run, arity, mode),
            // Unreachable: `eval_sequence` intercepts every directive by name
            // before a word is ever invoked. Reported rather than panicked so
            // the interpreter stays total.
            Body::Directive => Err(Error::MalformedToken(name.to_string())),
        }
    }

    /// Run `action`, restoring the flow if it fails.
    fn atomically(&mut self, action: impl FnOnce(&mut Interpreter) -> Result<()>) -> Result<()> {
        let saved = self.stack.clone();
        let result = action(self);
        if result.is_err() {
            self.stack = saved;
        }
        result
    }

    /// Invoke a word that draws its own operands.
    ///
    /// `STAK` is not available here: the common layer cannot re-drive a word
    /// that reaches into the interpreter. `KEEP` is, whenever the word declares
    /// a fixed stack effect — the operands are remembered, the word runs, and
    /// they are laid back underneath whatever it produced. Refusing `KEEP` here
    /// would be an implementation detail leaking into the language.
    fn apply_full(
        &mut self,
        name: &str,
        run: crate::contract::FullFn,
        arity: Arity,
        mode: Mode,
    ) -> Result<()> {
        if mode.selection == Selection::Stak {
            return Err(Error::ModeUnsupported {
                word: name.to_string(),
                mode,
            });
        }
        let Some((inn, out)) = arity.fixed() else {
            if !mode.is_default() {
                return Err(Error::ModeUnsupported {
                    word: name.to_string(),
                    mode,
                });
            }
            return self.atomically(run);
        };
        let depth = self.stack.len();
        if depth < inn {
            return Err(Error::StackUnderflow {
                word: name.to_string(),
                needed: inn,
                found: depth,
            });
        }
        let base = depth - inn;
        let operands = if mode.retention == Retention::Keep {
            self.stack[base..].to_vec()
        } else {
            Vec::new()
        };
        self.atomically(run)?;
        // The word declared a fixed effect; hold it to it, so that splicing the
        // kept operands back in cannot land in the wrong place.
        let expected = base + out;
        if self.stack.len() != expected {
            return Err(Error::ContractViolated {
                word: name.to_string(),
                expected,
                found: self.stack.len(),
            });
        }
        for (offset, operand) in operands.into_iter().enumerate() {
            self.stack.insert(base + offset, operand);
        }
        Ok(())
    }

    // --------------------------------------------------------- operand layer

    /// Select operands, run the word, commit the result — once, for every
    /// word with a fixed stack effect.
    ///
    /// The word's own function is called *before* the stack is touched, so a
    /// word that fails leaves the flow exactly as it found it. Word-level
    /// atomicity comes out of the shape of this function rather than out of a
    /// snapshot taken on every step.
    fn apply_op(
        &mut self,
        name: &str,
        op: crate::contract::OpFn,
        arity: Arity,
        stak: StakSupport,
        mode: Mode,
    ) -> Result<()> {
        let Some((inn, _out)) = arity.fixed() else {
            return Err(Error::ModeUnsupported {
                word: name.to_string(),
                mode,
            });
        };
        match mode.selection {
            Selection::Top => {
                let depth = self.stack.len();
                if depth < inn {
                    return Err(Error::StackUnderflow {
                        word: name.to_string(),
                        needed: inn,
                        found: depth,
                    });
                }
                let base = depth - inn;
                let results = op(name, &self.stack[base..])?;
                if mode.retention == Retention::Eat {
                    self.stack.truncate(base);
                }
                self.stack.extend(results);
                Ok(())
            }
            // What `STAK` means for a word is declared by the word, not
            // inferred from how many operands it takes.
            Selection::Stak => match stak {
                StakSupport::MapEach => {
                    debug_assert_eq!(inn, 1, "MapEach requires one input");
                    let mut results = Vec::with_capacity(self.stack.len());
                    for cell in self.stack.iter() {
                        results.extend(op(name, std::slice::from_ref(cell))?);
                    }
                    if mode.retention == Retention::Eat {
                        self.stack.clear();
                    }
                    self.stack.extend(results);
                    Ok(())
                }
                StakSupport::FoldLeft => {
                    debug_assert_eq!(inn, 2, "FoldLeft requires two inputs");
                    if self.stack.is_empty() {
                        return Err(Error::StackUnderflow {
                            word: name.to_string(),
                            needed: 1,
                            found: 0,
                        });
                    }
                    let mut accumulator = self.stack[0].clone();
                    for index in 1..self.stack.len() {
                        let operands = [accumulator, self.stack[index].clone()];
                        let mut produced = op(name, &operands)?;
                        accumulator = produced.pop().ok_or_else(|| Error::StackUnderflow {
                            word: name.to_string(),
                            needed: 1,
                            found: 0,
                        })?;
                    }
                    if mode.retention == Retention::Eat {
                        self.stack.clear();
                    }
                    self.stack.push(accumulator);
                    Ok(())
                }
                StakSupport::Unsupported => Err(Error::ModeUnsupported {
                    word: name.to_string(),
                    mode,
                }),
            },
        }
    }

    // ------------------------------------------------------------------ vent

    /// Release or block the next source unit.
    ///
    /// `VENT` reads one truth value off the surface and then decides whether
    /// the unit that follows is evaluated *at all*. A blocked unit is not
    /// evaluated, so it cannot divide by zero, cannot name a word that does
    /// not exist, and cannot change the dictionary.
    ///
    /// * `TRUE` — the unit runs. A unit that is a single quote is entered
    ///   rather than pushed: the flow goes through the quoted channel.
    /// * `FALSE` — the unit does not run and nothing is pushed.
    /// * `UNKNOWN` — the unit does not run and a single `UNKNOWN` is pushed,
    ///   marking that what would have been released is undetermined. Blocking
    ///   silently would make an undetermined gate indistinguishable from a
    ///   closed one, which is the operational form of reading `UNKNOWN` as
    ///   `FALSE`.
    /// * anything else, `NIL` included — [`Error::NotATruthValue`].
    ///
    /// Under `KEEP`, the gate is drawn off the flow, the unit runs against the
    /// flow beneath it, and the gate is then returned to the surface — so the
    /// gate ends up *above* whatever the unit released. That ordering is what
    /// makes the two-branch idiom work:
    ///
    /// ```text
    /// 5 0 GT KEEP VENT { "positive" } NOT VENT { "not positive" }
    /// ```
    fn run_vent(&mut self, unit: &[Node], mode: Mode) -> Result<()> {
        if mode.selection == Selection::Stak {
            return Err(Error::ModeUnsupported {
                word: "VENT".to_string(),
                mode,
            });
        }
        // Atomic like any other word: if the gate is not a truth value, or the
        // released unit fails, the flow is left exactly as it was found —
        // gate included.
        self.atomically(|interpreter| {
            let gate = interpreter
                .stack
                .pop()
                .ok_or_else(|| Error::StackUnderflow {
                    word: "VENT".to_string(),
                    needed: 1,
                    found: 0,
                })?;
            let truth = Truth::read("VENT", &gate)?;
            match truth {
                Truth::True => match unit {
                    [Node::Quote(body)] => interpreter.eval_body(body)?,
                    other => interpreter.eval_body(other)?,
                },
                Truth::False => {}
                Truth::Unknown => interpreter.stack.push(Value::unknown()),
            }
            if mode.retention == Retention::Keep {
                interpreter.stack.push(gate);
            }
            Ok(())
        })
    }

    // ------------------------------------------------- helpers for full words

    pub fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    pub fn pop(&mut self, word: &str) -> Result<Value> {
        self.stack.pop().ok_or_else(|| Error::StackUnderflow {
            word: word.to_string(),
            needed: 1,
            found: 0,
        })
    }

    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Run a body on a fresh flow seeded with `seed`, and return whatever
    /// stands in that flow afterwards. This is what `[ ... ]` does, and what
    /// `MAP`, `FILTER`, and `FOLD` use so that a quote cannot reach past its
    /// own operands into the surrounding flow.
    pub fn run_in_basin(&mut self, body: &[Node], seed: Vec<Value>) -> Result<Vec<Value>> {
        let outer = std::mem::replace(&mut self.stack, seed);
        let result = self.eval_body(body);
        let produced = std::mem::replace(&mut self.stack, outer);
        result.map(|()| produced)
    }

    /// Run a body against the current flow, as `EXEC` does.
    pub fn run_here(&mut self, body: &[Node]) -> Result<()> {
        self.eval_body(body)
    }

    // ------------------------------------------------------------ dictionary

    pub fn define(&mut self, name: String, body: Arc<Vec<Node>>) -> Result<()> {
        if self.registry.contains_key(name.as_str()) {
            return Err(Error::ReservedWord(name));
        }
        self.dictionary.insert(name, body);
        Ok(())
    }

    pub fn undefine(&mut self, name: &str) -> Result<()> {
        if self.registry.contains_key(name) {
            return Err(Error::ReservedWord(name.to_string()));
        }
        if self.dictionary.remove(name).is_none() {
            return Err(Error::UnknownWord(name.to_string()));
        }
        Ok(())
    }

    // -------------------------------------------------------------- packages

    /// Register an external package's words.
    ///
    /// This is the whole extension surface. A package adds words; it cannot
    /// add a value shape, a role, a mode, an error variant, or an execution
    /// path, and Ajisai Core does not know any package exists.
    ///
    /// Registration is all or nothing. Every word is checked first — against
    /// Ajisai Core, against the packages already registered, against the other
    /// words in this package, against the user's definitions, and against its
    /// own contract — and only then is any of it committed. A package that is
    /// rejected leaves the vocabulary exactly as it was, rather than half of
    /// its words installed and the rest missing.
    pub fn register_package(&mut self, package: crate::extension::Package) -> Result<()> {
        let mut incoming: BTreeMap<&'static str, Word> = BTreeMap::new();
        for word in package.words {
            let name = word.contract.name;
            let duplicate = || Error::DuplicateWord {
                package: package.name.to_string(),
                word: name.to_string(),
            };
            if self.registry.contains_key(name) || incoming.contains_key(name) {
                return Err(duplicate());
            }
            // A user definition already answers to this name, and the
            // dictionary is consulted first, so registering would install a
            // word nothing could reach.
            if self.dictionary.contains_key(name) {
                return Err(duplicate());
            }
            check_contract(package.name, &word)?;
            incoming.insert(name, word);
        }
        self.registry.extend(incoming);
        Ok(())
    }
}

/// Check that a package word's contract describes its implementation.
///
/// Ajisai Core holds itself to these in a test; a package is held to them here,
/// at registration, because there is no test of ours to run against it. Every
/// one of them is a rule the evaluator or the lint later relies on.
fn check_contract(package: &str, word: &Word) -> Result<()> {
    let contract = &word.contract;
    let reject = |reason: String| {
        Err(Error::MalformedContract {
            package: package.to_string(),
            word: contract.name.to_string(),
            reason,
        })
    };
    if contract.name.is_empty()
        || contract.name.chars().any(char::is_whitespace)
        || contract.name != crate::alias::canonical(contract.name)
    {
        return reject("the name is not a canonical word name".to_string());
    }
    if is_directive(contract.name) {
        return reject("the name is a flow directive".to_string());
    }
    if contract.summary.is_empty() {
        return reject("no summary".to_string());
    }
    match crate::contract::notation_arity(contract.stack_effect) {
        None => {
            return reject(format!(
                "stack effect {:?} is malformed",
                contract.stack_effect
            ))
        }
        Some(parsed) => {
            if let Some(declared) = contract.arity.fixed() {
                if parsed != declared {
                    return reject(format!(
                        "stack effect {:?} disagrees with the declared arity",
                        contract.stack_effect
                    ));
                }
            }
        }
    }
    if let Some((inn, out)) = contract.arity.fixed() {
        if contract.input_types.len() != inn || contract.output_types.len() != out {
            return reject("the type lists do not match the arity".to_string());
        }
    }
    if let Some((position, _)) = contract.role_required {
        if position >= contract.input_types.len() {
            return reject("role_required names an operand the word does not take".to_string());
        }
    }
    match contract.stak {
        StakSupport::Unsupported => {}
        _ if !matches!(word.body, Body::Op(_)) => {
            return reject(
                "only an operand-to-result word can be driven across a whole flow".to_string(),
            );
        }
        StakSupport::MapEach => {
            if contract.arity.fixed().map(|(inn, _)| inn) != Some(1) {
                return reject("MapEach needs exactly one input".to_string());
            }
        }
        StakSupport::FoldLeft => {
            let closed = contract.arity.fixed() == Some((2, 1))
                && contract.input_types.first() == contract.output_types.first();
            if !closed {
                return reject(
                    "FoldLeft needs a closed operation: two in, one out, and an output type \
                     identical to the first input type"
                        .to_string(),
                );
            }
        }
    }
    Ok(())
}

fn selection_word(name: &str) -> Option<Selection> {
    match name {
        "TOP" => Some(Selection::Top),
        "STAK" => Some(Selection::Stak),
        _ => None,
    }
}

fn retention_word(name: &str) -> Option<Retention> {
    match name {
        "EAT" => Some(Retention::Eat),
        "KEEP" => Some(Retention::Keep),
        _ => None,
    }
}

/// True for the words the evaluator handles directly.
pub fn is_directive(name: &str) -> bool {
    selection_word(name).is_some() || retention_word(name).is_some() || name == "VENT"
}

/// How many nodes make up the source unit starting at `start`.
///
/// A unit is one node — a literal, a basin, a quote, or a word — with two
/// rules that keep it from splitting something that must not be split:
///
/// * Mode words attach to the word they govern, so `STAK ADD` is one unit and
///   a blocked vent never leaves a mode armed with nothing to consume it.
/// * A nested `VENT` carries its own unit, so `VENT VENT X` is one unit.
pub fn unit_len(body: &[Node], start: usize) -> Result<usize> {
    let mut index = start;
    while let Some(Node::Word(name)) = body.get(index) {
        if selection_word(name).is_some() || retention_word(name).is_some() {
            index += 1;
            continue;
        }
        break;
    }
    match body.get(index) {
        None => Err(Error::VentMissingUnit),
        Some(Node::Word(name)) if name == "VENT" => {
            let inner = unit_len(body, index + 1)?;
            Ok(index + 1 + inner - start)
        }
        Some(_) => Ok(index + 1 - start),
    }
}
