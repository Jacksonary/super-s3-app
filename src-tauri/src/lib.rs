mod commands;
mod s3client;
pub mod types;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![
            // Config
            commands::config::get_config,
            commands::config::put_config,
            // Accounts
            commands::accounts::list_accounts,
            // Buckets
            commands::buckets::list_buckets,
            // Objects
            commands::objects::list_objects,
            commands::objects::search_objects,
            commands::objects::delete_objects,
            commands::objects::create_folder,
            commands::objects::rename_object,
            // Transfer
            commands::transfer::download_object,
            commands::transfer::batch_download,
            commands::transfer::upload_object,
            commands::transfer::upload_object_bytes,
            commands::transfer::presign_object,
            // Metadata
            commands::metadata::object_meta,
            commands::metadata::preview_object,
            commands::metadata::update_text,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
