mod commands;
mod utils;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::calculate_file_hash,
            commands::repo_mirror,
            commands::upload_to_s3,
            commands::fs::list_directory,
            commands::fs::copy_file
        ])
        .run(tauri::generate_context!())
        .expect("Error while running tauri application.");
}
