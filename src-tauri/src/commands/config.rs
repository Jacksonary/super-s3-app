use crate::s3client;
use crate::types::AccountConfig;

#[tauri::command]
pub fn get_config() -> Result<Vec<AccountConfig>, String> {
    s3client::load_config()
}

#[tauri::command]
pub fn put_config(accounts: Vec<AccountConfig>) -> Result<serde_json::Value, String> {
    s3client::save_config(&accounts)?;
    // Credentials changed — evict cached clients so the next request uses fresh keys.
    s3client::invalidate_client_cache();
    Ok(serde_json::json!({ "ok": true }))
}
