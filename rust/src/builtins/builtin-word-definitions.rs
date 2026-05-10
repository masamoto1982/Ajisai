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
    Unimport,
    UnimportOnly,
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
    Precompute,
}

#[derive(Clone, Copy, Debug)]
pub struct BuiltinSyntaxDoc {
    pub canonical: &'static str,
    pub shorthand: Option<&'static str>,
    pub description: Option<&'static str>,
}

#[derive(Clone, Copy, Debug)]
pub struct BuiltinExampleDoc {
    pub canonical: &'static str,
    pub shorthand: Option<&'static str>,
    pub result: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordShape {
    Map,
    Form,
    Fold,
    Other,
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
    #[allow(dead_code)]
    pub word_shape: WordShape,
    #[allow(dead_code)]
    pub detail_group: BuiltinDetailGroup,
    pub executor_key: Option<BuiltinExecutorKey>,

    // Layer 2 (LOOKUP) fields. See three-layer-documentation-model.md §3.4.
    pub summary: &'static str,
    pub role: Option<&'static str>,
    pub syntax_forms: &'static [BuiltinSyntaxDoc],
    pub stack_effect: &'static str,
    pub behavior: &'static str,
    pub examples: &'static [BuiltinExampleDoc],
    pub failure: Option<&'static str>,
    pub side_effects: &'static [&'static str],
    pub modifier_interaction: Option<&'static str>,
    pub related: &'static [&'static str],
    /// Must agree with the §7.14 contract metadata in
    /// `coreword_registry::apply_contract_overrides`. The mapping is:
    ///   safety_level A or B  -> "stable"
    ///   safety_level C or D  -> "experimental"
    ///   safety_level Quarantined -> "experimental"
    /// A consistency test asserts this invariant.
    pub stability: &'static str,
}

const SPEC_DEFAULT: BuiltinSpec = BuiltinSpec {
    name: "",
    category: "",
    hover_summary: "",
    hover_syntax: "",
    word_shape: WordShape::Other,
    detail_group: BuiltinDetailGroup::Modifier,
    executor_key: None,
    summary: "",
    role: None,
    syntax_forms: &[],
    stack_effect: "",
    behavior: "",
    examples: &[],
    failure: None,
    side_effects: &[],
    modifier_interaction: None,
    related: &[],
    stability: "stable",
};

const BUILTIN_SPECS: &[BuiltinSpec] = &[
    // === Modifiers ===
    BuiltinSpec {
        name: "TOP",
        category: "modifier",
        hover_summary: "TOP — apply operation to stack top",
        hover_syntax: ". +",
        detail_group: BuiltinDetailGroup::Modifier,
        summary: "Set the operation target mode to the top of the stack.",
        role: Some(
            "Modifier that scopes the next word's effect to the topmost stack entry.",
        ),
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "TOP <next-word>",
            shorthand: Some(". <next-word>"),
            description: Some(
                "Prefix any word to scope its effect to the top of the stack.",
            ),
        }],
        stack_effect: "no values popped or pushed",
        behavior:
            "Sets operation_target_mode to StackTop. The mode applies to\nthe next word and resets after that word executes.",
        examples: &[BuiltinExampleDoc {
            canonical: "1 2 3 TOP ADD",
            shorthand: Some("1 2 3 . +"),
            result: Some("next ADD operates on the top stack entry"),
        }],
        side_effects: &["Sets the next-word target mode."],
        related: &["STAK", "EAT", "KEEP", "SAFE"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "STAK",
        category: "modifier",
        hover_summary: "STAK — apply operation to whole stack",
        hover_syntax: ".. +",
        detail_group: BuiltinDetailGroup::Modifier,
        summary: "Set the operation target mode to the whole stack.",
        role: Some(
            "Modifier that scopes the next word's effect across all stack entries.",
        ),
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "STAK <next-word>",
            shorthand: Some(".. <next-word>"),
            description: Some(
                "Prefix any word to operate over the whole stack.",
            ),
        }],
        stack_effect: "no values popped or pushed",
        behavior:
            "Sets operation_target_mode to WholeStack. The next word\nconsumes the entire stack as its operand sequence.",
        examples: &[BuiltinExampleDoc {
            canonical: "1 2 3 STAK ADD",
            shorthand: Some("1 2 3 .. +"),
            result: Some("[ 6 ]"),
        }],
        side_effects: &["Sets the next-word target mode."],
        related: &["TOP", "EAT", "KEEP", "SAFE"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "EAT",
        category: "modifier",
        hover_summary: "EAT — consume operands",
        hover_syntax: ", +",
        detail_group: BuiltinDetailGroup::Modifier,
        summary: "Set the consumption mode to consume operands.",
        role: Some(
            "Modifier that switches the next word into operand-consuming mode.",
        ),
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "EAT <next-word>",
            shorthand: Some(", <next-word>"),
            description: None,
        }],
        stack_effect: "no values popped or pushed",
        behavior:
            "Sets consumption_mode to Consume. The next word will pop its\noperands from the stack as usual.",
        examples: &[BuiltinExampleDoc {
            canonical: "1 2 EAT ADD",
            shorthand: Some("1 2 , +"),
            result: Some("[ 3 ]"),
        }],
        side_effects: &["Sets the next-word consumption mode."],
        related: &["KEEP", "TOP", "STAK"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "KEEP",
        category: "modifier",
        hover_summary: "KEEP — keep operands and append result",
        hover_syntax: ",, +",
        detail_group: BuiltinDetailGroup::Modifier,
        summary: "Set the consumption mode to keep operands.",
        role: Some(
            "Modifier that preserves operands while appending the next word's result.",
        ),
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "KEEP <next-word>",
            shorthand: Some(",, <next-word>"),
            description: None,
        }],
        stack_effect: "operands preserved; result pushed",
        behavior:
            "Sets consumption_mode to Keep. The next word reads its operands\nwithout removing them from the stack.",
        examples: &[BuiltinExampleDoc {
            canonical: "1 2 KEEP ADD",
            shorthand: Some("1 2 ,, +"),
            result: Some("[ 1 2 3 ]"),
        }],
        side_effects: &["Sets the next-word consumption mode."],
        related: &["EAT", "TOP", "STAK"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "SAFE",
        category: "modifier",
        hover_summary: "SAFE — compatibility safety boundary",
        hover_syntax: "~ GET",
        detail_group: BuiltinDetailGroup::Modifier,
        summary:
            "Compatibility safety boundary for converting the next operation's error to NIL.",
        role: Some(
            "Modifier that preserves legacy SAFE behavior around one word.",
        ),
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "SAFE <next-word>",
            shorthand: Some("~ <next-word>"),
            description: None,
        }],
        stack_effect: "no values popped or pushed",
        behavior:
            "Sets the safe-mode flag. If the next word fails, the runtime\npushes NIL instead of raising the error. Words that already produce\nBubble/NIL keep working without SAFE.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 2 ] [ 9 ] SAFE GET",
            shorthand: Some("[ 1 2 ] [ 9 ] ~ GET"),
            result: Some("[ NIL ]"),
        }],
        side_effects: &["Enables safe mode for the next word."],
        related: &["FORC", "OR-NIL"],
        ..SPEC_DEFAULT
    },

    // === Vector ops ===
    BuiltinSpec {
        name: "GET",
        category: "vector",
        hover_summary: "GET — extract element at index",
        hover_syntax: "[ 10 20 30 ] [ 0 ] GET",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::VectorOps,
        executor_key: Some(BuiltinExecutorKey::Get),
        summary: "Extract one element of a vector by index.",
        role: Some("Random access into vectors and tensors."),
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] [ idx ] GET",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] [ idx ] -> [ elem ]",
        behavior:
            "Pops a vector and an index vector, returns the element at that\nindex. Multi-dimensional indexing follows the index vector\naxis-by-axis.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 10 20 30 ] [ 0 ] GET",
            shorthand: None,
            result: Some("[ 10 ]"),
        }],
        failure: Some(
            "Produces a Bubble/NIL when the index is out of range.\nRaises StructureError when the target is not indexable or the index is not numeric.",
        ),
        related: &["INSERT", "REPLACE", "REMOVE", "SAFE"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "INSERT",
        category: "vector",
        hover_summary: "INSERT — insert element at index",
        hover_syntax: "[ 1 3 ] [ 1 2 ] INSERT",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::VectorOps,
        executor_key: Some(BuiltinExecutorKey::Insert),
        summary: "Insert a value at a given index in a vector.",
        role: Some(
            "Extends a vector by inserting an element at the indicated position.",
        ),
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] [ idx val ] INSERT",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] [ idx val ] -> [ vec' ]",
        behavior:
            "Pops a vector and a [idx, val] pair; returns a new vector with\nval inserted at idx. Existing elements at and after idx shift right.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 3 ] [ 1 2 ] INSERT",
            shorthand: None,
            result: Some("[ 1 2 3 ]"),
        }],
        failure: Some("Out-of-range index raises IndexOutOfBounds."),
        related: &["REPLACE", "REMOVE", "GET"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "REPLACE",
        category: "vector",
        hover_summary: "REPLACE — replace element at index",
        hover_syntax: "[ 1 2 3 ] [ 0 9 ] REPLACE",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::VectorOps,
        executor_key: Some(BuiltinExecutorKey::Replace),
        summary: "Replace an element of a vector at a given index.",
        role: Some("In-place style update of a vector element."),
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] [ idx val ] REPLACE",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] [ idx val ] -> [ vec' ]",
        behavior:
            "Pops a vector and a [idx, val] pair; returns a new vector with\nthe element at idx replaced by val.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 2 3 ] [ 0 9 ] REPLACE",
            shorthand: None,
            result: Some("[ 9 2 3 ]"),
        }],
        failure: Some("Out-of-range index raises IndexOutOfBounds."),
        related: &["INSERT", "REMOVE", "GET"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "REMOVE",
        category: "vector",
        hover_summary: "REMOVE — remove element at index",
        hover_syntax: "[ 1 2 3 ] [ 0 ] REMOVE",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::VectorOps,
        executor_key: Some(BuiltinExecutorKey::Remove),
        summary: "Remove an element from a vector at a given index.",
        role: Some("Shrinks a vector by deleting one element."),
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] [ idx ] REMOVE",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] [ idx ] -> [ vec' ]",
        behavior:
            "Pops a vector and an index vector; returns a new vector with the\nelement at idx removed. Following elements shift left.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 2 3 ] [ 0 ] REMOVE",
            shorthand: None,
            result: Some("[ 2 3 ]"),
        }],
        failure: Some("Out-of-range index raises IndexOutOfBounds."),
        related: &["INSERT", "REPLACE", "TAKE"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "LENGTH",
        category: "vector",
        hover_summary: "LENGTH — return element count",
        hover_syntax: "[ 1 2 3 ] LENGTH",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::VectorOps,
        executor_key: Some(BuiltinExecutorKey::Length),
        summary: "Return the number of elements in a vector.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] LENGTH",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] -> [ count ]",
        behavior:
            "Pops a vector and pushes its top-level element count as a scalar.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 2 3 ] LENGTH",
            shorthand: None,
            result: Some("[ 3 ]"),
        }],
        failure: Some("NIL operand raises RejectsNil."),
        related: &["SHAPE", "RANK"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "TAKE",
        category: "vector",
        hover_summary: "TAKE — take N elements from start or end",
        hover_syntax: "[ 1 2 3 4 5 ] [ 3 ] TAKE",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::VectorOps,
        executor_key: Some(BuiltinExecutorKey::Take),
        summary: "Take the first N or last -N elements of a vector.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] [ n ] TAKE",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] [ n ] -> [ prefix ]",
        behavior:
            "Pops a vector and a scalar n. If n is positive, returns the\nfirst n elements; if negative, returns the last |n| elements.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 2 3 4 5 ] [ 3 ] TAKE",
            shorthand: None,
            result: Some("[ 1 2 3 ]"),
        }],
        failure: Some("|n| larger than length raises IndexOutOfBounds."),
        related: &["SPLIT", "REVERSE", "LENGTH"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "SPLIT",
        category: "vector",
        hover_summary: "SPLIT — split vector at sizes",
        hover_syntax: "[ 1 2 3 4 ] [ 2 2 ] SPLIT",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::VectorOps,
        executor_key: Some(BuiltinExecutorKey::Split),
        summary: "Split a vector into chunks at the specified sizes.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] [ sizes ] SPLIT",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] [ sizes ] -> [ chunks... ]",
        behavior:
            "Pops a vector and a sizes vector. Returns a vector of vectors,\neach chunk having the corresponding size; trailing remainder forms\nthe final chunk.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 2 3 4 5 6 ] [ 2 3 ] SPLIT",
            shorthand: None,
            result: Some("[ [ 1 2 ] [ 3 4 5 ] [ 6 ] ]"),
        }],
        failure: Some(
            "Sum of sizes exceeding length raises ShapeMismatch.",
        ),
        related: &["TAKE", "CONCAT", "REORDER"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "CONCAT",
        category: "vector",
        hover_summary: "CONCAT — flatten and concatenate vectors",
        hover_syntax: "[ 1 2 ] [ 3 4 ] CONCAT",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::VectorOps,
        executor_key: Some(BuiltinExecutorKey::Concat),
        summary: "Flatten and concatenate two vectors.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ a ] [ b ] CONCAT",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ a ] [ b ] -> [ a ++ b ]",
        behavior: "Pops two vectors and pushes the flattened concatenation.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 2 ] [ 3 4 ] CONCAT",
            shorthand: None,
            result: Some("[ 1 2 3 4 ]"),
        }],
        failure: Some("NIL operand raises RejectsNil."),
        related: &["SPLIT", "TAKE", "REVERSE"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "REVERSE",
        category: "vector",
        hover_summary: "REVERSE — reverse element order",
        hover_syntax: "[ 1 2 3 ] REVERSE",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::VectorOps,
        executor_key: Some(BuiltinExecutorKey::Reverse),
        summary: "Reverse the order of vector elements.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] REVERSE",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] -> [ reversed ]",
        behavior:
            "Pops a vector and pushes a new vector with elements in reverse\norder. Operates on the outermost axis only.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 2 3 ] REVERSE",
            shorthand: None,
            result: Some("[ 3 2 1 ]"),
        }],
        failure: Some("NIL operand raises RejectsNil."),
        related: &["REORDER", "RANGE"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "RANGE",
        category: "vector",
        hover_summary: "RANGE — generate numeric sequence",
        hover_syntax: "[ 0 5 ] RANGE",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::VectorOps,
        executor_key: Some(BuiltinExecutorKey::Range),
        summary: "Generate a numeric sequence from a [start, end] pair.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ start end ] RANGE",
            shorthand: None,
            description: Some(
                "Or [ end ] for the half-range [0, end].",
            ),
        }],
        stack_effect: "[ start end ] -> [ seq ]",
        behavior:
            "Pops a [start, end] (or [end]) vector and pushes the inclusive\ninteger sequence.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 0 5 ] RANGE",
            shorthand: None,
            result: Some("[ 0 1 2 3 4 5 ]"),
        }],
        failure: Some(
            "Non-integer or NIL operand raises RejectsNil.",
        ),
        related: &["REVERSE", "MAP"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "REORDER",
        category: "vector",
        hover_summary: "REORDER — reorder by index list",
        hover_syntax: "[ 'a' 'b' 'c' ] [ 2 0 1 ] REORDER",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::VectorOps,
        executor_key: Some(BuiltinExecutorKey::Reorder),
        summary: "Reorder vector elements according to an index permutation.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] [ indices ] REORDER",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] [ indices ] -> [ permuted ]",
        behavior:
            "Pops a vector and an index vector; returns a new vector whose\ni-th element is the element at indices[i] of the input.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 'a' 'b' 'c' ] [ 2 0 1 ] REORDER",
            shorthand: None,
            result: Some("[ 'c' 'a' 'b' ]"),
        }],
        failure: Some("Out-of-range index raises IndexOutOfBounds."),
        related: &["GET", "SHAPE"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "COLLECT",
        category: "vector",
        hover_summary: "COLLECT — collect N items into vector",
        hover_syntax: "1 2 3 3 COLLECT",
        detail_group: BuiltinDetailGroup::VectorOps,
        executor_key: Some(BuiltinExecutorKey::Collect),
        summary: "Collect N items off the stack into a new vector.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "v1 v2 ... vn n COLLECT",
            shorthand: None,
            description: None,
        }],
        stack_effect: "v1 ... vn n -> [ [ v1 ... vn ] ]",
        behavior:
            "Pops a count n from the stack, then pops n further values and\npacks them into a new vector (oldest first).",
        examples: &[BuiltinExampleDoc {
            canonical: "1 2 3 3 COLLECT",
            shorthand: None,
            result: Some("[ 1 2 3 ]"),
        }],
        failure: Some("Insufficient stack depth raises StackUnderflow."),
        related: &["RANGE", "FILL"],
        ..SPEC_DEFAULT
    },

    // === Constants ===
    BuiltinSpec {
        name: "TRUE",
        category: "constant",
        hover_summary: "TRUE — push TRUE",
        hover_syntax: "TRUE",
        detail_group: BuiltinDetailGroup::ArithmeticLogic,
        executor_key: Some(BuiltinExecutorKey::True),
        summary: "Push the boolean TRUE onto the stack.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "TRUE",
            shorthand: None,
            description: None,
        }],
        stack_effect: "-> [ TRUE ]",
        behavior: "Pushes the boolean value TRUE.",
        examples: &[BuiltinExampleDoc {
            canonical: "TRUE",
            shorthand: None,
            result: Some("[ TRUE ]"),
        }],
        related: &["FALSE", "NIL", "AND", "OR"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "FALSE",
        category: "constant",
        hover_summary: "FALSE — push FALSE",
        hover_syntax: "FALSE",
        detail_group: BuiltinDetailGroup::ArithmeticLogic,
        executor_key: Some(BuiltinExecutorKey::False),
        summary: "Push the boolean FALSE onto the stack.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "FALSE",
            shorthand: None,
            description: None,
        }],
        stack_effect: "-> [ FALSE ]",
        behavior: "Pushes the boolean value FALSE.",
        examples: &[BuiltinExampleDoc {
            canonical: "FALSE",
            shorthand: None,
            result: Some("[ FALSE ]"),
        }],
        related: &["TRUE", "NIL", "NOT"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "NIL",
        category: "constant",
        hover_summary: "NIL — push NIL",
        hover_syntax: "NIL",
        detail_group: BuiltinDetailGroup::ArithmeticLogic,
        executor_key: Some(BuiltinExecutorKey::Nil),
        summary: "Push the NIL value onto the stack.",
        role: Some(
            "Represents the absence of a value or a recoverable failure.",
        ),
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "NIL",
            shorthand: None,
            description: None,
        }],
        stack_effect: "-> [ NIL ]",
        behavior:
            "Pushes NIL. Many partial words use NIL to signal absence;\nthe failure of the runtime in safe mode is also rendered as NIL.",
        examples: &[BuiltinExampleDoc {
            canonical: "NIL",
            shorthand: None,
            result: Some("[ NIL ]"),
        }],
        related: &["TRUE", "FALSE", "OR-NIL", "SAFE"],
        ..SPEC_DEFAULT
    },

    // === Cast ===
    BuiltinSpec {
        name: "CHARS",
        category: "cast",
        hover_summary: "CHARS — split string into characters",
        hover_syntax: "'hi' CHARS",
        word_shape: WordShape::Map,
        detail_group: BuiltinDetailGroup::StringCast,
        executor_key: Some(BuiltinExecutorKey::Chars),
        summary: "Split a string into a vector of one-character strings.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ str ] CHARS",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ str ] -> [ chars ]",
        behavior:
            "Pops a string and pushes a vector containing each character as\na separate one-character string.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 'hi' ] CHARS",
            shorthand: None,
            result: Some("[ 'h' 'i' ]"),
        }],
        failure: Some(
            "Non-string operand raises TypeError.\nNIL operand raises RejectsNil.",
        ),
        related: &["JOIN", "STR"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "JOIN",
        category: "cast",
        hover_summary: "JOIN — join characters into string",
        hover_syntax: "[ 'h' 'i' ] JOIN",
        word_shape: WordShape::Map,
        detail_group: BuiltinDetailGroup::StringCast,
        executor_key: Some(BuiltinExecutorKey::Join),
        summary: "Join a vector of strings into a single string.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ chars ] JOIN",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ chars ] -> [ str ]",
        behavior:
            "Pops a vector of strings and pushes their concatenation as a\nsingle string.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 'h' 'i' ] JOIN",
            shorthand: None,
            result: Some("[ 'hi' ]"),
        }],
        failure: Some("Non-string element raises TypeError."),
        related: &["CHARS", "STR"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "NUM",
        category: "cast",
        hover_summary: "NUM — parse to number",
        hover_syntax: "'42' NUM",
        word_shape: WordShape::Map,
        detail_group: BuiltinDetailGroup::StringCast,
        executor_key: Some(BuiltinExecutorKey::Num),
        summary: "Parse text as a number; Bubble/NIL on parse failure.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ x ] NUM",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ x ] -> [ n | NIL ]",
        behavior:
            "Attempts to interpret the operand as a numeric value. Returns\nthe parsed number on success, Bubble/NIL on parse failure.",
        examples: &[BuiltinExampleDoc {
            canonical: "'42' NUM",
            shorthand: None,
            result: Some("[ 42 ]"),
        }],
        failure: Some(
            "Produces a Bubble/NIL when text cannot be parsed as a number.\nRaises StructureError when the input shape is not convertible text.",
        ),
        related: &["STR", "BOOL", "OR-NIL"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "STR",
        category: "cast",
        hover_summary: "STR — convert to string",
        hover_syntax: "42 STR",
        word_shape: WordShape::Map,
        detail_group: BuiltinDetailGroup::StringCast,
        executor_key: Some(BuiltinExecutorKey::Str),
        summary: "Convert a value to its string representation.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ x ] STR",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ x ] -> [ str ]",
        behavior:
            "Pops any value and pushes its canonical string representation.",
        examples: &[BuiltinExampleDoc {
            canonical: "42 STR",
            shorthand: None,
            result: Some("[ '42' ]"),
        }],
        failure: Some("NIL operand raises RejectsNil."),
        related: &["NUM", "BOOL", "CHR"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "BOOL",
        category: "cast",
        hover_summary: "BOOL — convert to boolean",
        hover_syntax: "1 BOOL",
        word_shape: WordShape::Map,
        detail_group: BuiltinDetailGroup::StringCast,
        executor_key: Some(BuiltinExecutorKey::Bool),
        summary: "Convert a value to a boolean by truthiness.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ x ] BOOL",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ x ] -> [ TRUE | FALSE ]",
        behavior:
            "Maps any non-NIL non-zero non-empty value to TRUE; otherwise\nFALSE.",
        examples: &[BuiltinExampleDoc {
            canonical: "1 BOOL",
            shorthand: None,
            result: Some("[ TRUE ]"),
        }],
        failure: Some("NIL operand raises RejectsNil."),
        related: &["TRUE", "FALSE", "NOT"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "CHR",
        category: "cast",
        hover_summary: "CHR — make a character",
        hover_syntax: "65 CHR",
        word_shape: WordShape::Map,
        detail_group: BuiltinDetailGroup::StringCast,
        executor_key: Some(BuiltinExecutorKey::Chr),
        summary:
            "Convert a numeric character code to a single-character string.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ n ] CHR",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ n ] -> [ char ]",
        behavior:
            "Pops a numeric code point and pushes the single-character\nstring for that code point.",
        examples: &[BuiltinExampleDoc {
            canonical: "65 CHR",
            shorthand: None,
            result: Some("[ 'A' ]"),
        }],
        failure: Some(
            "Produces a Bubble/NIL when the code point is invalid.\nRaises StructureError when the operand is not numeric.",
        ),
        related: &["CHARS", "STR"],
        ..SPEC_DEFAULT
    },

    // === Arithmetic ===
    BuiltinSpec {
        name: "ADD",
        category: "arithmetic",
        hover_summary: "ADD — add values",
        hover_syntax: "1 2 +",
        word_shape: WordShape::Fold,
        detail_group: BuiltinDetailGroup::ArithmeticLogic,
        executor_key: Some(BuiltinExecutorKey::Add),
        summary:
            "Add two numeric values, element-wise with broadcasting.",
        role: Some(
            "Numeric addition; one of the four arithmetic primitives.",
        ),
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ a ] [ b ] ADD",
            shorthand: Some("[ a ] [ b ] +"),
            description: None,
        }],
        stack_effect: "[ a ] [ b ] -> [ a + b ]",
        behavior:
            "Pops two numeric vectors and pushes their element-wise sum.\nA single-element side is broadcast across the other.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 2 ] [ 3 4 ] ADD",
            shorthand: Some("[ 1 2 ] [ 3 4 ] +"),
            result: Some("[ 4 6 ]"),
        }],
        failure: Some(
            "Mismatched non-broadcastable lengths raise ShapeMismatch.",
        ),
        related: &["SUB", "MUL", "DIV"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "SUB",
        category: "arithmetic",
        hover_summary: "SUB — subtract values",
        hover_syntax: "5 3 -",
        word_shape: WordShape::Fold,
        detail_group: BuiltinDetailGroup::ArithmeticLogic,
        executor_key: Some(BuiltinExecutorKey::Sub),
        summary:
            "Subtract two numeric values, element-wise with broadcasting.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ a ] [ b ] SUB",
            shorthand: Some("[ a ] [ b ] -"),
            description: None,
        }],
        stack_effect: "[ a ] [ b ] -> [ a - b ]",
        behavior:
            "Pops two numeric vectors and pushes their element-wise\ndifference.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 5 ] [ 3 ] SUB",
            shorthand: Some("[ 5 ] [ 3 ] -"),
            result: Some("[ 2 ]"),
        }],
        failure: Some(
            "Mismatched non-broadcastable lengths raise ShapeMismatch.",
        ),
        related: &["ADD", "MUL", "DIV"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "MUL",
        category: "arithmetic",
        hover_summary: "MUL — multiply values",
        hover_syntax: "2 4 *",
        word_shape: WordShape::Fold,
        detail_group: BuiltinDetailGroup::ArithmeticLogic,
        executor_key: Some(BuiltinExecutorKey::Mul),
        summary:
            "Multiply two numeric values, element-wise with broadcasting.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ a ] [ b ] MUL",
            shorthand: Some("[ a ] [ b ] *"),
            description: None,
        }],
        stack_effect: "[ a ] [ b ] -> [ a * b ]",
        behavior:
            "Pops two numeric vectors and pushes their element-wise product.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 2 ] [ 4 ] MUL",
            shorthand: Some("[ 2 ] [ 4 ] *"),
            result: Some("[ 8 ]"),
        }],
        failure: Some(
            "Mismatched non-broadcastable lengths raise ShapeMismatch.",
        ),
        related: &["ADD", "SUB", "DIV"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "DIV",
        category: "arithmetic",
        hover_summary: "DIV — divide values",
        hover_syntax: "10 2 /",
        word_shape: WordShape::Fold,
        detail_group: BuiltinDetailGroup::ArithmeticLogic,
        executor_key: Some(BuiltinExecutorKey::Div),
        summary: "Divide two numeric values exactly (fractional result).",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ a ] [ b ] DIV",
            shorthand: Some("[ a ] [ b ] /"),
            description: None,
        }],
        stack_effect: "[ a ] [ b ] -> [ a / b ]",
        behavior:
            "Pops two numeric vectors and pushes their element-wise quotient.\nResult is exact (fractional, not floating-point).",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 10 ] [ 2 ] DIV",
            shorthand: Some("[ 10 ] [ 2 ] /"),
            result: Some("[ 5 ]"),
        }],
        failure: Some(
            "Produces a Bubble/NIL on division by zero.\nRaises StructureError when operands are not numeric.",
        ),
        related: &["ADD", "SUB", "MUL", "MOD"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "EQ",
        category: "comparison",
        hover_summary: "EQ — test equality",
        hover_syntax: "1 1 =",
        word_shape: WordShape::Fold,
        detail_group: BuiltinDetailGroup::ArithmeticLogic,
        executor_key: Some(BuiltinExecutorKey::Eq),
        summary: "Test equality of two values.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ a ] [ b ] EQ",
            shorthand: Some("[ a ] [ b ] ="),
            description: None,
        }],
        stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
        behavior:
            "Pops two values and pushes TRUE iff they are structurally\nequal. Vectors are compared element-wise after broadcasting.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 ] [ 1 ] EQ",
            shorthand: Some("[ 1 ] [ 1 ] ="),
            result: Some("[ TRUE ]"),
        }],
        related: &["LT", "LTE", "AND", "OR"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "LT",
        category: "comparison",
        hover_summary: "LT — test less than",
        hover_syntax: "1 2 <",
        word_shape: WordShape::Fold,
        detail_group: BuiltinDetailGroup::ArithmeticLogic,
        executor_key: Some(BuiltinExecutorKey::Lt),
        summary: "Test less-than comparison.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ a ] [ b ] LT",
            shorthand: Some("[ a ] [ b ] <"),
            description: None,
        }],
        stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
        behavior:
            "Pops two numeric vectors and pushes the element-wise a < b.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 ] [ 2 ] LT",
            shorthand: Some("[ 1 ] [ 2 ] <"),
            result: Some("[ TRUE ]"),
        }],
        related: &["EQ", "LTE"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "LTE",
        category: "comparison",
        hover_summary: "LTE — test less than or equal",
        hover_syntax: "1 1 <=",
        word_shape: WordShape::Fold,
        detail_group: BuiltinDetailGroup::ArithmeticLogic,
        executor_key: Some(BuiltinExecutorKey::Le),
        summary: "Test less-than-or-equal comparison.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ a ] [ b ] LTE",
            shorthand: Some("[ a ] [ b ] <="),
            description: None,
        }],
        stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
        behavior:
            "Pops two numeric vectors and pushes the element-wise a <= b.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 ] [ 1 ] LTE",
            shorthand: Some("[ 1 ] [ 1 ] <="),
            result: Some("[ TRUE ]"),
        }],
        related: &["EQ", "LT"],
        ..SPEC_DEFAULT
    },

    // === Logic ===
    BuiltinSpec {
        name: "AND",
        category: "logic",
        hover_summary: "AND — logical AND",
        hover_syntax: "TRUE TRUE &",
        word_shape: WordShape::Fold,
        detail_group: BuiltinDetailGroup::ArithmeticLogic,
        executor_key: Some(BuiltinExecutorKey::And),
        summary: "Logical AND with three-valued (Kleene) NIL handling.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ a ] [ b ] AND",
            shorthand: Some("[ a ] [ b ] &"),
            description: None,
        }],
        stack_effect: "[ a ] [ b ] -> [ a AND b ]",
        behavior:
            "Pops two boolean vectors and pushes the element-wise AND.\nNIL acts as the unknown value in three-valued logic.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ TRUE FALSE ] [ TRUE TRUE ] AND",
            shorthand: Some("[ TRUE FALSE ] [ TRUE TRUE ] &"),
            result: Some("[ TRUE FALSE ]"),
        }],
        related: &["OR", "NOT"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "OR",
        category: "logic",
        hover_summary: "OR — logical OR",
        hover_syntax: "TRUE FALSE OR",
        word_shape: WordShape::Fold,
        detail_group: BuiltinDetailGroup::ArithmeticLogic,
        executor_key: Some(BuiltinExecutorKey::Or),
        summary: "Logical OR with three-valued (Kleene) NIL handling.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ a ] [ b ] OR",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ a ] [ b ] -> [ a OR b ]",
        behavior:
            "Pops two boolean vectors and pushes the element-wise OR.\nNIL acts as the unknown value in three-valued logic.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ TRUE FALSE ] [ FALSE FALSE ] OR",
            shorthand: None,
            result: Some("[ TRUE FALSE ]"),
        }],
        related: &["AND", "NOT"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "NOT",
        category: "logic",
        hover_summary: "NOT — logical negation",
        hover_syntax: "TRUE NOT",
        word_shape: WordShape::Map,
        detail_group: BuiltinDetailGroup::ArithmeticLogic,
        executor_key: Some(BuiltinExecutorKey::Not),
        summary: "Logical negation.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ a ] NOT",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ a ] -> [ NOT a ]",
        behavior:
            "Pops a boolean vector and pushes the element-wise negation.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ TRUE FALSE ] NOT",
            shorthand: None,
            result: Some("[ FALSE TRUE ]"),
        }],
        related: &["AND", "OR"],
        ..SPEC_DEFAULT
    },

    // === Control ===
    BuiltinSpec {
        name: "IDLE",
        category: "control",
        hover_summary: "IDLE — pass through unchanged",
        hover_syntax: "IDLE",
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Idle),
        summary: "Pass control through unchanged (no-op).",
        role: Some(
            "Placeholder body in conditional clauses; matches the\nalways-true branch.",
        ),
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "IDLE",
            shorthand: None,
            description: None,
        }],
        stack_effect: "no values popped or pushed",
        behavior:
            "Performs no work. Useful as the always-match guard or no-op\nbody in COND clauses.",
        examples: &[BuiltinExampleDoc {
            canonical: "IDLE",
            shorthand: None,
            result: Some("(no change)"),
        }],
        related: &["COND"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "COND",
        category: "control",
        hover_summary: "COND — evaluate guard/body clauses",
        hover_syntax: "1 { TRUE $ 'y' } { IDLE $ 'n' } COND",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::Cond,
        executor_key: Some(BuiltinExecutorKey::Cond),
        summary:
            "Evaluate guard/body clauses in order, executing the first match.",
        role: Some("General conditional dispatch with first-match semantics."),
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "value { g1 $ b1 } { g2 $ b2 } ... COND",
            shorthand: None,
            description: Some(
                "Each clause has the form { guard $ body }.",
            ),
        }],
        stack_effect: "value { ... } ... -> [ result ]",
        behavior:
            "Each clause is a code block of the form '{ guard $ body }'.\nGuards are evaluated against the value in order; the body of the\nfirst guard that returns TRUE is executed and its result returned.",
        examples: &[BuiltinExampleDoc {
            canonical: "1 { TRUE $ 'y' } { IDLE $ 'n' } COND",
            shorthand: None,
            result: Some("[ 'y' ]"),
        }],
        failure: Some("No matching clause raises NoMatch."),
        related: &["IDLE", "MAP", "EXEC"],
        ..SPEC_DEFAULT
    },

    // === Pipeline / coalescing ===
    BuiltinSpec {
        name: "PIPE",
        category: "modifier",
        hover_summary: "PIPE — pipeline marker",
        hover_syntax: "xs == { ... } MAP",
        detail_group: BuiltinDetailGroup::Modifier,
        summary: "Pipeline visual marker (no-op).",
        role: Some(
            "Whitespace separator with no runtime effect; helps visually\nanchor pipelines.",
        ),
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "<a> PIPE <b>",
            shorthand: Some("<a> == <b>"),
            description: None,
        }],
        stack_effect: "no values popped or pushed",
        behavior:
            "Does nothing at runtime. Consumed by the parser solely for\nreadability of pipeline-style code.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 2 3 ] PIPE { [ 2 ] MUL } MAP",
            shorthand: Some("[ 1 2 3 ] == { [ 2 ] * } MAP"),
            result: Some("[ 2 4 6 ]"),
        }],
        related: &["MAP", "FILTER"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "OR-NIL",
        category: "modifier",
        hover_summary: "OR-NIL — coalesce NIL to alternative",
        hover_syntax: "NIL => [ 0 ]",
        detail_group: BuiltinDetailGroup::Modifier,
        summary:
            "Bubble/NIL fallback operator: substitute an alternative if value is NIL.",
        role: Some(
            "Modifier that replaces a Bubble/NIL with a fallback value.",
        ),
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "<a> OR-NIL <b>",
            shorthand: Some("<a> => <b>"),
            description: None,
        }],
        stack_effect: "[a] [b] -> [a if a != NIL else b]",
        behavior:
            "If the left operand is Bubble/NIL, the right operand is taken;\notherwise the left operand is preserved and the right is dropped.",
        examples: &[
            BuiltinExampleDoc {
                canonical: "NIL OR-NIL [ 0 ]",
                shorthand: Some("NIL => [ 0 ]"),
                result: Some("[ 0 ]"),
            },
            BuiltinExampleDoc {
                canonical: "[ 7 ] OR-NIL [ 0 ]",
                shorthand: Some("[ 7 ] => [ 0 ]"),
                result: Some("[ 7 ]"),
            },
        ],
        related: &["SAFE", "NIL"],
        ..SPEC_DEFAULT
    },

    // === Higher-order ===
    BuiltinSpec {
        name: "MAP",
        category: "higher-order",
        hover_summary: "MAP — apply block to each element",
        hover_syntax: "[ 1 2 3 ] { [ 2 ] * } MAP",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Map),
        summary: "Apply a code block to each element of a vector.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] { body } MAP",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] { body } -> [ mapped ]",
        behavior:
            "Pops a vector and a code block. Executes the block once per\nelement, collecting the results in a vector of the same length.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 2 3 ] { [ 2 ] MUL } MAP",
            shorthand: Some("[ 1 2 3 ] { [ 2 ] * } MAP"),
            result: Some("[ 2 4 6 ]"),
        }],
        failure: Some("Block failure propagates unless SAFE is active."),
        related: &["FILTER", "FOLD", "SCAN"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "FILTER",
        category: "higher-order",
        hover_summary: "FILTER — keep elements matching predicate",
        hover_syntax: "[ 1 2 3 ] { [ 2 ] = } FILTER",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Filter),
        summary:
            "Keep only the elements for which a predicate block returns TRUE.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] { pred } FILTER",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] { pred } -> [ kept ]",
        behavior:
            "Pops a vector and a predicate block. Returns the subvector of\nelements for which the predicate evaluates to TRUE.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 2 3 4 ] { [ 2 ] MOD [ 0 ] EQ } FILTER",
            shorthand: None,
            result: Some("[ 2 4 ]"),
        }],
        failure: Some(
            "Predicate must return a boolean; otherwise TypeError.",
        ),
        related: &["MAP", "ANY", "ALL"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "FOLD",
        category: "higher-order",
        hover_summary: "FOLD — reduce with initial value",
        hover_syntax: "[ 1 2 3 ] [ 0 ] { + } FOLD",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Fold),
        summary:
            "Reduce a vector to a single value using an initial accumulator and combiner block.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] [ init ] { combine } FOLD",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] [ init ] { combine } -> [ result ]",
        behavior:
            "Pops a vector, an initial accumulator, and a combiner block.\nApplies the block left-to-right; each iteration sees the running\naccumulator and the next element.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 2 3 4 ] [ 0 ] { ADD } FOLD",
            shorthand: Some("[ 1 2 3 4 ] [ 0 ] { + } FOLD"),
            result: Some("[ 10 ]"),
        }],
        related: &["SCAN", "MAP"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "UNFOLD",
        category: "higher-order",
        hover_summary: "UNFOLD — generate from state transition",
        hover_syntax: "[ 1 ] { ... COND } UNFOLD",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Unfold),
        summary:
            "Generate a sequence by repeatedly applying a state transition.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ state ] { step } UNFOLD",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ state ] { step } -> [ seq ]",
        behavior:
            "Pops an initial state and a step block. The block returns either\n[emit, next_state] to continue or NIL to stop.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 ] { ... COND } UNFOLD",
            shorthand: None,
            result: Some("[ ... ]"),
        }],
        failure: Some(
            "Block must return a 2-vector or NIL; otherwise TypeError.",
        ),
        related: &["FOLD", "SCAN"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "ANY",
        category: "higher-order",
        hover_summary: "ANY — true if any element matches",
        hover_syntax: "[ 1 2 3 ] { [ 2 ] = } ANY",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Any),
        summary: "TRUE if at least one element satisfies the predicate.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] { pred } ANY",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] { pred } -> [ TRUE | FALSE ]",
        behavior:
            "Pops a vector and a predicate block. Pushes TRUE iff the\npredicate returns TRUE for any element.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 2 3 ] { [ 2 ] EQ } ANY",
            shorthand: None,
            result: Some("[ TRUE ]"),
        }],
        related: &["ALL", "FILTER"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "ALL",
        category: "higher-order",
        hover_summary: "ALL — true if all elements match",
        hover_syntax: "[ 2 4 ] { [ 2 ] MOD [ 0 ] = } ALL",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::All),
        summary: "TRUE if every element satisfies the predicate.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] { pred } ALL",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] { pred } -> [ TRUE | FALSE ]",
        behavior:
            "Pops a vector and a predicate block. Pushes TRUE iff the\npredicate returns TRUE for every element.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 2 4 ] { [ 2 ] MOD [ 0 ] EQ } ALL",
            shorthand: None,
            result: Some("[ TRUE ]"),
        }],
        related: &["ANY", "FILTER"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "COUNT",
        category: "higher-order",
        hover_summary: "COUNT — count matching elements",
        hover_syntax: "[ 1 2 3 ] { [ 2 ] = } COUNT",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Count),
        summary: "Count the elements that satisfy the predicate.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] { pred } COUNT",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] { pred } -> [ n ]",
        behavior:
            "Pops a vector and a predicate block. Pushes the count of\nelements for which the predicate returns TRUE.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 2 3 ] { [ 2 ] EQ } COUNT",
            shorthand: None,
            result: Some("[ 1 ]"),
        }],
        related: &["ANY", "ALL", "FILTER"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "SCAN",
        category: "higher-order",
        hover_summary: "SCAN — return intermediate fold results",
        hover_syntax: "[ 1 2 3 ] [ 0 ] { + } SCAN",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Scan),
        summary: "Return a vector of intermediate fold accumulators.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] [ init ] { combine } SCAN",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] [ init ] { combine } -> [ acc-history ]",
        behavior:
            "Like FOLD, but pushes the vector of running accumulators rather\nthan only the final value.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 2 3 4 ] [ 0 ] { ADD } SCAN",
            shorthand: Some("[ 1 2 3 4 ] [ 0 ] { + } SCAN"),
            result: Some("[ 1 3 6 10 ]"),
        }],
        related: &["FOLD", "MAP"],
        ..SPEC_DEFAULT
    },

    // === I/O ===
    BuiltinSpec {
        name: "PRINT",
        category: "io",
        hover_summary: "PRINT — output value to display",
        hover_syntax: "42 PRINT",
        word_shape: WordShape::Map,
        detail_group: BuiltinDetailGroup::IoModule,
        executor_key: Some(BuiltinExecutorKey::Print),
        summary: "Output a value to the display.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ x ] PRINT",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ x ] -> [ x ]",
        behavior:
            "Pops a value, writes it to the output stream, and pushes it\nback so the value remains available for subsequent operations.",
        examples: &[BuiltinExampleDoc {
            canonical: "42 PRINT",
            shorthand: None,
            result: Some("(prints 42)"),
        }],
        side_effects: &[
            "Writes to the output stream; capability IO required.",
        ],
        related: &["STR"],
        stability: "experimental",
        ..SPEC_DEFAULT
    },

    // === Dictionary ===
    BuiltinSpec {
        name: "PRECOMPUTE",
        category: "Control / Staging",
        hover_summary: "PRECOMPUTE — definition-time precompute marker",
        hover_syntax: "{ ... } PRECOMPUTE",
        word_shape: WordShape::Other,
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Precompute),
        summary: "Definition-time staging marker (not a macro).",
        role: Some("Definition-time only"),
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "{ ... } PRECOMPUTE",
            shorthand: None,
            description: Some("Evaluates the preceding CodeBlock during DEF and embeds result values as literals."),
        }],
        stack_effect: "CodeBlock -- value* (definition-time only)",
        behavior: "Not a macro: PRECOMPUTE does not generate arbitrary syntax and errors when executed at runtime.",
        examples: &[BuiltinExampleDoc {
            canonical: "{ { 1 2 ADD } PRECOMPUTE 3 MUL } 'X' DEF",
            shorthand: None,
            result: Some("X evaluates to 9"),
        }],
        failure: Some("Runtime error if executed outside DEF staging."),
        related: &["DEF", "EVAL"],
        side_effects: &["None at runtime; DEF-time rewrite marker only."],
        stability: "stable",
        modifier_interaction: None,
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "DEF",
        category: "dictionary",
        hover_summary: "DEF — define user word",
        hover_syntax: "{ 2 * } 'DOUBLE' DEF",
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Def),
        summary: "Define a user word from a body and a name.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "{ body } [ name ] DEF",
            shorthand: None,
            description: None,
        }],
        stack_effect: "{ body } [ name ] -> []",
        behavior:
            "Pops a code block and a string name, registering the body as\nthe definition of that name in the current dictionary.",
        examples: &[BuiltinExampleDoc {
            canonical: "{ 2 MUL } 'DOUBLE' DEF",
            shorthand: Some("{ 2 * } 'DOUBLE' DEF"),
            result: Some("(DOUBLE defined)"),
        }],
        failure: Some(
            "Existing protected entry raises ProtectedWord without FORC.",
        ),
        side_effects: &[
            "Modifies the dictionary; capability MUTATES_DICT required.",
        ],
        related: &["DEL", "FORC", "LOOKUP"],
        stability: "experimental",
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "DEL",
        category: "dictionary",
        hover_summary: "DEL — delete user word",
        hover_syntax: "'WORD' DEL",
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Del),
        summary: "Delete a user word from the dictionary.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ name ] DEL",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ name ] -> []",
        behavior:
            "Pops a string name and removes that entry from the current\ndictionary.",
        examples: &[BuiltinExampleDoc {
            canonical: "'WORD' DEL",
            shorthand: None,
            result: Some("(WORD removed)"),
        }],
        failure: Some(
            "Protected entry raises ProtectedWord unless preceded by FORC.",
        ),
        side_effects: &[
            "Modifies the dictionary; capability MUTATES_DICT required.",
        ],
        related: &["DEF", "FORC"],
        stability: "experimental",
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "LOOKUP",
        category: "dictionary",
        hover_summary: "LOOKUP — show word documentation",
        hover_syntax: "'ADD' ?",
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Lookup),
        summary: "Display the documentation for a named word.",
        role: Some("Provides word-level guidance from inside Ajisai."),
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ name ] LOOKUP",
            shorthand: Some("[ name ] ?"),
            description: None,
        }],
        stack_effect: "[ name ] -> []",
        behavior:
            "Pops a string name and loads its documentation into the editor.\nFor built-in words, this renders the structured LOOKUP template;\nfor user words, the original defining program is loaded.",
        examples: &[BuiltinExampleDoc {
            canonical: "'ADD' LOOKUP",
            shorthand: Some("'ADD' ?"),
            result: Some("(ADD documentation in editor)"),
        }],
        failure: Some("Unknown word name raises UnknownWord."),
        side_effects: &["Modifies the editor text area."],
        related: &["DEF", "DEL"],
        stability: "experimental",
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "FORC",
        category: "control",
        hover_summary: "FORC — force destructive operation",
        hover_syntax: "! 'WORD' DEL",
        detail_group: BuiltinDetailGroup::Modifier,
        executor_key: Some(BuiltinExecutorKey::Force),
        summary: "Force destructive dictionary operations to apply.",
        role: Some(
            "Modifier that authorizes destructive dictionary words such as\nDEL on protected entries.",
        ),
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "FORC <next-word>",
            shorthand: Some("! <next-word>"),
            description: None,
        }],
        stack_effect: "no values popped or pushed",
        behavior:
            "Marks the next dictionary-mutation word so that protection\nguards on the affected entry are bypassed.",
        examples: &[BuiltinExampleDoc {
            canonical: "FORC 'WORD' DEL",
            shorthand: Some("! 'WORD' DEL"),
            result: Some("forces deletion regardless of protection"),
        }],
        failure: Some(
            "Without FORC, deleting a protected dictionary entry raises\nProtectedWord.",
        ),
        side_effects: &[
            "Bypasses dictionary protection for the next mutating word.",
        ],
        related: &["DEF", "DEL", "SAFE"],
        stability: "experimental",
        ..SPEC_DEFAULT
    },

    // === Tensor ===
    BuiltinSpec {
        name: "SHAPE",
        category: "tensor",
        hover_summary: "SHAPE — return vector shape",
        hover_syntax: "[ 1 2 3 ] SHAPE",
        word_shape: WordShape::Map,
        detail_group: BuiltinDetailGroup::VectorOps,
        executor_key: Some(BuiltinExecutorKey::Shape),
        summary: "Return a vector describing the dimensions of a value.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] SHAPE",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] -> [ shape ]",
        behavior:
            "Pops a vector and pushes its shape: the size of each axis from\noutermost to innermost.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ [ 1 2 3 ] [ 4 5 6 ] ] SHAPE",
            shorthand: None,
            result: Some("[ 2 3 ]"),
        }],
        failure: Some("NIL operand raises RejectsNil."),
        related: &["RANK", "RESHAPE", "TRANSPOSE"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "RANK",
        category: "tensor",
        hover_summary: "RANK — return number of dimensions",
        hover_syntax: "[ [ 1 2 ] ] RANK",
        word_shape: WordShape::Map,
        detail_group: BuiltinDetailGroup::VectorOps,
        executor_key: Some(BuiltinExecutorKey::Rank),
        summary: "Return the number of dimensions of a value.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] RANK",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] -> [ rank ]",
        behavior:
            "Pops a vector and pushes the count of axes (its rank) as a\nscalar.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ [ 1 2 ] [ 3 4 ] ] RANK",
            shorthand: None,
            result: Some("[ 2 ]"),
        }],
        failure: Some("NIL operand raises RejectsNil."),
        related: &["SHAPE", "RESHAPE"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "RESHAPE",
        category: "tensor",
        hover_summary: "RESHAPE — reshape to specified shape",
        hover_syntax: "[ 1 2 3 4 ] [ 2 2 ] RESHAPE",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::VectorOps,
        executor_key: Some(BuiltinExecutorKey::Reshape),
        summary:
            "Reshape a vector to a target shape with the same total length.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ vec ] [ shape ] RESHAPE",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ vec ] [ shape ] -> [ vec' ]",
        behavior:
            "Pops a vector and a shape vector. Returns a new view of the\nelements arranged according to the target shape.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 2 3 4 ] [ 2 2 ] RESHAPE",
            shorthand: None,
            result: Some("[ [ 1 2 ] [ 3 4 ] ]"),
        }],
        failure: Some(
            "Total elements must match the shape product; otherwise\nShapeMismatch.",
        ),
        related: &["SHAPE", "TRANSPOSE", "FILL"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "TRANSPOSE",
        category: "tensor",
        hover_summary: "TRANSPOSE — transpose vector axes",
        hover_syntax: "[ ( 1 2 ) ( 3 4 ) ] TRANSPOSE",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::VectorOps,
        executor_key: Some(BuiltinExecutorKey::Transpose),
        summary: "Transpose the axes of a tensor.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ matrix ] TRANSPOSE",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ matrix ] -> [ transposed ]",
        behavior:
            "Pops a tensor and pushes its transpose. For a rank-2 input,\nswaps rows and columns.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ [ 1 2 ] [ 3 4 ] ] TRANSPOSE",
            shorthand: None,
            result: Some("[ [ 1 3 ] [ 2 4 ] ]"),
        }],
        related: &["SHAPE", "RESHAPE"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "FILL",
        category: "tensor",
        hover_summary: "FILL — fill shape with value",
        hover_syntax: "[ 2 2 0 ] FILL",
        word_shape: WordShape::Form,
        detail_group: BuiltinDetailGroup::VectorOps,
        executor_key: Some(BuiltinExecutorKey::Fill),
        summary: "Fill a target shape with a constant value.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ shape... value ] FILL",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ shape... value ] -> [ filled ]",
        behavior:
            "Pops a shape-and-value vector and pushes a tensor of that\nshape whose elements are all the given value.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 2 2 0 ] FILL",
            shorthand: None,
            result: Some("[ [ 0 0 ] [ 0 0 ] ]"),
        }],
        related: &["RESHAPE", "RANGE", "COLLECT"],
        ..SPEC_DEFAULT
    },

    // === Numeric helpers ===
    BuiltinSpec {
        name: "MOD",
        category: "arithmetic",
        hover_summary: "MOD — modulo",
        hover_syntax: "7 3 %",
        word_shape: WordShape::Fold,
        detail_group: BuiltinDetailGroup::ArithmeticLogic,
        executor_key: Some(BuiltinExecutorKey::Mod),
        summary: "Modulo (remainder) of two numeric values.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ a ] [ b ] MOD",
            shorthand: Some("[ a ] [ b ] %"),
            description: None,
        }],
        stack_effect: "[ a ] [ b ] -> [ a mod b ]",
        behavior:
            "Pops two numeric vectors and pushes their element-wise modulo.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 7 ] [ 3 ] MOD",
            shorthand: Some("[ 7 ] [ 3 ] %"),
            result: Some("[ 1 ]"),
        }],
        failure: Some("Modulo by zero raises DivisionByZero."),
        related: &["DIV", "ADD"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "FLOOR",
        category: "arithmetic",
        hover_summary: "FLOOR — round toward negative infinity",
        hover_syntax: "[ 7/3 ] FLOOR",
        word_shape: WordShape::Map,
        detail_group: BuiltinDetailGroup::ArithmeticLogic,
        executor_key: Some(BuiltinExecutorKey::Floor),
        summary: "Round toward negative infinity.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ x ] FLOOR",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ x ] -> [ floor x ]",
        behavior:
            "Pops a numeric vector and pushes its element-wise floor.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 7/3 ] FLOOR",
            shorthand: None,
            result: Some("[ 2 ]"),
        }],
        related: &["CEIL", "ROUND"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "CEIL",
        category: "arithmetic",
        hover_summary: "CEIL — round toward positive infinity",
        hover_syntax: "[ 7/3 ] CEIL",
        word_shape: WordShape::Map,
        detail_group: BuiltinDetailGroup::ArithmeticLogic,
        executor_key: Some(BuiltinExecutorKey::Ceil),
        summary: "Round toward positive infinity.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ x ] CEIL",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ x ] -> [ ceil x ]",
        behavior:
            "Pops a numeric vector and pushes its element-wise ceiling.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 7/3 ] CEIL",
            shorthand: None,
            result: Some("[ 3 ]"),
        }],
        related: &["FLOOR", "ROUND"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "ROUND",
        category: "arithmetic",
        hover_summary: "ROUND — round to nearest integer",
        hover_syntax: "[ 5/2 ] ROUND",
        word_shape: WordShape::Map,
        detail_group: BuiltinDetailGroup::ArithmeticLogic,
        executor_key: Some(BuiltinExecutorKey::Round),
        summary: "Round to nearest integer (half-up).",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ x ] ROUND",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ x ] -> [ round x ]",
        behavior:
            "Pops a numeric vector and pushes its element-wise rounding to\nthe nearest integer.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 5/2 ] ROUND",
            shorthand: None,
            result: Some("[ 3 ]"),
        }],
        related: &["FLOOR", "CEIL"],
        ..SPEC_DEFAULT
    },

    // === Code execution ===
    BuiltinSpec {
        name: "EXEC",
        category: "control",
        hover_summary: "EXEC — execute vector as code",
        hover_syntax: "[ 1 2 + ] EXEC",
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Exec),
        summary: "Execute a vector as Ajisai code.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ code ] EXEC",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ code ] -> [ result... ]",
        behavior:
            "Pops a code-vector and runs it in the current dictionary\ncontext.",
        examples: &[BuiltinExampleDoc {
            canonical: "[ 1 2 ADD ] EXEC",
            shorthand: Some("[ 1 2 + ] EXEC"),
            result: Some("[ 3 ]"),
        }],
        failure: Some(
            "Inner failures propagate as the originating error.",
        ),
        related: &["EVAL", "DEF"],
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "EVAL",
        category: "control",
        hover_summary: "EVAL — parse and execute string",
        hover_syntax: "'1 2 +' EVAL",
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Eval),
        summary: "Parse a string as Ajisai source code and execute it.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ str ] EVAL",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ str ] -> [ result... ]",
        behavior:
            "Pops a string and evaluates its contents as code. Useful for\nmetaprogramming; constrained by capability checks.",
        examples: &[BuiltinExampleDoc {
            canonical: "'1 2 ADD' EVAL",
            shorthand: Some("'1 2 +' EVAL"),
            result: Some("[ 3 ]"),
        }],
        failure: Some("Parse errors and inner failures propagate."),
        side_effects: &[
            "Executes arbitrary code; capability EVAL required.",
        ],
        related: &["EXEC", "DEF"],
        stability: "experimental",
        ..SPEC_DEFAULT
    },

    // === Module ops ===
    BuiltinSpec {
        name: "IMPORT",
        category: "module",
        hover_summary: "IMPORT — load module",
        hover_syntax: "'IO' IMPORT",
        detail_group: BuiltinDetailGroup::IoModule,
        executor_key: Some(BuiltinExecutorKey::Import),
        summary: "Load all public words of a module into the dictionary.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ name ] IMPORT",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ name ] -> []",
        behavior:
            "Pops a module name and brings all of its public words into the\ncurrent dictionary.",
        examples: &[BuiltinExampleDoc {
            canonical: "'IO' IMPORT",
            shorthand: None,
            result: Some("(IO words available)"),
        }],
        side_effects: &[
            "Modifies the dictionary; capability MUTATES_DICT required.",
        ],
        related: &["IMPORT-ONLY", "DEF"],
        stability: "experimental",
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "IMPORT-ONLY",
        category: "module",
        hover_summary: "IMPORT-ONLY — import selected words",
        hover_syntax: "'json' [ 'parse' ] IMPORT-ONLY",
        detail_group: BuiltinDetailGroup::IoModule,
        executor_key: Some(BuiltinExecutorKey::ImportOnly),
        summary: "Load only the listed public words of a module.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ name ] [ words ] IMPORT-ONLY",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ name ] [ words ] -> []",
        behavior:
            "Pops a module name and a vector of word names, importing only\nthose words into the current dictionary.",
        examples: &[BuiltinExampleDoc {
            canonical: "'json' [ 'parse' ] IMPORT-ONLY",
            shorthand: None,
            result: Some("(only JSON@PARSE imported)"),
        }],
        side_effects: &[
            "Modifies the dictionary; capability MUTATES_DICT required.",
        ],
        related: &["IMPORT"],
        stability: "experimental",
        ..SPEC_DEFAULT
    },

    BuiltinSpec {
        name: "UNIMPORT",
        category: "module",
        hover_summary: "UNIMPORT — hide imported module words",
        hover_syntax: "'IO' UNIMPORT",
        detail_group: BuiltinDetailGroup::IoModule,
        executor_key: Some(BuiltinExecutorKey::Unimport),
        summary: "Hide unused imported words from a module while keeping words referenced by user definitions.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ name ] UNIMPORT",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ name ] -> []",
        behavior:
            "Pops a module name and removes unreferenced imported module words from\nthe current vocabulary. Module words referenced by user definitions remain imported.",
        examples: &[BuiltinExampleDoc {
            canonical: "'MUSIC' UNIMPORT",
            shorthand: None,
            result: Some("(unreferenced MUSIC words hidden)"),
        }],
        side_effects: &[
            "Modifies the import table; capability MUTATES_DICT required.",
        ],
        related: &["IMPORT", "IMPORT-ONLY", "UNIMPORT-ONLY"],
        stability: "experimental",
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "UNIMPORT-ONLY",
        category: "module",
        hover_summary: "UNIMPORT-ONLY — hide selected module words",
        hover_syntax: "'json' [ 'parse' ] UNIMPORT-ONLY",
        detail_group: BuiltinDetailGroup::IoModule,
        executor_key: Some(BuiltinExecutorKey::UnimportOnly),
        summary: "Hide only the listed imported module words.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ name ] [ words ] UNIMPORT-ONLY",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ name ] [ words ] -> []",
        behavior:
            "Pops a module name and a vector of word names, removing those words\nfrom the current vocabulary. Referenced module words are rejected.",
        examples: &[BuiltinExampleDoc {
            canonical: "'json' [ 'parse' ] UNIMPORT-ONLY",
            shorthand: None,
            result: Some("(JSON@PARSE hidden)"),
        }],
        side_effects: &[
            "Modifies the import table; capability MUTATES_DICT required.",
        ],
        related: &["IMPORT", "IMPORT-ONLY", "UNIMPORT"],
        stability: "experimental",
        ..SPEC_DEFAULT
    },

    // === Runtime / parallel ===
    BuiltinSpec {
        name: "SPAWN",
        category: "control",
        hover_summary: "SPAWN — spawn isolated child runtime",
        hover_syntax: "{ 1 2 + } SPAWN",
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Spawn),
        summary: "Spawn an isolated child runtime from a code block.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "{ body } SPAWN",
            shorthand: None,
            description: None,
        }],
        stack_effect: "{ body } -> [ handle ]",
        behavior:
            "Pops a code block and starts a new isolated child runtime\nrunning that block. Pushes a process handle for later coordination.",
        examples: &[BuiltinExampleDoc {
            canonical: "{ 1 2 ADD } SPAWN",
            shorthand: Some("{ 1 2 + } SPAWN"),
            result: Some("[ <handle> ]"),
        }],
        side_effects: &[
            "Creates a child runtime; capability SPAWN required.",
        ],
        related: &[
            "AWAIT", "STATUS", "KILL", "MONITOR", "SUPERVISE",
        ],
        stability: "experimental",
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "AWAIT",
        category: "control",
        hover_summary: "AWAIT — wait for child runtime",
        hover_syntax: "{ 1 2 + } SPAWN AWAIT",
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Await),
        summary:
            "Wait for a child runtime to finish and return its exit tuple.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ handle ] AWAIT",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ handle ] -> [ exit-tuple ]",
        behavior:
            "Pops a process handle, blocks until the child finishes, and\npushes a tuple of the form [ status, value ] describing the result.",
        examples: &[BuiltinExampleDoc {
            canonical: "{ 1 2 ADD } SPAWN AWAIT",
            shorthand: None,
            result: Some("[ 'completed' [ 3 ] ]"),
        }],
        failure: Some("Invalid handle raises ProcessHandleInvalid."),
        side_effects: &[
            "Blocks the calling runtime; capability SPAWN required.",
        ],
        related: &["SPAWN", "STATUS", "KILL"],
        stability: "experimental",
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "STATUS",
        category: "control",
        hover_summary: "STATUS — read child status",
        hover_syntax: "{ 1 2 + } SPAWN STATUS",
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Status),
        summary: "Read the current status of a child runtime.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ handle ] STATUS",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ handle ] -> [ status ]",
        behavior:
            "Pops a process handle and pushes a string describing its\ncurrent state ('running', 'completed', 'failed', 'killed', or\n'timeout').",
        examples: &[BuiltinExampleDoc {
            canonical: "{ ... } SPAWN STATUS",
            shorthand: None,
            result: Some("[ 'running' ]"),
        }],
        failure: Some("Invalid handle raises ProcessHandleInvalid."),
        side_effects: &[
            "Reads child-runtime state; capability SPAWN required.",
        ],
        related: &["SPAWN", "AWAIT", "KILL"],
        stability: "experimental",
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "KILL",
        category: "control",
        hover_summary: "KILL — terminate child runtime",
        hover_syntax: "{ 1 2 + } SPAWN KILL",
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Kill),
        summary: "Forcibly terminate a child runtime.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ handle ] KILL",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ handle ] -> [ 'killed' ]",
        behavior:
            "Pops a process handle and forcibly terminates the corresponding\nchild runtime.",
        examples: &[BuiltinExampleDoc {
            canonical: "{ ... } SPAWN KILL",
            shorthand: None,
            result: Some("[ 'killed' ]"),
        }],
        side_effects: &[
            "Terminates a child runtime; capability SPAWN required.",
        ],
        related: &["SPAWN", "AWAIT", "STATUS"],
        stability: "experimental",
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "MONITOR",
        category: "control",
        hover_summary: "MONITOR — register monitor on child",
        hover_syntax: "{ 1 2 + } SPAWN MONITOR",
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Monitor),
        summary: "Register a monitor on a child handle.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "[ handle ] MONITOR",
            shorthand: None,
            description: None,
        }],
        stack_effect: "[ handle ] -> [ handle ]",
        behavior:
            "Pops a process handle, attaches a monitor that observes its\nlifecycle, and pushes the handle back so it can continue to be\nused for AWAIT or STATUS.",
        examples: &[BuiltinExampleDoc {
            canonical: "{ ... } SPAWN MONITOR",
            shorthand: None,
            result: Some("[ <handle> ]"),
        }],
        side_effects: &[
            "Registers a monitor; capability SPAWN required.",
        ],
        related: &["SPAWN", "SUPERVISE"],
        stability: "experimental",
        ..SPEC_DEFAULT
    },
    BuiltinSpec {
        name: "SUPERVISE",
        category: "control",
        hover_summary: "SUPERVISE — run under restart policy",
        hover_syntax: "{ 1 2 + } [ 3 ] SUPERVISE",
        detail_group: BuiltinDetailGroup::ControlHigherOrder,
        executor_key: Some(BuiltinExecutorKey::Supervise),
        summary: "Run a code block under a one-for-one restart policy.",
        syntax_forms: &[BuiltinSyntaxDoc {
            canonical: "{ body } [ retries ] SUPERVISE",
            shorthand: None,
            description: None,
        }],
        stack_effect: "{ body } [ retries ] -> [ result | NIL ]",
        behavior:
            "Pops a code block and a retries scalar. Runs the block in a\nchild runtime; on failure, restarts up to retries times. Returns\nthe final result, or NIL if all retries are exhausted.",
        examples: &[BuiltinExampleDoc {
            canonical: "{ 1 2 ADD } [ 3 ] SUPERVISE",
            shorthand: None,
            result: Some("[ 3 ]"),
        }],
        side_effects: &[
            "Creates supervised child runtimes; capability SPAWN required.",
        ],
        related: &["SPAWN", "MONITOR"],
        stability: "experimental",
        ..SPEC_DEFAULT
    },
];

pub fn builtin_specs() -> &'static [BuiltinSpec] {
    BUILTIN_SPECS
}

pub fn lookup_builtin_spec(name: &str) -> Option<&'static BuiltinSpec> {
    let canonical = crate::core_word_aliases::canonicalize_core_word_name(name);
    BUILTIN_SPECS.iter().find(|spec| spec.name == canonical)
}

/// WASM/GUI tuple shape: `(name, hover_summary, hover_syntax)`.
/// Position 1 (`hover_summary`) is the native button-title text;
/// position 2 (`hover_syntax`) is the inline word-info preview.
/// See three-layer-documentation-model.md §4.
#[allow(dead_code)]
pub fn collect_builtin_definitions() -> Vec<(&'static str, &'static str, &'static str)> {
    BUILTIN_SPECS
        .iter()
        .map(|spec| (spec.name, spec.hover_summary, spec.hover_syntax))
        .collect()
}

pub fn collect_core_builtin_definitions() -> Vec<(&'static str, &'static str, &'static str)> {
    BUILTIN_SPECS
        .iter()
        .map(|spec| (spec.name, spec.hover_summary, spec.hover_syntax))
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn builtin_specs_do_not_contain_symbol_aliases_or_input_helpers() {
        let forbidden = [
            "+", "-", "*", "/", "%", "=", "<", "<=", ".", "..", ",", ",,", "~", "!", "'", "$", "?",
            "==", "=>",
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

    #[test]
    fn builtin_specs_have_required_lookup_content() {
        for spec in super::builtin_specs() {
            assert!(!spec.summary.is_empty(), "{} missing summary", spec.name);
            assert!(
                !spec.stack_effect.is_empty(),
                "{} missing stack_effect",
                spec.name
            );
            assert!(!spec.behavior.is_empty(), "{} missing behavior", spec.name);
            assert!(
                !spec.syntax_forms.is_empty(),
                "{} has no syntax_forms",
                spec.name
            );
            assert!(!spec.examples.is_empty(), "{} has no examples", spec.name);
            assert!(
                spec.stability == "stable"
                    || spec.stability == "experimental"
                    || spec.stability == "deprecated",
                "{} has invalid stability {}",
                spec.name,
                spec.stability
            );
        }
    }

    #[test]
    fn builtin_specs_lookup_text_is_ascii() {
        // §3.3: LOOKUP body must be ASCII English plain text.
        let check = |label: &str, name: &str, text: &str| {
            assert!(
                text.is_ascii(),
                "{} field of {} must be ASCII; got: {:?}",
                label,
                name,
                text
            );
        };
        for spec in super::builtin_specs() {
            check("summary", spec.name, spec.summary);
            if let Some(role) = spec.role {
                check("role", spec.name, role);
            }
            check("stack_effect", spec.name, spec.stack_effect);
            check("behavior", spec.name, spec.behavior);
            for syn in spec.syntax_forms {
                check("syntax_forms.canonical", spec.name, syn.canonical);
                if let Some(s) = syn.shorthand {
                    check("syntax_forms.shorthand", spec.name, s);
                }
                if let Some(d) = syn.description {
                    check("syntax_forms.description", spec.name, d);
                }
            }
            for ex in spec.examples {
                check("examples.canonical", spec.name, ex.canonical);
                if let Some(s) = ex.shorthand {
                    check("examples.shorthand", spec.name, s);
                }
                if let Some(r) = ex.result {
                    check("examples.result", spec.name, r);
                }
            }
            if let Some(f) = spec.failure {
                check("failure", spec.name, f);
            }
            for s in spec.side_effects {
                check("side_effects", spec.name, s);
            }
            if let Some(mi) = spec.modifier_interaction {
                check("modifier_interaction", spec.name, mi);
            }
        }
    }
}
