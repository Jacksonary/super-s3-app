use crate::s3client;
use crate::types::{DownloadProgress, ExpandedEntry, TaskProgress};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::Emitter;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Semaphore;

fn emit_progress(app: &tauri::AppHandle, event: &str, task_id: &Option<String>, progress: u8) {
    if let Some(tid) = task_id {
        let _ = app.emit(event, TaskProgress { task_id: tid.clone(), progress });
    }
}

/// Returns a temp path alongside `dest` that won't clobber the original extension.
/// e.g. `photo.jpg` → `photo.jpg.a3f1b2c0.part`
fn tmp_path_for(dest: &std::path::Path) -> std::path::PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Use total nanos since epoch (not subsec_nanos) to minimise collision risk
    // between concurrent downloads targeting the same directory.
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let fname = dest
        .file_name()
        .map(|n| format!("{}.{nanos:016x}.part", n.to_string_lossy()))
        .unwrap_or_else(|| format!("download.{nanos:016x}.part"));
    dest.parent().unwrap_or(dest).join(fname)
}

/// Validate that `relative` (an S3 key suffix) does not escape `base` via `..` or absolute paths.
fn safe_dest(base: &std::path::Path, relative: &str) -> Option<std::path::PathBuf> {
    if relative.starts_with('/') || relative.starts_with('\\') {
        return None;
    }
    let dest = base.join(relative);
    // Lexically normalise both paths and confirm dest is a child of base.
    let norm_dest = normalise_path(&dest);
    let norm_base = normalise_path(base);
    if norm_dest.starts_with(&norm_base) { Some(dest) } else { None }
}

fn normalise_path(path: &std::path::Path) -> std::path::PathBuf {
    let mut out = std::path::PathBuf::new();
    for c in path.components() {
        match c {
            std::path::Component::ParentDir => { out.pop(); }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

#[tauri::command]
pub async fn download_object(
    app: tauri::AppHandle,
    account_idx: usize,
    bucket: String,
    key: String,
    save_path: String,
    task_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let save = std::path::Path::new(&save_path);
    let tmp_path = tmp_path_for(save);

    let result =
        download_inner(&app, account_idx, &bucket, &key, &tmp_path, &task_id).await;

    match result {
        Ok(()) => {
            tokio::fs::rename(&tmp_path, save)
                .await
                .map_err(|e| format!("Failed to finalise download: {e}"))?;
            Ok(serde_json::json!({ "success": true }))
        }
        Err(e) => {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            Err(e)
        }
    }
}

async fn download_inner(
    app: &tauri::AppHandle,
    account_idx: usize,
    bucket: &str,
    key: &str,
    tmp_path: &std::path::Path,
    task_id: &Option<String>,
) -> Result<(), String> {
    let client = s3client::get_client(account_idx)?;

    let resp = client
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| format!("Failed to download object: {e}"))?;

    let total = resp.content_length().unwrap_or(0) as u64;
    let mut body = resp.body;
    let mut file = tokio::fs::File::create(tmp_path)
        .await
        .map_err(|e| format!("Failed to create file: {e}"))?;

    emit_progress(app, "download-single-progress", task_id, 0);
    let mut received: u64 = 0;
    let mut last_pct: u8 = 0;

    while let Some(chunk) = body.next().await {
        let bytes = chunk.map_err(|e| format!("Failed to read body: {e}"))?;
        file.write_all(&bytes)
            .await
            .map_err(|e| format!("Failed to write file: {e}"))?;
        received += bytes.len() as u64;
        if total > 0 {
            let pct = (((received as u128 * 99) / total as u128).min(99)) as u8;
            if pct > last_pct {
                last_pct = pct;
                emit_progress(app, "download-single-progress", task_id, pct);
            }
        }
    }

    file.flush().await.map_err(|e| format!("Failed to flush file: {e}"))?;
    emit_progress(app, "download-single-progress", task_id, 100);
    Ok(())
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
    let ct = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

    let metadata = tokio::fs::metadata(&file_path)
        .await
        .map_err(|e| format!("Failed to stat file: {e}"))?;
    let size = metadata.len();
    let content_len = i64::try_from(size).map_err(|_| "File too large (>9.2 EB)".to_string())?;

    emit_progress(&app, "upload-progress", &task_id, 0);

    // S3 multipart minimum part size is 5 MB. Use multipart for files ≥5 MB
    // so we can report real progress. Tiny files (<5 MB) read into memory once.
    const MULTIPART_THRESHOLD: u64 = 5 * 1024 * 1024; // 5 MB
    const PART_SIZE: usize = 5 * 1024 * 1024; // 5 MB

    if size < MULTIPART_THRESHOLD {
        // Tiny file: read into memory, single put_object (no meaningful progress)
        let data = tokio::fs::read(&file_path)
            .await
            .map_err(|e| format!("Failed to read file: {e}"))?;
        client
            .put_object()
            .bucket(&bucket)
            .key(&key)
            .body(ByteStream::from(data))
            .content_length(content_len)
            .content_type(&ct)
            .send()
            .await
            .map_err(|e| format!("Failed to upload: {e}"))?;
    } else {
        // ≥5 MB: multipart upload with per-part progress
        let upload_id = client
            .create_multipart_upload()
            .bucket(&bucket)
            .key(&key)
            .content_type(&ct)
            .send()
            .await
            .map_err(|e| format!("Failed to create multipart upload: {e}"))?
            .upload_id
            .ok_or_else(|| "Missing upload_id".to_string())?;

        let total_parts = (size as usize).div_ceil(PART_SIZE) as u32;
        debug_assert!(total_parts > 0);
        let mut completed_parts: Vec<CompletedPart> = Vec::new();
        let mut file = tokio::fs::File::open(&file_path)
            .await
            .map_err(|e| format!("Failed to open file: {e}"))?;
        let mut part_number = 1i32;
        let mut buf = vec![0u8; PART_SIZE];

        loop {
            let mut bytes_read = 0usize;
            loop {
                let n = file
                    .read(&mut buf[bytes_read..])
                    .await
                    .map_err(|e| format!("Failed to read file: {e}"))?;
                if n == 0 {
                    break;
                }
                bytes_read += n;
                if bytes_read == PART_SIZE {
                    break;
                }
            }
            if bytes_read == 0 {
                break;
            }

            let chunk = buf[..bytes_read].to_vec();
            let part_result = client
                .upload_part()
                .bucket(&bucket)
                .key(&key)
                .upload_id(&upload_id)
                .part_number(part_number)
                .body(ByteStream::from(chunk))
                .send()
                .await;

            let part = match part_result {
                Ok(p) => p,
                Err(e) => {
                    let _ = client
                        .abort_multipart_upload()
                        .bucket(&bucket)
                        .key(&key)
                        .upload_id(&upload_id)
                        .send()
                        .await;
                    return Err(format!("Failed to upload: {e}"));
                }
            };

            let etag = part.e_tag().ok_or_else(|| {
                format!("Part {part_number}: S3 returned no ETag — cannot complete multipart upload")
            })?;

            completed_parts.push(
                CompletedPart::builder()
                    .e_tag(etag)
                    .part_number(part_number)
                    .build(),
            );

            let progress = ((part_number as u32) * 99 / total_parts).min(99) as u8;
            emit_progress(&app, "upload-progress", &task_id, progress);
            part_number += 1;
        }

        let completed = CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts))
            .build();

        let complete_result = client
            .complete_multipart_upload()
            .bucket(&bucket)
            .key(&key)
            .upload_id(&upload_id)
            .multipart_upload(completed)
            .send()
            .await;

        if let Err(e) = complete_result {
            let _ = client
                .abort_multipart_upload()
                .bucket(&bucket)
                .key(&key)
                .upload_id(&upload_id)
                .send()
                .await;
            return Err(format!("Failed to complete multipart upload: {e}"));
        }
    }

    emit_progress(&app, "upload-progress", &task_id, 100);
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
/// Uses up to 5 concurrent downloads for performance.
#[tauri::command]
pub async fn batch_download(
    app: tauri::AppHandle,
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
    let file_keys: Vec<&String> = keys.iter().filter(|k| !k.ends_with('/')).collect();
    let total = file_keys.len() as u32;

    let completed = Arc::new(AtomicU32::new(0));
    let failed = Arc::new(AtomicU32::new(0));
    let errors = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
    let semaphore = Arc::new(Semaphore::new(5));

    let mut handles = Vec::new();
    for key in file_keys {
        let client = client.clone();
        let bucket = bucket.clone();
        let key = key.clone();
        let strip = strip.clone();
        let base = base.clone();
        let app = app.clone();
        let completed = Arc::clone(&completed);
        let failed = Arc::clone(&failed);
        let errors = Arc::clone(&errors);
        let semaphore = Arc::clone(&semaphore);

        handles.push(tokio::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();

            let _ = app.emit(
                "download-progress",
                DownloadProgress {
                    total,
                    completed: completed.load(Ordering::Relaxed),
                    failed: failed.load(Ordering::Relaxed),
                    current_key: key.clone(),
                },
            );

            let relative = if !strip.is_empty() && key.starts_with(&strip) {
                &key[strip.len()..]
            } else {
                key.as_str()
            };

            let dest = match safe_dest(&base, relative) {
                Some(d) => d,
                None => {
                    errors.lock().await.push(format!("{key}: unsafe path rejected"));
                    failed.fetch_add(1, Ordering::Relaxed);
                    return;
                }
            };

            if let Some(parent) = dest.parent() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    errors.lock().await.push(format!("{key}: failed to create dir: {e}"));
                    failed.fetch_add(1, Ordering::Relaxed);
                    return;
                }
            }

            match client.get_object().bucket(&bucket).key(&key).send().await {
                Ok(resp) => {
                    let mut body = resp.body;
                    let tmp = tmp_path_for(&dest);
                    let write_result: Result<(), String> = async {
                        let mut file = tokio::fs::File::create(&tmp)
                            .await
                            .map_err(|e| format!("create failed: {e}"))?;
                        while let Some(chunk) = body.next().await {
                            let bytes = chunk.map_err(|e| format!("read failed: {e}"))?;
                            file.write_all(&bytes).await.map_err(|e| format!("write failed: {e}"))?;
                        }
                        file.flush().await.map_err(|e| format!("flush failed: {e}"))?;
                        Ok(())
                    }.await;
                    match write_result {
                        Ok(()) => match tokio::fs::rename(&tmp, &dest).await {
                            Ok(()) => { completed.fetch_add(1, Ordering::Relaxed); }
                            Err(e) => {
                                let _ = tokio::fs::remove_file(&tmp).await;
                                errors.lock().await.push(format!("{key}: rename failed: {e}"));
                                failed.fetch_add(1, Ordering::Relaxed);
                            }
                        },
                        Err(e) => {
                            let _ = tokio::fs::remove_file(&tmp).await;
                            errors.lock().await.push(format!("{key}: {e}"));
                            failed.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
                Err(e) => {
                    errors.lock().await.push(format!("{key}: download failed: {e}"));
                    failed.fetch_add(1, Ordering::Relaxed);
                }
            }

            let _ = app.emit(
                "download-progress",
                DownloadProgress {
                    total,
                    completed: completed.load(Ordering::Relaxed),
                    failed: failed.load(Ordering::Relaxed),
                    current_key: String::new(),
                },
            );
        }));
    }

    for h in handles {
        if let Err(e) = h.await {
            errors.lock().await.push(format!("Task panicked: {e}"));
            failed.fetch_add(1, Ordering::Relaxed);
        }
    }

    let final_completed = completed.load(Ordering::Relaxed);
    let final_errors = errors.lock().await.clone();

    Ok(serde_json::json!({
        "success": final_errors.is_empty(),
        "downloaded": final_completed,
        "errors": final_errors,
    }))
}

/// Expand a list of local paths (files or directories) into a flat list of
/// ExpandedEntry for upload. Directories are walked recursively; files produce
/// a single entry whose relative_path is the filename.
#[tauri::command]
pub async fn expand_paths(paths: Vec<String>) -> Result<Vec<ExpandedEntry>, String> {
    let mut result = Vec::new();

    for path_str in &paths {
        let path = std::path::Path::new(path_str);
        let meta = tokio::fs::metadata(path)
            .await
            .map_err(|e| format!("Cannot stat {path_str}: {e}"))?;

        if meta.is_file() {
            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file")
                .to_string();
            result.push(ExpandedEntry { local_path: path_str.clone(), relative_path: filename });
        } else if meta.is_dir() {
            let parent = path.parent().unwrap_or(path);
            walk_dir(path, parent, &mut result).await?;
        }
    }

    Ok(result)
}

async fn walk_dir(
    dir: &std::path::Path,
    base: &std::path::Path,
    out: &mut Vec<ExpandedEntry>,
) -> Result<(), String> {
    let mut read_dir = tokio::fs::read_dir(dir)
        .await
        .map_err(|e| format!("Cannot read dir {}: {e}", dir.display()))?;

    while let Some(entry) = read_dir.next_entry().await.map_err(|e| e.to_string())? {
        let path = entry.path();
        let ft = entry.file_type().await.map_err(|e| e.to_string())?;
        if ft.is_file() {
            let local = path.to_string_lossy().to_string();
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.push(ExpandedEntry { local_path: local, relative_path: rel });
        } else if ft.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !name.starts_with('.') {
                Box::pin(walk_dir(&path, base, out)).await?;
            }
        }
        // symlinks and other special files are silently skipped
    }
    Ok(())
}
