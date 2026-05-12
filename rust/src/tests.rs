//! DO-178B style requirement-based tests for Phase 1.
//!
//! Each test names the requirement it covers. Coverage targets:
//!  * REQ-CF-001: integer → CF round-trip preserves value.
//!  * REQ-CF-002: rational → CF round-trip preserves reduced ratio.
//!  * REQ-CF-003: decimal literal → CF round-trip preserves value.
//!  * REQ-CF-004: nested display matches `(a0 (a1 (a2)))` form.
//!  * REQ-INT-001: ADD/SUB/MUL/DIV operate on continued fractions exactly.
//!  * REQ-INT-002: stack underflow returns three-layer error.
//!  * REQ-INT-003: Nil propagates through arithmetic.
//!  * REQ-INT-004: DEF / DEL register and remove user words.
//!  * REQ-INT-005: `.` prints rational form to the output buffer.
//!  * REQ-TOK-001: tokenizer classifies integer / fraction / decimal / symbol.
//!  * REQ-TOK-002: comments starting with `#` are skipped to end-of-line.

use num_bigint::BigInt;

use crate::cf::{self, ContinuedFraction};
use crate::interpreter::Interpreter;
use crate::tokenizer::{tokenize, Token};
use crate::value::Value;

fn run(code: &str) -> Interpreter {
    let mut i = Interpreter::new();
    i.execute(code).expect("execution should succeed");
    i
}

fn ratio(v: &Value) -> (BigInt, BigInt) {
    match v {
        Value::Number(cf) => cf.to_ratio().expect("expected rational"),
        Value::Nil => panic!("expected Number, got Nil"),
    }
}

#[test]
fn req_cf_001_integer_round_trip() {
    let cf = ContinuedFraction::from_int(BigInt::from(42));
    let (p, q) = cf.to_ratio().unwrap();
    assert_eq!(p, BigInt::from(42));
    assert_eq!(q, BigInt::from(1));
    assert_eq!(cf.nested_display(), "(42)");
}

#[test]
fn req_cf_002_rational_round_trip() {
    let cf = ContinuedFraction::from_ratio(BigInt::from(355), BigInt::from(113));
    let (p, q) = cf.to_ratio().unwrap();
    assert_eq!(p, BigInt::from(355));
    assert_eq!(q, BigInt::from(113));
    // 355/113 = 3 + 1/(7 + 1/16)
    assert_eq!(cf.nested_display(), "(3 (7 (16)))");
}

#[test]
fn req_cf_003_decimal_round_trip() {
    let cf = ContinuedFraction::from_decimal_str("3.25").unwrap();
    let (p, q) = cf.to_ratio().unwrap();
    assert_eq!(p, BigInt::from(13));
    assert_eq!(q, BigInt::from(4));
}

#[test]
fn req_cf_004_nested_display_form() {
    let cf = ContinuedFraction::from_ratio(BigInt::from(13), BigInt::from(4));
    assert_eq!(cf.nested_display(), "(3 (4))");
}

#[test]
fn req_int_001_arithmetic_is_exact() {
    let i = run("1/3 1/6 +");
    let stack = i.stack();
    assert_eq!(stack.len(), 1);
    let (p, q) = ratio(&stack[0]);
    assert_eq!(p, BigInt::from(1));
    assert_eq!(q, BigInt::from(2));

    let i = run("7 3 -");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(4), BigInt::from(1)));

    let i = run("3/4 2 *");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(3), BigInt::from(2)));

    let i = run("1 3 /");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(3)));
}

#[test]
fn req_int_002_stack_underflow_is_three_layer() {
    let mut i = Interpreter::new();
    let err = i.execute("+").unwrap_err();
    assert!(err.summary.contains("Stack underflow"));
    assert!(err.detail.contains("requires 2"));
    assert!(!err.diagnosis.is_empty());
}

#[test]
fn req_int_003_nil_propagates() {
    let i = run("NIL 1 +");
    assert!(matches!(i.stack()[0], Value::Nil));

    let i = run("1 0 /");
    assert!(matches!(i.stack()[0], Value::Nil));
}

#[test]
fn req_int_004_def_and_del_register_and_remove() {
    let mut i = Interpreter::new();
    i.execute("DEF DOUBLE DUP +").unwrap();
    assert_eq!(i.user_words().count(), 1);
    i.execute("21 DOUBLE").unwrap();
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(42), BigInt::from(1)));
    i.execute("DEL DOUBLE").unwrap();
    assert_eq!(i.user_words().count(), 0);
}

#[test]
fn req_int_005_dot_writes_rational_to_output() {
    let mut i = Interpreter::new();
    i.execute("3 4 / .").unwrap();
    assert_eq!(i.take_output(), "3/4");
}

#[test]
fn req_tok_001_classifies_tokens() {
    let toks = tokenize("42 3/4 3.14 FOO");
    assert_eq!(
        toks,
        vec![
            Token::Integer("42".into()),
            Token::Fraction("3".into(), "4".into()),
            Token::Decimal("3.14".into()),
            Token::Symbol("FOO".into()),
        ]
    );
}

#[test]
fn req_tok_002_skips_line_comments() {
    let toks = tokenize("1 # comment ignored\n 2 +");
    assert_eq!(
        toks,
        vec![
            Token::Integer("1".into()),
            Token::Integer("2".into()),
            Token::Symbol("+".into()),
        ]
    );
}

#[test]
fn cf_add_function_is_exact_on_thirds_and_sixths() {
    let a = ContinuedFraction::from_ratio(BigInt::from(1), BigInt::from(3));
    let b = ContinuedFraction::from_ratio(BigInt::from(1), BigInt::from(6));
    let c = cf::add(&a, &b);
    assert_eq!(c.to_ratio().unwrap(), (BigInt::from(1), BigInt::from(2)));
}
