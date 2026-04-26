use crate::interpreter::Interpreter;

async fn assert_same_stack(left_code: &str, right_code: &str) {
    let mut left = Interpreter::new();
    left.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
    left.execute(left_code).await.unwrap();

    let mut right = Interpreter::new();
    right.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
    right.execute(right_code).await.unwrap();

    assert_eq!(left.get_stack(), right.get_stack());
}

async fn assert_def_rejected(name: &str) {
    let mut interp = Interpreter::new();
    interp.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
    let code = format!("{{ [ 1 ] }} '{}' DEF", name);
    let result = interp.execute(&code).await;
    assert!(result.is_err(), "expected DEF rejection for {}", name);
}

#[tokio::test]
async fn symbol_aliases_execute_same_as_canonical_words() {
    assert_same_stack("1 2 +", "1 2 ADD").await;
    assert_same_stack("5 3 -", "5 3 SUB").await;
    assert_same_stack("2 4 *", "2 4 MUL").await;
    assert_same_stack("8 2 /", "8 2 DIV").await;
    assert_same_stack("1 1 =", "1 1 EQ").await;
    assert_same_stack("1 2 <", "1 2 LT").await;
    assert_same_stack("1 1 <=", "1 1 LTE").await;
}

#[tokio::test]
async fn syntax_sugar_executes_same_as_canonical_mode_words() {
    assert_same_stack("1 2 . +", "1 2 TOP ADD").await;
    assert_same_stack("1 2 ,, +", "1 2 KEEP ADD").await;
    assert_same_stack("[ 1 2 3 ] [ 10 ] ~ GET", "[ 1 2 3 ] [ 10 ] SAFE GET").await;
}

#[tokio::test]
async fn force_alias_executes_same_as_forc() {
    let mut a = Interpreter::new();
    a.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
    a.execute("{ [ 1 ] } 'TMP' DEF").await.unwrap();
    a.execute("! 'TMP' DEL").await.unwrap();

    let mut b = Interpreter::new();
    b.execute("'json' IMPORT 'io' IMPORT").await.unwrap();
    b.execute("{ [ 1 ] } 'TMP' DEF").await.unwrap();
    b.execute("FORC 'TMP' DEL").await.unwrap();

    assert_eq!(a.get_stack(), b.get_stack());
}

#[tokio::test]
async fn symbol_aliases_and_canonical_core_words_are_reserved_for_user_definitions() {
    assert_def_rejected("+").await;
    assert_def_rejected("ADD").await;
    assert_def_rejected(".").await;
    assert_def_rejected("TOP").await;
    assert_def_rejected("!").await;
    assert_def_rejected("FORC").await;
    assert_def_rejected("?").await;
    assert_def_rejected("LOOKUP").await;
}

#[tokio::test]
async fn lookup_alias_canonicalizes_to_english_word() {
    use crate::core_word_aliases::canonicalize_core_word_name;
    assert_eq!(canonicalize_core_word_name("?"), "LOOKUP");
    assert_eq!(canonicalize_core_word_name("=="), "PIPE");
    assert_eq!(canonicalize_core_word_name("=>"), "OR-NIL");
}
