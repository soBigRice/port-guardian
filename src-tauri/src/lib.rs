mod commands;
mod port_scanner;
mod process_resolver;
mod process_tree;
mod safety_checker;
mod service_classifier;
mod terminator;

use commands::{
    get_process_detail, get_source_icon, open_directory, scan_ports, scan_ports_stream,
    terminate_process,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    println!("[Port Guardian] Starting application...");
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            scan_ports,
            scan_ports_stream,
            get_process_detail,
            terminate_process,
            open_directory,
            get_source_icon,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
