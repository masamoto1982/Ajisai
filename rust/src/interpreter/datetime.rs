//! TIME@NOW: the single host-clock boundary of the TIME module.
//!
//! All civil <-> instant conversions live in `time_ops` / `time_calendar` and
//! are exact and host-independent. `NOW` is the one observable word: it reads
//! the host wall clock and yields an exact-rational instant (seconds since the
//! Unix epoch). The host clock has millisecond resolution, so the result is an
//! exact multiple of `1/1000`.

use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::create_datetime_value;
use crate::interpreter::{Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;

// TODO(portability): Route NOW through HostEnv instead of the default clock.
// A deterministic clock is required for time-dependent conformance cases
// (e.g. a DeterministicHost { now_millis, random_bytes }).

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

#[cfg(feature = "wasm")]
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

    let now_ms = default_now_millis();
    let ms_bigint = BigInt::from(now_ms);
    let timestamp = Fraction::new(ms_bigint, BigInt::from(1000));

    interp.stack.push(create_datetime_value(timestamp));
    Ok(())
}
