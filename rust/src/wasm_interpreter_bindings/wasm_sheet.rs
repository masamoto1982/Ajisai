//! The sheet view of the User dictionary, for the Dictionary panel
//! (spec/gui-semantics.md, Sheet). Reading a cell runs nothing of the
//! session: see `interpreter::sheet_cell`.

use super::{set_js_prop, AjisaiInterpreter};
use crate::interpreter::sheet_cell::CellValue;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl AjisaiInterpreter {
    /// `{ formula, inputs, outputs, signature, identity }` for the User Word
    /// `name`, `inputs` and `outputs` being `null` where inference cannot
    /// fix them; `null` when no User Word holds the name.
    #[wasm_bindgen]
    pub fn describe_sheet_cell(&self, name: &str) -> JsValue {
        let Some(cell) = self.interpreter.sheet_cell(name) else {
            return JsValue::NULL;
        };
        let obj = js_sys::Object::new();
        set_js_prop(&obj, "formula", &cell.formula.into());
        let (inputs, outputs) = match cell.effect {
            Some((i, o)) => (JsValue::from(i), JsValue::from(o)),
            None => (JsValue::NULL, JsValue::NULL),
        };
        set_js_prop(&obj, "inputs", &inputs);
        set_js_prop(&obj, "outputs", &outputs);
        set_js_prop(
            &obj,
            "signature",
            &cell.signature.map_or(JsValue::NULL, Into::into),
        );
        set_js_prop(
            &obj,
            "identity",
            &cell.identity.map_or(JsValue::NULL, Into::into),
        );
        obj.into()
    }

    /// What the User Word `name` leaves on an empty stack, run apart from the
    /// session: `{ value, suggestedType, table? }` or `{ error }`, `table`
    /// being `{ columns, cells, rows }` when the value is a Record of columns.
    #[wasm_bindgen]
    pub async fn evaluate_sheet_cell(&self, name: &str) -> JsValue {
        let obj = js_sys::Object::new();
        match self.interpreter.evaluate_sheet_cell(name).await {
            CellValue::Value {
                display,
                suggested_type,
                table,
            } => {
                set_js_prop(&obj, "value", &display.into());
                set_js_prop(
                    &obj,
                    "suggestedType",
                    &suggested_type.map_or(JsValue::NULL, Into::into),
                );
                if let Some(table) = table {
                    let strings = |items: &[String]| {
                        items
                            .iter()
                            .map(|item| JsValue::from_str(item))
                            .collect::<js_sys::Array>()
                    };
                    let shown = js_sys::Object::new();
                    set_js_prop(&shown, "columns", &strings(&table.columns).into());
                    let cells: js_sys::Array = table
                        .cells
                        .iter()
                        .map(|row| JsValue::from(strings(row)))
                        .collect();
                    set_js_prop(&shown, "cells", &cells.into());
                    set_js_prop(&shown, "rows", &JsValue::from(table.rows as u32));
                    set_js_prop(&obj, "table", &shown.into());
                }
            }
            CellValue::Error(message) => set_js_prop(&obj, "error", &message.into()),
        }
        obj.into()
    }
}
