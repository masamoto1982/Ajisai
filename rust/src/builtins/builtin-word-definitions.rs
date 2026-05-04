#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
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
    Shape,
    Rank,
    Reshape,
    Transpose,
    Fill,
    Floor,
    Ceil,
    Round,
    Mod,
    Str,
    Num,
    Bool,
    Chr,
    Chars,
    Join,
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
    /// Layer 3 (hover): one-line "WORD — short verb phrase" shown in the
    /// native button title attribute. See three-layer-documentation-model.md
    /// §4.2.
    pub hover_summary: &'static str,
    /// Layer 3 (hover): shortest useful invocation (operands included, sugar
    /// preferred when shorter) shown in the inline word-info strip. See
    /// three-layer-documentation-model.md §4.3.
    pub hover_syntax: &'static str,
    pub signature_type: &'static str,
    #[allow(dead_code)]
    pub detail_group: BuiltinDetailGroup,
    pub executor_key: Option<BuiltinExecutorKey>,
}

macro_rules! builtin_spec {
    ($name:expr, $category:expr, $hover_summary:expr, $hover_syntax:expr, $signature:expr, $detail:expr, $executor:expr) => {
        BuiltinSpec {
            name: $name,
            category: $category,
            hover_summary: $hover_summary,
            hover_syntax: $hover_syntax,
            signature_type: $signature,
            detail_group: $detail,
            executor_key: $executor,
        }
    };
}

// Hover text follows three-layer-documentation-model.md §4.2/§4.3:
//   hover_summary = "WORD — short verb phrase"
//   hover_syntax  = shortest useful invocation, sugar preferred when shorter
const BUILTIN_SPECS: &[BuiltinSpec] = &[
    builtin_spec!(
        "TOP",
        "modifier",
        "TOP — apply operation to stack top",
        ". +",
        "none",
        BuiltinDetailGroup::Modifier,
        None
    ),
    builtin_spec!(
        "STAK",
        "modifier",
        "STAK — apply operation to whole stack",
        ".. +",
        "none",
        BuiltinDetailGroup::Modifier,
        None
    ),
    builtin_spec!(
        "EAT",
        "modifier",
        "EAT — consume operands",
        ", +",
        "none",
        BuiltinDetailGroup::Modifier,
        None
    ),
    builtin_spec!(
        "KEEP",
        "modifier",
        "KEEP — keep operands and append result",
        ",, +",
        "none",
        BuiltinDetailGroup::Modifier,
        None
    ),
    builtin_spec!(
        "SAFE",
        "modifier",
        "SAFE — return NIL on error",
        "~ GET",
        "none",
        BuiltinDetailGroup::Modifier,
        None
    ),
    builtin_spec!(
        "GET",
        "vector",
        "GET — extract element at index",
        "[ 10 20 30 ] [ 0 ] GET",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Get)
    ),
    builtin_spec!(
        "INSERT",
        "vector",
        "INSERT — insert element at index",
        "[ 1 3 ] [ 1 2 ] INSERT",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Insert)
    ),
    builtin_spec!(
        "REPLACE",
        "vector",
        "REPLACE — replace element at index",
        "[ 1 2 3 ] [ 0 9 ] REPLACE",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Replace)
    ),
    builtin_spec!(
        "REMOVE",
        "vector",
        "REMOVE — remove element at index",
        "[ 1 2 3 ] [ 0 ] REMOVE",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Remove)
    ),
    builtin_spec!(
        "LENGTH",
        "vector",
        "LENGTH — return element count",
        "[ 1 2 3 ] LENGTH",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Length)
    ),
    builtin_spec!(
        "TAKE",
        "vector",
        "TAKE — take N elements from start or end",
        "[ 1 2 3 4 5 ] [ 3 ] TAKE",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Take)
    ),
    builtin_spec!(
        "SPLIT",
        "vector",
        "SPLIT — split vector at sizes",
        "[ 1 2 3 4 ] [ 2 2 ] SPLIT",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Split)
    ),
    builtin_spec!(
        "CONCAT",
        "vector",
        "CONCAT — flatten and concatenate vectors",
        "[ 1 2 ] [ 3 4 ] CONCAT",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Concat)
    ),
    builtin_spec!(
        "REVERSE",
        "vector",
        "REVERSE — reverse element order",
        "[ 1 2 3 ] REVERSE",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Reverse)
    ),
    builtin_spec!(
        "RANGE",
        "vector",
        "RANGE — generate numeric sequence",
        "[ 0 5 ] RANGE",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Range)
    ),
    builtin_spec!(
        "REORDER",
        "vector",
        "REORDER — reorder by index list",
        "[ 'a' 'b' 'c' ] [ 2 0 1 ] REORDER",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Reorder)
    ),
    builtin_spec!(
        "COLLECT",
        "vector",
        "COLLECT — collect N items into vector",
        "1 2 3 3 COLLECT",
        "none",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Collect)
    ),
    builtin_spec!(
        "TRUE",
        "constant",
        "TRUE — push TRUE",
        "TRUE",
        "none",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::True)
    ),
    builtin_spec!(
        "FALSE",
        "constant",
        "FALSE — push FALSE",
        "FALSE",
        "none",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::False)
    ),
    builtin_spec!(
        "NIL",
        "constant",
        "NIL — push NIL",
        "NIL",
        "none",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Nil)
    ),
    builtin_spec!(
        "CHARS",
        "cast",
        "CHARS — split string into characters",
        "'hi' CHARS",
        "map",
        BuiltinDetailGroup::StringCast,
        Some(BuiltinExecutorKey::Chars)
    ),
    builtin_spec!(
        "JOIN",
        "cast",
        "JOIN — join characters into string",
        "[ 'h' 'i' ] JOIN",
        "map",
        BuiltinDetailGroup::StringCast,
        Some(BuiltinExecutorKey::Join)
    ),
    builtin_spec!(
        "NUM",
        "cast",
        "NUM — parse to number",
        "'42' NUM",
        "map",
        BuiltinDetailGroup::StringCast,
        Some(BuiltinExecutorKey::Num)
    ),
    builtin_spec!(
        "STR",
        "cast",
        "STR — convert to string",
        "42 STR",
        "map",
        BuiltinDetailGroup::StringCast,
        Some(BuiltinExecutorKey::Str)
    ),
    builtin_spec!(
        "BOOL",
        "cast",
        "BOOL — convert to boolean",
        "1 BOOL",
        "map",
        BuiltinDetailGroup::StringCast,
        Some(BuiltinExecutorKey::Bool)
    ),
    builtin_spec!(
        "CHR",
        "cast",
        "CHR — convert code to character",
        "65 CHR",
        "map",
        BuiltinDetailGroup::StringCast,
        Some(BuiltinExecutorKey::Chr)
    ),
    builtin_spec!(
        "ADD",
        "arithmetic",
        "ADD — add values",
        "1 2 +",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Add)
    ),
    builtin_spec!(
        "SUB",
        "arithmetic",
        "SUB — subtract values",
        "5 3 -",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Sub)
    ),
    builtin_spec!(
        "MUL",
        "arithmetic",
        "MUL — multiply values",
        "2 4 *",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Mul)
    ),
    builtin_spec!(
        "DIV",
        "arithmetic",
        "DIV — divide values",
        "10 2 /",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Div)
    ),
    builtin_spec!(
        "EQ",
        "comparison",
        "EQ — test equality",
        "1 1 =",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Eq)
    ),
    builtin_spec!(
        "LT",
        "comparison",
        "LT — test less than",
        "1 2 <",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Lt)
    ),
    builtin_spec!(
        "LTE",
        "comparison",
        "LTE — test less than or equal",
        "1 1 <=",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Le)
    ),
    builtin_spec!(
        "AND",
        "logic",
        "AND — logical AND",
        "TRUE TRUE &",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::And)
    ),
    builtin_spec!(
        "OR",
        "logic",
        "OR — logical OR",
        "TRUE FALSE OR",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Or)
    ),
    builtin_spec!(
        "NOT",
        "logic",
        "NOT — logical negation",
        "TRUE NOT",
        "map",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Not)
    ),
    builtin_spec!(
        "IDLE",
        "control",
        "IDLE — pass through unchanged",
        "IDLE",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Idle)
    ),
    builtin_spec!(
        "COND",
        "control",
        "COND — evaluate guard/body clauses",
        "1 { TRUE $ 'y' } { IDLE $ 'n' } COND",
        "form",
        BuiltinDetailGroup::Cond,
        Some(BuiltinExecutorKey::Cond)
    ),
    builtin_spec!(
        "PIPE",
        "modifier",
        "PIPE — pipeline marker",
        "xs == { ... } MAP",
        "none",
        BuiltinDetailGroup::Modifier,
        None
    ),
    builtin_spec!(
        "OR-NIL",
        "modifier",
        "OR-NIL — coalesce NIL to alternative",
        "NIL => [ 0 ]",
        "none",
        BuiltinDetailGroup::Modifier,
        None
    ),
    builtin_spec!(
        "MAP",
        "higher-order",
        "MAP — apply block to each element",
        "[ 1 2 3 ] { [ 2 ] * } MAP",
        "form",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Map)
    ),
    builtin_spec!(
        "FILTER",
        "higher-order",
        "FILTER — keep elements matching predicate",
        "[ 1 2 3 ] { [ 2 ] = } FILTER",
        "form",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Filter)
    ),
    builtin_spec!(
        "FOLD",
        "higher-order",
        "FOLD — reduce with initial value",
        "[ 1 2 3 ] [ 0 ] { + } FOLD",
        "form",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Fold)
    ),
    builtin_spec!(
        "UNFOLD",
        "higher-order",
        "UNFOLD — generate from state transition",
        "[ 1 ] { ... COND } UNFOLD",
        "form",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Unfold)
    ),
    builtin_spec!(
        "ANY",
        "higher-order",
        "ANY — true if any element matches",
        "[ 1 2 3 ] { [ 2 ] = } ANY",
        "form",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Any)
    ),
    builtin_spec!(
        "ALL",
        "higher-order",
        "ALL — true if all elements match",
        "[ 2 4 ] { [ 2 ] MOD [ 0 ] = } ALL",
        "form",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::All)
    ),
    builtin_spec!(
        "COUNT",
        "higher-order",
        "COUNT — count matching elements",
        "[ 1 2 3 ] { [ 2 ] = } COUNT",
        "form",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Count)
    ),
    builtin_spec!(
        "SCAN",
        "higher-order",
        "SCAN — return intermediate fold results",
        "[ 1 2 3 ] [ 0 ] { + } SCAN",
        "form",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Scan)
    ),
    builtin_spec!(
        "PRINT",
        "io",
        "PRINT — output value to display",
        "42 PRINT",
        "map",
        BuiltinDetailGroup::IoModule,
        Some(BuiltinExecutorKey::Print)
    ),
    builtin_spec!(
        "DEF",
        "dictionary",
        "DEF — define user word",
        "{ 2 * } 'DOUBLE' DEF",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Def)
    ),
    builtin_spec!(
        "DEL",
        "dictionary",
        "DEL — delete user word",
        "'WORD' DEL",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Del)
    ),
    builtin_spec!(
        "LOOKUP",
        "dictionary",
        "LOOKUP — show word documentation",
        "'ADD' ?",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Lookup)
    ),
    builtin_spec!(
        "FORC",
        "control",
        "FORC — force destructive operation",
        "! 'WORD' DEL",
        "none",
        BuiltinDetailGroup::Modifier,
        Some(BuiltinExecutorKey::Force)
    ),
    builtin_spec!(
        "SHAPE",
        "tensor",
        "SHAPE — return vector shape",
        "[ 1 2 3 ] SHAPE",
        "map",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Shape)
    ),
    builtin_spec!(
        "RANK",
        "tensor",
        "RANK — return number of dimensions",
        "[ [ 1 2 ] ] RANK",
        "map",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Rank)
    ),
    builtin_spec!(
        "RESHAPE",
        "tensor",
        "RESHAPE — reshape to specified shape",
        "[ 1 2 3 4 ] [ 2 2 ] RESHAPE",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Reshape)
    ),
    builtin_spec!(
        "TRANSPOSE",
        "tensor",
        "TRANSPOSE — transpose vector axes",
        "[ ( 1 2 ) ( 3 4 ) ] TRANSPOSE",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Transpose)
    ),
    builtin_spec!(
        "FILL",
        "tensor",
        "FILL — fill shape with value",
        "[ 2 2 0 ] FILL",
        "form",
        BuiltinDetailGroup::VectorOps,
        Some(BuiltinExecutorKey::Fill)
    ),
    builtin_spec!(
        "MOD",
        "arithmetic",
        "MOD — modulo",
        "7 3 %",
        "fold",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Mod)
    ),
    builtin_spec!(
        "FLOOR",
        "arithmetic",
        "FLOOR — round toward negative infinity",
        "[ 7/3 ] FLOOR",
        "map",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Floor)
    ),
    builtin_spec!(
        "CEIL",
        "arithmetic",
        "CEIL — round toward positive infinity",
        "[ 7/3 ] CEIL",
        "map",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Ceil)
    ),
    builtin_spec!(
        "ROUND",
        "arithmetic",
        "ROUND — round to nearest integer",
        "[ 5/2 ] ROUND",
        "map",
        BuiltinDetailGroup::ArithmeticLogic,
        Some(BuiltinExecutorKey::Round)
    ),
    builtin_spec!(
        "EXEC",
        "control",
        "EXEC — execute vector as code",
        "[ 1 2 + ] EXEC",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Exec)
    ),
    builtin_spec!(
        "EVAL",
        "control",
        "EVAL — parse and execute string",
        "'1 2 +' EVAL",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Eval)
    ),
    builtin_spec!(
        "IMPORT",
        "module",
        "IMPORT — load module",
        "'IO' IMPORT",
        "none",
        BuiltinDetailGroup::IoModule,
        Some(BuiltinExecutorKey::Import)
    ),
    builtin_spec!(
        "IMPORT-ONLY",
        "module",
        "IMPORT-ONLY — import selected words",
        "'json' [ 'parse' ] IMPORT-ONLY",
        "none",
        BuiltinDetailGroup::IoModule,
        Some(BuiltinExecutorKey::ImportOnly)
    ),
    builtin_spec!(
        "SPAWN",
        "control",
        "SPAWN — spawn isolated child runtime",
        "{ 1 2 + } SPAWN",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Spawn)
    ),
    builtin_spec!(
        "AWAIT",
        "control",
        "AWAIT — wait for child runtime",
        "{ 1 2 + } SPAWN AWAIT",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Await)
    ),
    builtin_spec!(
        "STATUS",
        "control",
        "STATUS — read child status",
        "{ 1 2 + } SPAWN STATUS",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Status)
    ),
    builtin_spec!(
        "KILL",
        "control",
        "KILL — terminate child runtime",
        "{ 1 2 + } SPAWN KILL",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Kill)
    ),
    builtin_spec!(
        "MONITOR",
        "control",
        "MONITOR — register monitor on child",
        "{ 1 2 + } SPAWN MONITOR",
        "none",
        BuiltinDetailGroup::ControlHigherOrder,
        Some(BuiltinExecutorKey::Monitor)
    ),
    builtin_spec!(
        "SUPERVISE",
        "control",
        "SUPERVISE — run under restart policy",
        "{ 1 2 + } [ 3 ] SUPERVISE",
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

/// WASM/GUI tuple shape: `(name, hover_summary, hover_syntax, signature_type)`.
/// Position 1 (`hover_summary`) is the native button-title text;
/// position 2 (`hover_syntax`) is the inline word-info preview.
/// See three-layer-documentation-model.md §4.
#[allow(dead_code)]
pub fn collect_builtin_definitions() -> Vec<(&'static str, &'static str, &'static str, &'static str)>
{
    BUILTIN_SPECS
        .iter()
        .map(|spec| {
            (
                spec.name,
                spec.hover_summary,
                spec.hover_syntax,
                spec.signature_type,
            )
        })
        .collect()
}

pub fn collect_core_builtin_definitions(
) -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    BUILTIN_SPECS
        .iter()
        .map(|spec| {
            (
                spec.name,
                spec.hover_summary,
                spec.hover_syntax,
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
            "?", "==", "=>",
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
            "SAFE", "FORC", "LOOKUP", "PIPE", "OR-NIL",
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
