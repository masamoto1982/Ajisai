// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// ( --- ここに src/rational.rs, vstack.rs, operator.rs, trie_store.rs, interpreter.rs の
//     mod宣言が必要です --- )
mod rational;
mod vstack;
mod operator;
mod trie_store;
mod interpreter;

use interpreter::{AjisaiError, Interpreter};
use std::sync::{Arc, RwLock};
use tauri::Manager;

// InterpreterをTauriのStateとして管理するためのラッパー
struct AppState(Arc<RwLock<Interpreter>>);

// フロントエンド(TS)から呼び出されるTauriコマンド
#[tauri::command]
async fn eval_code(code: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    // Interpreterのevalを呼び出す
    state.0.write().unwrap().eval(&code)
         .map_err(|e: AjisaiError| e.to_string())
}

// (Tauriコマンド) Operatorリストを取得
#[tauri::command]
async fn get_operators(state: tauri::State<'_, AppState>) -> Result<Vec<(String, String)>, String> {
    let operators = state.0.read().unwrap().get_operators_store();
    let pairs = operators.read().unwrap().get_all_pairs();
    
    // (Arc<Operator>) を (Name, Signature) のタプルに変換
    let result = pairs.into_iter()
        .map(|(name, op)| (name, op.signature()))
        .collect();
    Ok(result)
}

// (Tauriコマンド) Operandリストを取得
#[tauri::command]
async fn get_operands(state: tauri::State<'_, AppState>) -> Result<Vec<(String, String)>, String> {
    let operands = state.0.read().unwrap().get_operands_store();
    let pairs = operands.read().unwrap().get_all_pairs();
    
    // (Arc<RwLock<VStack>>) を (Name, Content) のタプルに変換
    let result = pairs.into_iter()
        .map(|(name, v)| (name, v.read().unwrap().to_string()))
        .collect();
    Ok(result)
}


fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Interpreterを初期化し、Stateとして登録
            let app_handle = app.handle();
            let interpreter = Interpreter::new(app_handle);
            let app_state = AppState(Arc::new(RwLock::new(interpreter)));
            app.manage(app_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            eval_code,
            get_operators,
            get_operands
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
