//! `PROBE`'s inference entry point, split from `word_contract.rs` to keep
//! that file under the per-file line budget (§14.1). The algorithm itself is
//! unchanged: this is a thin adapter that lets `infer_word_contract_inner`
//! walk an anonymous CodeBlock's tokens the same way it already walks a
//! named dictionary Word's body.

use std::collections::HashSet;
use std::sync::Arc;

use crate::types::{Capabilities, ExecutionLine, Stability, Tier, Token, WordDefinition};

use super::word_contract::WordContract;
use super::Interpreter;

impl Interpreter {
    /// The same walk `infer_word_contract` runs for a named dictionary Word,
    /// run instead over an anonymous CodeBlock's own tokens. The block is
    /// wrapped in a throwaway `WordDefinition` that is never inserted into
    /// the dictionary — probing resolves the names the block calls but
    /// writes nothing back, matching `PROBE`'s declared purity.
    ///
    /// The synthetic definition's `registration_order` is freshly drawn from
    /// the interpreter's own counter (`next_registration_order`) on every
    /// call. That is not incidental: `contract_cache_key` falls back to
    /// `"unidentified:{name}:{registration_order}"` whenever `word_identity`
    /// has nothing to look up — true for every anonymous block, which is
    /// never named — and two different code blocks that happen to call the
    /// same dependencies would otherwise collide on the same cache key and
    /// silently return each other's inferred contract. A fresh order per
    /// call makes that collision impossible at the cost of never sharing the
    /// cache across probes, which is the correct trade for a Word whose
    /// input is, by construction, unnamed.
    pub(crate) fn infer_contract_for_block(&mut self, tokens: &[Token]) -> Arc<WordContract> {
        let lines: Vec<ExecutionLine> =
            crate::interpreter::execute_def::parse_definition_body(tokens).unwrap_or_else(|_| {
                vec![ExecutionLine {
                    body_tokens: Arc::from(Vec::new()),
                }]
            });
        let def = Arc::new(WordDefinition {
            lines: lines.into(),
            is_builtin: false,
            tier: Tier::Contrib,
            stability: Stability::Stable,
            capabilities: Capabilities::PURE,
            description: None,
            dependencies: HashSet::new(),
            text_references: HashSet::new(),
            original_source: None,
            namespace: None,
            registration_order: self.next_registration_order(),
            execution_plans: None,
        });
        let mut visiting = HashSet::new();
        self.infer_word_contract_inner("", &def, &mut visiting)
            .expect("a freshly synthesized WordDefinition always yields Some")
    }
}
