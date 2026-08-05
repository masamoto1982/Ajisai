use crate::error::{AjisaiError, Result};
use crate::interpreter::{ConsumptionMode, HostEffect, Interpreter};
use crate::types::Value;
use std::fmt::Write;

fn extract_value_for_print(interp: &mut Interpreter, keep_mode: bool) -> Result<Value> {
    if keep_mode {
        return interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow);
    }
    interp.stack.pop().ok_or(AjisaiError::StackUnderflow)
}

pub fn op_print(interp: &mut Interpreter) -> Result<()> {
    interp.run_effect_schema(|interp| {
        let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
        let val = extract_value_for_print(interp, is_keep_mode)?;
        // PRINT is an output boundary: a Text-role value is emitted as its raw
        // character content, without the `'...'` quotes the Stack projection
        // uses to mark it as a string (SPEC §7.9).
        let payload = crate::types::display::format_for_output(&val);
        // One PRINT, one line. LANG.EFFECTS.OUTPUT makes the output stream an
        // *ordered sequence of emissions* and leaves rendering to the host, and
        // the structured `HostEffect::Print` channel below already carries one
        // item per call — this buffer is the host-readable rendering of that
        // same sequence. Joining the items with a space instead collapsed every
        // emission into one long line, so nine `PRINT`s of a Pascal's triangle
        // row produced a single row of numbers and the language had no way at
        // all to produce multi-line output. The native CLI has always rendered
        // one payload per line; this brings the buffer that the browser host
        // reads in line with it.
        writeln!(&mut interp.output_buffer, "{}", payload)
            .map_err(|e| AjisaiError::from(format!("PRINT failed: {}", e)))?;
        Ok(HostEffect::Print(payload))
    })
}

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    /// A string is shown on the stack as `'TEST'` but printed as its raw
    /// content `TEST`: the surrounding quotes are a Stack affordance only
    /// (SPEC §7.9).
    #[tokio::test]
    async fn test_print_strips_display_quotes_from_string() {
        let mut interp = Interpreter::new();
        interp.execute("'TEST' PRINT").await.unwrap();
        let output = interp.collect_output();
        assert_eq!(output.trim(), "TEST", "unexpected output: {:?}", output);
    }

    /// Quote characters that are part of the string content survive: the
    /// content `T'ES'T` (shown as `'T'ES'T'` on the stack) prints unchanged.
    #[tokio::test]
    async fn test_print_keeps_content_quote_characters() {
        let mut interp = Interpreter::new();
        interp.execute("'T'ES'T' PRINT").await.unwrap();
        let output = interp.collect_output();
        assert_eq!(output.trim(), "T'ES'T", "unexpected output: {:?}", output);
    }

    /// Non-text values print exactly as they render on the stack.
    #[tokio::test]
    async fn test_print_numbers_and_booleans_unchanged() {
        let mut interp = Interpreter::new();
        interp.execute("[ 42 ] PRINT").await.unwrap();
        assert_eq!(interp.collect_output().trim(), "[ 42/1 ]");

        interp.execute("TRUE PRINT").await.unwrap();
        assert_eq!(interp.collect_output().trim(), "TRUE");
    }

    /// PRINT consumes only the top value; KEEP (`,,`) prints the raw text and
    /// leaves the quoted string on the stack.
    #[tokio::test]
    async fn test_print_keep_mode_leaves_string_on_stack() {
        let mut interp = Interpreter::new();
        interp.execute("'TEST' KEEP PRINT").await.unwrap();
        assert_eq!(interp.collect_output().trim(), "TEST");
        assert_eq!(interp.stack.len(), 1);
        // The value still renders with its Stack-projection quotes.
        assert_eq!(interp.stack.last().unwrap().to_string(), "'TEST'");
    }

    /// A string nested inside a collection stays a string: printing a vector
    /// of strings shows them quoted, never as their codepoint fractions. Only
    /// the outer `'...'` of a top-level string is a display affordance.
    #[tokio::test]
    async fn test_print_vector_of_strings_keeps_them_as_strings() {
        let mut interp = Interpreter::new();
        interp.execute("[ 'AB' 'CD' ] PRINT").await.unwrap();
        assert_eq!(interp.collect_output().trim(), "[ 'AB' 'CD' ]");
    }

    /// A mixed collection renders each element in its own role: strings
    /// quoted, numbers as fractions.
    #[tokio::test]
    async fn test_print_mixed_vector_renders_each_role() {
        let mut interp = Interpreter::new();
        interp.execute("[ 'mix' 42 ] KEEP PRINT").await.unwrap();
        assert_eq!(interp.collect_output().trim(), "[ 'mix' 42/1 ]");
        // The Stack projection shows the same structure.
        assert_eq!(interp.stack.last().unwrap().to_string(), "[ 'mix' 42/1 ]");
    }
}
