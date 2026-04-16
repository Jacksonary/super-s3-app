use crate::s3client;

#[tauri::command]
pub async fn list_buckets(account_idx: usize) -> Result<serde_json::Value, String> {
    let configs = s3client::load_config()?;
    let account = configs
        .get(account_idx)
        .ok_or_else(|| "Account not found".to_string())?;

    let configured = &account.buckets;
    if !configured.is_empty() {
        return Ok(serde_json::json!({ "buckets": configured }));
    }

    let client = s3client::make_client(account);
    let resp = client
        .list_buckets()
        .send()
        .await
        .map_err(|e| format!("Failed to list buckets: {e}"))?;

    let names: Vec<String> = resp
        .buckets()
        .iter()
        .filter_map(|b| b.name().map(|s| s.to_string()))
        .collect();

    Ok(serde_json::json!({ "buckets": names }))
}
