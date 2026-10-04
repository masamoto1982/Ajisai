//! Lowering a run of a line to a typed segment (`segment`), from a compiled
//! line's ops or from the program's own tokens.
//!
//! A run is cut at the first op a segment cannot hold, and kept only when it
//! dispatches at least two Words: one alone is `quickened`'s already.
//!
//! A name the run's own frame bound earlier is a slot read; any other name is
//! read when the run starts, from the frames the line can see, and a run
//! whose name is not a plain value then declines. A User Word call is
//! inlined when its whole body lowers: the body is a barrier frame, so it
//! reads only names it binds itself, and those end with the call.

use std::collections::HashMap;
use std::sync::Arc;

use crate::interpreter::compiled_plan::{lower_tokens_to_ops, CompiledOp};
use crate::interpreter::fused_block_lower::bindable_name;
use crate::interpreter::interpreter_core::MAX_USER_WORD_DEPTH;
use crate::interpreter::quickened::{Kind, Slot};
use crate::interpreter::segment::{LineSegment, SegOp, Segment};
use crate::interpreter::{is_plan_valid, Interpreter};
use crate::types::{Token, Value};

/// Ops one segment may grow to through inlining: a call tree can be far
/// larger than the line that makes it.
const MAX_SEGMENT_OPS: usize = 4096;

/// One thing a run does, read from either source.
enum Item {
    Literal(Slot),
    /// `TRUE`/`FALSE`, and whether its route checks nesting after it.
    WordLiteral(bool, bool),
    Word(Kind),
    /// `'NAME' BIND`, the name already checked bindable.
    Bind(String),
    Name(String),
    Call(String),
}

fn op_item(interp: &Interpreter, ops: &[CompiledOp], i: usize) -> Option<(Item, usize)> {
    Some(match &ops[i] {
        CompiledOp::PushLiteral(value) => match Slot::of(value) {
            Some(slot) => (Item::Literal(slot), 1),
            None => match ops.get(i + 1) {
                Some(CompiledOp::CallBuiltin(call))
                    if call.word.map(|w| w.id) == Some(crate::kernel::generated::WordId::Bind) =>
                {
                    (Item::Bind(bindable_name(interp, value)?), 2)
                }
                _ => return None,
            },
        },
        CompiledOp::PushWordLiteral(value, _) => match Slot::of(value)? {
            Slot::Bool(b) => (Item::WordLiteral(b, false), 1),
            Slot::Num(_) => return None,
        },
        CompiledOp::CallBuiltin(call) => (Item::Word(Kind::of(call.word?.id)?), 1),
        CompiledOp::CallUserWord(name) => (Item::Call(name.clone()), 1),
        CompiledOp::FallbackToken(Token::Symbol(name)) => (
            Item::Name(crate::word_name::canonical_word_name(name).into_owned()),
            1,
        ),
        CompiledOp::PushVectorLiteral(_) | CompiledOp::FallbackToken(_) => return None,
    })
}

fn token_item(interp: &Interpreter, tokens: &[Token], i: usize) -> Option<(Item, usize)> {
    Some(match &tokens[i] {
        Token::Number(literal) => (
            Item::Literal(Slot::of(&Value::from_fraction(literal.parsed().ok()?))?),
            1,
        ),
        Token::String(name) => match tokens.get(i + 1) {
            Some(Token::Symbol(word))
                if crate::word_name::canonical_word_name(word).as_ref() == "BIND" =>
            {
                (
                    Item::Bind(bindable_name(interp, &Value::from_string(name))?),
                    2,
                )
            }
            _ => return None,
        },
        Token::Symbol(symbol) => {
            let name = crate::word_name::canonical_word_name(symbol);
            let item = match name.as_ref() {
                "TRUE" => Item::WordLiteral(true, true),
                "FALSE" => Item::WordLiteral(false, true),
                core if interp.core_vocabulary.contains_key(core) => {
                    let word = interp.core_vocabulary.get(core)?.generated?;
                    Item::Word(Kind::of(word.id)?)
                }
                user if interp.user_words.contains_key(user) => Item::Call(user.to_string()),
                other => Item::Name(other.to_string()),
            };
            (item, 1)
        }
        Token::VectorStart | Token::VectorEnd | Token::Value(_) => return None,
    })
}

struct Builder<'a> {
    interp: &'a Interpreter,
    ops: Vec<SegOp>,
    slots: u32,
    /// Stack depth relative to the run's start, and the lowest it reached.
    depth: isize,
    lowest: isize,
    top: HashMap<String, u32>,
    reads: Vec<(String, u32)>,
    binds: Vec<(String, u32)>,
    steps: usize,
    callees: Vec<String>,
    calls: u64,
    call_depth: usize,
}

impl<'a> Builder<'a> {
    fn new(interp: &'a Interpreter) -> Self {
        Self {
            interp,
            ops: Vec::new(),
            slots: 0,
            depth: 0,
            lowest: 0,
            top: HashMap::new(),
            reads: Vec::new(),
            binds: Vec::new(),
            steps: 0,
            callees: Vec::new(),
            calls: 0,
            call_depth: 0,
        }
    }

    fn pop_push(&mut self, pops: usize) {
        self.depth -= pops as isize;
        self.lowest = self.lowest.min(self.depth);
        self.depth += 1;
    }

    fn slot(&mut self, frame: &mut HashMap<String, u32>, name: &str) -> u32 {
        if let Some(slot) = frame.get(name) {
            return *slot;
        }
        self.slots += 1;
        frame.insert(name.to_string(), self.slots - 1);
        self.slots - 1
    }

    /// Lower `item` in the run's own frame (`frame` `None`) or in a called
    /// Word's, `level` frames deep, or answer `false` with nothing changed.
    fn accept(
        &mut self,
        item: Item,
        frame: Option<&mut HashMap<String, u32>>,
        level: usize,
    ) -> bool {
        self.lower(item, frame, level).is_some()
    }

    fn lower(
        &mut self,
        item: Item,
        frame: Option<&mut HashMap<String, u32>>,
        level: usize,
    ) -> Option<()> {
        // Checked first, so an item refused for it has changed nothing.
        if self.ops.len() >= MAX_SEGMENT_OPS {
            return None;
        }
        match item {
            Item::Literal(slot) => {
                self.ops.push(SegOp::Push(slot));
                self.depth += 1;
            }
            Item::WordLiteral(value, checked) => {
                self.ops.push(SegOp::PushWord { value, checked });
                self.depth += 1;
                self.steps += 1;
            }
            Item::Word(kind) => {
                self.ops.push(SegOp::Word(kind));
                self.pop_push(kind.arity());
                self.steps += 1;
            }
            Item::Bind(name) => {
                let slot = match frame {
                    Some(frame) => self.slot(frame, &name),
                    None => {
                        let mut top = std::mem::take(&mut self.top);
                        let slot = self.slot(&mut top, &name);
                        self.top = top;
                        if !self.binds.iter().any(|(bound, _)| *bound == name) {
                            self.binds.push((name, slot));
                        }
                        slot
                    }
                };
                self.ops.push(SegOp::Bind(slot));
                self.pop_push(1);
                self.depth -= 1;
                self.steps += 1;
            }
            Item::Name(name) => {
                let slot = match frame {
                    // A called Word reads only what it bound itself.
                    Some(frame) => *frame.get(&name)?,
                    None => match self.top.get(&name) {
                        Some(slot) => *slot,
                        None => {
                            let mut top = std::mem::take(&mut self.top);
                            let slot = self.slot(&mut top, &name);
                            self.top = top;
                            self.reads.push((name, slot));
                            slot
                        }
                    },
                };
                self.ops.push(SegOp::Load(slot));
                self.depth += 1;
            }
            Item::Call(name) => {
                let mark = (
                    self.ops.len(),
                    self.slots,
                    self.depth,
                    self.lowest,
                    self.steps,
                    self.callees.len(),
                    self.calls,
                    self.call_depth,
                );
                if self.inline(&name, level + 1).is_none() {
                    self.ops.truncate(mark.0);
                    self.slots = mark.1;
                    self.depth = mark.2;
                    self.lowest = mark.3;
                    self.steps = mark.4;
                    self.callees.truncate(mark.5);
                    self.calls = mark.6;
                    self.call_depth = mark.7;
                    return None;
                }
            }
        }
        Some(())
    }

    /// Inline a call of the User Word `name`, `level` frames deep.
    fn inline(&mut self, name: &str, level: usize) -> Option<()> {
        if level > MAX_USER_WORD_DEPTH {
            return None;
        }
        let def = self.interp.user_words.get(name)?;
        if def.body.is_empty() {
            return None;
        }
        let ops: Arc<[CompiledOp]> = match &def.compiled_plan {
            Some(plan) if is_plan_valid(plan, self.interp) => plan.line.ops.clone().into(),
            _ => lower_tokens_to_ops(&def.body, self.interp).into(),
        };
        self.steps += 1;
        self.calls += 1;
        self.call_depth = self.call_depth.max(level);
        if !self.callees.iter().any(|callee| callee == name) {
            self.callees.push(name.to_string());
        }
        let mut frame = HashMap::new();
        let mut i = 0;
        while i < ops.len() {
            let (item, consumed) = op_item(self.interp, &ops, i)?;
            if !self.accept(item, Some(&mut frame), level) {
                return None;
            }
            i += consumed;
        }
        Some(())
    }

    /// Lower the items from `start` in the run's own frame for as long as
    /// they lower, and answer where the run ends.
    fn extend(
        &mut self,
        start: usize,
        len: usize,
        item_at: impl Fn(usize) -> Option<(Item, usize)>,
    ) -> usize {
        let mut i = start;
        while i < len {
            let Some((item, consumed)) = item_at(i) else {
                break;
            };
            if !self.accept(item, None, 0) {
                break;
            }
            i += consumed;
        }
        i
    }

    fn finish(self) -> Option<Segment> {
        (self.steps >= 2).then(|| Segment {
            ops: self.ops,
            inputs: (-self.lowest) as usize,
            slots: self.slots as usize,
            reads: self.reads,
            binds: self.binds,
            steps: self.steps,
            callees: self.callees,
            calls: self.calls,
            call_depth: self.call_depth,
            dictionary_epoch: self.interp.dictionary_epoch,
        })
    }
}

/// The segments of a compiled line, each as long as it can be made.
pub(crate) fn segment_line(ops: &[CompiledOp], interp: &Interpreter) -> Vec<LineSegment> {
    let mut segments = Vec::new();
    if !interp.segments_enabled {
        return segments;
    }
    let mut i = 0;
    while i < ops.len() {
        let start = i;
        let mut builder = Builder::new(interp);
        i = builder.extend(start, ops.len(), |i| op_item(interp, ops, i));
        if let Some(code) = builder.finish() {
            segments.push(LineSegment {
                start,
                end: i,
                code: Arc::new(code),
            });
        }
        if i == start {
            i += 1;
        }
    }
    segments
}

/// The segment starting at `tokens[start]`, if one is worth running, and
/// where the run it was lowered from ends.
pub(crate) fn segment_tokens(
    interp: &Interpreter,
    tokens: &[Token],
    start: usize,
) -> (Option<Segment>, usize) {
    let mut builder = Builder::new(interp);
    let end = builder.extend(start, tokens.len(), |i| token_item(interp, tokens, i));
    (builder.finish(), end.max(start + 1))
}
