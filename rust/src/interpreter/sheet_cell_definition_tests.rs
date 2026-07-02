//! Tests for the Sheet-view host definition API
//! (`op_def_forced_in_dictionary`, exported to the host as
//! `define_word_forced`; see docs/dev/ajisai-spreadsheet-app-redesign-plan.md
//! §2.4).
//!
//! A spreadsheet cell is a word in a sheet dictionary (`SHEET@A1`), and
//! overwriting a cell that other cells reference is normal spreadsheet
//! operation. These tests pin the three properties the host relies on:
//! the redefinition guard is bypassed (force path), interpreter state that
//! the Editor view owns (active dictionary, output buffer, force flag) is
//! left untouched, and the reverse-dependency index — the dirty-set source
//! for recalculation — stays correct across cell redefinitions.

#[cfg(test)]
mod tests {
    use crate::interpreter::execute_def::op_def_forced_in_dictionary;
    use crate::interpreter::Interpreter;
    use crate::tokenizer::tokenize;
    use std::collections::HashSet;

    fn set(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn define_cell(interp: &mut Interpreter, name: &str, body: &str) {
        let tokens = tokenize(body).unwrap();
        op_def_forced_in_dictionary(interp, "SHEET", name, &tokens).unwrap();
    }

    /// The host API lands the word in the requested dictionary, not in the
    /// interactively selected one, and restores the selection afterwards.
    #[test]
    fn defines_into_target_dictionary_and_restores_selection() {
        let mut interp = Interpreter::new();
        assert_eq!(interp.active_user_dictionary, "EXAMPLE");

        define_cell(&mut interp, "A1", "[ 42 ]");

        assert_eq!(
            interp.active_user_dictionary, "EXAMPLE",
            "cell definition must not move the Editor view's active dictionary"
        );
        assert!(
            interp
                .user_dictionaries
                .get("SHEET")
                .is_some_and(|dict| dict.words.contains_key("A1")),
            "A1 must be defined in the SHEET dictionary"
        );
    }

    /// Redefining a cell that other cells reference succeeds without the `!`
    /// force prefix: the guard that protects interactively defined words is
    /// bypassed on the host path.
    #[test]
    fn redefinition_guard_is_bypassed_for_cells() {
        let mut interp = Interpreter::new();
        define_cell(&mut interp, "A1", "[ 1 ]");
        define_cell(&mut interp, "B1", "SHEET@A1");

        assert_eq!(
            interp.collect_dependents("SHEET@A1"),
            set(&["SHEET@B1"]),
            "B1 references A1"
        );

        // Would fail with "Cannot redefine ... Use ! ..." on the DEF path.
        define_cell(&mut interp, "A1", "[ 2 ]");
    }

    /// Cell definitions are host bookkeeping: DEF's "Defined word: ..." and
    /// redefinition warnings must not leak into the next execution's output.
    #[test]
    fn output_buffer_is_not_polluted() {
        let mut interp = Interpreter::new();
        define_cell(&mut interp, "A1", "[ 1 ]");
        define_cell(&mut interp, "B1", "SHEET@A1");
        define_cell(&mut interp, "A1", "[ 2 ]");

        assert!(
            interp.output_buffer.is_empty(),
            "cell definitions must leave the output buffer untouched, got: {:?}",
            interp.output_buffer
        );
    }

    /// The reverse-dependency index gives the recalculation dirty set: a
    /// chain of cell references is reported transitively, and survives the
    /// forced redefinition of the upstream cell.
    #[test]
    fn transitive_dependents_provide_the_dirty_set() {
        let mut interp = Interpreter::new();
        define_cell(&mut interp, "A1", "[ 1 ]");
        define_cell(&mut interp, "B1", "SHEET@A1 [ 2 ] *");
        define_cell(&mut interp, "C1", "SHEET@B1 [ 3 ] +");

        assert_eq!(
            interp.collect_transitive_dependents("SHEET@A1"),
            set(&["SHEET@B1", "SHEET@C1"]),
            "the dirty set of A1 reaches C1 through B1"
        );

        define_cell(&mut interp, "A1", "[ 10 ]");
        assert_eq!(
            interp.collect_transitive_dependents("SHEET@A1"),
            set(&["SHEET@B1", "SHEET@C1"]),
            "redefining A1 must not lose its dependents"
        );
    }

    /// Editor-view words referencing a cell take part in the same index, so
    /// the host can invalidate word-dependent cells and cell-dependent words
    /// through one mechanism.
    #[tokio::test]
    async fn editor_words_and_cells_share_one_dependency_index() {
        let mut interp = Interpreter::new();
        define_cell(&mut interp, "A1", "[ 1 ]");
        interp
            .execute("{ SHEET@A1 [ 2 ] * } 'DOUBLED' DEF")
            .await
            .unwrap();

        assert_eq!(
            interp.collect_transitive_dependents("SHEET@A1"),
            set(&["EXAMPLE@DOUBLED"]),
            "an Editor word reading a cell appears in the cell's dirty set"
        );
    }

    /// On failure the API restores the Editor view's state (active
    /// dictionary, force flag, output buffer) exactly as on success.
    #[test]
    fn failure_restores_interpreter_state() {
        let mut interp = Interpreter::new();
        let result = op_def_forced_in_dictionary(&mut interp, "SHEET", "A1", &[]);

        assert!(result.is_err(), "an empty body must be rejected");
        assert_eq!(interp.active_user_dictionary, "EXAMPLE");
        assert!(!interp.force_flag);
        assert!(interp.output_buffer.is_empty());
    }
}
