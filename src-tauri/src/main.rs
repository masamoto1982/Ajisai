#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod serial;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(serial::SerialState::default())
        .invoke_handler(tauri::generate_handler![
            serial::serial_list_ports,
            serial::serial_open,
            serial::serial_configure,
            serial::serial_write,
            serial::serial_flush,
            serial::serial_close,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}
