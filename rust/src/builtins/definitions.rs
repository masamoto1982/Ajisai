// rust/src/builtins/definitions.rs
//
// Built-in word definitions (name, description, syntax_example, signature_type)

/// Returns the list of all built-in word definitions.
/// Each tuple contains: (word_name, description, syntax_example, signature_type)
/// signature_type: "map" | "form" | "fold" | "none"
pub fn get_builtin_definitions() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
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
            "Set consumption mode to keep (preserve operands)",
            "[1] [2] ,, + → [1] [2] [3]",
            "none",
        ),
        // Safe mode
        (
            "~",
            "Safe mode - suppress errors and return NIL",
            "[ 1 2 3 ] [ 10 ] ~ GET → NIL",
            "none",
        ),
        // Input helpers
        ("'", "Input single quote", "' → '", "none"),
        (
            "FRAME",
            "Create bracket structure with shape",
            "[ 2 3 ] FRAME → { ( ) ( ) ( ) } { ( ) ( ) ( ) }",
            "none",
        ),
        // Position operations (0-indexed)
        (
            "GET",
            "Get element at position (0-indexed)",
            "[ 10 20 30 ] [ 0 ] GET → [ 10 ]",
            "form",
        ),
        (
            "INSERT",
            "Insert element at position",
            "[ 1 3 ] [ 1 2 ] INSERT → [ 1 2 3 ]",
            "form",
        ),
        (
            "REPLACE",
            "Replace element at position",
            "[ 1 2 3 ] [ 0 9 ] REPLACE → [ 9 2 3 ]",
            "form",
        ),
        (
            "REMOVE",
            "Remove element at position",
            "[ 1 2 3 ] [ 0 ] REMOVE → [ 2 3 ]",
            "form",
        ),
        // Quantity operations
        (
            "LENGTH",
            "Get vector length",
            "[ 1 2 3 4 5 ] LENGTH → [ 5 ]",
            "form",
        ),
        (
            "TAKE",
            "Take N elements from start or end",
            "[ 1 2 3 4 5 ] [ 3 ] TAKE → [ 1 2 3 ]",
            "form",
        ),
        // Vector operations
        (
            "SPLIT",
            "Split vector at specified sizes",
            "[ 1 2 3 4 5 6 ] [ 2 3 ] SPLIT → [ 1 2 ] [ 3 4 5 ] [ 6 ]",
            "form",
        ),
        (
            "CONCAT",
            "Concatenate vectors",
            "[ 1 2 ] [ 3 4 ] CONCAT → [ 1 2 3 4 ]",
            "form",
        ),
        (
            "REVERSE",
            "Reverse vector elements",
            "[ 1 2 3 ] REVERSE → [ 3 2 1 ]",
            "form",
        ),
        (
            "RANGE",
            "Generate numeric range",
            "[ 0 5 ] RANGE → [ 0 1 2 3 4 5 ]",
            "form",
        ),
        (
            "REORDER",
            "Reorder elements by index list",
            "[ a b c ] [ 2 0 1 ] REORDER → [ c a b ]",
            "form",
        ),
        (
            "COLLECT",
            "Collect N items from stack into vector",
            "1 2 3 3 COLLECT → [ 1 2 3 ]",
            "none",
        ),
        (
            "SORT",
            "Sort vector elements ascending",
            "[ 3 1 2 ] SORT → [ 1 2 3 ]",
            "form",
        ),
        // Constants
        ("TRUE", "Push TRUE to stack", "TRUE → TRUE", "none"),
        ("FALSE", "Push FALSE to stack", "FALSE → FALSE", "none"),
        ("NIL", "Push NIL (empty) to stack", "NIL → NIL", "none"),
        // String operations
        (
            "CHARS",
            "Split string into character vector",
            "[ 'hello' ] CHARS → [ 'h' 'e' 'l' 'l' 'o' ]",
            "map",
        ),
        (
            "JOIN",
            "Join character vector into string",
            "[ 'h' 'e' 'l' 'l' 'o' ] JOIN → [ 'hello' ]",
            "map",
        ),
        // Parse/Convert
        (
            "NUM",
            "Parse string to number",
            "'123' NUM → [ 123 ], returns NIL on failure",
            "map",
        ),
        (
            "STR",
            "Convert value to string (Stringify)",
            "123 STR → '123'",
            "map",
        ),
        (
            "BOOL",
            "Normalize to boolean",
            "'true' BOOL → TRUE, 100 BOOL → TRUE",
            "map",
        ),
        (
            "CHR",
            "Convert number to Unicode character",
            "65 CHR → 'A'",
            "map",
        ),
        // DateTime
        (
            "NOW",
            "Get current Unix timestamp",
            "NOW → [ 1732531200 ]",
            "none",
        ),
        (
            "DATETIME",
            "Convert timestamp to datetime vector (TZ required)",
            "[ 1732531200 ] 'LOCAL' DATETIME → [ 2024 11 25 23 0 0 ]",
            "none",
        ),
        (
            "TIMESTAMP",
            "Convert datetime vector to timestamp (TZ required)",
            "[ 2024 11 25 23 0 0 ] 'LOCAL' TIMESTAMP → [ 1732531200 ]",
            "none",
        ),
        // Arithmetic
        (
            "+",
            "Element-wise addition or aggregation",
            "[ 1 2 ] [ 3 4 ] + → [ 4 6 ]",
            "fold",
        ),
        (
            "-",
            "Element-wise subtraction or aggregation",
            "[ 5 3 ] [ 2 1 ] - → [ 3 2 ]",
            "fold",
        ),
        (
            "*",
            "Element-wise multiplication or aggregation",
            "[ 2 3 ] [ 4 5 ] * → [ 8 15 ]",
            "fold",
        ),
        (
            "/",
            "Element-wise division or aggregation",
            "[ 10 20 ] [ 2 4 ] / → [ 5 5 ]",
            "fold",
        ),
        // Comparison
        (
            "=",
            "Check if vectors are equal",
            "[ 1 2 ] [ 1 2 ] = → [ TRUE ]",
            "fold",
        ),
        (
            "<",
            "Check if less than",
            "[ 1 ] [ 2 ] < → [ TRUE ]",
            "fold",
        ),
        (
            "<=",
            "Check if less than or equal",
            "[ 1 ] [ 1 ] <= → [ TRUE ]",
            "fold",
        ),
        // > と >= は廃止されました。< と <= のみ使用可能です。

        // Logic
        (
            "AND",
            "Logical AND",
            "[ TRUE FALSE ] [ TRUE TRUE ] AND → [ TRUE FALSE ]",
            "fold",
        ),
        (
            "OR",
            "Logical OR",
            "[ TRUE FALSE ] [ FALSE FALSE ] OR → [ TRUE FALSE ]",
            "fold",
        ),
        (
            "NOT",
            "Logical NOT",
            "[ TRUE FALSE ] NOT → [ FALSE TRUE ]",
            "map",
        ),
        // Control (chevron branching)
        (
            ">>",
            "Chevron branch (condition/action)",
            ">> condition >> action >>> default",
            "none",
        ),
        (
            ">>>",
            "Chevron branch (default)",
            ">>> default_action",
            "none",
        ),
        // Code block
        (
            ":",
            "Code block start",
            ": code ; → pushes code block to stack",
            "none",
        ),
        (";", "Code block end", ": code ; → ends code block", "none"),
        // Pipeline and Nil Coalescing
        (
            "==",
            "Pipeline operator (visual marker)",
            "[ 1 2 3 ] == : [ 2 ] * ; MAP",
            "none",
        ),
        (
            "=>",
            "Nil coalescing operator",
            "NIL => [ 0 ] → [ 0 ]",
            "none",
        ),
        // Higher-order functions
        (
            "MAP",
            "Apply code to each element",
            "[ 1 2 3 ] : [ 2 ] * ; MAP → [ 2 4 6 ]",
            "form",
        ),
        (
            "FILTER",
            "Filter elements by condition",
            "[ 1 2 3 4 ] : [ 2 ] MOD [ 0 ] = ; FILTER → [ 2 4 ]",
            "form",
        ),
        (
            "FOLD",
            "Fold with initial value",
            "[ 1 2 3 4 ] [ 0 ] : + ; FOLD → [ 10 ]",
            "form",
        ),
        // I/O
        (
            "PRINT",
            "Print and pop stack top",
            "[ 42 ] PRINT → (outputs 42)",
            "map",
        ),
        // Word management
        (
            "DEF",
            "Define custom word",
            ": [ 2 ] * ; 'DOUBLE' DEF",
            "none",
        ),
        ("DEL", "Delete custom word", "'WORD' DEL", "none"),
        ("?", "Show word definition", "'DOUBLE' ?", "none"),
        // Control flow
        (
            "TIMES",
            "Repeat code N times",
            ": [ 1 ] + ; [ 5 ] TIMES",
            "none",
        ),
        (
            "WAIT",
            "Execute word after delay (ms)",
            "'PROCESS' [ 1000 ] WAIT",
            "none",
        ),
        (
            "!",
            "Force flag - allow DEL/DEF of dependent words",
            "! 'WORD' DEL",
            "none",
        ),
        // Shape operations
        (
            "SHAPE",
            "Get vector shape",
            "[ 1 2 3 ] SHAPE → [ 3 ]",
            "map",
        ),
        (
            "RANK",
            "Get number of dimensions",
            "[ [ 1 2 ] [ 3 4 ] ] RANK → [ 2 ]",
            "map",
        ),
        (
            "RESHAPE",
            "Reshape vector to new dimensions",
            "[ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE → { ( 1 2 3 ) ( 4 5 6 ) }",
            "form",
        ),
        (
            "TRANSPOSE",
            "Transpose 2D vector",
            "{ ( 1 2 3 ) ( 4 5 6 ) } TRANSPOSE → { ( 1 4 ) ( 2 5 ) ( 3 6 ) }",
            "form",
        ),
        (
            "FILL",
            "Generate vector filled with value",
            "[ 2 3 0 ] FILL → { ( 0 0 0 ) ( 0 0 0 ) }",
            "form",
        ),
        // Math functions
        (
            "MOD",
            "Modulo (mathematical)",
            "[ 7 ] [ 3 ] MOD → [ 1 ]",
            "fold",
        ),
        (
            "FLOOR",
            "Floor (toward negative infinity)",
            "[ 7/3 ] FLOOR → [ 2 ]",
            "map",
        ),
        (
            "CEIL",
            "Ceiling (toward positive infinity)",
            "[ 7/3 ] CEIL → [ 3 ]",
            "map",
        ),
        (
            "ROUND",
            "Round (away from zero)",
            "[ 5/2 ] ROUND → [ 3 ]",
            "map",
        ),
        // Cryptographic random
        (
            "CSPRNG",
            "Generate cryptographic random",
            "[ 6 ] [ 1 ] CSPRNG → [ 0 ] to [ 5/6 ], [ 5 ] CSPRNG → 5 randoms",
            "none",
        ),
        // Hash
        (
            "HASH",
            "Deterministic hash of any value",
            "'hello' HASH → [ 0.xxx ], [ 128 ] 'hello' HASH → 128-bit",
            "none",
        ),
        // Meta-programming
        (
            "EXEC",
            "Execute vector (or stack) as code",
            "[ 1 2 + ] EXEC → 3, 1 2 + .. EXEC → 3",
            "none",
        ),
        (
            "EVAL",
            "Parse and execute string (or stack chars)",
            "'1 2 +' EVAL → 3",
            "none",
        ),
        // Module system
        (
            "IMPORT",
            "Import standard library module",
            "'music' IMPORT → registers MUSIC::* words",
            "none",
        ),
    ]
}
