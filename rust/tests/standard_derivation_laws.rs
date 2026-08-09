//! Executable Kernel-only witnesses for the `derivable` Standard Words.
//!
//! Each pair runs the native Standard Word and a program written in the
//! Semantic Kernel alone, then compares the whole observed stack. The witness
//! establishes that the Standard adds no expressive power over the Kernel; what
//! it deliberately does not claim is that a User Word would reproduce a native
//! Word's effect count, short-circuit point, complexity, or resource ceiling —
//! that separate guarantee belongs to the `operational` Standards and lives in
//! `standard_operational_laws.rs`.

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

/// Strip `'...'` String literals so their contents are read as data, not as
/// candidate Word names.
fn without_string_literals(source: &str) -> String {
    let mut stripped = String::with_capacity(source.len());
    let mut inside = false;
    for character in source.chars() {
        match character {
            '\'' => inside = !inside,
            _ if inside => {}
            _ => stripped.push(character),
        }
    }
    stripped
}

fn assert_kernel_only(source: &str) {
    for token in without_string_literals(source).split_whitespace() {
        if token.chars().any(char::is_alphabetic) {
            let upper = token.to_ascii_uppercase();
            // `IDLE` is COND's else-marker in the guard position, not a Word:
            // it has no entry in `spec/words.json` and cannot be called.
            assert!(
                upper == "IDLE" || KERNEL_WORDS.contains(&upper.as_str()),
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
        // QUANTIZE is ROUND scaled by the denominator: round(x*d)/d. Written
        // in the Kernel the scaling is explicit, which is the point — the
        // Standard Word exists so the resolution is named once instead of
        // spelled out with a magic constant at every step of a loop.
        ("119/125 10 QUANTIZE", "119/125 10 MUL 1/2 ADD FLOOR 10 DIV"),
        ("32/125 10 QUANTIZE", "32/125 10 MUL 1/2 ADD FLOOR 10 DIV"),
        (
            "-32/125 10 QUANTIZE",
            "-32/125 NEG 10 MUL 1/2 ADD FLOOR NEG 10 DIV",
        ),
        // At d = 1 the two Words coincide, which is the boundary that makes
        // QUANTIZE a generalization rather than a second rounding rule.
        ("5/2 1 QUANTIZE", "5/2 1/2 ADD FLOOR"),
        // |x| = sqrt(x*x): exact over the rationals closed under SQRT, so the
        // witness needs no case split on the sign.
        ("-7 ABS", "-7 -7 MUL SQRT"),
        ("7 ABS", "7 7 MUL SQRT"),
        // min(a,b) = ((a+b) - |a-b|) / 2 and max(a,b) = ((a+b) + |a-b|) / 2.
        (
            "2 5 MIN",
            "2 5 ADD 2 5 NEG ADD 2 5 NEG ADD MUL SQRT NEG ADD 2 DIV",
        ),
        (
            "5 2 MIN",
            "5 2 ADD 5 2 NEG ADD 5 2 NEG ADD MUL SQRT NEG ADD 2 DIV",
        ),
        (
            "2 5 MAX",
            "2 5 ADD 2 5 NEG ADD 2 5 NEG ADD MUL SQRT ADD 2 DIV",
        ),
        (
            "5 2 MAX",
            "5 2 ADD 5 2 NEG ADD 5 2 NEG ADD MUL SQRT ADD 2 DIV",
        ),
    ] {
        equivalent(native, witness).await;
    }
}

#[tokio::test]
async fn collection_standards_have_kernel_only_witnesses() {
    for (native, witness) in [
        // TAKE is the prefix (or, for a negative count, the suffix) that GET
        // and COLLECT already reach index by index.
        (
            "[ 10 20 30 40 50 ] [ 3 ] TAKE",
            "[ 10 20 30 40 50 ] [ 0 ] GET [ 10 20 30 40 50 ] [ 1 ] GET \
             [ 10 20 30 40 50 ] [ 2 ] GET 3 COLLECT",
        ),
        (
            "[ 10 20 30 40 50 ] [ -2 ] TAKE",
            "[ 10 20 30 40 50 ] [ 3 ] GET [ 10 20 30 40 50 ] [ 4 ] GET 2 COLLECT",
        ),
        // REVERSE reads the same indices in descending order.
        (
            "[ 1 2 3 ] REVERSE",
            "[ 1 2 3 ] [ 2 ] GET [ 1 2 3 ] [ 1 ] GET [ 1 2 3 ] [ 0 ] GET 3 COLLECT",
        ),
        // INDEX-OF is COND's ordered first match over GET and EQ; the absent
        // case falls through to the IDLE clause exactly as the Word projects
        // NIL.
        (
            "[ 5 7 9 ] 7 INDEX-OF",
            "[ 5 7 9 ] { [ 0 ] GET 7 EQ } { 0 } { [ 1 ] GET 7 EQ } { 1 } \
             { [ 2 ] GET 7 EQ } { 2 } { IDLE } { NIL } COND",
        ),
        (
            "[ 5 7 9 ] 4 INDEX-OF",
            "[ 5 7 9 ] { [ 0 ] GET 4 EQ } { 0 } { [ 1 ] GET 4 EQ } { 1 } \
             { [ 2 ] GET 4 EQ } { 2 } { IDLE } { NIL } COND",
        ),
    ] {
        equivalent(native, witness).await;
    }
}

#[tokio::test]
async fn text_standards_have_kernel_only_witnesses() {
    for (native, witness) in [
        // Every text Standard is a projection of the code-point Vector that
        // CHARS exposes and JOIN closes again.
        (
            "'  hi  ' TRIM",
            "'  hi  ' CHARS [ 2 ] GET '  hi  ' CHARS [ 3 ] GET 2 COLLECT JOIN",
        ),
        (
            "'a,b,c' ',' TOKENIZE",
            "'a,b,c' CHARS [ 0 ] GET 'a,b,c' CHARS [ 2 ] GET \
             'a,b,c' CHARS [ 4 ] GET 3 COLLECT",
        ),
    ] {
        equivalent(native, witness).await;
    }
}

/// `SUM` is `0 { ADD } FOLD` — the same fold, given a name because it is the
/// phrase every inner product, mean, variance, norm and loss is written with.
/// Its Core slot is earned on frequency, not on power: the witness below is
/// exactly the four tokens it replaces.
#[tokio::test]
async fn sum_is_the_addition_fold() {
    for (native, witness) in [
        ("[ 1 2 3 4 ] SUM", "[ 1 2 3 4 ] 0 { ADD } FOLD"),
        ("[ ] SUM", "[ ] 0 { ADD } FOLD"),
        ("[ 1/2 1/3 ] SUM", "[ 1/2 1/3 ] 0 { ADD } FOLD"),
        (
            "[ [ 1 2 ] [ 3 4 ] ] SUM",
            "[ [ 1 2 ] [ 3 4 ] ] 0 { ADD } FOLD",
        ),
    ] {
        equivalent(native, witness).await;
    }
}
