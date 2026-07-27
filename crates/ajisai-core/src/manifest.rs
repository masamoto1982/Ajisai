//! The vocabulary manifest.
//!
//! Word contracts are machine-readable, and this is where they become
//! machine-readable *text*. The manifest is generated from the live registry
//! on demand, so it cannot drift from the implementation: there is no checked-
//! in copy to fall out of date, and no generator to re-run.
//!
//! JSON is written by hand rather than pulled in as a dependency. The shape is
//! small and fixed, and Ajisai Core's dependency list is worth more than the
//! twenty lines saved.

use crate::alias;
use crate::contract::{Arity, Effect, Word};
use crate::interpreter::Interpreter;

/// The whole vocabulary as JSON.
pub fn vocabulary_json(interpreter: &Interpreter) -> String {
    let words = interpreter.contracts();
    let mut out = String::from("{\n  \"words\": [\n");
    for (index, word) in words.iter().enumerate() {
        out.push_str(&word_json(word));
        if index + 1 < words.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}\n");
    out
}

fn word_json(word: &Word) -> String {
    let contract = &word.contract;
    let aliases = alias::aliases_for(contract.name);
    let (arity_in, arity_out) = match contract.arity {
        Arity::Fixed { inn, out } => (inn.to_string(), out.to_string()),
        Arity::Dynamic => ("null".to_string(), "null".to_string()),
    };
    format!(
        concat!(
            "    {{\n",
            "      \"name\": {},\n",
            "      \"package\": {},\n",
            "      \"aliases\": [{}],\n",
            "      \"stack_effect\": {},\n",
            "      \"arity_in\": {},\n",
            "      \"arity_out\": {},\n",
            "      \"input_types\": [{}],\n",
            "      \"output_types\": [{}],\n",
            "      \"rejects_nil\": {},\n",
            "      \"may_produce_nil\": {},\n",
            "      \"rejects_unknown\": {},\n",
            "      \"may_produce_unknown\": {},\n",
            "      \"effect\": {},\n",
            "      \"summary\": {}\n",
            "    }}"
        ),
        quote(contract.name),
        quote(word.package),
        aliases
            .iter()
            .map(|a| quote(a))
            .collect::<Vec<_>>()
            .join(", "),
        quote(contract.stack_effect),
        arity_in,
        arity_out,
        contract
            .input_types
            .iter()
            .map(|t| quote(&t.to_string()))
            .collect::<Vec<_>>()
            .join(", "),
        contract
            .output_types
            .iter()
            .map(|t| quote(&t.to_string()))
            .collect::<Vec<_>>()
            .join(", "),
        contract.nil_policy.rejects,
        contract.nil_policy.may_produce,
        contract.unknown_policy.rejects,
        contract.unknown_policy.may_produce,
        quote(match contract.effect {
            Effect::Pure => "pure",
            Effect::Dictionary => "dictionary",
        }),
        quote(contract.summary),
    )
}

fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}
