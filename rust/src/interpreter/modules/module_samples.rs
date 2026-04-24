use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::error::{AjisaiError, Result};
use crate::types::{Capabilities, ExecutionLine, Stability, Tier, Token, WordDefinition};

use super::module_word_types::SampleWord;

pub(super) fn build_sample_words(
    module_name: &str,
    sample_words: &[SampleWord],
) -> Result<HashMap<String, Arc<WordDefinition>>> {
    let mut result = HashMap::new();
    for sample in sample_words {
        let tokens = crate::tokenizer::tokenize(sample.definition).map_err(|e| {
            AjisaiError::from(format!(
                "Failed to tokenize sample word '{}': {}",
                sample.name, e
            ))
        })?;
        let lines = parse_sample_definition_body(&tokens)?;
        result.insert(
            sample.name.to_uppercase(),
            Arc::new(WordDefinition {
                lines: lines.into(),
                is_builtin: false,
                tier: Tier::Standard,
                stability: Stability::Stable,
                capabilities: Capabilities::PURE,
                description: Some(sample.description.to_string()),
                dependencies: HashSet::new(),
                original_source: None,
                namespace: Some(module_name.to_string()),
                registration_order: 0,
                execution_plans: None,
            }),
        );
    }
    Ok(result)
}

fn parse_sample_definition_body(tokens: &[Token]) -> Result<Vec<ExecutionLine>> {
    let mut lines = Vec::new();
    let mut current_tokens = Vec::new();

    for token in tokens {
        match token {
            Token::LineBreak => {
                if !current_tokens.is_empty() {
                    lines.push(ExecutionLine {
                        body_tokens: current_tokens.clone().into(),
                    });
                    current_tokens.clear();
                }
            }
            _ => {
                current_tokens.push(token.clone());
            }
        }
    }

    if !current_tokens.is_empty() {
        lines.push(ExecutionLine {
            body_tokens: current_tokens.into(),
        });
    }

    if lines.is_empty() {
        return Err(AjisaiError::from("Sample word definition cannot be empty"));
    }

    Ok(lines)
}
