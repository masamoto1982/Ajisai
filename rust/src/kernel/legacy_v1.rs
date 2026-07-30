//! HostProtocolV1 anti-corruption adapter (migration plan §16).
//!
//! HostProtocolV1 is frozen: it keeps a legacy surface — `semanticKind`,
//! `shape`, `capabilities`, an absence `origin`/`recoverability`/`diagnosis`
//! block, `importedModules` — that the reduced language no longer has. This
//! module is the boundary that lets V1 keep that surface after the legacy
//! semantic fields are removed from the kernel path (Phase 9): it reconstructs
//! the parts of the V1 semantics block that are a pure function of the value
//! **from the spine's [`KernelValue`]**, so the runtime need not carry them.
//!
//! Anti-corruption is one-directional. V1 vocabulary lives here and nowhere
//! else on the spine side, and it never flows back: the adapter reads a
//! `KernelValue` and returns V1 protocol strings; it does not — and cannot —
//! add a module, tensor, or origin field to `KernelValue`.
//!
//! Scope. The three axes below (`semanticKind`, `shape`, `capabilities`) are
//! functions of the value domain, so the spine can supply them. The remaining
//! V1-only data is not a function of the value and is **not** reconstructed
//! here: an absence's `origin`/`recoverability`/`diagnosis` is diagnostic state
//! (§9) and `importedModules` is legacy runtime state (§10). A full V1 document
//! defaults those in the adapter (origin/recoverability `unknown`, no modules)
//! or takes them from a legacy annotation passed alongside — never from the
//! spine value. Only the value-derived axes are modeled in this phase.

use super::value::KernelValue;

/// V1 `semanticKind` for a value, reconstructed from the spine. Mirrors the
/// legacy `Value::semantic_kind`: a string is a collection (its legacy
/// representation is a vector), a boolean reports `number` on this coarse axis
/// for protocol stability (its truth lives in `truthValued`), and a NIL is an
/// absence.
pub fn v1_semantic_kind(value: &KernelValue) -> &'static str {
    match value {
        KernelValue::Scalar(_) | KernelValue::Boolean(_) => "number",
        KernelValue::String(_) | KernelValue::Vector(_) => "collection",
        KernelValue::Nil(_) => "absence",
        KernelValue::CodeBlock(_) => "code",
    }
}

/// V1 `shape` for a value, reconstructed from the spine. Mirrors the legacy
/// `Value::shape_kind`.
pub fn v1_shape(value: &KernelValue) -> &'static str {
    match value {
        KernelValue::Scalar(_) | KernelValue::Boolean(_) => "scalar",
        KernelValue::String(_) | KernelValue::Vector(_) => "vector",
        KernelValue::Nil(_) => "absence",
        KernelValue::CodeBlock(_) => "codeBlock",
    }
}

/// V1 `capabilities` for a value, reconstructed from the spine in the legacy
/// order. Mirrors the legacy `Value::capabilities`: a base set for every stack
/// value, then domain-specific capabilities, then `truthValued` for a boolean.
pub fn v1_capabilities(value: &KernelValue) -> Vec<&'static str> {
    let mut capabilities = vec!["stackItem", "serializable", "displayable"];
    match value {
        KernelValue::Scalar(_) => {
            capabilities.extend(["numeric", "exactNumeric", "userEditable"]);
        }
        KernelValue::String(_) | KernelValue::Vector(_) => {
            capabilities.extend(["iterable", "indexable", "userEditable"]);
        }
        KernelValue::Nil(_) => {
            capabilities.extend(["nilPassthrough", "diagnosable", "aiExplainable"]);
        }
        KernelValue::CodeBlock(_) => capabilities.push("callable"),
        KernelValue::Boolean(_) => {}
    }
    // A boolean is truth-valued; it advertises `truthValued` so consumers read
    // the `truthValue` axis (SPEC §2.3).
    if matches!(value, KernelValue::Boolean(_)) {
        capabilities.push("truthValued");
    }
    capabilities
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::scalar::Scalar;
    use crate::kernel::value::CodeBlock;
    use crate::types::fraction::Fraction;
    use crate::types::Value;
    use std::sync::Arc;

    // The legacy `Value` methods are HostProtocolV1's source of truth. Raising a
    // KernelValue to a Value and reading them back is the oracle the adapter's
    // spine-side reconstruction must match.
    fn oracle_semantic_kind(value: &KernelValue) -> &'static str {
        Value::from(value).semantic_kind().as_protocol_str()
    }
    fn oracle_shape(value: &KernelValue) -> &'static str {
        Value::from(value).shape_kind().as_protocol_str()
    }
    fn oracle_capabilities(value: &KernelValue) -> Vec<&'static str> {
        Value::from(value)
            .capabilities()
            .into_iter()
            .map(|capability| capability.as_protocol_str())
            .collect()
    }

    fn representative_values() -> Vec<KernelValue> {
        vec![
            KernelValue::Scalar(Scalar::from_fraction(Fraction::from(3_i64))),
            KernelValue::Boolean(true),
            KernelValue::Boolean(false),
            // Non-empty: an empty string raises to NIL in the legacy model.
            KernelValue::String(Arc::from("hi")),
            KernelValue::Vector(Arc::from([KernelValue::Scalar(Scalar::from_fraction(
                Fraction::from(1_i64),
            ))])),
            KernelValue::Nil(None),
            KernelValue::CodeBlock(CodeBlock::new(Arc::from(Vec::new()))),
        ]
    }

    #[test]
    fn adapter_reconstructs_the_v1_semantic_axes_from_the_spine() {
        for value in representative_values() {
            assert_eq!(
                v1_semantic_kind(&value),
                oracle_semantic_kind(&value),
                "semanticKind mismatch for {value:?}"
            );
            assert_eq!(
                v1_shape(&value),
                oracle_shape(&value),
                "shape mismatch for {value:?}"
            );
            assert_eq!(
                v1_capabilities(&value),
                oracle_capabilities(&value),
                "capabilities mismatch for {value:?}"
            );
        }
    }
}
