// rust/src/builtins/builtin-word-definitions.rs
//
// Built-in word definitions (name, description, syntax_example, signature_type)

/// Returns the list of all built-in word definitions.
/// Each tuple contains: (word_name, description, syntax_example, signature_type)
/// signature_type: "map" | "form" | "fold" | "none"
pub fn collect_builtin_definitions() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        // Target specification (Operation Target Mode)
        (
            ".",
            "Set operation target to stack top (default)",
            ". + → add to stack top",
            "none",
        ),
        (
            "..",
            "Set operation target to entire stack",
            ".. + [ 3 ] → add 3 to all stack elements",
            "none",
        ),
        // Consumption mode specification
        (
            ",",
            "Set consumption mode to consume (default)",
            ", + → consume operands",
            "none",
        ),
        (
            ",,",
            "Set bifurcation mode. Preserve operands and append result",
            "[1] [2] ,, + → [1] [2] [3]",
            "none",
        ),
        // Safe mode
        (
            "~",
            "Safe mode. Return NIL on error",
            "[ 1 2 3 ] [ 10 ] ~ GET → NIL",
            "none",
        ),
        // Input helpers
        ("'", "Input single quote (input helper)", "' → '", "none"),
        (
            "FRAME",
            "Generate empty vector structure from shape. Shape Vector -> Vector",
            "[ 2 3 ] FRAME → [ [ ] [ ] [ ] ] [ [ ] [ ] [ ] ]",
            "none",
        ),
        // Position operations (0-indexed)
        (
            "GET",
            "Form: Extract element at index. Vector, Index Vector -> element",
            "[ 10 20 30 ] [ 0 ] GET → [ 10 ]",
            "form",
        ),
        (
            "INSERT",
            "Form: Insert element at index. Vector, [Index, Value] Vector -> Vector",
            "[ 1 3 ] [ 1 2 ] INSERT → [ 1 2 3 ]",
            "form",
        ),
        (
            "REPLACE",
            "Form: Replace element at index. Vector, [Index, Value] Vector -> Vector",
            "[ 1 2 3 ] [ 0 9 ] REPLACE → [ 9 2 3 ]",
            "form",
        ),
        (
            "REMOVE",
            "Form: Remove element at index. Vector, Index Vector -> Vector",
            "[ 1 2 3 ] [ 0 ] REMOVE → [ 2 3 ]",
            "form",
        ),
        // Quantity operations
        (
            "LENGTH",
            "Form: Return element count. Vector -> Scalar",
            "[ 1 2 3 4 5 ] LENGTH → [ 5 ]",
            "form",
        ),
        (
            "TAKE",
            "Form: Take N elements from start or end. Vector, Scalar -> Vector",
            "[ 1 2 3 4 5 ] [ 3 ] TAKE → [ 1 2 3 ]",
            "form",
        ),
        // Vector operations
        (
            "SPLIT",
            "Form: Split vector at specified sizes. Vector, Sizes Vector -> Vectors",
            "[ 1 2 3 4 5 6 ] [ 2 3 ] SPLIT → [ 1 2 ] [ 3 4 5 ] [ 6 ]",
            "form",
        ),
        (
            "CONCAT",
            "Form: Flatten-concatenate two vectors. Vector, Vector -> Vector",
            "[ 1 2 ] [ 3 4 ] CONCAT → [ 1 2 3 4 ]",
            "form",
        ),
        (
            "REVERSE",
            "Form: Reverse element order. Vector -> Vector",
            "[ 1 2 3 ] REVERSE → [ 3 2 1 ]",
            "form",
        ),
        (
            "RANGE",
            "Form: Generate numeric sequence. [start, end] or [end] -> Vector",
            "[ 0 5 ] RANGE → [ 0 1 2 3 4 5 ]",
            "form",
        ),
        (
            "REORDER",
            "Form: Reorder elements by index list. Vector, Index Vector -> Vector",
            "[ a b c ] [ 2 0 1 ] REORDER → [ c a b ]",
            "form",
        ),
        (
            "COLLECT",
            "Collect N items from stack into vector. ...values, Scalar -> Vector",
            "1 2 3 3 COLLECT → [ 1 2 3 ]",
            "none",
        ),
        (
            "SORT",
            "Form: Sort elements ascending. Vector -> Vector",
            "[ 3 1 2 ] SORT → [ 1 2 3 ]",
            "form",
        ),
        // Constants
        ("TRUE", "Push TRUE to stack", "TRUE → TRUE", "none"),
        ("FALSE", "Push FALSE to stack", "FALSE → FALSE", "none"),
        ("NIL", "Push NIL to stack", "NIL → NIL", "none"),
        // String operations
        (
            "CHARS",
            "Map: Split string into character code vector. String -> Numeric Vector",
            "[ 'hello' ] CHARS → [ 'h' 'e' 'l' 'l' 'o' ]",
            "map",
        ),
        (
            "JOIN",
            "Map: Join character code vector into string. Numeric Vector -> String",
            "[ 'h' 'e' 'l' 'l' 'o' ] JOIN → [ 'hello' ]",
            "map",
        ),
        // Parse/Convert
        (
            "NUM",
            "Map: Parse to number. Any -> Numeric (NIL on failure)",
            "'123' NUM → [ 123 ], returns NIL on failure",
            "map",
        ),
        (
            "STR",
            "Map: Convert to string representation. Any -> String",
            "123 STR → '123'",
            "map",
        ),
        (
            "BOOL",
            "Map: Convert to boolean. Any -> Boolean",
            "'true' BOOL → TRUE, 100 BOOL → TRUE",
            "map",
        ),
        (
            "CHR",
            "Map: Convert character code to single-char string. Numeric -> String",
            "65 CHR → 'A'",
            "map",
        ),
        // DateTime
        (
            "NOW",
            "Get current UNIX timestamp. -> Scalar",
            "NOW → [ 1732531200 ]",
            "none",
        ),
        (
            "DATETIME",
            "Convert timestamp to datetime string. Scalar -> String",
            "[ 1732531200 ] 'LOCAL' DATETIME → [ 2024 11 25 23 0 0 ]",
            "none",
        ),
        (
            "TIMESTAMP",
            "Convert datetime string to timestamp. String -> Scalar",
            "[ 2024 11 25 23 0 0 ] 'LOCAL' TIMESTAMP → [ 1732531200 ]",
            "none",
        ),
        // Arithmetic
        (
            "+",
            "Fold: Addition (broadcast). Numeric, Numeric -> Numeric",
            "[ 1 2 ] [ 3 4 ] + → [ 4 6 ]",
            "fold",
        ),
        (
            "-",
            "Fold: Subtraction (broadcast). Numeric, Numeric -> Numeric",
            "[ 5 3 ] [ 2 1 ] - → [ 3 2 ]",
            "fold",
        ),
        (
            "*",
            "Fold: Multiplication (broadcast). Numeric, Numeric -> Numeric",
            "[ 2 3 ] [ 4 5 ] * → [ 8 15 ]",
            "fold",
        ),
        (
            "/",
            "Fold: Division (broadcast). Numeric, Numeric -> Numeric",
            "[ 10 20 ] [ 2 4 ] / → [ 5 5 ]",
            "fold",
        ),
        // Comparison
        (
            "=",
            "Fold: Equality comparison (broadcast). Any, Any -> Boolean",
            "[ 1 2 ] [ 1 2 ] = → [ TRUE ]",
            "fold",
        ),
        (
            "<",
            "Fold: Less-than comparison (broadcast). Numeric, Numeric -> Boolean",
            "[ 1 ] [ 2 ] < → [ TRUE ]",
            "fold",
        ),
        (
            "<=",
            "Fold: Less-or-equal comparison (broadcast). Numeric, Numeric -> Boolean",
            "[ 1 ] [ 1 ] <= → [ TRUE ]",
            "fold",
        ),
        // > と >= は廃止されました。< と <= のみ使用可能です。

        // Logic
        (
            "AND",
            "Fold: Logical AND (Kleene three-valued). Boolean, Boolean -> Boolean",
            "[ TRUE FALSE ] [ TRUE TRUE ] AND → [ TRUE FALSE ]",
            "fold",
        ),
        (
            "OR",
            "Fold: Logical OR (Kleene three-valued). Boolean, Boolean -> Boolean",
            "[ TRUE FALSE ] [ FALSE FALSE ] OR → [ TRUE FALSE ]",
            "fold",
        ),
        (
            "NOT",
            "Map: Logical negation. Boolean -> Boolean",
            "[ TRUE FALSE ] NOT → [ FALSE TRUE ]",
            "map",
        ),
        // Control
        ("IDLE", "Pass through flow unchanged (no-op)", "IDLE", "none"),
        (
            "COND",
            "Form: Evaluate guard/body pairs, execute first match. Any, ...CodeBlock pairs -> Any",
            "value { guard1 } { body1 } { IDLE } { else_body } COND",
            "form",
        ),
        // Pipeline and Nil Coalescing
        (
            "==",
            "Pipeline visual marker (no-op)",
            "[ 1 2 3 ] == { [ 2 ] * } MAP",
            "none",
        ),
        (
            "=>",
            "Nil coalescing. Return alternative if NIL. Any, Any -> Any",
            "NIL => [ 0 ] → [ 0 ]",
            "none",
        ),
        // Higher-order functions
        (
            "MAP",
            "Form: Apply code block to each element. Vector, CodeBlock -> Vector",
            "[ 1 2 3 ] { [ 2 ] * } MAP → [ 2 4 6 ]",
            "form",
        ),
        (
            "FILTER",
            "Form: Extract elements matching predicate. Vector, CodeBlock -> Vector",
            "[ 1 2 3 4 ] { [ 2 ] MOD [ 0 ] = } FILTER → [ 2 4 ]",
            "form",
        ),
        (
            "FOLD",
            "Form: Reduce with initial value. Vector, Scalar, CodeBlock -> Scalar",
            "[ 1 2 3 4 ] [ 0 ] { + } FOLD → [ 10 ]",
            "form",
        ),
        // I/O
        (
            "PRINT",
            "Map: Output value to display. Any -> Any",
            "[ 42 ] PRINT → (outputs 42)",
            "map",
        ),
        // Word management
        (
            "DEF",
            "Define user word in dictionary. CodeBlock, String [, String] -> (dictionary effect)",
            "{ [ 2 ] * } 'DOUBLE' DEF",
            "none",
        ),
        (
            "DEL",
            "Delete user word from dictionary. String -> (dictionary effect)",
            "'WORD' DEL",
            "none",
        ),
        (
            "?",
            "Display word definition and details. String -> (output effect)",
            "'DOUBLE' ?",
            "none",
        ),
        (
            "!",
            "Force flag. Allow DEL/DEF of dependent user words",
            "! 'WORD' DEL",
            "none",
        ),
        // Shape operations
        (
            "SHAPE",
            "Map: Return vector shape. Vector -> Numeric Vector",
            "[ 1 2 3 ] SHAPE → [ 3 ]",
            "map",
        ),
        (
            "RANK",
            "Map: Return number of dimensions. Vector -> Scalar",
            "[ [ 1 2 ] [ 3 4 ] ] RANK → [ 2 ]",
            "map",
        ),
        (
            "RESHAPE",
            "Form: Reshape vector to specified shape. Vector, Shape Vector -> Vector",
            "[ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE → { ( 1 2 3 ) ( 4 5 6 ) }",
            "form",
        ),
        (
            "TRANSPOSE",
            "Form: Transpose vector axes. Vector -> Vector",
            "{ ( 1 2 3 ) ( 4 5 6 ) } TRANSPOSE → { ( 1 4 ) ( 2 5 ) ( 3 6 ) }",
            "form",
        ),
        (
            "FILL",
            "Form: Fill shape with value. Scalar, Shape Vector -> Vector",
            "[ 2 3 0 ] FILL → { ( 0 0 0 ) ( 0 0 0 ) }",
            "form",
        ),
        // Math functions
        (
            "MOD",
            "Fold: Modulo (broadcast). Numeric, Numeric -> Numeric",
            "[ 7 ] [ 3 ] MOD → [ 1 ]",
            "fold",
        ),
        (
            "FLOOR",
            "Map: Floor (round toward negative infinity). Numeric -> Numeric",
            "[ 7/3 ] FLOOR → [ 2 ]",
            "map",
        ),
        (
            "CEIL",
            "Map: Ceiling (round toward positive infinity). Numeric -> Numeric",
            "[ 7/3 ] CEIL → [ 3 ]",
            "map",
        ),
        (
            "ROUND",
            "Map: Round to nearest integer. Numeric -> Numeric",
            "[ 5/2 ] ROUND → [ 3 ]",
            "map",
        ),
        // Cryptographic random
        (
            "CSPRNG",
            "Generate cryptographic pseudorandom number. -> Scalar",
            "[ 6 ] [ 1 ] CSPRNG → [ 0 ] to [ 5/6 ], [ 5 ] CSPRNG → 5 randoms",
            "none",
        ),
        // Hash
        (
            "HASH",
            "Compute hash value. Any -> Numeric",
            "'hello' HASH → [ 0.xxx ], [ 128 ] 'hello' HASH → 128-bit",
            "none",
        ),
        // Meta-programming
        (
            "EXEC",
            "Interpret vector as code and execute. Vector -> (execution result)",
            "[ 1 2 + ] EXEC → 3, 1 2 + .. EXEC → 3",
            "none",
        ),
        (
            "EVAL",
            "Parse string as code and execute. String -> (execution result)",
            "'1 2 +' EVAL → 3",
            "none",
        ),
        // Module system
        (
            "IMPORT",
            "Load standard library module. String -> (dictionary effect)",
            "'music' IMPORT → registers MUSIC::* words",
            "none",
        ),
    ]
}
