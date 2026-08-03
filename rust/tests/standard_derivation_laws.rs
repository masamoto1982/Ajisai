//! Executable Kernel-only witnesses for the truth and exact-arithmetic Standards.

use ajisai_core::interpreter::Interpreter;

const KERNEL_WORDS: &[&str] = &[
    "TRUE",
    "FALSE",
    "AND",
    "NOT",
    "EQ",
    "LT",
    "GT",
    "ADD",
    "MUL",
    "DIV",
    "FLOOR",
    "NEG",
    "SQRT",
    "GET",
    "LENGTH",
    "CONCAT",
    "COLLECT",
    "RANGE",
    "FOLD",
    "CHARS",
    "JOIN",
    "NUM",
    "STR",
    "COND",
    "EXEC",
    "NIL",
    "NIL?",
    "NIL-REASON",
    "VENT",
    "KEEP",
    "DEF",
    "DEL",
    "LOOKUP",
    "PRINT",
    "REFLECT",
];

async fn observe(source: &str) -> Vec<String> {
    let mut interpreter = Interpreter::new();
    interpreter
        .execute(source)
        .await
        .unwrap_or_else(|error| panic!("witness failed: {source}: {error}"));
    interpreter
        .get_stack()
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn assert_kernel_only(source: &str) {
    for token in source.split_whitespace() {
        if token.chars().any(char::is_alphabetic) {
            let upper = token.to_ascii_uppercase();
            assert!(
                KERNEL_WORDS.contains(&upper.as_str()),
                "non-Kernel Word {upper} in witness: {source}"
            );
        }
    }
}

async fn equivalent(native: &str, witness: &str) {
    assert_kernel_only(witness);
    assert_eq!(observe(native).await, observe(witness).await, "{native}");
}

#[tokio::test]
async fn truth_standards_have_kernel_only_witnesses() {
    for (native, witness) in [
        ("TRUE FALSE OR", "TRUE NOT FALSE NOT AND NOT"),
        ("FALSE FALSE OR", "FALSE NOT FALSE NOT AND NOT"),
        ("2 3 NEQ", "2 3 EQ NOT"),
        ("2 2 NEQ", "2 2 EQ NOT"),
        ("2 3 LTE", "2 3 GT NOT"),
        ("3 2 LTE", "3 2 GT NOT"),
        ("3 2 GTE", "3 2 LT NOT"),
        ("2 3 GTE", "2 3 LT NOT"),
    ] {
        equivalent(native, witness).await;
    }
}

#[tokio::test]
async fn arithmetic_standards_have_kernel_only_witnesses() {
    for (native, witness) in [
        ("7 3 SUB", "7 3 NEG ADD"),
        ("-7 3 SUB", "-7 3 NEG ADD"),
        ("7 3 MOD", "7 3 DIV FLOOR 3 MUL NEG 7 ADD"),
        ("-7 3 MOD", "-7 3 DIV FLOOR 3 MUL NEG -7 ADD"),
        ("5/2 ROUND", "5/2 1/2 ADD FLOOR"),
        ("-5/2 ROUND", "-5/2 NEG 1/2 ADD FLOOR NEG"),
        ("-7 ABS", "-7 NEG"),
        ("7 ABS", "7"),
        ("2 5 MIN", "2"),
        ("5 2 MIN", "2"),
        ("2 5 MAX", "5"),
        ("5 2 MAX", "5"),
    ] {
        equivalent(native, witness).await;
    }
}
