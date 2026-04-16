use crate::s3client;
use crate::types::UploadProgress;
use std::time::Duration;
use tauri::Emitter;

#[tauri::command]
pub async fn download_object(
    account_idx: usize,
    bucket: String,
    key: String,
    save_path: String,
) -> Result<serde_json::Value, String> {
    let client = s3client::get_client(account_idx)?;

    let resp = client
        .get_object()
        .bucket(&bucket)
        .key(&key)
        .send()
        .await
        .map_err(|e| format!("Failed to download object: {e}"))?;

    let body = resp
        .body
        .collect()
        .await
        .map_err(|e| format!("Failed to read body: {e}"))?;

    tokio::fs::write(&save_path, body.into_bytes())
        .await
        .map_err(|e| format!("Failed to write file: {e}"))?;

    Ok(serde_json::json!({ "success": true }))
}

#[tauri::command]
pub async fn upload_object(
    app: tauri::AppHandle,
    account_idx: usize,
    bucket: String,
    key: String,
    file_path: String,
    content_type: Option<String>,
    task_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let client = s3client::get_client(account_idx)?;

    let data = tokio::fs::read(&file_path)
        .await
        .map_err(|e| format!("Failed to read file: {e}"))?;

    let size = data.len();
    let ct = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

    // Emit progress: started
    if let Some(ref tid) = task_id {
        let _ = app.emit(
            "upload-progress",
            UploadProgress {
                task_id: tid.clone(),
                progress: 0,
            },
        );
    }

    client
        .put_object()
        .bucket(&bucket)
        .key(&key)
        .body(aws_sdk_s3::primitives::ByteStream::from(data))
        .content_type(&ct)
        .send()
        .await
        .map_err(|e| format!("Failed to upload: {e}"))?;

    // Emit progress: done
    if let Some(ref tid) = task_id {
        let _ = app.emit(
            "upload-progress",
            UploadProgress {
                task_id: tid.clone(),
                progress: 100,
            },
        );
    }

    Ok(serde_json::json!({ "success": true, "key": key, "size": size }))
}

/// Upload from raw bytes (for drag-drop where we don't have a file path).
#[tauri::command]
pub async fn upload_object_bytes(
    account_idx: usize,
    bucket: String,
    key: String,
    data: Vec<u8>,
    content_type: Option<String>,
) -> Result<serde_json::Value, String> {
    let client = s3client::get_client(account_idx)?;

    let size = data.len();
    let ct = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

    client
        .put_object()
        .bucket(&bucket)
        .key(&key)
        .body(aws_sdk_s3::primitives::ByteStream::from(data))
        .content_type(&ct)
        .send()
        .await
        .map_err(|e| format!("Failed to upload: {e}"))?;

    Ok(serde_json::json!({ "success": true, "key": key, "size": size }))
}

#[tauri::command]
pub async fn presign_object(
    account_idx: usize,
    bucket: String,
    key: String,
    expires: Option<u64>,
) -> Result<serde_json::Value, String> {
    let client = s3client::get_client(account_idx)?;
    let expires_in = Duration::from_secs(expires.unwrap_or(3600));

    let presigning_config = aws_sdk_s3::presigning::PresigningConfig::expires_in(expires_in)
        .map_err(|e| format!("Invalid expiration: {e}"))?;

    let presigned = client
        .get_object()
        .bucket(&bucket)
        .key(&key)
        .presigned(presigning_config)
        .await
        .map_err(|e| format!("Failed to generate presigned URL: {e}"))?;

    Ok(serde_json::json!({ "url": presigned.uri().to_string() }))
}
