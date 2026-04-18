use serde::Deserialize;

const GITHUB_API: &str =
    "https://api.github.com/repos/Jacksonary/super-s3-app/releases/latest";
const GITEE_API: &str =
    "https://gitee.com/api/v5/repos/weiguoliu/super-s3-app/releases/latest";

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: Option<String>,
    html_url: Option<String>,
}

#[derive(Deserialize)]
struct GiteeRelease {
    tag_name: Option<String>,
}

async fn fetch_github() -> Result<(String, String), String> {
    let client = reqwest::Client::builder()
        .user_agent("super-s3-app")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let resp: GithubRelease = client
        .get(GITHUB_API)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let tag = resp.tag_name.ok_or("no tag_name")?;
    let url = resp
        .html_url
        .unwrap_or_else(|| "https://github.com/Jacksonary/super-s3-app/releases".to_string());
    Ok((tag, url))
}

async fn fetch_gitee() -> Result<(String, String), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let resp: GiteeRelease = client
        .get(GITEE_API)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let tag = resp.tag_name.ok_or("no tag_name")?;
    let url = format!(
        "https://gitee.com/weiguoliu/super-s3-app/releases/tag/{}",
        tag
    );
    Ok((tag, url))
}

#[tauri::command]
pub async fn check_update() -> Result<serde_json::Value, String> {
    // Race both sources, return whichever succeeds first
    let (github, gitee) = tokio::join!(fetch_github(), fetch_gitee());

    let (tag, url) = github.or(gitee).map_err(|e| format!("Update check failed: {e}"))?;
    let version = tag.trim_start_matches('v').to_string();
    Ok(serde_json::json!({
        "latestVersion": version,
        "releaseUrl": url,
    }))
}
