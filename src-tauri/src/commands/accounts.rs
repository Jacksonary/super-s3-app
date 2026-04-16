use crate::s3client;
use crate::types::Account;

#[tauri::command]
pub fn list_accounts() -> Result<Vec<Account>, String> {
    let configs = s3client::load_config()?;
    let accounts = configs
        .iter()
        .enumerate()
        .map(|(i, acct)| {
            let name = acct
                .name
                .as_ref()
                .filter(|n| !n.is_empty())
                .cloned()
                .unwrap_or_else(|| s3client::provider_name(&acct.endpoint));
            Account {
                id: i,
                name,
                endpoint: acct.endpoint.clone(),
                region: acct.region.clone(),
                buckets: acct.buckets.clone(),
            }
        })
        .collect();
    Ok(accounts)
}
