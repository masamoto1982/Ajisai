#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinDetailGroup {
    Modifier,
    ArithmeticLogic,
    VectorOps,
    StringCast,
    ControlHigherOrder,
    Cond,
    IoModule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinExecutorKey {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Lt,
    Le,
    Map,
    Filter,
    Fold,
    Unfold,
    Any,
    All,
    Count,
    Scan,
    Get,
    Length,
    Concat,
    And,
    Or,
    Not,
    True,
    False,
    Nil,
    Idle,
    Exec,
    Eval,
    Cond,
    Def,
    Del,
    Lookup,
    Import,
    ImportOnly,
    Force,
    Print,
    Insert,
    Replace,
    Remove,
    Take,
    Split,
    Reverse,
    Range,
    Reorder,
    Collect,
    Sort,
    Shape,
    Rank,
    Reshape,
    Transpose,
    Fill,
    Floor,
    Ceil,
    Round,
    Sqrt,
    SqrtEps,
    Interval,
    Lower,
    Upper,
    Width,
    IsExact,
    Mod,
    Str,
    Num,
    Bool,
    Chr,
    Chars,
    Join,
    Now,
    Datetime,
    Timestamp,
    Csprng,
    Hash,
    Spawn,
    Await,
    Status,
    Kill,
    Monitor,
    Supervise,
}

#[derive(Clone, Copy, Debug)]
pub struct BuiltinSpec {
    pub name: &'static str,
    pub category: &'static str,
    pub short_description: &'static str,
    pub syntax: &'static str,
    pub signature_type: &'static str,
    pub detail_group: BuiltinDetailGroup,
    pub executor_key: Option<BuiltinExecutorKey>,
}

macro_rules! builtin_spec {
    ($name:expr, $category:expr, $description:expr, $syntax:expr, $signature:expr, $detail:expr, $executor:expr) => {
        BuiltinSpec {
            name: $name,
            category: $category,
            short_description: $description,
            syntax: $syntax,
            signature_type: $signature,
            detail_group: $detail,
            executor_key: $executor,
        }
    };
}

const BUILTIN_SPECS: &[BuiltinSpec] = &[
    builtin_spec!(
        "TOP",
        "modifier",
        "Set operation target to stack top",
        ". ADD → apply ADD to stack top",
        "none",
        BuiltinDetailGroup::Modifier,
        None
    ),
    builtin_spec!(
        "STAK",
        "modifier",
        "Set operation target to the whole stack",
        ".. ADD → apply ADD to the whole stack",
        "none",
        BuiltinDetailGroup::Modifier,
        None
    ),
    builtin_spec!(
        "EAT",
        "modifier",
        "Set consumption mode to consume operands",
        ", ADD → consume operands",
        "none",
        BuiltinDetailGroup::Modifier,
        None
    ),
    builtin_spec!(
        "KEEP",
        "modifier",
        "Set consumption mode to keep operands",
        ",, ADD → keep operands and append result",
        "none",
        BuiltinDetailGroup::Modifier,
        None
    ),
    builtin_spec!(
        "SAFE",
        "modifier",
        "Enable safe mode and return NIL on error",
        "~ GET → NIL on out-of-range access",
        "none",
        BuiltinDetailGroup::Modifier,
        None
    ),
    builtin_spec!(
        "FRAME",
        "vector",
        "Generate empty vector structure from shape. Shape Vector -> Vector",
        "[ 2 3 ] FRAME → [ [ ] [ ] [ ] ] [ [ ] [ ] [ ] ]",
        "none",
        BuiltinDetailGroup::VectorOps,
        None
    ),
    builtin_spec!(
        "GET",
        "vector",
        "Form: Extract element at index. Vector, Index Vector -> element",
        "[ 10 20 30 ] [ 0 ] GET → [ 10 ]",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Get)
    ),
    builtin_spec!(
        "INSERT",
        "vector",
        "Form: Insert element at index. Vector, [Index, Value] Vector -> Vector",
        "[ 1 3 ] [ 1 2 ] INSERT → [ 1 2 3 ]",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Insert)
    ),
    builtin_spec!(
        "REPLACE",
        "vector",
        "Form: Replace element at index. Vector, [Index, Value] Vector -> Vector",
        "[ 1 2 3 ] [ 0 9 ] REPLACE → [ 9 2 3 ]",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Replace)
    ),
    builtin_spec!(
        "REMOVE",
        "vector",
        "Form: Remove element at index. Vector, Index Vector -> Vector",
        "[ 1 2 3 ] [ 0 ] REMOVE → [ 2 3 ]",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Remove)
    ),
    builtin_spec!(
        "LENGTH",
        "vector",
        "Form: Return element count. Vector -> Scalar",
        "[ 1 2 3 4 5 ] LENGTH → [ 5 ]",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Length)
    ),
    builtin_spec!(
        "TAKE",
        "vector",
        "Form: Take N elements from start or end. Vector, Scalar -> Vector",
        "[ 1 2 3 4 5 ] [ 3 ] TAKE → [ 1 2 3 ]",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Take)
    ),
    builtin_spec!(
        "SPLIT",
        "vector",
        "Form: Split vector at specified sizes. Vector, Sizes Vector -> Vectors",
        "[ 1 2 3 4 5 6 ] [ 2 3 ] SPLIT → [ 1 2 ] [ 3 4 5 ] [ 6 ]",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Split)
    ),
    builtin_spec!(
        "CONCAT",
        "vector",
        "Form: Flatten-concatenate two vectors. Vector, Vector -> Vector",
        "[ 1 2 ] [ 3 4 ] CONCAT → [ 1 2 3 4 ]",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Concat)
    ),
    builtin_spec!(
        "REVERSE",
        "vector",
        "Form: Reverse element order. Vector -> Vector",
        "[ 1 2 3 ] REVERSE → [ 3 2 1 ]",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Reverse)
    ),
    builtin_spec!(
        "RANGE",
        "vector",
        "Form: Generate numeric sequence. [start, end] or [end] -> Vector",
        "[ 0 5 ] RANGE → [ 0 1 2 3 4 5 ]",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Range)
    ),
    builtin_spec!(
        "REORDER",
        "vector",
        "Form: Reorder elements by index list. Vector, Index Vector -> Vector",
        "[ a b c ] [ 2 0 1 ] REORDER → [ c a b ]",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Reorder)
    ),
    builtin_spec!(
        "COLLECT",
        "vector",
        "Collect N items from stack into vector. ...values, Scalar -> Vector",
        "1 2 3 3 COLLECT → [ 1 2 3 ]",
        "none",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Collect)
    ),
    builtin_spec!(
        "TRUE",
        "constant",
        "Push TRUE to stack",
        "TRUE → TRUE",
        "none",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::True)
    ),
    builtin_spec!(
        "FALSE",
        "constant",
        "Push FALSE to stack",
        "FALSE → FALSE",
        "none",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::False)
    ),
    builtin_spec!(
        "NIL",
        "constant",
        "Push NIL to stack",
        "NIL → NIL",
        "none",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Nil)
    ),
    builtin_spec!(
        "CHARS",
        "cast",
        "Map: Split string into character code vector. String -> Numeric Vector",
        "[ 'hello' ] CHARS → [ 'h' 'e' 'l' 'l' 'o' ]",
        "map",
        BuiltinDetailGroup::StringCast,
        Some(BuiltinExecutorKey::Chars)
    ),
    builtin_spec!(
        "JOIN",
        "cast",
        "Map: Join character code vector into string. Numeric Vector -> String",
        "[ 'h' 'e' 'l' 'l' 'o' ] JOIN → [ 'hello' ]",
        "map",
        BuiltinDetailGroup::StringCast,
        Some(BuiltinExecutorKey::Join)
    ),
    builtin_spec!(
        "NUM",
        "cast",
        "Map: Parse to number. Any -> Numeric (NIL on failure)",
        "'123' NUM → [ 123 ], returns NIL on failure",
        "map",
        BuiltinDetailGroup::StringCast,
        Some(BuiltinExecutorKey::Num)
    ),
    builtin_spec!(
        "STR",
        "cast",
        "Map: Convert to string representation. Any -> String",
        "123 STR → '123'",
        "map",
        BuiltinDetailGroup::StringCast,
        Some(BuiltinExecutorKey::Str)
    ),
    builtin_spec!(
        "BOOL",
        "cast",
        "Map: Convert to boolean. Any -> Boolean",
        "'true' BOOL → TRUE, 100 BOOL → TRUE",
        "map",
        BuiltinDetailGroup::StringCast,
        Some(BuiltinExecutorKey::Bool)
    ),
    builtin_spec!(
        "CHR",
        "cast",
        "Map: Convert character code to single-char string. Numeric -> String",
        "65 CHR → 'A'",
        "map",
        BuiltinDetailGroup::StringCast,
        Some(BuiltinExecutorKey::Chr)
    ),
    builtin_spec!(
        "ADD",
        "arithmetic",
        "Fold: add values with broadcasting where supported",
        "[ 1 2 ] [ 3 4 ] + → [ 4 6 ]",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Add)
    ),
    builtin_spec!(
        "SUB",
        "arithmetic",
        "Fold: subtract values with broadcasting where supported",
        "[ 5 3 ] [ 2 1 ] - → [ 3 2 ]",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Sub)
    ),
    builtin_spec!(
        "MUL",
        "arithmetic",
        "Fold: multiply values with broadcasting where supported",
        "[ 2 3 ] [ 4 5 ] * → [ 8 15 ]",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Mul)
    ),
    builtin_spec!(
        "DIV",
        "arithmetic",
        "Fold: divide values with broadcasting where supported",
        "[ 10 20 ] [ 2 4 ] / → [ 5 5 ]",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Div)
    ),
    builtin_spec!(
        "EQ",
        "comparison",
        "Fold: equality comparison with broadcasting where supported",
        "[ 1 2 ] [ 1 2 ] = → [ TRUE ]",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Eq)
    ),
    builtin_spec!(
        "LT",
        "comparison",
        "Fold: less-than comparison with broadcasting where supported",
        "[ 1 ] [ 2 ] < → [ TRUE ]",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Lt)
    ),
    builtin_spec!(
        "LTE",
        "comparison",
        "Fold: less-than-or-equal comparison with broadcasting where supported",
        "[ 1 ] [ 1 ] <= → [ TRUE ]",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Le)
    ),
    builtin_spec!(
        "AND",
        "logic",
        "Fold: Logical AND (Kleene three-valued). Alias: & (postfix sugar). Boolean, Boolean -> Boolean",
        "[ TRUE FALSE ] [ TRUE TRUE ] AND → [ TRUE FALSE ], [ TRUE FALSE ] [ TRUE TRUE ] & → [ TRUE FALSE ]",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::And)
    ),
    builtin_spec!(
        "OR",
        "logic",
        "Fold: Logical OR (Kleene three-valued). Boolean, Boolean -> Boolean",
        "[ TRUE FALSE ] [ FALSE FALSE ] OR → [ TRUE FALSE ]",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Or)
    ),
    builtin_spec!(
        "NOT",
        "logic",
        "Map: Logical negation. Boolean -> Boolean",
        "[ TRUE FALSE ] NOT → [ FALSE TRUE ]",
        "map",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Not)
    ),
    builtin_spec!(
        "IDLE",
        "control",
        "Pass through flow unchanged (no-op)",
        "IDLE",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Idle)
    ),
    builtin_spec!(
        "COND",
        "control",
        "Form: Evaluate guard/body clauses, execute first match. Any, ...CodeBlock clauses -> Any",
        "value { guard1 $ body1 } { IDLE $ else_body } COND",
        "form",
        BuiltinDetailGroup::Cond,
        Some(BuiltinExecutorKey::Cond)
    ),
    builtin_spec!(
        "==",
        "modifier",
        "Pipeline visual marker (no-op)",
        "[ 1 2 3 ] == { [ 2 ] * } MAP",
        "none",
        BuiltinDetailGroup::Modifier,
        None
    ),
    builtin_spec!(
        "=>",
        "modifier",
        "Nil coalescing. Return alternative if NIL. Any, Any -> Any",
        "NIL => [ 0 ] → [ 0 ]",
        "none",
        BuiltinDetailGroup::Modifier,
        None
    ),
    builtin_spec!(
        "MAP",
        "higher-order",
        "Form: Apply code block to each element. Vector, CodeBlock -> Vector",
        "[ 1 2 3 ] { [ 2 ] * } MAP → [ 2 4 6 ]",
        "form",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Map)
    ),
    builtin_spec!(
        "FILTER",
        "higher-order",
        "Form: Extract elements matching predicate. Vector, CodeBlock -> Vector",
        "[ 1 2 3 4 ] { [ 2 ] MOD [ 0 ] = } FILTER → [ 2 4 ]",
        "form",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Filter)
    ),
    builtin_spec!(
        "FOLD",
        "higher-order",
        "Form: Reduce with initial value. Vector, Scalar, CodeBlock -> Scalar",
        "[ 1 2 3 4 ] [ 0 ] { + } FOLD → [ 10 ]",
        "form",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Fold)
    ),
    builtin_spec!(
        "UNFOLD",
        "higher-order",
        "Form: Generate sequence from state transition. State, CodeBlock -> Vector/NIL",
        "[ 1 ] { { [ 1 ] = } { [ 1 2 ] } { [ 2 ] = } { [ 2 3 ] } { [ 3 ] = } { [ 3 NIL ] } { IDLE } { NIL } COND } UNFOLD",
        "form",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Unfold)
    ),
    builtin_spec!(
        "ANY",
        "higher-order",
        "Form: Return TRUE if any element satisfies predicate. Vector, CodeBlock -> Boolean",
        "[ 1 3 5 8 ] { [ 2 ] MOD [ 0 ] = } ANY → TRUE",
        "form",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Any)
    ),
    builtin_spec!(
        "ALL",
        "higher-order",
        "Form: Return TRUE if all elements satisfy predicate. Vector, CodeBlock -> Boolean",
        "[ 2 4 6 8 ] { [ 2 ] MOD [ 0 ] = } ALL → TRUE",
        "form",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::All)
    ),
    builtin_spec!(
        "COUNT",
        "higher-order",
        "Form: Count elements satisfying predicate. Vector, CodeBlock -> Scalar",
        "[ 1 2 3 4 5 6 ] { [ 2 ] MOD [ 0 ] = } COUNT → [ 3 ]",
        "form",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Count)
    ),
    builtin_spec!(
        "SCAN",
        "higher-order",
        "Form: Return intermediate fold accumulators. Vector, Scalar, CodeBlock -> Vector/NIL",
        "[ 1 2 3 4 ] [ 0 ] { + } SCAN → [ 1 3 6 10 ]",
        "form",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Scan)
    ),
    builtin_spec!(
        "PRINT",
        "io",
        "Map: Output value to display. Any -> Any",
        "[ 42 ] PRINT → (outputs 42)",
        "map",
        BuiltinDetailGroup::IoModule,
        Some(BuiltinExecutorKey::Print)
    ),
    builtin_spec!(
        "DEF",
        "dictionary",
        "Define user word in dictionary. CodeBlock, String [, String] -> (dictionary effect)",
        "{ [ 2 ] * } 'DOUBLE' DEF",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Def)
    ),
    builtin_spec!(
        "DEL",
        "dictionary",
        "Delete user word from dictionary. String -> (dictionary effect)",
        "'WORD' DEL",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Del)
    ),
    builtin_spec!(
        "?",
        "dictionary",
        "Display word definition and details. String -> (output effect)",
        "'DOUBLE' ?",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Lookup)
    ),
    builtin_spec!(
        "FORC",
        "control",
        "Force destructive dictionary operations such as DEF and DEL",
        "! 'WORD' DEL",
        "none",
        BuiltinDetailGroup::Modifier,
        Some(BuiltinExecutorKey::Force)
    ),
    builtin_spec!(
        "SHAPE",
        "tensor",
        "Map: Return vector shape. Vector -> Numeric Vector",
        "[ 1 2 3 ] SHAPE → [ 3 ]",
        "map",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Shape)
    ),
    builtin_spec!(
        "RANK",
        "tensor",
        "Map: Return number of dimensions. Vector -> Scalar",
        "[ [ 1 2 ] [ 3 4 ] ] RANK → [ 2 ]",
        "map",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Rank)
    ),
    builtin_spec!(
        "RESHAPE",
        "tensor",
        "Form: Reshape vector to specified shape. Vector, Shape Vector -> Vector",
        "[ 1 2 3 4 5 6 ] [ 2 3 ] RESHAPE → { ( 1 2 3 ) ( 4 5 6 ) }",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Reshape)
    ),
    builtin_spec!(
        "TRANSPOSE",
        "tensor",
        "Form: Transpose vector axes. Vector -> Vector",
        "{ ( 1 2 3 ) ( 4 5 6 ) } TRANSPOSE → { ( 1 4 ) ( 2 5 ) ( 3 6 ) }",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Transpose)
    ),
    builtin_spec!(
        "FILL",
        "tensor",
        "Form: Fill shape with value. Scalar, Shape Vector -> Vector",
        "[ 2 3 0 ] FILL → { ( 0 0 0 ) ( 0 0 0 ) }",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Fill)
    ),
    builtin_spec!(
        "SQRT",
        "arithmetic",
        "Map: Square root. Exact rational roots stay exact; otherwise returns sound interval.",
        "[ 2 ] SQRT → [lo, hi]",
        "map",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Sqrt)
    ),
    builtin_spec!(
        "SQRT_EPS",
        "arithmetic",
        "Form: Square root with explicit interval width bound eps.",
        "[ 2 ] [ 1/100 ] SQRT_EPS → interval width <= 1/100",
        "form",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::SqrtEps)
    ),
    builtin_spec!(
        "INTERVAL",
        "arithmetic",
        "Form: Create interval [lo, hi].",
        "[ 1 ] [ 2 ] INTERVAL → [1, 2]",
        "form",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Interval)
    ),
    builtin_spec!(
        "LOWER",
        "arithmetic",
        "Map: Lower endpoint of number/interval.",
        "[1, 2] LOWER → [ 1 ]",
        "map",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Lower)
    ),
    builtin_spec!(
        "UPPER",
        "arithmetic",
        "Map: Upper endpoint of number/interval.",
        "[1, 2] UPPER → [ 2 ]",
        "map",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Upper)
    ),
    builtin_spec!(
        "WIDTH",
        "arithmetic",
        "Map: Interval width hi-lo.",
        "[1, 2] WIDTH → [ 1 ]",
        "map",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Width)
    ),
    builtin_spec!(
        "IS_EXACT",
        "arithmetic",
        "Map: True for exact number or degenerate interval.",
        "[1, 1] IS_EXACT → TRUE",
        "map",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::IsExact)
    ),
    builtin_spec!(
        "MOD",
        "arithmetic",
        "Fold: Modulo (broadcast). Alias: % (postfix sugar). Numeric, Numeric -> Numeric",
        "[ 7 ] [ 3 ] MOD → [ 1 ], [ 7 ] [ 3 ] % → [ 1 ]",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Mod)
    ),
    builtin_spec!(
        "FLOOR",
        "arithmetic",
        "Map: Floor (round toward negative infinity). Numeric -> Numeric",
        "[ 7/3 ] FLOOR → [ 2 ]",
        "map",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Floor)
    ),
    builtin_spec!(
        "CEIL",
        "arithmetic",
        "Map: Ceiling (round toward positive infinity). Numeric -> Numeric",
        "[ 7/3 ] CEIL → [ 3 ]",
        "map",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Ceil)
    ),
    builtin_spec!(
        "ROUND",
        "arithmetic",
        "Map: Round to nearest integer. Numeric -> Numeric",
        "[ 5/2 ] ROUND → [ 3 ]",
        "map",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Round)
    ),
    builtin_spec!(
        "EXEC",
        "control",
        "Interpret vector as code and execute. Vector -> (execution result)",
        "[ 1 2 + ] EXEC → 3, 1 2 + .. EXEC → 3",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Exec)
    ),
    builtin_spec!(
        "EVAL",
        "control",
        "Parse string as code and execute. String -> (execution result)",
        "'1 2 +' EVAL → 3",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Eval)
    ),
    builtin_spec!(
        "IMPORT",
        "module",
        "Load standard library module. String -> (dictionary effect)",
        "'music' IMPORT → imports all public words from MUSIC",
        "none",
        BuiltinDetailGroup::IoModule,
        Some(BuiltinExecutorKey::Import)
    ),
    builtin_spec!(
        "IMPORT-ONLY",
        "module",
        "Import selected public words from a module. String Vector -> (dictionary effect)",
        "'json' [ 'parse' ] IMPORT-ONLY → imports JSON@PARSE only",
        "none",
        BuiltinDetailGroup::IoModule,
        Some(BuiltinExecutorKey::ImportOnly)
    ),
    builtin_spec!(
        "SPAWN",
        "control",
        "Spawn an isolated child runtime from a code block. Block -> ProcessHandle",
        "{ [ 1 ] [ 0 ] / } SPAWN",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Spawn)
    ),
    builtin_spec!(
        "AWAIT",
        "control",
        "Run/wait a child runtime and return exit tuple.",
        "{ ... } SPAWN AWAIT → [ 'completed' [ ... ] ] / [ 'failed' [ ... ] ]",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Await)
    ),
    builtin_spec!(
        "STATUS",
        "control",
        "Read child status. ProcessHandle -> String",
        "{ ... } SPAWN STATUS → 'running'|'completed'|'failed'|'killed'|'timeout'",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Status)
    ),
    builtin_spec!(
        "KILL",
        "control",
        "Force child termination. ProcessHandle -> 'killed'",
        "{ ... } SPAWN KILL",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Kill)
    ),
    builtin_spec!(
        "MONITOR",
        "control",
        "Register monitor on a child handle.",
        "{ ... } SPAWN MONITOR",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Monitor)
    ),
    builtin_spec!(
        "SUPERVISE",
        "control",
        "Run block under one_for_one restart policy.",
        "{ unstable } [ 3 ] SUPERVISE",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Supervise)
    ),
];

pub fn builtin_specs() -> &'static [BuiltinSpec] {
    BUILTIN_SPECS
}

pub fn lookup_builtin_spec(name: &str) -> Option<&'static BuiltinSpec> {
    let canonical = crate::core_word_aliases::canonicalize_core_word_name(name);
    BUILTIN_SPECS.iter().find(|spec| spec.name == canonical)
}

#[allow(dead_code)]
pub fn collect_builtin_definitions() -> Vec<(&'static str, &'static str, &'static str, &'static str)>
{
    BUILTIN_SPECS
        .iter()
        .map(|spec| {
            (
                spec.name,
                spec.short_description,
                spec.syntax,
                spec.signature_type,
            )
        })
        .collect()
}

#[allow(dead_code)]
pub fn collect_core_builtin_definitions(
) -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    const CORE_WORDS: &[&str] = &[
        "ADD", "SUB", "MUL", "DIV", "MOD", "EQ", "LT", "LTE", "TOP", "STAK", "EAT", "KEEP", "SAFE",
        "FORC",
    ];
    BUILTIN_SPECS
        .iter()
        .filter(|spec| CORE_WORDS.contains(&spec.name))
        .map(|spec| {
            (
                spec.name,
                spec.short_description,
                spec.syntax,
                spec.signature_type,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn builtin_specs_do_not_contain_symbol_aliases_or_input_helpers() {
        let forbidden = [
            "+", "-", "*", "/", "%", "=", "<", "<=", ".", "..", ",", ",,", "~", "!", "'", "$",
        ];

        for spec in super::builtin_specs() {
            assert!(
                !forbidden.contains(&spec.name),
                "builtin spec must not contain symbol/helper word: {}",
                spec.name
            );
        }
    }

    #[test]
    fn builtin_specs_contain_canonical_core_words() {
        let required = [
            "ADD", "SUB", "MUL", "DIV", "MOD", "EQ", "LT", "LTE", "TOP", "STAK", "EAT", "KEEP",
            "SAFE", "FORC",
        ];

        for name in required {
            assert!(
                super::lookup_builtin_spec(name).is_some(),
                "missing canonical core word: {}",
                name
            );
        }
    }
}
