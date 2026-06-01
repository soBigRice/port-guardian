mod port_scanner;
mod process_resolver;
mod process_tree;
mod service_classifier;
mod safety_checker;
mod terminator;
mod commands;

use commands::{scan_ports, get_process_detail, terminate_process};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    println!("[Port Guardian] Starting application...");
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            scan_ports,
            get_process_detail,
            terminate_process,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
