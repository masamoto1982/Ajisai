#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueData;

    #[tokio::test]
    async fn test_cond_basic_two_branch() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ -5 ] { [ 0 ] < } { 'negative' } { IDLE } { 'positive' } COND")
            .await;
        assert!(result.is_ok(), "COND should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_cond_else_branch_runs() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 5 ] { [ 0 ] < } { 'negative' } { IDLE } { 'positive' } COND")
            .await;
        assert!(result.is_ok(), "COND should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        assert!(matches!(interp.stack.last().unwrap().data, ValueData::Vector(_)));
    }

    #[tokio::test]
    async fn test_cond_exhausted_error() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 5 ] { [ 0 ] < } { 'negative' } COND")
            .await;
        assert!(result.is_err(), "COND should fail without else: {:?}", result);
        let message = result.err().unwrap().to_string();
        assert!(
            message.contains("COND: all guards failed and no else clause"),
            "unexpected error: {}",
            message
        );
    }

    #[tokio::test]
    async fn test_cond_invalid_pair_count_error() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 ] { [ 0 ] < } COND").await;
        assert!(result.is_err(), "COND should fail on odd blocks: {:?}", result);
        let message = result.err().unwrap().to_string();
        assert!(
            message.contains("COND: expected even number of code blocks"),
            "unexpected error: {}",
            message
        );
    }

    #[tokio::test]
    async fn test_cond_keep_mode_no_duplicate() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ -5 ] ,, { [ 0 ] < } { 'negative' } { IDLE } { 'positive' } COND")
            .await;
        assert!(result.is_ok(), "COND with ,, should succeed: {:?}", result);
        assert_eq!(
            interp.stack.len(),
            2,
            "Keep mode should leave original + result (2 items), got {}",
            interp.stack.len()
        );
    }

    #[tokio::test]
    async fn test_cond_non_boolean_guard_error() {
        let mut interp = Interpreter::new();
        let result = interp
            .execute("[ 1 ] { [ 2 ] } { 'x' } { IDLE } { 'else' } COND")
            .await;
        assert!(result.is_err(), "COND should fail for non-boolean guard");
        let message = result.err().unwrap().to_string();
        assert!(
            message.contains("COND: guard must return TRUE or FALSE"),
            "unexpected error: {}",
            message
        );
    }
}

#[cfg(test)]
mod demo_word_gui_mode_tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueData;

    fn create_gui_interpreter() -> Interpreter {
        let mut interp = Interpreter::new();
        interp.gui_mode = true;
        interp
    }

    async fn setup_demo_words(interp: &mut Interpreter) {
        interp.execute("{ 'Hello' ,, PRINT } 'SAY-HELLO' DEF").await.unwrap();
        interp.execute("{ 'World' ,, PRINT } 'SAY-WORLD' DEF").await.unwrap();
        interp.execute("{ '!' ,, PRINT } 'SAY-BANG' DEF").await.unwrap();
        interp.execute("{ { [ 1 ] = } { SAY-HELLO } { [ 2 ] = } { SAY-WORLD } { IDLE } { SAY-BANG } COND } 'GREET' DEF").await.unwrap();
        interp.execute("{ { GREET } MAP } 'GREET-ALL' DEF").await.unwrap();
        let _ = interp.collect_output();
    }

    #[tokio::test]
    async fn test_cond_guard_unwraps_vector_boolean_in_gui_mode() {
        let mut interp = create_gui_interpreter();
        let r = interp
            .execute("[ 1 ] { [ 1 ] = } { 'yes' } { IDLE } { 'no' } COND")
            .await;
        assert!(r.is_ok(), "COND should handle gui-mode vector boolean: {:?}", r);
    }

    #[tokio::test]
    async fn test_greet_in_gui_mode() {
        let mut interp = create_gui_interpreter();
        setup_demo_words(&mut interp).await;

        let r = interp.execute("[ 1 ] GREET").await;
        assert!(r.is_ok(), "GREET 1 failed: {:?}", r);
        let output = interp.collect_output();
        assert!(output.contains("Hello"), "Expected Hello, got: {}", output);
    }

    #[tokio::test]
    async fn test_greet_branch_2_in_gui_mode() {
        let mut interp = create_gui_interpreter();
        setup_demo_words(&mut interp).await;

        let r = interp.execute("[ 2 ] GREET").await;
        assert!(r.is_ok(), "GREET 2 failed: {:?}", r);
        let output = interp.collect_output();
        assert!(output.contains("World"), "Expected World, got: {}", output);
    }

    #[tokio::test]
    async fn test_greet_else_branch_in_gui_mode() {
        let mut interp = create_gui_interpreter();
        setup_demo_words(&mut interp).await;

        let r = interp.execute("[ 99 ] GREET").await;
        assert!(r.is_ok(), "GREET 99 failed: {:?}", r);
        let output = interp.collect_output();
        assert!(output.contains("!"), "Expected !, got: {}", output);
    }

    #[tokio::test]
    async fn test_greet_all_in_gui_mode() {
        let mut interp = create_gui_interpreter();
        setup_demo_words(&mut interp).await;

        let r = interp.execute("[ 1 2 3 ] GREET-ALL").await;
        assert!(r.is_ok(), "GREET-ALL failed: {:?}", r);
        let output = interp.collect_output();
        assert!(output.contains("Hello"), "Expected Hello in output, got: {}", output);
        assert!(output.contains("World"), "Expected World in output, got: {}", output);
        assert!(output.contains("!"), "Expected ! in output, got: {}", output);


        assert_eq!(interp.stack.len(), 1);
        let result = &interp.stack[0];
        assert_eq!(result.len(), 3);

        let bang = result.get_child(2).unwrap();
        assert!(
            matches!(bang.data, ValueData::Vector(_)),
            "Expected '!' to remain as a vector (string), got: {:?}",
            bang.data
        );
    }
}

