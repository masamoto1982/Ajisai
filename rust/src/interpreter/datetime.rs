//! TIME@NOW: the single host-clock boundary of the TIME module.
//!
//! All civil <-> instant conversions live in `time_ops` / `time_calendar` and
//! are exact and host-independent. `NOW` is the one observable word: it reads
//! the host wall clock and yields an exact-rational instant (seconds since the
//! Unix epoch). The host clock has millisecond resolution, so the result is an
//! exact multiple of `1/1000`.

use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::create_datetime_value;
use crate::interpreter::{HostCapability, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;

/// The single host-clock boundary. Returns wall-clock milliseconds since the
/// Unix epoch from whichever host the current build targets. WASM-specific
/// access to `Date.now()` is isolated behind the `wasm` feature so the Core
/// (native std) build never references wasm-bindgen.
pub(crate) fn default_now_millis() -> i64 {
    #[cfg(feature = "wasm")]
    {
        wasm_now_millis()
    }
    #[cfg(all(not(feature = "wasm"), feature = "std"))]
    {
        std_now_millis()
    }
}

// `#[wasm_bindgen] extern "C"` expands to generated glue containing `unsafe`,
// so this one host-clock boundary re-permits `unsafe_code` over the crate-root
// `#![deny(unsafe_code)]` (structural-memory-safety roadmap Phase 4). No
// hand-written `unsafe` lives here; the allow only covers macro-generated code.
#[cfg(feature = "wasm")]
#[allow(unsafe_code)]
fn wasm_now_millis() -> i64 {
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = Date, js_name = now)]
        fn date_now() -> f64;
    }
    date_now() as i64
}

#[cfg(all(not(feature = "wasm"), feature = "std"))]
fn std_now_millis() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn op_now(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported {
            word: "NOW".into(),
            mode: "Stack".into(),
        });
    }

    interp.require_host_capability("NOW", HostCapability::Clock)?;

    let now_ms = interp.host_env.now_millis();
    let ms_bigint = BigInt::from(now_ms);
    let timestamp = Fraction::new(ms_bigint, BigInt::from(1000));

    interp.stack.push(create_datetime_value(timestamp));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::{DeterministicHostEnv, HostCapability};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_now_uses_deterministic_host_clock() {
        let host = Arc::new(DeterministicHostEnv::new(
            1_700_000_000_123,
            vec![],
            vec![HostCapability::Clock],
        ));
        let mut interp = Interpreter::with_host(host);

        let result = interp.execute("'time' IMPORT NOW").await;
        assert!(result.is_ok(), "NOW should succeed: {:?}", result);
        assert_eq!(interp.stack[0].to_string(), "1700000000123/1000");
    }

    #[tokio::test]
    async fn test_now_missing_capability_emits_diagnostic_and_errors() {
        let host = Arc::new(DeterministicHostEnv::new(0, vec![], vec![]));
        let mut interp = Interpreter::with_host(host);

        let result = interp.execute("'time' IMPORT NOW").await;
        assert!(result.is_err(), "NOW should fail without Clock");
        assert_eq!(interp.host_effects().len(), 1);
        assert_eq!(interp.host_effects()[0].kind(), "diagnostic");
        assert!(interp.host_effects()[0]
            .payload()
            .contains("missingCapability"));
    }
}
