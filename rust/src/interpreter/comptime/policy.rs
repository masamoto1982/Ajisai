use crate::error::{AjisaiError, Result};
use crate::interpreter::Interpreter;
use crate::types::{Capabilities, Token};
use std::collections::HashSet;

pub(crate) fn assert_comptime_safe_tokens(
    interp: &mut Interpreter,
    tokens: &[Token],
    visiting: &mut HashSet<String>,
) -> Result<()> {
    for token in tokens {
        if let Token::Symbol(name) = token {
            if name.eq_ignore_ascii_case("PRECOMPUTE") {
                return Err(AjisaiError::from(
                    "PRECOMPUTE rejected: nested PRECOMPUTE is not supported in Phase 1",
                ));
            }

            let Some((resolved_name, def)) = interp.resolve_word_entry(name) else {
                return Err(AjisaiError::from(format!(
                    "PRECOMPUTE rejected: unresolved word {}",
                    name
                )));
            };

            if def.is_builtin {
                if !def.capabilities.contains(Capabilities::PURE)
                    || def.capabilities.contains(Capabilities::EVAL)
                    || def.capabilities.contains(Capabilities::IO)
                    || def.capabilities.contains(Capabilities::SPAWN)
                    || def.capabilities.contains(Capabilities::MUTATES_DICT)
                {
                    return Err(AjisaiError::from(format!(
                        "PRECOMPUTE rejected: word {} is not comptime-safe",
                        name
                    )));
                }
            } else {
                if visiting.contains(&resolved_name) {
                    return Err(AjisaiError::from(format!(
                        "PRECOMPUTE rejected: recursive dependency detected at {}",
                        resolved_name
                    )));
                }
                visiting.insert(resolved_name.clone());
                for line in def.lines.iter() {
                    assert_comptime_safe_tokens(interp, &line.body_tokens, visiting)?;
                }
                visiting.remove(&resolved_name);
            }
        }
    }

    Ok(())
}
