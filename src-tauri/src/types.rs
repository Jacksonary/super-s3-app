use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// YAML config entry — one S3-compatible account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    #[serde(default)]
    pub name: Option<String>,
    pub ak: String,
    pub sk: String,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default = "default_region")]
    pub region: String,
    #[serde(default)]
    pub buckets: Vec<String>,
}

fn default_region() -> String {
    "us-east-1".to_string()
}

/// Frontend-facing account summary (no credentials).
#[derive(Debug, Serialize)]
pub struct Account {
    pub id: usize,
    pub name: String,
    pub endpoint: String,
    pub region: String,
    pub buckets: Vec<String>,
}

/// A single object or folder entry in a listing.
#[derive(Debug, Serialize)]
pub struct ObjectItem {
    pub key: String,
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub size: Option<i64>,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
    pub storage_class: Option<String>,
}

/// Response for list_objects.
#[derive(Debug, Serialize)]
pub struct ListResult {
    pub prefix: String,
    pub delimiter: String,
    pub items: Vec<ObjectItem>,
    pub next_continuation_token: Option<String>,
    pub is_truncated: bool,
    pub key_count: i32,
}

/// Response for search_objects.
#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub items: Vec<ObjectItem>,
    pub is_truncated: bool,
    pub next_continuation_token: Option<String>,
}

/// Response for delete_objects.
#[derive(Debug, Serialize)]
pub struct DeleteResult {
    pub deleted: i32,
    pub errors: Vec<DeleteError>,
}

#[derive(Debug, Serialize)]
pub struct DeleteError {
    #[serde(rename = "Key")]
    pub key: String,
    #[serde(rename = "Message")]
    pub message: String,
}

/// HEAD object metadata.
#[derive(Debug, Serialize)]
pub struct ObjectMeta {
    pub content_type: Option<String>,
    pub content_length: Option<i64>,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
    pub expires: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Upload result.
#[derive(Debug, Serialize)]
pub struct UploadResult {
    pub success: bool,
    pub key: String,
    pub size: usize,
}

/// Upload progress event payload.
#[derive(Debug, Clone, Serialize)]
pub struct UploadProgress {
    pub task_id: String,
    pub progress: u8,
}

/// Batch download progress event payload.
#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub total: u32,
    pub completed: u32,
    pub failed: u32,
    pub current_key: String,
}
