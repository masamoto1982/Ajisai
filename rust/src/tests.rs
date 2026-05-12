//! DO-178B style requirement-based tests.
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
//!  * REQ-REG-001: STORE moves the top of the stack into the Register.
//!  * REQ-REG-002: RECALL pushes the Register and resets it to Nil.
//!  * REQ-REG-003: PEEK pushes a copy of the Register without clearing it.
//!  * REQ-REG-004: Sugar `>R` / `R>` / `R@` resolve to the same primitives.
//!  * REQ-REG-005: RESET clears Register together with the stack.
//!  * REQ-CMP-001: EQ/NE/LT/LE/GT/GE compare exact rationals correctly.
//!  * REQ-CMP-002: Symbolic comparison sugar (`=`, `<>`, `<`, `<=`, `>`, `>=`) work.
//!  * REQ-CMP-003: Comparisons involving Nil yield Nil.
//!  * REQ-LOG-001: AND/OR/NOT realise Kleene K3 three-valued logic.
//!  * REQ-LOG-002: Symbolic logic sugar (`&`, `|`, `!`) work.
//!  * REQ-STR-001: `'TEXT'` lexes to a single StringLit token.
//!  * REQ-STR-002: Executing a string literal pushes a rank-1 tensor of
//!    UTF-8 bytes with `display_hint = "string"`.
//!  * REQ-STR-003: `.` prints a string literal as `'TEXT'`.
//!  * REQ-STR-004: Strings round-trip multibyte UTF-8 content (e.g. ひらがな).
//!  * REQ-STR-005: An unterminated string literal raises a three-layer error.

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
        other => panic!("expected Number, got {:?}", other),
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
    let toks = tokenize("42 3/4 3.14 FOO").expect("tokenize");
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
    let toks = tokenize("1 # comment ignored\n 2 +").expect("tokenize");
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

#[test]
fn req_reg_001_store_moves_top_to_register() {
    let i = run("42 STORE");
    assert_eq!(i.stack().len(), 0);
    assert_eq!(ratio(i.register()), (BigInt::from(42), BigInt::from(1)));
}

#[test]
fn req_reg_002_recall_pushes_and_clears() {
    let i = run("42 STORE RECALL");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(42), BigInt::from(1)));
    assert!(i.register().is_nil());
}

#[test]
fn req_reg_003_peek_does_not_clear() {
    let i = run("42 STORE PEEK PEEK");
    assert_eq!(i.stack().len(), 2);
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(42), BigInt::from(1)));
    assert_eq!(ratio(&i.stack()[1]), (BigInt::from(42), BigInt::from(1)));
    assert_eq!(ratio(i.register()), (BigInt::from(42), BigInt::from(1)));
}

#[test]
fn req_reg_004_sugar_resolves_to_primitives() {
    let i = run("7 >R R@ R>");
    // 7 >R puts 7 into register; R@ copies it to stack; R> pulls it.
    // Stack should be [7 (peeked), 7 (recalled)].
    assert_eq!(i.stack().len(), 2);
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(7), BigInt::from(1)));
    assert_eq!(ratio(&i.stack()[1]), (BigInt::from(7), BigInt::from(1)));
    assert!(i.register().is_nil());
}

#[test]
fn req_reg_005_reset_clears_register() {
    let mut i = Interpreter::new();
    i.execute("9 STORE 1 2 3").unwrap();
    assert_eq!(i.stack().len(), 3);
    assert_eq!(ratio(i.register()), (BigInt::from(9), BigInt::from(1)));
    i.reset();
    assert_eq!(i.stack().len(), 0);
    assert!(i.register().is_nil());
}

#[test]
fn req_cmp_001_exact_rational_comparison() {
    let i = run("1/3 1/6 GT");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
    let i = run("1/3 2/6 EQ");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
    let i = run("1 2 LT");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
    let i = run("3 3 LE");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
    let i = run("3 4 GE");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(0), BigInt::from(1)));
    let i = run("3 4 NE");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
}

#[test]
fn req_cmp_002_symbolic_comparison_sugar() {
    let i = run("1 1 =");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
    let i = run("1 2 <>");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
    let i = run("1 2 <");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
    let i = run("2 2 <=");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
    let i = run("2 1 >");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
    let i = run("3 3 >=");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
}

#[test]
fn req_cmp_002b_symmetric_directions_are_consistent() {
    // `a b LT` and `b a GT` must agree across all rationals tested.
    let i = run("1 2 LT 2 1 GT EQ");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
    let i = run("3 3 LE 3 3 GE EQ");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
}

#[test]
fn req_cmp_003_nil_in_comparison_yields_nil() {
    let i = run("NIL 1 EQ");
    assert!(matches!(i.stack()[0], Value::Nil));
    let i = run("1 NIL LT");
    assert!(matches!(i.stack()[0], Value::Nil));
}

#[test]
fn req_log_001_kleene_three_valued_logic() {
    // AND
    let i = run("1 1 AND");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
    let i = run("1 0 AND");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(0), BigInt::from(1)));
    let i = run("0 NIL AND"); // False dominates Nil
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(0), BigInt::from(1)));
    let i = run("1 NIL AND"); // Nil dominates True
    assert!(matches!(i.stack()[0], Value::Nil));

    // OR
    let i = run("0 0 OR");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(0), BigInt::from(1)));
    let i = run("1 NIL OR"); // True dominates Nil
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
    let i = run("0 NIL OR"); // Nil dominates False
    assert!(matches!(i.stack()[0], Value::Nil));

    // NOT
    let i = run("1 NOT");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(0), BigInt::from(1)));
    let i = run("0 NOT");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
    let i = run("NIL NOT");
    assert!(matches!(i.stack()[0], Value::Nil));
}

#[test]
fn req_log_002_symbolic_logic_sugar() {
    let i = run("1 1 &");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
    let i = run("0 1 |");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
    let i = run("0 !");
    assert_eq!(ratio(&i.stack()[0]), (BigInt::from(1), BigInt::from(1)));
}

#[test]
fn req_str_001_string_literal_lexes_as_one_token() {
    let toks = tokenize("'TEST'").expect("tokenize");
    assert_eq!(toks, vec![Token::StringLit("TEST".into())]);

    let toks = tokenize("'hello world' DROP").expect("tokenize");
    assert_eq!(
        toks,
        vec![
            Token::StringLit("hello world".into()),
            Token::Symbol("DROP".into()),
        ]
    );

    // Apostrophes inside a word remain part of that symbol.
    let toks = tokenize("O'Brien").expect("tokenize");
    assert_eq!(toks, vec![Token::Symbol("O'Brien".into())]);
}

#[test]
fn req_str_002_string_literal_pushes_byte_tensor() {
    let i = run("'TEST'");
    assert_eq!(i.stack().len(), 1);
    match &i.stack()[0] {
        Value::Tensor { shape, data, display_hint } => {
            assert_eq!(shape, &vec![4]);
            assert_eq!(display_hint.as_deref(), Some("string"));
            let bytes: Vec<u8> = data
                .iter()
                .map(|cf| {
                    let (p, _) = cf.to_ratio().unwrap();
                    use num_traits::ToPrimitive;
                    p.to_u8().unwrap()
                })
                .collect();
            assert_eq!(bytes, b"TEST".to_vec());
        }
        other => panic!("expected Tensor, got {:?}", other),
    }
}

#[test]
fn req_str_003_dot_prints_string_in_quotes() {
    let mut i = Interpreter::new();
    i.execute("'TEST' .").unwrap();
    assert_eq!(i.take_output(), "'TEST'");
}

#[test]
fn req_str_004_multibyte_utf8_round_trip() {
    let mut i = Interpreter::new();
    i.execute("'こんにちは' .").unwrap();
    assert_eq!(i.take_output(), "'こんにちは'");
}

#[test]
fn req_str_005_unterminated_string_raises_three_layer_error() {
    let mut i = Interpreter::new();
    let err = i.execute("'unfinished").unwrap_err();
    assert!(err.summary.contains("Unterminated string"));
    assert!(!err.detail.is_empty());
    assert!(!err.diagnosis.is_empty());
}
