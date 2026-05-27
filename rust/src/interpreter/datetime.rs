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
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Date, js_name = now)]
    fn date_now() -> f64;
}

pub fn op_now(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported {
            word: "NOW".into(),
            mode: "Stack".into(),
        });
    }

    let now_ms = date_now();
    let ms_bigint = BigInt::from(now_ms as i64);
    let timestamp = Fraction::new(ms_bigint, BigInt::from(1000));

    interp.stack.push(create_datetime_value(timestamp));
    Ok(())
}
