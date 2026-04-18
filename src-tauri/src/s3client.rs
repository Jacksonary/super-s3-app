use crate::types::AccountConfig;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{BehaviorVersion, Region, SharedCredentialsProvider};
use aws_smithy_types::checksum_config::{RequestChecksumCalculation, ResponseChecksumValidation};
use std::path::PathBuf;

/// Detect cloud provider name from endpoint URL.
pub fn provider_name(endpoint: &str) -> String {
    if endpoint.is_empty() {
        return "AWS S3".to_string();
    }
    let ep = endpoint.to_lowercase();
    if ep.contains("myhuaweicloud") {
        "华为云 OBS".to_string()
    } else if ep.contains("aliyuncs") {
        "阿里云 OSS".to_string()
    } else if ep.contains("volcengineapi") || ep.contains("volces.com") || ep.contains("tos-") {
        "火山云 TOS".to_string()
    } else if ep.contains("bcebos") {
        "百度云 BOS".to_string()
    } else if ep.contains("qiniucs") || ep.contains("qbox") {
        "七牛云 Kodo".to_string()
    } else if ep.contains("amazonaws") {
        "AWS S3".to_string()
    } else if ep.contains("tencentcos") || ep.contains("myqcloud") {
        "腾讯云 COS".to_string()
    } else {
        // Extract hostname
        endpoint
            .split("//")
            .last()
            .unwrap_or(endpoint)
            .split('/')
            .next()
            .unwrap_or(endpoint)
            .to_string()
    }
}

/// Build an S3 client for the given account config.
pub fn make_client(account: &AccountConfig) -> aws_sdk_s3::Client {
    let ep = account.endpoint.to_lowercase();
    let is_tos =
        ep.contains("volces.com") || ep.contains("volcengineapi") || ep.contains("tos-s3");
    let force_path_style = !is_tos;

    let creds = Credentials::new(&account.ak, &account.sk, None, None, "super-s3-static");
    let region = Region::new(account.region.clone());

    let mut builder = aws_sdk_s3::config::Builder::new()
        .behavior_version(BehaviorVersion::latest())
        .credentials_provider(SharedCredentialsProvider::new(creds))
        .region(region)
        .force_path_style(force_path_style)
        .request_checksum_calculation(RequestChecksumCalculation::WhenRequired)
        .response_checksum_validation(ResponseChecksumValidation::WhenRequired);

    if !account.endpoint.is_empty() {
        builder = builder.endpoint_url(&account.endpoint);
    }

    aws_sdk_s3::Client::from_conf(builder.build())
}

/// Platform-specific config directory for Super S3.
pub fn config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("super-s3").join("config.yaml")
}

/// Load config from YAML. Returns empty vec if file doesn't exist (first run).
pub fn load_config() -> Result<Vec<AccountConfig>, String> {
    let path = config_path();
    if !path.exists() {
        return Ok(vec![]);
    }
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("Failed to read config: {e}"))?;
    if content.trim().is_empty() {
        return Ok(vec![]);
    }
    // The YAML could be a list or a single mapping
    let value: serde_yaml::Value =
        serde_yaml::from_str(&content).map_err(|e| format!("Failed to parse config: {e}"))?;
    match value {
        serde_yaml::Value::Sequence(_) => {
            serde_yaml::from_value(value).map_err(|e| format!("Failed to parse config: {e}"))
        }
        serde_yaml::Value::Mapping(_) => {
            let single: AccountConfig = serde_yaml::from_value(value)
                .map_err(|e| format!("Failed to parse config: {e}"))?;
            Ok(vec![single])
        }
        _ => Ok(vec![]),
    }
}

/// Save config to YAML.
pub fn save_config(accounts: &[AccountConfig]) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config dir: {e}"))?;
    }
    let yaml =
        serde_yaml::to_string(accounts).map_err(|e| format!("Failed to serialize config: {e}"))?;
    std::fs::write(&path, yaml).map_err(|e| format!("Failed to write config: {e}"))?;
    Ok(())
}

/// Check if the endpoint is Qiniu Kodo (needs V1 list fallback).
pub fn is_qiniu(endpoint: &str) -> bool {
    let ep = endpoint.to_lowercase();
    ep.contains("qiniucs") || ep.contains("qbox")
}

/// Get account config by index.
pub fn get_account(account_idx: usize) -> Result<AccountConfig, String> {
    let accounts = load_config()?;
    accounts
        .into_iter()
        .nth(account_idx)
        .ok_or_else(|| "Account not found".to_string())
}

/// Get client for account by index.
pub fn get_client(account_idx: usize) -> Result<aws_sdk_s3::Client, String> {
    let account = get_account(account_idx)?;
    Ok(make_client(&account))
}

/// Get client together with the endpoint string.
pub fn get_client_with_endpoint(account_idx: usize) -> Result<(aws_sdk_s3::Client, String), String> {
    let account = get_account(account_idx)?;
    let endpoint = account.endpoint.clone();
    Ok((make_client(&account), endpoint))
}
