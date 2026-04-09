use crate::builtins::{lookup_builtin_spec, BuiltinExecutorKey};
use crate::error::{AjisaiError, Result};
use crate::types::fraction::Fraction;
use crate::types::{DisplayHint, FlowToken, Token, Value};

use super::{
    arithmetic, cast, comparison, control, control_cond, datetime, execute_def, execute_del,
    execute_lookup, hash, higher_order, higher_order_fold, io, logic, modules, random, sort,
    tensor_cmds, vector_ops, Interpreter,
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

        self.execution_step_count += 1;
        if self.execution_step_count > self.max_execution_steps {
            return Err(AjisaiError::ExecutionLimitExceeded {
                limit: self.max_execution_steps,
            });
        }

        if def.lines.is_empty() {
            return self.execute_builtin(&resolved_name);
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
        if let Some(spec) = lookup_builtin_spec(name) {
            if let Some(executor_key) = spec.executor_key {
                return self.execute_builtin_by_key(executor_key);
            }
        }

        modules::execute_module_word(self, name)
            .unwrap_or_else(|| Err(AjisaiError::UnknownWord(name.to_string())))
    }

    fn execute_builtin_by_key(&mut self, key: BuiltinExecutorKey) -> Result<()> {
        match key {
            BuiltinExecutorKey::Add => arithmetic::op_add(self),
            BuiltinExecutorKey::Sub => arithmetic::op_sub(self),
            BuiltinExecutorKey::Mul => arithmetic::op_mul(self),
            BuiltinExecutorKey::Div => arithmetic::op_div(self),
            BuiltinExecutorKey::Eq => comparison::op_eq(self),
            BuiltinExecutorKey::Lt => comparison::op_lt(self),
            BuiltinExecutorKey::Le => comparison::op_le(self),
            BuiltinExecutorKey::Map => higher_order::op_map(self),
            BuiltinExecutorKey::Filter => higher_order::op_filter(self),
            BuiltinExecutorKey::Fold => higher_order_fold::op_fold(self),
            BuiltinExecutorKey::Unfold => higher_order_fold::op_unfold(self),
            BuiltinExecutorKey::Any => higher_order::op_any(self),
            BuiltinExecutorKey::All => higher_order::op_all(self),
            BuiltinExecutorKey::Count => higher_order::op_count(self),
            BuiltinExecutorKey::Scan => higher_order_fold::op_scan(self),
            BuiltinExecutorKey::Get => vector_ops::op_get(self),
            BuiltinExecutorKey::Length => vector_ops::op_length(self),
            BuiltinExecutorKey::Concat => vector_ops::op_concat(self),
            BuiltinExecutorKey::And => logic::op_and(self),
            BuiltinExecutorKey::Or => logic::op_or(self),
            BuiltinExecutorKey::Not => logic::op_not(self),
            BuiltinExecutorKey::True => {
                self.stack.push(Value::from_bool(true));
                self.semantic_registry.push_hint(DisplayHint::Boolean);
                Ok(())
            }
            BuiltinExecutorKey::False => {
                self.stack.push(Value::from_bool(false));
                self.semantic_registry.push_hint(DisplayHint::Boolean);
                Ok(())
            }
            BuiltinExecutorKey::Nil => {
                self.stack.push(Value::nil());
                self.semantic_registry.push_hint(DisplayHint::Nil);
                Ok(())
            }
            BuiltinExecutorKey::Idle => Ok(()),
            BuiltinExecutorKey::Exec => control::op_exec(self),
            BuiltinExecutorKey::Eval => control::op_eval(self),
            BuiltinExecutorKey::Cond => control_cond::op_cond(self),
            BuiltinExecutorKey::Def => execute_def::op_def(self),
            BuiltinExecutorKey::Del => execute_del::op_del(self),
            BuiltinExecutorKey::Lookup => execute_lookup::op_lookup(self),
            BuiltinExecutorKey::Import => modules::op_import(self),
            BuiltinExecutorKey::ImportOnly => modules::op_import_only(self),
            BuiltinExecutorKey::Force => {
                self.force_flag = true;
                Ok(())
            }
            BuiltinExecutorKey::Print => io::op_print(self),
            BuiltinExecutorKey::Insert => vector_ops::op_insert(self),
            BuiltinExecutorKey::Replace => vector_ops::op_replace(self),
            BuiltinExecutorKey::Remove => vector_ops::op_remove(self),
            BuiltinExecutorKey::Take => vector_ops::op_take(self),
            BuiltinExecutorKey::Split => vector_ops::op_split(self),
            BuiltinExecutorKey::Reverse => vector_ops::op_reverse(self),
            BuiltinExecutorKey::Range => vector_ops::op_range(self),
            BuiltinExecutorKey::Reorder => vector_ops::op_reorder(self),
            BuiltinExecutorKey::Collect => vector_ops::op_collect(self),
            BuiltinExecutorKey::Sort => sort::op_sort(self),
            BuiltinExecutorKey::Shape => tensor_cmds::op_shape(self),
            BuiltinExecutorKey::Rank => tensor_cmds::op_rank(self),
            BuiltinExecutorKey::Reshape => tensor_cmds::op_reshape(self),
            BuiltinExecutorKey::Transpose => tensor_cmds::op_transpose(self),
            BuiltinExecutorKey::Fill => tensor_cmds::op_fill(self),
            BuiltinExecutorKey::Floor => tensor_cmds::op_floor(self),
            BuiltinExecutorKey::Ceil => tensor_cmds::op_ceil(self),
            BuiltinExecutorKey::Round => tensor_cmds::op_round(self),
            BuiltinExecutorKey::Mod => tensor_cmds::op_mod(self),
            BuiltinExecutorKey::Str => cast::op_str(self),
            BuiltinExecutorKey::Num => cast::op_num(self),
            BuiltinExecutorKey::Bool => cast::op_bool(self),
            BuiltinExecutorKey::Chr => cast::op_chr(self),
            BuiltinExecutorKey::Chars => cast::op_chars(self),
            BuiltinExecutorKey::Join => cast::op_join(self),
            BuiltinExecutorKey::Now => datetime::op_now(self),
            BuiltinExecutorKey::Datetime => datetime::op_datetime(self),
            BuiltinExecutorKey::Timestamp => datetime::op_timestamp(self),
            BuiltinExecutorKey::Csprng => random::op_csprng(self),
            BuiltinExecutorKey::Hash => hash::op_hash(self),
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
