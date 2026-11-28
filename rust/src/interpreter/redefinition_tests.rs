//! Redefinition of a Word other Words call (LANG.DICTIONARY.MUTATION).

use crate::interpreter::Interpreter;

/// A referenced Word may be redefined to a body of the same stack effect,
/// and its callers call the new body from then on (LANG.DICTIONARY.MUTATION):
/// the cell a sheet's other cells read can be edited.
#[tokio::test]
async fn test_def_with_dependents_keeping_the_stack_effect_replaces_it() {
    let mut interp = Interpreter::new();
    interp.execute("[ 120 ] 'PRICE' DEF").await.unwrap();
    interp.execute("[ PRICE 3 MUL ] 'TOTAL' DEF").await.unwrap();
    let before = interp.execute("[ TOTAL ] 0 GET DIGEST").await.map(|_| ());
    assert!(before.is_ok());
    let digest_before = format!("{:?}", interp.get_stack().last());
    interp.execute("[ 150 ] 'PRICE' DEF").await.unwrap();
    interp
        .execute("[ TOTAL ] 0 GET DIGEST TOTAL")
        .await
        .unwrap();
    let stack = interp.get_stack();
    assert_eq!(format!("{}", stack.last().unwrap()), "450/1");
    let digest_after = format!("{:?}", stack.get(stack.len() - 2));
    assert_ne!(digest_before, digest_after, "the caller's identity follows");
    // The new body may not reach back through a caller.
    let err = interp
        .execute("[ TOTAL ] 'PRICE' DEF")
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("names itself"), "{err}");
    // An effect the inference cannot fix is refused too.
    let err = interp
        .execute("[ 'X' BIND 1 X RANGE ] 'PRICE' DEF")
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("Cannot redefine"), "{err}");
}
