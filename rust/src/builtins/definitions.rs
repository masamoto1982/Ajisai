// rust/src/builtins/definitions.rs
//
// Built-in word definitions (name, description, syntax_example, category)

/// Returns the list of all built-in word definitions.
/// Each tuple contains: (word_name, description, syntax_example, category)
pub fn get_builtin_definitions() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        // Target specification
        (".", "Set operation target to stack top (default)", ". + → add to stack top", "Target"),
        ("..", "Set operation target to entire stack", ".. + [ 3 ] → add 3 to all stack elements", "Target"),

        // Input helpers
        ("'", "Input single quote", "' → '", "Input Helper"),
        ("FRAME", "Create bracket structure with shape", "[ 2 3 ] FRAME → { ( ) ( ) ( ) } { ( ) ( ) ( ) }", "Input Helper"),

        // Position operations (0-indexed)
        ("GET", "Get element at position (0-indexed)", "[ 10 20 30 ] [ 0 ] GET → [ 10 20 30 ] [ 10 ]", "Position"),
        ("INSERT", "Insert element at position", "[ 1 3 ] [ 1 2 ] INSERT → [ 1 2 3 ]", "Position"),
        ("REPLACE", "Replace element at position", "[ 1 2 3 ] [ 0 9 ] REPLACE → [ 9 2 3 ]", "Position"),
        ("REMOVE", "Remove element at position", "[ 1 2 3 ] [ 0 ] REMOVE → [ 2 3 ]", "Position"),

        // Quantity operations
        ("LENGTH", "Get vector length", "[ 1 2 3 4 5 ] LENGTH → [ 1 2 3 4 5 ] [ 5 ]", "Quantity"),
        ("TAKE", "Take N elements from start or end", "[ 1 2 3 4 5 ] [ 3 ] TAKE → [ 1 2 3 ]", "Quantity"),

        // Vector operations
        ("SPLIT", "Split vector at specified sizes", "[ 1 2 3 4 5 6 ] [ 2 3 ] SPLIT → [ 1 2 ] [ 3 4 5 ] [ 6 ]", "Vector"),
        ("CONCAT", "Concatenate vectors", "[ 1 2 ] [ 3 4 ] CONCAT → [ 1 2 3 4 ]", "Vector"),
        ("REVERSE", "Reverse vector elements", "[ 1 2 3 ] REVERSE → [ 3 2 1 ]", "Vector"),
        ("RANGE", "Generate numeric range", "[ 0 5 ] RANGE → [ 0 1 2 3 4 5 ]", "Vector"),
        ("REORDER", "Reorder elements by index list", "[ a b c ] [ 2 0 1 ] REORDER → [ c a b ]", "Vector"),
        ("SORT", "Sort vector elements ascending", "[ 3 1 2 ] SORT → [ 1 2 3 ]", "Sorting"),

        // Constants
        ("TRUE", "Push TRUE to stack", "TRUE → TRUE", "Constant"),
        ("FALSE", "Push FALSE to stack", "FALSE → FALSE", "Constant"),
        ("NIL", "Push NIL (empty) to stack", "NIL → NIL", "Constant"),

        // String operations
        ("CHARS", "Split string into character vector", "[ 'hello' ] CHARS → [ 'h' 'e' 'l' 'l' 'o' ]", "String"),
        ("JOIN", "Join character vector into string", "[ 'h' 'e' 'l' 'l' 'o' ] JOIN → [ 'hello' ]", "String"),

        // Parse/Convert
        ("NUM", "Parse string to number", "'123' NUM → [ 123 ], returns NIL on failure", "Parse/Convert"),
        ("STR", "Convert value to string (Stringify)", "123 STR → '123'", "Parse/Convert"),
        ("BOOL", "Normalize to boolean", "'true' BOOL → TRUE, 100 BOOL → TRUE", "Parse/Convert"),
        ("CHR", "Convert number to Unicode character", "65 CHR → 'A'", "Parse/Convert"),

        // DateTime
        ("NOW", "Get current Unix timestamp", "NOW → [ 1732531200 ]", "DateTime"),
        ("DATETIME", "Convert timestamp to datetime vector (TZ required)", "[ 1732531200 ] 'LOCAL' DATETIME → [ 2024 11 25 23 0 0 ]", "DateTime"),
        ("TIMESTAMP", "Convert datetime vector to timestamp (TZ required)", "[ 2024 11 25 23 0 0 ] 'LOCAL' TIMESTAMP → [ 1732531200 ]", "DateTime"),

        // Arithmetic
        ("+", "Element-wise addition or aggregation", "[ 1 2 ] [ 3 4 ] + → [ 4 6 ]", "Arithmetic"),
        ("-", "Element-wise subtraction or aggregation", "[ 5 3 ] [ 2 1 ] - → [ 3 2 ]", "Arithmetic"),
        ("*", "Element-wise multiplication or aggregation", "[ 2 3 ] [ 4 5 ] * → [ 8 15 ]", "Arithmetic"),
        ("/", "Element-wise division or aggregation", "[ 10 20 ] [ 2 4 ] / → [ 5 5 ]", "Arithmetic"),

        // Comparison
        ("=", "Check if vectors are equal", "[ 1 2 ] [ 1 2 ] = → [ TRUE ]", "Comparison"),
        ("<", "Check if less than", "[ 1 ] [ 2 ] < → [ TRUE ]", "Comparison"),
        ("<=", "Check if less than or equal", "[ 1 ] [ 1 ] <= → [ TRUE ]", "Comparison"),
        (">", "Check if greater than", "[ 3 ] [ 2 ] > → [ TRUE ]", "Comparison"),
        (">=", "Check if greater than or equal", "[ 2 ] [ 2 ] >= → [ TRUE ]", "Comparison"),

        // Logic
        ("AND", "Logical AND", "[ TRUE FALSE ] [ TRUE TRUE ] AND → [ TRUE FALSE ]", "Logic"),
        ("OR", "Logical OR", "[ TRUE FALSE ] [ FALSE FALSE ] OR → [ TRUE FALSE ]", "Logic"),
        ("NOT", "Logical NOT", "[ TRUE FALSE ] NOT → [ FALSE TRUE ]", "Logic"),

        // Control (guards)
        (":", "Guard separator for conditional branching", "condition : then : condition : then : else", "Control"),

        // Higher-order functions
        ("MAP", "Apply word to each element", "[ 1 2 3 ] 'DOUBLE' MAP → [ 2 4 6 ]", "Higher-Order"),
        ("FILTER", "Filter elements by condition", "[ 1 2 3 4 ] 'EVEN?' FILTER → [ 2 4 ]", "Higher-Order"),
        ("FOLD", "Fold with initial value", "[ 1 2 3 4 ] [ 0 ] '+' FOLD → [ 10 ]", "Higher-Order"),

        // I/O
        ("PRINT", "Print and pop stack top", "[ 42 ] PRINT → (outputs 42)", "I/O"),

        // Music DSL
        ("SEQ", "Set sequential playback mode", "[ 440 550 660 ] SEQ PLAY → play 3 notes sequentially", "Music"),
        ("SIM", "Set simultaneous playback mode", "[ 440 550 660 ] SIM PLAY → play 3 notes as chord", "Music"),
        ("PLAY", "Play audio", "[ 440/2 550 NIL 660 ] PLAY → 440Hz for 2 slots, 550Hz, rest, 660Hz", "Music"),

        // Word management
        ("DEF", "Define custom word", "[ [ 2 ] * ] 'DOUBLE' DEF", "Word Management"),
        ("DEL", "Delete custom word", "'WORD' DEL", "Word Management"),
        ("?", "Show word definition", "'DOUBLE' ?", "Word Management"),

        // Control flow
        ("TIMES", "Repeat code N times", "[ [ 1 ] + ] [ 5 ] TIMES", "Control Flow"),
        ("WAIT", "Execute word after delay (ms)", "'PROCESS' [ 1000 ] WAIT", "Control Flow"),
        ("!", "Force flag - allow DEL/DEF of dependent words", "! 'WORD' DEL", "Control Flow"),

        // Math functions
        ("MOD", "Modulo (mathematical)", "[ 7 ] [ 3 ] MOD → [ 1 ]", "Math"),
        ("FLOOR", "Floor (toward negative infinity)", "[ 7/3 ] FLOOR → [ 2 ]", "Math"),
        ("CEIL", "Ceiling (toward positive infinity)", "[ 7/3 ] CEIL → [ 3 ]", "Math"),
        ("ROUND", "Round (away from zero)", "[ 5/2 ] ROUND → [ 3 ]", "Math"),

        // Cryptographic random
        ("CSPRNG", "Generate cryptographic random", "[ 6 ] [ 1 ] CSPRNG → [ 0 ] to [ 5/6 ], [ 5 ] CSPRNG → 5 randoms", "Random"),

        // Hash
        ("HASH", "Deterministic hash of any value", "'hello' HASH → [ 0.xxx ], [ 128 ] 'hello' HASH → 128-bit", "Hash"),
    ]
}
