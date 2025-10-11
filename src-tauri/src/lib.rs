mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::fs::file_hash,
            commands::fs::list_directory,
            commands::fs::copy_file,
            commands::fs::remove_path,
            commands::command_executor::execute_command_sync,
            commands::s3_atomic::list_buckets,
            commands::s3_atomic::list_objects,
            commands::s3_atomic::upload_file,
            commands::s3_atomic::delete_object,
            commands::s3_atomic::clear_s3_client_cache
        ])
        .run(tauri::generate_context!())
        .expect("Error while running tauri application.");
}
