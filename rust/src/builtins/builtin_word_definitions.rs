use crate::coreword_registry::{
    ExecutionForm, MassContract, NilPolicy, Partiality, SafetyLevel, WordPurity,
};

use super::builtin_word_types::BuiltinExecutorKey;

#[derive(Clone, Copy, Debug)]
pub struct BuiltinSpec {
    pub name: &'static str,
    pub category: &'static str,
    /// Layer 3 (hover): one-line "WORD — short verb phrase" shown in the
    /// native button title attribute. See three-layer-documentation-model.md
    /// §4.2. Retained during Phase 5 only as a migration-validation oracle.
    #[allow(dead_code)]
    pub hover_summary: &'static str,
    /// Layer 3 (hover): shortest useful invocation (operands included, sugar
    /// preferred when shorter) shown in the inline word-info strip. See
    /// three-layer-documentation-model.md §4.3.
    pub hover_syntax: &'static str,
    pub executor_key: Option<BuiltinExecutorKey>,
    /// Static flow-mass contract (SPEC §13.1). This is the canonical
    /// per-builtin source consumed by the Coreword registry and analyzers.
    pub mass: MassContract,

    // Layer 2 (LOOKUP) fields. Four-section template:
    //   Category / Summary / Role / Stack Effect
    // Stability is shown in the header (e.g. `# ADD  (experimental)`).
    pub summary: &'static str,
    pub role: &'static str,
    pub stack_effect: &'static str,
    /// Must agree with the `safety_level` field below. The mapping is:
    ///   safety_level A or B  -> "stable"
    ///   safety_level C or D  -> "experimental"
    ///   safety_level Quarantined -> "experimental"
    /// A consistency test asserts this invariant.
    pub stability: &'static str,

    // §7.14 contract metadata. Canonical per-word source of truth; the
    // coreword registry reads these directly. `effects` is non-empty only
    // for Observable / Effectful words.
    pub purity: WordPurity,
    pub effects: &'static [&'static str],
    pub deterministic: bool,
    pub safe_preview: bool,
    pub partiality: Partiality,
    pub nil_policy: NilPolicy,
    pub safety_level: SafetyLevel,
    /// How the word takes effect (SPEC §6.4). Defaults to `RuntimeWord`; the
    /// lazy/no-op control directives (`VENT`, `FLOW`) set this so the
    /// classification is machine-checkable rather than inferred from prose.
    pub execution_form: ExecutionForm,
}

const SPEC_DEFAULT: BuiltinSpec = BuiltinSpec {
    name: "",
    category: "",
    hover_summary: "",
    hover_syntax: "",
    executor_key: None,
    mass: MassContract::Dynamic,
    summary: "",
    role: "",
    stack_effect: "",
    stability: "stable",
    purity: WordPurity::Pure,
    effects: &[],
    deterministic: true,
    safe_preview: true,
    partiality: Partiality::Total,
    nil_policy: NilPolicy::Passthrough,
    safety_level: SafetyLevel::A,
    execution_form: ExecutionForm::RuntimeWord,
};

const BUILTIN_SPECS: &[BuiltinSpec] = &[

    BuiltinSpec {

        name: "EAT",
        category: "modifier",
        hover_summary: "EAT — consume operands",
        hover_syntax: ", +",
        summary: "Set the consumption mode to consume operands.",
        role: "Modifier that switches the next word into operand-consuming mode.",

        stack_effect: "no values popped or pushed",
        nil_policy: NilPolicy::PreservesReason,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "KEEP",
        category: "modifier",
        hover_summary: "KEEP — keep operands and append result",
        hover_syntax: ",, +",
        summary: "Set the consumption mode to keep operands.",
        role: "Modifier that preserves operands while appending the next word's result.",

        stack_effect: "operands preserved; result pushed",
        nil_policy: NilPolicy::PreservesReason,
        ..SPEC_DEFAULT
        },
    // === Vector ops ===
    BuiltinSpec {

        name: "GET",
        category: "vector",
        hover_summary: "GET — extract element at index",
        hover_syntax: "[ 10 20 30 ] [ 0 ] GET",
        executor_key: Some(BuiltinExecutorKey::Get),
        summary: "Extract one element of a vector by index.",
        role: "Random access into vectors and tensors.",

        stack_effect: "[ vec ] [ idx ] -> [ elem ]",
        partiality: Partiality::Projecting,
        nil_policy: NilPolicy::CreatesNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "INSERT",
        category: "vector",
        hover_summary: "INSERT — insert element at index",
        hover_syntax: "[ 1 3 ] [ 1 2 ] INSERT",
        executor_key: Some(BuiltinExecutorKey::Insert),
        summary: "Insert a value at a given index in a vector.",
        role: "Extends a vector by inserting an element at the indicated position.",

        stack_effect: "[ vec ] [ idx val ] -> [ vec' ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "REPLACE",
        category: "vector",
        hover_summary: "REPLACE — replace element at index",
        hover_syntax: "[ 1 2 3 ] [ 0 9 ] REPLACE",
        executor_key: Some(BuiltinExecutorKey::Replace),
        summary: "Replace an element of a vector at a given index.",
        role: "In-place style update of a vector element.",

        stack_effect: "[ vec ] [ idx val ] -> [ vec' ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "REMOVE",
        category: "vector",
        hover_summary: "REMOVE — remove element at index",
        hover_syntax: "[ 1 2 3 ] [ 0 ] REMOVE",
        executor_key: Some(BuiltinExecutorKey::Remove),
        summary: "Remove an element from a vector at a given index.",
        role: "Shrinks a vector by deleting one element.",

        stack_effect: "[ vec ] [ idx ] -> [ vec' ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "LENGTH",
        category: "vector",
        hover_summary: "LENGTH — return element count",
        hover_syntax: "[ 1 2 3 ] LENGTH",
        executor_key: Some(BuiltinExecutorKey::Length),
        summary: "Return the number of elements in a vector.",
        role: "Vector primitive: Return the number of elements in a vector.",

        stack_effect: "[ vec ] -> [ count ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "TAKE",
        category: "vector",
        hover_summary: "TAKE — take N elements from start or end",
        hover_syntax: "[ 1 2 3 4 5 ] [ 3 ] TAKE",
        executor_key: Some(BuiltinExecutorKey::Take),
        summary: "Take the first N or last -N elements of a vector.",
        role: "Vector primitive: Take the first N or last -N elements of a vector.",

        stack_effect: "[ vec ] [ n ] -> [ prefix ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "SPLIT",
        category: "vector",
        hover_summary: "SPLIT — split vector at sizes",
        hover_syntax: "[ 1 2 3 4 ] [ 2 2 ] SPLIT",
        executor_key: Some(BuiltinExecutorKey::Split),
        summary: "Split a vector into chunks at the specified sizes.",
        role: "Vector primitive: Split a vector into chunks at the specified sizes.",

        stack_effect: "[ vec ] [ sizes ] -> [ chunks... ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "CONCAT",
        category: "vector",
        hover_summary: "CONCAT — flatten and concatenate vectors",
        hover_syntax: "[ 1 2 ] [ 3 4 ] CONCAT",
        executor_key: Some(BuiltinExecutorKey::Concat),
        summary: "Flatten and concatenate two vectors.",
        role: "Vector primitive: Flatten and concatenate two vectors.",

        stack_effect: "[ a ] [ b ] -> [ a ++ b ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "REVERSE",
        category: "vector",
        hover_summary: "REVERSE — reverse element order",
        hover_syntax: "[ 1 2 3 ] REVERSE",
        executor_key: Some(BuiltinExecutorKey::Reverse),
        summary: "Reverse the order of vector elements.",
        role: "Vector primitive: Reverse the order of vector elements.",

        stack_effect: "[ vec ] -> [ reversed ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "RANGE",
        category: "vector",
        hover_summary: "RANGE — generate numeric sequence",
        hover_syntax: "[ 0 5 ] RANGE",
        executor_key: Some(BuiltinExecutorKey::Range),
        summary: "Generate a numeric sequence from a [start, end] pair.",
        role: "Vector primitive: Generate a numeric sequence from a [start, end] pair.",

        stack_effect: "[ start end ] -> [ seq ]",
        // Projecting/CreatesNil for the space-budget miss: a well-formed but
        // over-budget range projects onto Bubble/NIL (SPEC §7.14, §11.2),
        // matching the DIV/GET/NUM/CHR family. Malformed ranges (zero step,
        // infinite direction) remain ordinary errors.
        partiality: Partiality::Projecting,
        nil_policy: NilPolicy::CreatesNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "REORDER",
        category: "vector",
        hover_summary: "REORDER — reorder by index list",
        hover_syntax: "[ 'a' 'b' 'c' ] [ 2 0 1 ] REORDER",
        executor_key: Some(BuiltinExecutorKey::Reorder),
        summary: "Reorder vector elements according to an index permutation.",
        role: "Vector primitive: Reorder vector elements according to an index permutation.",

        stack_effect: "[ vec ] [ indices ] -> [ permuted ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "COLLECT",
        category: "vector",
        hover_summary: "COLLECT — collect N items into vector",
        hover_syntax: "1 2 3 3 COLLECT",
        executor_key: Some(BuiltinExecutorKey::Collect),
        summary: "Collect N items off the stack into a new vector.",
        role: "Vector primitive: Collect N items off the stack into a new vector.",

        stack_effect: "v1 ... vn n -> [ [ v1 ... vn ] ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },

    // === Constants ===
    BuiltinSpec {

        name: "TRUE",
        category: "constant",
        hover_summary: "TRUE — push TRUE",
        hover_syntax: "TRUE",
        executor_key: Some(BuiltinExecutorKey::True),
        summary: "Push the boolean TRUE onto the stack.",
        role: "Constant primitive: Push the boolean TRUE onto the stack.",

        stack_effect: "-> [ TRUE ]",
        nil_policy: NilPolicy::PreservesReason,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "FALSE",
        category: "constant",
        hover_summary: "FALSE — push FALSE",
        hover_syntax: "FALSE",
        executor_key: Some(BuiltinExecutorKey::False),
        summary: "Push the boolean FALSE onto the stack.",
        role: "Constant primitive: Push the boolean FALSE onto the stack.",

        stack_effect: "-> [ FALSE ]",
        nil_policy: NilPolicy::PreservesReason,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "NIL",
        category: "constant",
        hover_summary: "NIL — push NIL",
        hover_syntax: "NIL",
        executor_key: Some(BuiltinExecutorKey::Nil),
        summary: "Push the NIL value onto the stack.",
        role: "Represents the absence of a value or a recoverable failure.",

        stack_effect: "-> [ NIL ]",
        nil_policy: NilPolicy::PreservesReason,
        ..SPEC_DEFAULT
        },

    // === Diagnostic absence accessors (SPEC §4.5.0 / §7.15) ===
    // All five retain the inspected value on the stack and push their result
    // above it, mirroring the LENGTH/GET inspection-word precedent of §7.1.1
    // (a diagnostic observation is not a consumption). They gate on
    // operational NIL only: the logical Unknown (U), which shares NIL storage,
    // is never reported as absent (SPEC §2.3 / §7.5 firewall). Their
    // nil_policy is ConsumesNil — they inspect NIL rather than propagate it.
    BuiltinSpec {

        name: "NIL?",
        category: "absence",
        hover_summary: "NIL? — test whether a value is absent",
        hover_syntax: "1 0 / NIL?",
        executor_key: Some(BuiltinExecutorKey::NilCheck),
        summary: "Test whether the top value is an operational NIL (absent).",
        role: "Diagnostic predicate: TRUE when the retained value is absent, FALSE otherwise. Never branches on the reason (SPEC §4.5.0).",

        stack_effect: "[ x ] -> [ x ] [ bool ]",
        nil_policy: NilPolicy::ConsumesNil,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "NIL-REASON",
        category: "absence",
        hover_summary: "NIL-REASON — read the NIL reason protocol string",
        hover_syntax: "1 0 / NIL-REASON",
        executor_key: Some(BuiltinExecutorKey::NilReason),
        summary: "Read the direct reason of an operational NIL as a protocol-string Text.",
        role: "Diagnostic accessor: the lowerCamelCase reason protocol string (SPEC §4.5.0), or NIL when there is no reason or the value is not an operational NIL.",

        stack_effect: "[ x ] -> [ x ] [ text|NIL ]",
        nil_policy: NilPolicy::ConsumesNil,
        ..SPEC_DEFAULT
        },

    BuiltinSpec {

        name: "CHARS",
        category: "cast",
        hover_summary: "CHARS — split string into characters",
        hover_syntax: "'hi' CHARS",
        executor_key: Some(BuiltinExecutorKey::Chars),
        summary: "Split a string into a vector of one-character strings.",
        role: "Cast primitive: Split a string into a vector of one-character strings.",

        stack_effect: "[ str ] -> [ chars ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "JOIN",
        category: "cast",
        hover_summary: "JOIN — join characters into string",
        hover_syntax: "[ 'h' 'i' ] JOIN",
        executor_key: Some(BuiltinExecutorKey::Join),
        summary: "Join a vector of strings into a single string.",
        role: "Cast primitive: Join a vector of strings into a single string.",

        stack_effect: "[ chars ] -> [ str ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "TRIM",
        category: "cast",
        hover_summary: "TRIM — strip leading and trailing whitespace",
        hover_syntax: "'  hi  ' TRIM",
        executor_key: Some(BuiltinExecutorKey::Trim),
        summary: "Remove whitespace from both ends of a string.",
        role: "Cast primitive: Remove whitespace from both ends of a string.",

        stack_effect: "[ str ] -> [ str' ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },

    BuiltinSpec {

        name: "TOKENIZE",
        category: "cast",
        hover_summary: "TOKENIZE — split string by separator",
        hover_syntax: "'a,b,c' ',' TOKENIZE",
        executor_key: Some(BuiltinExecutorKey::Tokenize),
        summary: "Split a string into a vector of substrings using a separator.",
        role: "Cast primitive: Split a string into a vector of substrings using a separator.",

        stack_effect: "[ str ] [ sep ] -> [ parts ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "SUBSTITUTE",
        category: "cast",
        hover_summary: "SUBSTITUTE — replace substring occurrences",
        hover_syntax: "'hello' 'l' 'L' SUBSTITUTE",
        executor_key: Some(BuiltinExecutorKey::Substitute),
        summary: "Replace every occurrence of a substring with another.",
        role: "Cast primitive: Replace every occurrence of a substring with another.",

        stack_effect: "[ str ] [ from ] [ to ] -> [ str' ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "STARTS-WITH?",
        category: "cast",
        hover_summary: "STARTS-WITH? — prefix predicate",
        hover_syntax: "'hello' 'he' STARTS-WITH?",
        executor_key: Some(BuiltinExecutorKey::StartsWith),
        summary: "Test whether a string begins with the given prefix.",
        role: "Cast primitive: Test whether a string begins with the given prefix.",

        stack_effect: "[ str ] [ prefix ] -> [ TRUE | FALSE ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "ENDS-WITH?",
        category: "cast",
        hover_summary: "ENDS-WITH? — suffix predicate",
        hover_syntax: "'hello' 'lo' ENDS-WITH?",
        executor_key: Some(BuiltinExecutorKey::EndsWith),
        summary: "Test whether a string ends with the given suffix.",
        role: "Cast primitive: Test whether a string ends with the given suffix.",

        stack_effect: "[ str ] [ suffix ] -> [ TRUE | FALSE ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "NUM",
        category: "cast",
        hover_summary: "NUM — parse to number",
        hover_syntax: "'42' NUM",
        executor_key: Some(BuiltinExecutorKey::Num),
        summary: "Parse text as a number; Bubble/NIL on parse failure.",
        role: "Cast primitive: Parse text as a number; Bubble/NIL on parse failure.",

        stack_effect: "[ x ] -> [ n | NIL ]",
        partiality: Partiality::Projecting,
        nil_policy: NilPolicy::CreatesNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "STR",
        mass: MassContract::Fixed { consumes: 1, produces: 1 },
        category: "cast",
        hover_summary: "STR — convert to string",
        hover_syntax: "42 STR",
        executor_key: Some(BuiltinExecutorKey::Str),
        summary: "Convert a value to its string representation.",
        role: "Cast primitive: Convert a value to its string representation.",

        stack_effect: "[ x ] -> [ str ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },

    BuiltinSpec {

        name: "CHR",
        category: "cast",
        hover_summary: "CHR — make a character",
        hover_syntax: "65 CHR",
        executor_key: Some(BuiltinExecutorKey::Chr),
        summary:
            "Convert a numeric character code to a single-character string.",
        role: "Cast primitive: Convert a numeric character code to a single-character string.",

        stack_effect: "[ n ] -> [ char ]",
        partiality: Partiality::Projecting,
        nil_policy: NilPolicy::CreatesNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },

    // === Arithmetic ===
    BuiltinSpec {

        name: "ADD",
        mass: MassContract::Fixed { consumes: 2, produces: 1 },
        category: "arithmetic",
        hover_summary: "ADD — add values",
        hover_syntax: "1 2 +",
        executor_key: Some(BuiltinExecutorKey::Add),
        summary:
            "Add two numeric values, element-wise with broadcasting.",
        role: "Numeric addition; one of the four arithmetic primitives.",

        stack_effect: "[ a ] [ b ] -> [ a + b ]",
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "SUB",
        mass: MassContract::Fixed { consumes: 2, produces: 1 },
        category: "arithmetic",
        hover_summary: "SUB — subtract values",
        hover_syntax: "5 3 -",
        executor_key: Some(BuiltinExecutorKey::Sub),
        summary:
            "Subtract two numeric values, element-wise with broadcasting.",
        role: "Arithmetic primitive: Subtract two numeric values, element-wise with broadcasting.",

        stack_effect: "[ a ] [ b ] -> [ a - b ]",
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "MUL",
        mass: MassContract::Fixed { consumes: 2, produces: 1 },
        category: "arithmetic",
        hover_summary: "MUL — multiply values",
        hover_syntax: "2 4 *",
        executor_key: Some(BuiltinExecutorKey::Mul),
        summary:
            "Multiply two numeric values, element-wise with broadcasting.",
        role: "Arithmetic primitive: Multiply two numeric values, element-wise with broadcasting.",

        stack_effect: "[ a ] [ b ] -> [ a * b ]",
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "DIV",
        mass: MassContract::Fixed { consumes: 2, produces: 1 },
        category: "arithmetic",
        hover_summary: "DIV — divide values",
        hover_syntax: "10 2 /",
        executor_key: Some(BuiltinExecutorKey::Div),
        summary: "Divide two numeric values exactly (fractional result).",
        role: "Arithmetic primitive: Divide two numeric values exactly (fractional result).",

        stack_effect: "[ a ] [ b ] -> [ a / b ]",
        partiality: Partiality::Projecting,
        nil_policy: NilPolicy::CreatesNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },

    BuiltinSpec {

        name: "EQ",
        mass: MassContract::Fixed { consumes: 2, produces: 1 },
        category: "comparison",
        hover_summary: "EQ — test equality",
        hover_syntax: "1 1 =",
        executor_key: Some(BuiltinExecutorKey::Eq),
        summary: "Test equality of two values.",
        role: "Comparison primitive: Test equality of two values.",

        stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
        partiality: Partiality::Projecting,
        nil_policy: NilPolicy::Passthrough,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "LT",
        mass: MassContract::Fixed { consumes: 2, produces: 1 },
        category: "comparison",
        hover_summary: "LT — test less than",
        hover_syntax: "1 2 <",
        executor_key: Some(BuiltinExecutorKey::Lt),
        summary: "Test less-than comparison.",
        role: "Comparison primitive: Test less-than comparison.",

        stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
        partiality: Partiality::Projecting,
        nil_policy: NilPolicy::Passthrough,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "LTE",
        mass: MassContract::Fixed { consumes: 2, produces: 1 },
        category: "comparison",
        hover_summary: "LTE — test less than or equal",
        hover_syntax: "1 1 <=",
        executor_key: Some(BuiltinExecutorKey::Le),
        summary: "Test less-than-or-equal comparison.",
        role: "Comparison primitive: Test less-than-or-equal comparison.",

        stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
        partiality: Partiality::Projecting,
        nil_policy: NilPolicy::Passthrough,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "GT",
        mass: MassContract::Fixed { consumes: 2, produces: 1 },
        category: "comparison",
        hover_summary: "GT — test greater than",
        hover_syntax: "2 1 >",
        executor_key: Some(BuiltinExecutorKey::Gt),
        summary: "Test greater-than comparison.",
        role: "Comparison primitive: Test greater-than comparison.",

        stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
        partiality: Partiality::Projecting,
        nil_policy: NilPolicy::Passthrough,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "GTE",
        mass: MassContract::Fixed { consumes: 2, produces: 1 },
        category: "comparison",
        hover_summary: "GTE — test greater than or equal",
        hover_syntax: "1 1 >=",
        executor_key: Some(BuiltinExecutorKey::Gte),
        summary: "Test greater-than-or-equal comparison.",
        role: "Comparison primitive: Test greater-than-or-equal comparison.",

        stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
        partiality: Partiality::Projecting,
        nil_policy: NilPolicy::Passthrough,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "NEQ",
        mass: MassContract::Fixed { consumes: 2, produces: 1 },
        category: "comparison",
        hover_summary: "NEQ — test inequality",
        hover_syntax: "1 2 <>",
        executor_key: Some(BuiltinExecutorKey::Neq),
        summary: "Test inequality of two values.",
        role: "Comparison primitive: Test inequality of two values.",

        stack_effect: "[ a ] [ b ] -> [ TRUE | FALSE ]",
        partiality: Partiality::Projecting,
        nil_policy: NilPolicy::Passthrough,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },

    // === Logic ===
    BuiltinSpec {

        name: "AND",
        mass: MassContract::Fixed { consumes: 2, produces: 1 },
        category: "logic",
        hover_summary: "AND — logical AND",
        hover_syntax: "TRUE TRUE &",
        executor_key: Some(BuiltinExecutorKey::And),
        summary: "Logical AND with three-valued (Kleene) NIL handling.",
        role: "Logic primitive: Logical AND with three-valued (Kleene) NIL handling.",

        stack_effect: "[ a ] [ b ] -> [ a AND b ]",
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "OR",
        mass: MassContract::Fixed { consumes: 2, produces: 1 },
        category: "logic",
        hover_summary: "OR — logical OR",
        hover_syntax: "TRUE FALSE OR",
        executor_key: Some(BuiltinExecutorKey::Or),
        summary: "Logical OR with three-valued (Kleene) NIL handling.",
        role: "Logic primitive: Logical OR with three-valued (Kleene) NIL handling.",

        stack_effect: "[ a ] [ b ] -> [ a OR b ]",
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "NOT",
        mass: MassContract::Fixed { consumes: 1, produces: 1 },
        category: "logic",
        hover_summary: "NOT — logical negation",
        hover_syntax: "TRUE NOT",
        executor_key: Some(BuiltinExecutorKey::Not),
        summary: "Logical negation.",
        role: "Logic primitive: Logical negation.",

        stack_effect: "[ a ] -> [ NOT a ]",
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "COND",
        category: "control",
        hover_summary: "COND — evaluate guard/body clauses",
        hover_syntax: "1 { TRUE } { 'y' } { IDLE } { 'n' } COND",
        executor_key: Some(BuiltinExecutorKey::Cond),
        summary:
            "Evaluate guard/body clauses in order, executing the first match.",
        role: "General conditional dispatch with first-match semantics.",

        stack_effect: "value { ... } ... -> [ result ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },    // Both are emitted by the tokenizer as dedicated control tokens (`~`/`FLOW`
    // -> Pipeline, `^`/`VENT` -> NilCoalesce), not dispatched as stack-consuming
    // words; `execution_form` records that so the contract is machine-checkable.

    BuiltinSpec {

        name: "VENT",
        category: "control-directive",
        hover_summary: "VENT — lazy NIL-coalescing fallback",
        hover_syntax: "NIL ^ [ 0 ]",
        summary:
            "Lazy NIL-coalescing control directive: keep a non-NIL top and skip \
             the following source unit; on a NIL top, discard it and evaluate \
             the following source unit as the fallback.",
        role: "Control directive that inspects the stack top. If the top is \
               non-NIL it is kept and the following source unit is skipped \
               UNEVALUATED. If the top is NIL it is discarded and the following \
               source unit is evaluated as the fallback. The fallback is the \
               source that follows the directive, not a value already on the \
               stack.",

        stack_effect: "top non-NIL: keeps top, skips next source unit unevaluated; \
                       top NIL: discards top, evaluates next source unit as fallback",
        nil_policy: NilPolicy::PreservesReason,
        execution_form: ExecutionForm::LazyNextUnitFallback,
        ..SPEC_DEFAULT
        },

    // === Higher-order ===
    BuiltinSpec {

        name: "MAP",
        category: "higher-order",
        hover_summary: "MAP — apply block to each element",
        hover_syntax: "[ 1 2 3 ] { [ 2 ] * } MAP",
        executor_key: Some(BuiltinExecutorKey::Map),
        summary: "Apply a code block to each element of a vector.",
        role: "Higher-order primitive: Apply a code block to each element of a vector.",

        stack_effect: "[ vec ] { body } -> [ mapped ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "FILTER",
        category: "higher-order",
        hover_summary: "FILTER — keep elements matching predicate",
        hover_syntax: "[ 1 2 3 ] { [ 2 ] = } FILTER",
        executor_key: Some(BuiltinExecutorKey::Filter),
        summary:
            "Keep only the elements for which a predicate block returns TRUE.",
        role: "Higher-order primitive: Keep only the elements for which a predicate block returns TRUE.",

        stack_effect: "[ vec ] { pred } -> [ kept ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "FOLD",
        category: "higher-order",
        hover_summary: "FOLD — reduce with initial value",
        hover_syntax: "[ 1 2 3 ] [ 0 ] { + } FOLD",
        executor_key: Some(BuiltinExecutorKey::Fold),
        summary:
            "Reduce a vector to a single value using an initial accumulator and combiner block.",
        role: "Higher-order primitive: Reduce a vector to a single value using an initial accumulator and combiner block.",

        stack_effect: "[ vec ] [ init ] { combine } -> [ result ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },

    BuiltinSpec {

        name: "ANY",
        category: "higher-order",
        hover_summary: "ANY — true if any element matches",
        hover_syntax: "[ 1 2 3 ] { [ 2 ] = } ANY",
        executor_key: Some(BuiltinExecutorKey::Any),
        summary: "TRUE if at least one element satisfies the predicate.",
        role: "Higher-order primitive: TRUE if at least one element satisfies the predicate.",

        stack_effect: "[ vec ] { pred } -> [ TRUE | FALSE ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "ALL",
        category: "higher-order",
        hover_summary: "ALL — true if all elements match",
        hover_syntax: "[ 2 4 ] { [ 2 ] MOD [ 0 ] = } ALL",
        executor_key: Some(BuiltinExecutorKey::All),
        summary: "TRUE if every element satisfies the predicate.",
        role: "Higher-order primitive: TRUE if every element satisfies the predicate.",

        stack_effect: "[ vec ] { pred } -> [ TRUE | FALSE ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },

    // === I/O ===
    BuiltinSpec {

        name: "PRINT",
        category: "io",
        hover_summary: "PRINT — output value to display",
        hover_syntax: "42 PRINT",
        executor_key: Some(BuiltinExecutorKey::Print),
        summary: "Output the top stack value. A string is written as its raw text, without the quotes the stack shows ('TEST' prints as TEST); nested strings keep their quotes, and numbers and other values print as they appear on the stack.",
        role: "Io primitive: output the top stack value at the output boundary, where a string is emitted as its raw character content (the stack's surrounding quotes are a display affordance only).",

        stack_effect: "[ x ] -> [ x ]",
        stability: "experimental",
        purity: WordPurity::Effectful,
        effects: &["console-write"],
        deterministic: false,
        safe_preview: false,
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::PreservesReason,
        safety_level: SafetyLevel::D,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "DEF",
        category: "dictionary",
        hover_summary: "DEF — define user word",
        hover_syntax: "{ 2 * } 'DOUBLE' DEF",
        executor_key: Some(BuiltinExecutorKey::Def),
        summary: "Define a user word from a body and a name.",
        role: "Dictionary primitive: Define a user word from a body and a name.",

        stack_effect: "{ body } [ name ] -> []",
        stability: "experimental",
        purity: WordPurity::Effectful,
        effects: &["dictionary-write", "dictionary-register"],
        deterministic: false,
        safe_preview: false,
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::D,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "DEL",
        category: "dictionary",
        hover_summary: "DEL — delete user word",
        hover_syntax: "{ [ 1 ] } 'W' DEF 'W' DEL",
        executor_key: Some(BuiltinExecutorKey::Del),
        summary: "Delete a user word from the dictionary.",
        role: "Dictionary primitive: Delete a user word from the dictionary.",

        stack_effect: "[ name ] -> []",
        stability: "experimental",
        purity: WordPurity::Effectful,
        effects: &["dictionary-delete"],
        deterministic: false,
        safe_preview: false,
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::D,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "LOOKUP",
        category: "dictionary",
        hover_summary: "LOOKUP — show word documentation",
        hover_syntax: "'ADD' ?",
        executor_key: Some(BuiltinExecutorKey::Lookup),
        summary: "Display the documentation for a named word.",
        role: "Provides word-level guidance from inside Ajisai.",

        stack_effect: "[ name ] -> []",
        stability: "experimental",
        purity: WordPurity::Observable,
        effects: &["dictionary-read"],
        safe_preview: false,
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::C,
        ..SPEC_DEFAULT
        },

    BuiltinSpec {

        name: "FILL",
        category: "tensor",
        hover_summary: "FILL — fill shape with value",
        hover_syntax: "[ 2 2 0 ] FILL",
        executor_key: Some(BuiltinExecutorKey::Fill),
        summary: "Fill a target shape with a constant value.",
        role: "Tensor primitive: Fill a target shape with a constant value.",

        stack_effect: "[ shape... value ] -> [ filled ]",
        // Projecting/CreatesNil for the space-budget miss: a well-formed but
        // over-budget (or product-overflowing) shape projects onto Bubble/NIL
        // (SPEC §7.14, §11.2). A malformed shape remains an ordinary error.
        partiality: Partiality::Projecting,
        nil_policy: NilPolicy::CreatesNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },

    // === Numeric helpers ===
    BuiltinSpec {

        name: "MOD",
        mass: MassContract::Fixed { consumes: 2, produces: 1 },
        category: "arithmetic",
        hover_summary: "MOD — modulo",
        hover_syntax: "7 3 %",
        executor_key: Some(BuiltinExecutorKey::Mod),
        summary: "Modulo (remainder) of two numeric values.",
        role: "Arithmetic primitive: Modulo (remainder) of two numeric values.",

        stack_effect: "[ a ] [ b ] -> [ a mod b ]",
        partiality: Partiality::Projecting,
        nil_policy: NilPolicy::CreatesNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "FLOOR",
        mass: MassContract::Fixed { consumes: 1, produces: 1 },
        category: "arithmetic",
        hover_summary: "FLOOR — round toward negative infinity",
        hover_syntax: "[ 7/3 ] FLOOR",
        executor_key: Some(BuiltinExecutorKey::Floor),
        summary: "Round toward negative infinity.",
        role: "Arithmetic primitive: Round toward negative infinity.",

        stack_effect: "[ x ] -> [ floor x ]",
        partiality: Partiality::Projecting,
        nil_policy: NilPolicy::CreatesNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "CEIL",
        mass: MassContract::Fixed { consumes: 1, produces: 1 },
        category: "arithmetic",
        hover_summary: "CEIL — round toward positive infinity",
        hover_syntax: "[ 7/3 ] CEIL",
        executor_key: Some(BuiltinExecutorKey::Ceil),
        summary: "Round toward positive infinity.",
        role: "Arithmetic primitive: Round toward positive infinity.",

        stack_effect: "[ x ] -> [ ceil x ]",
        partiality: Partiality::Projecting,
        nil_policy: NilPolicy::CreatesNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },
    BuiltinSpec {

        name: "ROUND",
        mass: MassContract::Fixed { consumes: 1, produces: 1 },
        category: "arithmetic",
        hover_summary: "ROUND — round to nearest integer",
        hover_syntax: "[ 5/2 ] ROUND",
        executor_key: Some(BuiltinExecutorKey::Round),
        summary: "Round to nearest integer (half-up).",
        role: "Arithmetic primitive: Round to nearest integer (half-up).",

        stack_effect: "[ x ] -> [ round x ]",
        partiality: Partiality::Projecting,
        nil_policy: NilPolicy::CreatesNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },

    // === Code execution ===
    BuiltinSpec {

        name: "EXEC",
        category: "control",
        hover_summary: "EXEC — execute vector as code",
        hover_syntax: "[ 1 2 + ] EXEC",
        executor_key: Some(BuiltinExecutorKey::Exec),
        summary: "Execute a vector as Ajisai code.",
        role: "Control primitive: Execute a vector as Ajisai code.",

        stack_effect: "[ code ] -> [ result... ]",
        partiality: Partiality::Partial,
        nil_policy: NilPolicy::RejectsNil,
        safety_level: SafetyLevel::B,
        ..SPEC_DEFAULT
        },

    // === Exact math ===
    BuiltinSpec {
        name: "SQRT",
        category: "math",
        hover_summary: "SQRT — exact square root",
        hover_syntax: "2 SQRT",
        executor_key: Some(BuiltinExecutorKey::Sqrt),
        summary: "Exact square root of a non-negative rational.",
        role: "The only Word that leaves the rationals: it produces the multiquadratic \u{221a}d.",
        stack_effect: "[ x ] -> [ sqrt(x) ]",
        partiality: Partiality::Projecting,
        nil_policy: NilPolicy::CreatesNil,
        safety_level: SafetyLevel::B,
        mass: MassContract::Fixed { consumes: 1, produces: 1 },
        ..SPEC_DEFAULT
        },
    BuiltinSpec {
        name: "ABS",
        category: "math",
        hover_summary: "ABS — absolute value",
        hover_syntax: "-3 ABS",
        executor_key: Some(BuiltinExecutorKey::Abs),
        summary: "Absolute value of an exact scalar.",
        role: "Magnitude without sign.",
        stack_effect: "[ x ] -> [ |x| ]",
        mass: MassContract::Fixed { consumes: 1, produces: 1 },
        ..SPEC_DEFAULT
        },
    BuiltinSpec {
        name: "NEG",
        category: "math",
        hover_summary: "NEG — negate",
        hover_syntax: "3 NEG",
        executor_key: Some(BuiltinExecutorKey::Neg),
        summary: "Negate an exact scalar.",
        role: "Additive inverse.",
        stack_effect: "[ x ] -> [ -x ]",
        mass: MassContract::Fixed { consumes: 1, produces: 1 },
        ..SPEC_DEFAULT
        },
    BuiltinSpec {
        name: "SIGN",
        category: "math",
        hover_summary: "SIGN — sign of a scalar",
        hover_syntax: "-3 SIGN",
        executor_key: Some(BuiltinExecutorKey::Sign),
        summary: "Sign of an exact scalar as -1, 0 or 1.",
        role: "Direction without magnitude.",
        stack_effect: "[ x ] -> [ sign ]",
        mass: MassContract::Fixed { consumes: 1, produces: 1 },
        ..SPEC_DEFAULT
        },
    BuiltinSpec {
        name: "MIN",
        category: "math",
        hover_summary: "MIN — smaller of two scalars",
        hover_syntax: "3 5 MIN",
        executor_key: Some(BuiltinExecutorKey::Min),
        summary: "The smaller of two exact scalars.",
        role: "Comparison-selected operand; comparison is total, so it always decides.",
        stack_effect: "[ a ] [ b ] -> [ min ]",
        mass: MassContract::Fixed { consumes: 2, produces: 1 },
        ..SPEC_DEFAULT
        },
    BuiltinSpec {
        name: "MAX",
        category: "math",
        hover_summary: "MAX — larger of two scalars",
        hover_syntax: "3 5 MAX",
        executor_key: Some(BuiltinExecutorKey::Max),
        summary: "The larger of two exact scalars.",
        role: "Comparison-selected operand; comparison is total, so it always decides.",
        stack_effect: "[ a ] [ b ] -> [ max ]",
        mass: MassContract::Fixed { consumes: 2, produces: 1 },
        ..SPEC_DEFAULT
        },
    // === Ordering and search ===
    BuiltinSpec {
        name: "SORT",
        category: "vector",
        hover_summary: "SORT — ascending sort",
        hover_syntax: "[ 3 1 2 ] SORT",
        executor_key: Some(BuiltinExecutorKey::Sort),
        summary: "Sort a vector into ascending order.",
        role: "Total ordering over exact scalars.",
        stack_effect: "[ vec ] -> [ sorted ]",
        mass: MassContract::Fixed { consumes: 1, produces: 1 },
        ..SPEC_DEFAULT
        },
    BuiltinSpec {
        name: "UNIQUE",
        category: "vector",
        hover_summary: "UNIQUE — remove duplicates",
        hover_syntax: "[ 1 1 2 ] UNIQUE",
        executor_key: Some(BuiltinExecutorKey::Unique),
        summary: "Remove duplicate elements, keeping first occurrence order.",
        role: "Set-like reduction that preserves order.",
        stack_effect: "[ vec ] -> [ vec ]",
        mass: MassContract::Fixed { consumes: 1, produces: 1 },
        ..SPEC_DEFAULT
        },
    BuiltinSpec {
        name: "CONTAINS",
        category: "vector",
        hover_summary: "CONTAINS — membership test",
        hover_syntax: "[ 1 2 3 ] [ 2 ] CONTAINS",
        executor_key: Some(BuiltinExecutorKey::Contains),
        summary: "TRUE when the vector contains the value.",
        role: "Membership as a definite truth value.",
        stack_effect: "[ vec ] [ x ] -> [ bool ]",
        mass: MassContract::Fixed { consumes: 2, produces: 1 },
        ..SPEC_DEFAULT
        },
    BuiltinSpec {
        name: "INDEX-OF",
        category: "vector",
        hover_summary: "INDEX-OF — position of a value",
        hover_syntax: "[ 1 2 3 ] [ 2 ] INDEX-OF",
        executor_key: Some(BuiltinExecutorKey::IndexOf),
        summary: "0-origin index of the first matching element.",
        role: "Search that projects to NIL when the value is absent.",
        stack_effect: "[ vec ] [ x ] -> [ idx ]",
        partiality: Partiality::Projecting,
        nil_policy: NilPolicy::CreatesNil,
        safety_level: SafetyLevel::B,
        mass: MassContract::Fixed { consumes: 2, produces: 1 },
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
///
/// Consumed only by the wasm bindings (feature = "wasm").
#[cfg_attr(not(feature = "wasm"), allow(dead_code))]
pub fn collect_core_builtin_definitions() -> Vec<(&'static str, &'static str, &'static str)> {
    super::generated_core_word_docs::GENERATED_CORE_WORD_DOCS
        .iter()
        .map(|doc| (doc.name, doc.hover_summary, doc.hover_syntax))
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn builtin_specs_do_not_contain_symbol_aliases_or_input_helpers() {
        let forbidden = [
            "+", "-", "*", "/", "%", "=", "<", "<=", ">", ">=", "<>", ".", "..", ",", ",,", "~",
            "!", "'", "|", "?", "^",
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
            "ADD", "SUB", "MUL", "DIV", "MOD", "EQ", "NEQ", "LT", "LTE", "GT", "GTE", "TOP",
            "STAK", "EAT", "KEEP", "FORC", "LOOKUP", "FLOW", "VENT",
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
    fn generated_core_docs_preserve_the_legacy_observation() {
        let generated = super::super::generated_core_word_docs::GENERATED_CORE_WORD_DOCS;
        assert_eq!(generated.len(), super::builtin_specs().len());

        for (doc, spec) in generated.iter().zip(super::builtin_specs()) {
            assert_eq!(doc.name, spec.name);
            assert_eq!(doc.hover_summary, spec.hover_summary);
            assert_eq!(doc.hover_syntax, spec.hover_syntax);
        }
    }

    #[test]
    fn builtin_specs_have_required_lookup_content() {
        for spec in super::builtin_specs() {
            assert!(!spec.summary.is_empty(), "{} missing summary", spec.name);
            assert!(!spec.role.is_empty(), "{} missing role", spec.name);
            assert!(!spec.category.is_empty(), "{} missing category", spec.name);
            assert!(
                !spec.stack_effect.is_empty(),
                "{} missing stack_effect",
                spec.name
            );
            assert!(
                spec.stability == "stable" || spec.stability == "experimental",
                "{} has invalid stability {}",
                spec.name,
                spec.stability
            );
        }
    }

    #[test]
    fn builtin_specs_stack_effect_grammar() {
        for spec in super::builtin_specs() {
            // Control directives (SPEC §6.4) act positionally on the source
            // stream, not as a stack `X -> Y` transformation, so the arrow
            // grammar does not apply to them; their contract is carried by
            // `execution_form` and a prose stack-effect note.
            if spec.execution_form != crate::coreword_registry::ExecutionForm::RuntimeWord {
                continue;
            }
            let s = spec.stack_effect;
            let is_literal_no_op =
                s == "no values popped or pushed" || s == "operands preserved; result pushed";
            if is_literal_no_op {
                continue;
            }
            assert!(
                s.contains("->"),
                "{} stack_effect missing '->' arrow: {:?}",
                spec.name,
                s
            );
        }
    }

    #[test]
    fn builtin_specs_lookup_text_is_utf8_plain_text() {
        let check = |label: &str, name: &str, text: &str| {
            assert!(
                !text.chars().any(|c| c.is_control() && c != '\n'),
                "{} field of {} must be UTF-8 plain text without control characters; got: {:?}",
                label,
                name,
                text
            );
        };
        for spec in super::builtin_specs() {
            check("summary", spec.name, spec.summary);
            check("role", spec.name, spec.role);
            check("stack_effect", spec.name, spec.stack_effect);
            check("category", spec.name, spec.category);
        }
    }
}
