use std::{path::PathBuf, process::Command};
use tauri::AppHandle;

#[tauri::command]
pub async fn probe_media(app: AppHandle, file_path: String) -> Result<serde_json::Value, String> {
    let tool = crate::resolve_tool_path(&app)?;
    let path = PathBuf::from(file_path);
    if !path.is_file() {
        return Err("Media file does not exist.".to_string());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let output = Command::new(tool)
            .arg("--probe")
            .arg(path)
            .output()
            .map_err(|error| format!("Could not read media information: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "Could not read media information: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("Invalid media information: {error}"))
    })
    .await
    .map_err(|error| format!("Media information task failed: {error}"))?
}
