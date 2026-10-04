use crate::error::{AjisaiError, Result};
use crate::types::{Token, Value};

use super::Interpreter;

/// If the bracketed literal spanning `tokens[start..start+consumed)` is
/// immediately followed (no tokens between) by `<name-string> DEF`,
/// return its inner tokens (brackets excluded) — the body `DEF` is about to
/// define, as written. `None` on any mismatch, which just means this literal
/// is not a `DEF` body written in place — see `pending_def_body_tokens`'s
/// doc comment for why `op_def` wants this at all.
fn def_body_tokens_if_literal_precedes_def(
    tokens: &[Token],
    start: usize,
    consumed: usize,
) -> Option<Vec<Token>> {
    let mut j = start + consumed;
    if !matches!(tokens.get(j), Some(Token::String(_))) {
        return None;
    }
    j += 1;
    match tokens.get(j) {
        Some(Token::Symbol(s)) if crate::word_name::canonical_word_name(s).as_ref() == "DEF" => {
            let body = &tokens[start + 1..start + consumed - 1];
            // A stream bridged from a Vector (`EXEC`, a higher-order block —
            // `value_as_code.rs`) carries a value no source denotes whole, as
            // a `Token::Value`, and such a literal is not the body as
            // written: a definition is kept as its source
            // (LANG.DICTIONARY.MUTATION), and only `op_def`'s fallback writes
            // a carried value back as what builds it, or refuses one that
            // nothing builds. Declining sends the body there. Taken as
            // written, `… [ 'X' DEF ] CONCAT EXEC` kept a Record as its
            // display text `{ 'k' 1/1 }`, which no later session could read,
            // and let a reasoned NIL past `invalidDefinitionBody`.
            if body.iter().any(|token| matches!(token, Token::Value(_))) {
                return None;
            }
            Some(body.to_vec())
        }
        _ => None,
    }
}

impl Interpreter {
    pub(crate) fn execute_section_core(
        &mut self,
        tokens: &[Token],
        start_index: usize,
    ) -> Result<usize> {
        // The numeric-literal ceiling is not re-applied here. Every stream
        // that reaches this loop was already held to it where its lexemes were
        // read — the program's own text in `execute`, a body at `DEF` — and
        // the streams bridged from values (`EXEC`, a higher-order block) carry
        // Scalars already built within `bigintBits`, which the compiled route
        // for the same block never held to the literal ceiling either.
        // Checking them here made a block's outcome depend on which route ran
        // it (LANG.AUTHORITY.FREEDOM).

        // Depth 1 is the program's own token stream, the one `source_spans`
        // describes. A nested block, a word body or a COND clause is a
        // different stream with no source of its own, so the cursor is left
        // pointing at the top-level token that reached it — which is exactly
        // the token a reader needs to be sent to.
        self.section_depth += 1;
        let track = self.section_depth == 1 && !self.source_spans.is_empty();
        let result = self.execute_section_tokens(&tokens[start_index..], start_index, track);
        self.section_depth -= 1;
        result
    }

    fn execute_section_tokens(
        &mut self,
        execute_tokens: &[Token],
        start_index: usize,
        track_source_position: bool,
    ) -> Result<usize> {
        let mut i: usize = 0;
        // The program's own stream runs its scalar runs as typed segments
        // (`segment`); tokens before `walk_until` were already lowered once
        // and declined, and are walked.
        let segments = self.segments_enabled && self.section_depth == 1;
        let mut walk_until = 0;

        while i < execute_tokens.len() {
            if segments && i >= walk_until {
                let (segment, end) = super::segment_lower::segment_tokens(self, execute_tokens, i);
                if segment.is_some_and(|segment| segment.try_run(self)) {
                    if track_source_position {
                        self.current_source_span =
                            self.source_spans.get(start_index + end - 1).copied();
                    }
                    i = end;
                    continue;
                }
                walk_until = end;
            }
            if track_source_position {
                self.current_source_span = self.source_spans.get(start_index + i).copied();
            }
            match &execute_tokens[i] {
                Token::Number(literal) => {
                    // Parsed when the lexeme was read, not here — this line ran
                    // once per element of every `MAP` block that mentioned a
                    // number. `parsed` only re-derives anything on the refusal
                    // path, which ends the program.
                    let frac = literal.parsed().map_err(AjisaiError::MalformedSource)?;
                    self.stack.push(Value::from_fraction(frac));
                }
                Token::String(s) => {
                    self.stack.push(Value::from_string(s));
                }
                Token::VectorStart => {
                    // `[ ]` is the sole bracket, built through
                    // `vector_literal.rs`'s collector — see that module's
                    // doc comment. `COND` takes its clause blocks as a
                    // single ordinary Vector operand (one more literal built
                    // and pushed exactly like this one) rather than a
                    // variable-length run recognized here — see
                    // `control_cond.rs::op_cond`'s doc comment for why.
                    let (values, consumed) = Self::collect_bracketed_with_depth(
                        execute_tokens,
                        i,
                        1,
                        &self.runtime_limits,
                    )?;
                    // A literal immediately followed by `<name> DEF`
                    // is that DEF's body — captured here, as written, for
                    // `op_def` to prefer over re-deriving it from the Value
                    // just built (see `pending_def_body_tokens`'s doc
                    // comment). A mismatch here is not an error: it just
                    // means this literal is not a `DEF` body written
                    // in place, and `op_def` falls back normally.
                    self.pending_def_body_tokens =
                        def_body_tokens_if_literal_precedes_def(execute_tokens, i, consumed);
                    self.stack.push(Value::from_vector_promoted(values));
                    i += consumed;
                    continue;
                }
                Token::Symbol(s) => {
                    let canonical = crate::word_name::canonical_word_name(s);
                    {
                        let upper = canonical;

                        let witness = self.begin_dispatch();
                        match self.execute_word_core(upper.as_ref()) {
                            Ok(()) => self.trace_nil_outcome(upper.as_ref(), &witness),
                            Err(err) => {
                                self.record_word_dispatch_failure(
                                    upper.as_ref(),
                                    &err,
                                    witness.stack_len_before,
                                );
                                return Err(err);
                            }
                        }
                    }
                }
                Token::Value(value) => {
                    // A value the Vector-as-code bridge carried in whole: it
                    // is pushed as it is, like a literal. It is never a `DEF`
                    // body, so a pending body capture is cleared.
                    self.pending_def_body_tokens = None;
                    self.stack.push((**value).clone());
                    i += 1;
                    continue;
                }
                Token::VectorEnd => {
                    return Err(AjisaiError::MalformedSource(
                        "Unexpected vector end".to_string(),
                    ));
                }
            }
            i += 1;
        }

        Ok(start_index + i)
    }

    /// Evaluate the tokens of a *nested* code block — the block a higher-order
    /// Word (`MAP`, `FILTER`, `FOLD`, `SCAN`) applies, or the one `EXEC`
    /// runs — as its own token stream.
    pub(crate) fn execute_nested_block(&mut self, tokens: &[Token]) -> Result<()> {
        // A transparent frame: the block reads the names of the frame it was
        // written in, and the names it makes are gone when it ends. That is
        // what lets a bound threshold be used inside `{ T LT } FILTER` — and
        // why a `BIND` inside a `MAP` block is a fresh name per element rather
        // than a collision on the second.
        self.open_binding_scope(false);
        let result = self.execute_section_core(tokens, 0).map(|_| ());
        self.close_binding_scope();
        result
    }

    pub async fn execute(&mut self, code: &str) -> Result<()> {
        // CS5: bound the input before it is expanded into values. Source bytes
        // are checked before tokenization allocates per-character buffers; each
        // numeric literal's digit count is checked after tokenization but
        // before `Fraction::from_str` parses it into a (potentially enormous)
        // BigInt-backed value.
        self.runtime_limits.check_source_bytes(code.len())?;
        self.execution_step_count = 0;
        self.numeric_work_used = 0;
        self.collection_work_used = 0;
        self.dictionary_changes_this_run.clear();
        // A literal captured for a `DEF` that never ran — the dispatch failed
        // before `op_def` took it — must not become the body of the next run's
        // first `DEF`.
        self.pending_def_body_tokens = None;
        self.reset_binding_scopes();
        // Merge rather than replace: a `#:contract` line and the `DEF` it
        // documents can arrive in separate `execute()` calls (the Playground
        // runs one submission at a time), so an entry here must survive until
        // `op_def` actually consumes it for the Word it names.
        for (name, description) in
            crate::interpreter::execute_def::extract_pending_word_descriptions(code)
        {
            self.pending_word_descriptions.insert(name, description);
        }
        // Source entry is the one place a token has a position, so it is the
        // one place the positions are recorded. They are index-aligned with
        // `tokens` and consumed by the depth-1 cursor in `execute_section_core`.
        // A lexical failure is a fault in the writing, not in any value, so it
        // is classified as one rather than arriving as an uncategorized
        // `Custom` whose only next check was "read the message".
        let (tokens, spans) =
            crate::tokenizer::tokenize_with_spans(code).map_err(AjisaiError::MalformedSource)?;
        self.source_spans = spans;
        self.current_source_span = None;
        self.check_source_numeric_literals(&tokens)?;
        self.execute_section_core(&tokens, 0)?;
        self.check_fresh_nesting()
    }

    /// Enforce the numeric-literal digit ceiling on every `Token::Number`
    /// produced from source, before any of them is parsed into a value. The
    /// count is of the digits the literal denotes — written digits plus the
    /// exponent's magnitude (`tokenizer::denoted_digit_count`) — so the bound
    /// tracks the magnitude of the BigInt that would be built: `1e99999999`
    /// is nine characters and used to spend minutes building its integer.
    pub(crate) fn check_source_numeric_literals(&self, tokens: &[Token]) -> Result<()> {
        for token in tokens {
            if let Token::Number(literal) = token {
                let digits = crate::tokenizer::denoted_digit_count(literal.lexeme());
                self.runtime_limits
                    .check_numeric_literal_digits(usize::try_from(digits).unwrap_or(usize::MAX))?;
            }
        }
        Ok(())
    }
}

// Building a Vector literal (`[ ... ]`) from source tokens.
//
// Split out of `execution_loop` when that file outgrew the file-size budget
// in docs/dev/specification-implementation-rules.md. The two concerns are
// genuinely separate: the execution loop decides *which* token runs next,
// and this decides what a delimited token sequence denotes.
//
// What an *element* denotes is one piece of knowledge: a literal value, with
// a bare name (other than `TRUE`/`FALSE`/`NIL`, which still denote their
// values) becoming a `Value::Symbol` rather than a `Value::Text`. Nothing is
// looked up: a literal denotes the same value under every dictionary state.
impl Interpreter {
    /// The elements of one Vector literal, and how many tokens it spans.
    pub(crate) fn collect_bracketed_with_depth(
        tokens: &[Token],
        start_index: usize,
        depth: usize,
        limits: &crate::interpreter::RuntimeLimits,
    ) -> Result<(Vec<Value>, usize)> {
        if tokens.get(start_index) != Some(&Token::VectorStart) {
            return Err(AjisaiError::MalformedSource(
                "Expected a literal start".to_string(),
            ));
        }

        // The nesting ceiling (LANG.MACHINE.LIMITS), checked before recursing:
        // this builder descends one native frame per level, so a few thousand
        // levels of `[ [ [ ... ] ] ]` would overflow the native stack here,
        // before the value could be checked at all. It is the same ceiling a
        // value Words build meets, and fails the same way.
        limits.check_nesting_depth(depth)?;

        let mut values = Vec::new();
        let mut i = start_index + 1;

        while i < tokens.len() {
            match &tokens[i] {
                Token::VectorStart => {
                    let (nested_values, consumed) =
                        Self::collect_bracketed_with_depth(tokens, i, depth + 1, limits)?;
                    values.push(Value::from_vector_promoted(nested_values));
                    i += consumed;
                }
                Token::VectorEnd => {
                    return Ok((values, i - start_index + 1));
                }
                Token::Value(value) => {
                    values.push((**value).clone());
                    i += 1;
                }
                Token::Number(literal) => {
                    values.push(Value::from_number(
                        literal.parsed().map_err(AjisaiError::MalformedSource)?,
                    ));
                    i += 1;
                }
                Token::String(s) => {
                    values.push(Value::from_string(s));
                    i += 1;
                }
                Token::Symbol(s) => {
                    let upper = Self::normalize_symbol(s);
                    match upper.as_ref() {
                        "TRUE" => values.push(Value::from_bool(true)),
                        "FALSE" => values.push(Value::from_bool(false)),
                        "NIL" => values.push(Value::nil()),
                        // A bare name is a Symbol: data until something
                        // executes it, dictionary-independent (building the
                        // literal never looks anything up).
                        _ => values.push(Value::from_symbol(s)),
                    }
                    i += 1;
                }
            }
        }
        Err(AjisaiError::MalformedSource(
            "Unclosed bracketed literal".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    //! Tests for compile-time literal-vector lowering (`CompiledOp::PushVectorLiteral`).
    //!
    //! A fully-literal vector is prebuilt once at compile time with the same
    //! promoted value `collect_vector` produces, so the line runs
    //! compiled instead of falling back to the interpreter. These tests pin that the
    //! lowered path is byte-for-byte identical to the interpreted one across element
    //! kinds, that non-literal vectors still fall back, and that errors are kept.

    use crate::agent::block_on;
    use crate::interpreter::Interpreter;

    /// Run `src` twice — lowering on and off — and assert the resulting stacks are
    /// identical (value and rendered form).
    fn assert_on_equals_off(src: &str) -> String {
        let mut on = Interpreter::new();
        on.set_vector_literal_enabled(true);
        block_on(on.execute(src)).unwrap();

        let mut off = Interpreter::new();
        off.set_vector_literal_enabled(false);
        block_on(off.execute(src)).unwrap();

        assert_eq!(
            format!("{:?}", on.get_stack()),
            format!("{:?}", off.get_stack()),
            "lowering ON vs OFF diverged for: {src}"
        );
        let render_on = render(&on);
        assert_eq!(render_on, render(&off), "rendered form diverged for: {src}");
        render_on
    }

    fn render(interp: &Interpreter) -> String {
        interp
            .get_stack()
            .last()
            .map(|v| format!("{v}"))
            .unwrap_or_default()
    }

    #[test]
    fn literal_vector_shapes_match_interpreter() {
        // Numeric (tensor-promoted), boolean, string,
        // NIL-bearing, nested, and arithmetic-over-literals all agree.
        let cases = [
            "[ [ 1 2 3 ] [ 4 5 6 ] ADD ] 'W' DEF W",
            "[ [ TRUE FALSE TRUE ] ] 'W' DEF W",
            "[ [ 'a' 'b' 'c' ] ] 'W' DEF W",
            "[ [ 1 NIL 3 ] ] 'W' DEF W",
            "[ [ [ 1 2 ] [ 3 4 ] ] ] 'W' DEF W",
            "[ [ 1 2 3 4 ] [ 2 2 2 2 ] MUL [ 1 1 1 1 ] SUB ] 'W' DEF W",
        ];
        for src in cases {
            assert_on_equals_off(src);
        }
    }

    #[test]
    fn boolean_vector_renders_its_booleans() {
        let rendered = assert_on_equals_off("[ [ TRUE FALSE ] ] 'W' DEF W");
        assert!(
            rendered.contains("TRUE") && rendered.contains("FALSE"),
            "boolean vector should render as TRUE/FALSE, got: {rendered}"
        );
    }

    #[test]
    fn symbol_in_vector_is_data_not_executed() {
        // LANG.VALUES.VECTOR: a name inside a Vector literal is its own text as
        // data, even when it names a defined user word. `[ TEN 2 3 ]`
        // is therefore a fully literal vector — TEN is the string "TEN", never the
        // word's result — and lowers identically on the compiled and interpreted
        // paths. This is the regression guard for the retired word-execution behavior.
        let src = "[ [ 10 ] ] 'TEN' DEF\n[ [ TEN 2 3 ] ] 'W' DEF\nW";
        let rendered = assert_on_equals_off(src);
        assert!(
            rendered.contains("TEN"),
            "the symbol must appear as data, got: {rendered}"
        );
        assert!(
            !rendered.contains("10"),
            "the user word must NOT be executed inside the vector, got: {rendered}"
        );
    }

    #[test]
    fn vector_literal_is_independent_of_dictionary_state() {
        // The core of LANG.VALUES.VECTOR: the *same* source vector produces
        // the *same* value whether or not the symbol names a defined word. Before,
        // `[ FOO 1 ]` executed FOO when defined and was data otherwise — a
        // dictionary-state-dependent meaning. Now both are the data `[ "FOO" 1 ]`.
        let mut with_word = Interpreter::new();
        block_on(with_word.execute("[ [ 99 ] ] 'FOO' DEF\n[ FOO 1 ]")).unwrap();

        let mut without_word = Interpreter::new();
        block_on(without_word.execute("[ FOO 1 ]")).unwrap();

        assert_eq!(
            format!("{}", with_word.get_stack().last().unwrap()),
            format!("{}", without_word.get_stack().last().unwrap()),
            "a vector literal must not depend on whether the symbol is a defined word"
        );
        assert!(
            !format!("{}", with_word.get_stack().last().unwrap()).contains("99"),
            "the defined word must not be executed inside the vector"
        );
    }

    #[test]
    fn empty_vector_lowers_identically_both_paths() {
        // `[ ]` used to be rejected, and this pinned that the lowering did not
        // paper over the rejection. It is a value now, so what must agree is the
        // value both paths produce.
        for enabled in [true, false] {
            let mut interp = Interpreter::new();
            interp.set_vector_literal_enabled(enabled);
            block_on(interp.execute("[ [ ] ] 'W' DEF\nW")).expect("`[ ]` is a value");
            let val = interp.get_stack().last().expect("a result").clone();
            assert!(!val.is_nil(), "the empty vector is not an absence");
            assert_eq!(
                val.len(),
                0,
                "empty in both lowering modes (enabled={enabled})"
            );
        }
    }

    #[test]
    fn matches_readme_vector_example() {
        let rendered = assert_on_equals_off("[ [ 1 2 3 ] [ 4 5 6 ] ADD ] 'W' DEF W");
        assert!(
            rendered.contains("5/1") && rendered.contains("7/1") && rendered.contains("9/1"),
            "expected [ 5/1 7/1 9/1 ], got: {rendered}"
        );
    }
}
