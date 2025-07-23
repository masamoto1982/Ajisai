mod builtins;
mod interpreter;
mod stack;
mod tokenizer;
mod types;

use std::cell::RefCell;

use wasm_bindgen::prelude::*;

use interpreter::Interpreter;
use types::Type;

thread_local! {
    static INTERPRETER: RefCell<Interpreter> = RefCell::new(Interpreter::new());
}

#[wasm_bindgen]
pub fn run(code: &str) -> Result<JsValue, JsValue> {
    console_error_panic_hook::set_once();
    INTERPRETER.with(|cell| {
        let mut interpreter = cell.borrow_mut();
        interpreter
            .run(code)
            .map_err(|e| JsValue::from_str(&e))?;
        let stack_snapshot = interpreter
            .stack
            .iter()
            .map(type_to_js)
            .collect::<js_sys::Array>();
        Ok(stack_snapshot.into())
    })
}

// Helper function to convert our Rust `Type` into a JavaScript `JsValue`
fn type_to_js(t: &Type) -> JsValue {
    let obj = js_sys::Object::new();
    match t {
        Type::Number(n) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"number".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &n.to_string().into()).unwrap();
        }
        Type::String(s) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"string".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &s.as_str().into()).unwrap();
        }
        Type::Bool(b) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"boolean".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &(*b).into()).unwrap();
        }
        Type::Symbol(s) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"symbol".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &s.as_str().into()).unwrap();
        }
        Type::Vector(v) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"vector".into()).unwrap();
            let arr = v
                .borrow()
                .iter()
                .map(type_to_js)
                .collect::<js_sys::Array>();
            js_sys::Reflect::set(&obj, &"value".into(), &arr.into()).unwrap();
        }
        Type::Quotation(q) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"quotation".into()).unwrap();
            let s = q
                .iter()
                .map(|tok| format!("{:?}", tok))
                .collect::<Vec<_>>()
                .join(" ");
            js_sys::Reflect::set(&obj, &"value".into(), &format!("{{ {} }}", s).into()).unwrap();
        }
        Type::Word(w) => {
            js_sys::Reflect::set(&obj, &"type".into(), &"word".into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &format!("{:?}", w).into()).unwrap();
        }
    }
    obj.into()
}
