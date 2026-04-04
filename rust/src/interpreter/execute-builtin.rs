use crate::error::{AjisaiError, Result};
use crate::types::fraction::Fraction;
use crate::types::{DisplayHint, FlowToken, Token, Value};

use super::interpreter_core::MAX_CALL_DEPTH;
use super::{
    arithmetic, cast, comparison, control, control_cond, datetime, execute_def, execute_del,
    execute_lookup, hash, higher_order, higher_order_fold, io, logic, modules, random, sort, tensor_cmds,
    tensor_ops, vector_ops, Interpreter,
};

impl Interpreter {
    pub(crate) fn execute_word_core(&mut self, name: &str) -> Result<()> {
        let (resolved_name, def) = self
            .resolve_word_entry(name)
            .ok_or_else(|| {
                let ambiguous = self.check_ambiguity(name);
                if !ambiguous.is_empty() {
                    AjisaiError::from(format!(
                        "Ambiguous word '{}': found in {}. Use a qualified path to specify which one you mean.",
                        name.to_uppercase(), ambiguous.join(", ")
                    ))
                } else {
                    AjisaiError::UnknownWord(name.to_string())
                }
            })?;

        if def.lines.is_empty() {
            return self.execute_builtin(&resolved_name);
        }

        if self.call_stack.len() >= MAX_CALL_DEPTH {
            let chain = format!("{} -> {}", self.call_stack.join(" -> "), resolved_name);
            return Err(AjisaiError::DepthLimitExceeded {
                depth: MAX_CALL_DEPTH,
                chain,
            });
        }

        self.call_stack.push(resolved_name.clone());
        let result = self.execute_guard_structure(&def.lines);
        self.call_stack.pop();
        result
    }

    pub(crate) fn execute_builtin(&mut self, name: &str) -> Result<()> {
        if name != "DEL" && name != "DEF" && name != "!" {
            self.force_flag = false;
        }

        let pre_snapshot = if self.flow_tracking {
            Some(self.collect_stack_totals_snapshot())
        } else {
            None
        };

        let result = self.execute_builtin_with_conservation(name);

        if let Some(pre) = pre_snapshot {
            if result.is_ok() {
                let post = self.collect_stack_totals_snapshot();
                let _delta = post.sub(&pre);
            }
        }

        result
    }

    pub(crate) fn collect_stack_totals_snapshot(&self) -> Fraction {
        let mut total = Fraction::from(0);
        for val in &self.stack {
            let token = FlowToken::from_value(val);
            total = total.add(&token.total);
        }
        total
    }

    pub(crate) fn execute_builtin_with_conservation(&mut self, name: &str) -> Result<()> {
        match name {
            "+" => arithmetic::op_add(self),
            "-" => arithmetic::op_sub(self),
            "*" => arithmetic::op_mul(self),
            "/" => arithmetic::op_div(self),
            "=" => comparison::op_eq(self),
            "<" => comparison::op_lt(self),
            "<=" => comparison::op_le(self),
            "MAP" => higher_order::op_map(self),
            "FILTER" => higher_order::op_filter(self),
            "FOLD" => higher_order_fold::op_fold(self),
            "GET" => vector_ops::op_get(self),
            "LENGTH" => vector_ops::op_length(self),
            "CONCAT" => vector_ops::op_concat(self),
            "AND" => logic::op_and(self),
            "OR" => logic::op_or(self),
            "NOT" => logic::op_not(self),
            "TRUE" => {
                self.stack.push(Value::from_bool(true));
                self.semantic_registry.push_hint(DisplayHint::Boolean);
                Ok(())
            }
            "FALSE" => {
                self.stack.push(Value::from_bool(false));
                self.semantic_registry.push_hint(DisplayHint::Boolean);
                Ok(())
            }
            "NIL" => {
                self.stack.push(Value::nil());
                self.semantic_registry.push_hint(DisplayHint::Nil);
                Ok(())
            }
            "IDLE" => Ok(()),
            "EXEC" => control::op_exec(self),
            "EVAL" => control::op_eval(self),
            "COND" => control_cond::op_cond(self),
            "DEF" => execute_def::op_def(self),
            "DEL" => execute_del::op_del(self),
            "?" => execute_lookup::op_lookup(self),
            "IMPORT" => modules::op_import(self),
            "!" => {
                self.force_flag = true;
                Ok(())
            }
            "PRINT" => io::op_print(self),
            "INSERT" => vector_ops::op_insert(self),
            "REPLACE" => vector_ops::op_replace(self),
            "REMOVE" => vector_ops::op_remove(self),
            "TAKE" => vector_ops::op_take(self),
            "SPLIT" => vector_ops::op_split(self),
            "REVERSE" => vector_ops::op_reverse(self),
            "RANGE" => vector_ops::op_range(self),
            "REORDER" => vector_ops::op_reorder(self),
            "COLLECT" => vector_ops::op_collect(self),
            "SORT" => sort::op_sort(self),
            "SHAPE" => tensor_cmds::op_shape(self),
            "RANK" => tensor_cmds::op_rank(self),
            "RESHAPE" => tensor_cmds::op_reshape(self),
            "TRANSPOSE" => tensor_cmds::op_transpose(self),
            "FILL" => tensor_cmds::op_fill(self),
            "FLOOR" => tensor_cmds::op_floor(self),
            "CEIL" => tensor_cmds::op_ceil(self),
            "ROUND" => tensor_cmds::op_round(self),
            "MOD" => tensor_cmds::op_mod(self),
            "STR" => cast::op_str(self),
            "NUM" => cast::op_num(self),
            "BOOL" => cast::op_bool(self),
            "CHR" => cast::op_chr(self),
            "CHARS" => cast::op_chars(self),
            "JOIN" => cast::op_join(self),
            "NOW" => datetime::op_now(self),
            "DATETIME" => datetime::op_datetime(self),
            "TIMESTAMP" => datetime::op_timestamp(self),
            "CSPRNG" => random::op_csprng(self),
            "HASH" => hash::op_hash(self),
            _ => modules::execute_module_word(self, name)
                .unwrap_or_else(|| Err(AjisaiError::UnknownWord(name.to_string()))),
        }
    }

    pub(crate) fn format_token_to_string(&self, token: &Token) -> String {
        match token {
            Token::Number(n) => n.to_string(),
            Token::String(s) => format!("'{}'", s),
            Token::Symbol(s) => s.to_string(),
            Token::VectorStart => "[".to_string(),
            Token::VectorEnd => "]".to_string(),
            Token::BlockStart => "{".to_string(),
            Token::BlockEnd => "}".to_string(),
            Token::Pipeline => "==".to_string(),
            Token::NilCoalesce => "=>".to_string(),
            Token::SafeMode => "~".to_string(),
            Token::LineBreak => "\n".to_string(),
        }
    }

    pub fn lookup_word_definition_tokens(&self, name: &str) -> Option<String> {
        let (_, def) = self.resolve_word_entry(name)?;
        if def.is_builtin || def.lines.is_empty() {
            return None;
        }

        let mut result = String::new();
        for (i, line) in def.lines.iter().enumerate() {
            if i > 0 {
                result.push('\n');
            }
            for token in line.body_tokens.iter() {
                result.push_str(&self.format_token_to_string(token));
                result.push(' ');
            }
        }
        Some(result.trim().to_string())
    }
}
