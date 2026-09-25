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

/// The Semantic Kernel: the 48 Words every Standard must be derivable from.
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
    "SQRT",
    "POW",
    "GET",
    "LENGTH",
    "CONCAT",
    "COLLECT",
    "RANGE",
    "FOLD",
    "MAP",
    "SHAPE",
    "RESHAPE",
    "FLATTEN",
    "DEPTH",
    "RECORD",
    "KEYS",
    "VALUES",
    "PUT",
    "WITHOUT",
    "HAS?",
    "MERGE",
    "CHARS",
    "JOIN",
    "NUM",
    "STR",
    "SELECT",
    "EXEC",
    "CONTRACT",
    "FAIL",
    "NIL",
    "NIL?",
    "NIL-REASON",
    "ABSENT",
    "BIND",
    "DEF",
    "DEL",
    "DIGEST",
    "PRINT",
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
async fn arithmetic_standards_have_kernel_only_witnesses() {
    for (native, witness) in [
        // Subtraction is adding the additive inverse, and the inverse is a
        // multiplication by -1.
        ("7 3 SUB", "7 3 -1 MUL ADD"),
        ("-7 3 SUB", "-7 3 -1 MUL ADD"),
        ("5/2 ROUND", "5/2 1/2 ADD FLOOR"),
        ("-5/2 ROUND", "-5/2 -1 MUL 1/2 ADD FLOOR -1 MUL"),
        // min(a,b) = ((a+b) - |a-b|) / 2 and max(a,b) = ((a+b) + |a-b|) / 2,
        // with |x| = sqrt(x*x): exact over the rationals closed under SQRT, so
        // the witness needs no case split on the sign.
        (
            "2 5 MIN",
            "2 5 ADD 2 5 -1 MUL ADD 2 5 -1 MUL ADD MUL SQRT -1 MUL ADD 2 DIV",
        ),
        (
            "5 2 MIN",
            "5 2 ADD 5 2 -1 MUL ADD 5 2 -1 MUL ADD MUL SQRT -1 MUL ADD 2 DIV",
        ),
        (
            "2 5 MAX",
            "2 5 ADD 2 5 -1 MUL ADD 2 5 -1 MUL ADD MUL SQRT ADD 2 DIV",
        ),
        (
            "5 2 MAX",
            "5 2 ADD 5 2 -1 MUL ADD 5 2 -1 MUL ADD MUL SQRT ADD 2 DIV",
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
            "[ 10 20 30 40 50 ] 3 TAKE",
            "[ 10 20 30 40 50 ] 0 GET [ 10 20 30 40 50 ] 1 GET \
             [ 10 20 30 40 50 ] 2 GET 3 COLLECT",
        ),
        (
            "[ 10 20 30 40 50 ] -2 TAKE",
            "[ 10 20 30 40 50 ] 3 GET [ 10 20 30 40 50 ] 4 GET 2 COLLECT",
        ),
        // DROP is the other half of the same cut.
        (
            "[ 10 20 30 40 50 ] 3 DROP",
            "[ 10 20 30 40 50 ] 3 GET [ 10 20 30 40 50 ] 4 GET 2 COLLECT",
        ),
        (
            "[ 10 20 30 40 50 ] -2 DROP",
            "[ 10 20 30 40 50 ] 0 GET [ 10 20 30 40 50 ] 1 GET \
             [ 10 20 30 40 50 ] 2 GET 3 COLLECT",
        ),
        // REVERSE reads the same indices in descending order.
        (
            "[ 1 2 3 ] REVERSE",
            "[ 1 2 3 ] 2 GET [ 1 2 3 ] 1 GET [ 1 2 3 ] 0 GET 3 COLLECT",
        ),
        // INDEX-OF is a first match over GET and EQ, which is a chain of
        // SELECTs: each one answers its own index or defers to the rest, and
        // the innermost `NIL` is what an exhausted chain answers — the same
        // projection the Word makes. The chain is written outermost-first
        // because SELECT takes its candidates before the truth that chooses.
        (
            "[ 5 7 9 ] 7 INDEX-OF",
            "0 1 2 NIL [ 5 7 9 ] 2 GET 7 EQ SELECT \
             [ 5 7 9 ] 1 GET 7 EQ SELECT [ 5 7 9 ] 0 GET 7 EQ SELECT",
        ),
        (
            "[ 5 7 9 ] 4 INDEX-OF",
            "0 1 2 NIL [ 5 7 9 ] 2 GET 4 EQ SELECT \
             [ 5 7 9 ] 1 GET 4 EQ SELECT [ 5 7 9 ] 0 GET 4 EQ SELECT",
        ),
        // MEMBER? is the same chain answering TRUE instead of an index, with
        // FALSE where INDEX-OF's chain ends in NIL.
        (
            "[ 5 7 9 ] 7 MEMBER?",
            "TRUE TRUE TRUE FALSE [ 5 7 9 ] 2 GET 7 EQ SELECT \
             [ 5 7 9 ] 1 GET 7 EQ SELECT [ 5 7 9 ] 0 GET 7 EQ SELECT",
        ),
        (
            "[ 5 7 9 ] 4 MEMBER?",
            "TRUE TRUE TRUE FALSE [ 5 7 9 ] 2 GET 4 EQ SELECT \
             [ 5 7 9 ] 1 GET 4 EQ SELECT [ 5 7 9 ] 0 GET 4 EQ SELECT",
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
            "'  hi  ' CHARS 2 GET '  hi  ' CHARS 3 GET 2 COLLECT JOIN",
        ),
        (
            "'a,b,c' ',' TOKENIZE",
            "'a,b,c' CHARS 0 GET 'a,b,c' CHARS 2 GET \
             'a,b,c' CHARS 4 GET 3 COLLECT",
        ),
    ] {
        equivalent(native, witness).await;
    }
}
