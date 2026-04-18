use crate::s3client;
use crate::types::UploadProgress;
use std::path::PathBuf;
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

/// Batch download files to a local folder, preserving relative paths.
#[tauri::command]
pub async fn batch_download(
    account_idx: usize,
    bucket: String,
    keys: Vec<String>,
    save_dir: String,
    strip_prefix: Option<String>,
) -> Result<serde_json::Value, String> {
    if keys.is_empty() {
        return Ok(serde_json::json!({ "success": true, "downloaded": 0 }));
    }

    let client = s3client::get_client(account_idx)?;
    let strip = strip_prefix.unwrap_or_default();
    let base = PathBuf::from(&save_dir);
    let mut downloaded = 0u32;
    let mut errors: Vec<String> = vec![];

    for key in &keys {
        if key.ends_with('/') {
            continue;
        }
        let relative = if !strip.is_empty() && key.starts_with(&strip) {
            &key[strip.len()..]
        } else {
            key.as_str()
        };
        let dest = base.join(relative);

        if let Some(parent) = dest.parent() {
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                errors.push(format!("{key}: failed to create dir: {e}"));
                continue;
            }
        }

        match client.get_object().bucket(&bucket).key(key).send().await {
            Ok(resp) => {
                match resp.body.collect().await {
                    Ok(body) => {
                        if let Err(e) = tokio::fs::write(&dest, body.into_bytes()).await {
                            errors.push(format!("{key}: write failed: {e}"));
                        } else {
                            downloaded += 1;
                        }
                    }
                    Err(e) => errors.push(format!("{key}: read body failed: {e}")),
                }
            }
            Err(e) => errors.push(format!("{key}: download failed: {e}")),
        }
    }

    Ok(serde_json::json!({
        "success": errors.is_empty(),
        "downloaded": downloaded,
        "errors": errors,
    }))
}
