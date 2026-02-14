pub mod commands;
pub mod encryption;
pub mod storage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::validate_batch,
            commands::construct_batch,
            commands::generate_qr,
            commands::save_receipt,
            commands::set_storage_passphrase,
            commands::is_storage_unlocked,
            commands::clear_sensitive_memory,
            commands::init_local_storage,
            commands::create_contact,
            commands::list_contacts,
            commands::update_contact,
            commands::delete_contact,
            commands::save_draft,
            commands::list_drafts,
            commands::load_draft,
            commands::delete_draft,
            commands::export_plaintext_data,
            commands::export_plaintext_data_to_file,
            commands::encrypt_field,
            commands::decrypt_field,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
