mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::archive::compress_with_7z,
            commands::command_executor::execute_command_sync,
            commands::fs::list_directory,
            commands::fs::copy_file,
            commands::fs::remove_path,
            commands::fs::calculate_directory_size,
            commands::hash::file_hash,
            commands::s3_atomic::list_s3_buckets,
            commands::s3_atomic::list_s3_objects,
            commands::s3_atomic::upload_file_to_s3,
            commands::s3_atomic::download_file_from_s3,
            commands::s3_atomic::delete_s3_object,
            commands::s3_atomic::clear_s3_client_cache
        ])
        .run(tauri::generate_context!())
        .expect("Error while running tauri application.");
}
