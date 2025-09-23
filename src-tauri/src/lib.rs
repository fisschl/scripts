mod commands;
mod utils;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::calculate_file_hash,
            commands::repo_mirror,
            commands::copy_files_with_options,
            commands::upload_to_s3
        ])
        .run(tauri::generate_context!())
        .expect("Error while running tauri application.");
}
