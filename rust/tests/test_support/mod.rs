use ajisai_core::interpreter::Interpreter;
use ajisai_core::types::Value;

pub async fn exec_ok_gui(code: &str) -> Result<Vec<Value>, String> {
    let mut interp = Interpreter::new();
    interp.gui_mode = true;
    interp.execute(code).await.map_err(|e| e.to_string())?;
    Ok(interp.get_stack().clone())
}

pub async fn exec_err_gui(code: &str) -> bool {
    let mut interp = Interpreter::new();
    interp.gui_mode = true;
    interp.execute(code).await.is_err()
}
