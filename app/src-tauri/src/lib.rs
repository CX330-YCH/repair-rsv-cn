use serde::Serialize;
mod media;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    thread,
};
use tauri::{
    menu::{AboutMetadataBuilder, Menu, MenuBuilder, SubmenuBuilder},
    path::BaseDirectory,
    AppHandle, Emitter, Manager, Wry,
};

struct RepairState {
    child_pid: Mutex<Option<u32>>,
    cancelled: AtomicBool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RepairEvent {
    kind: String,
    percent: Option<f64>,
    status: Option<String>,
    output_path: Option<String>,
}

#[tauri::command]
fn start_repair(
    app: AppHandle,
    state: tauri::State<'_, RepairState>,
    donor_path: String,
    broken_path: String,
) -> Result<(), String> {
    let mut child_guard = state
        .child_pid
        .lock()
        .map_err(|_| "Repair state is locked")?;
    if child_guard.is_some() {
        return Err("A repair is already running.".to_string());
    }

    let donor = PathBuf::from(&donor_path);
    let broken = PathBuf::from(&broken_path);

    if !donor.is_file() {
        return Err("Donor clip does not exist.".to_string());
    }

    if !broken.is_file() {
        return Err("Broken RSV file does not exist.".to_string());
    }

    let tool = resolve_tool_path(&app)?;
    let output_path = expected_output_path(&broken);

    let tool_display = tool.to_string_lossy().to_string();

    let mut child = Command::new(&tool)
        .arg("-rsv")
        .arg("-dst")
        .arg(&output_path)
        .arg(&donor)
        .arg(&broken)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to start repair: {error}"))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let child_pid = child.id();

    state.cancelled.store(false, Ordering::SeqCst);
    *child_guard = Some(child_pid);
    drop(child_guard);

    emit_event(
        &app,
        "started",
        Some(0.0),
        Some("Repair started.".to_string()),
        None,
    );
    emit_event(
        &app,
        "status",
        None,
        Some(format!("Running helper: {tool_display}")),
        None,
    );

    let stdout_reader = stdout.map(|reader| spawn_output_reader(app.clone(), reader, true));
    let stderr_reader = stderr.map(|reader| spawn_output_reader(app.clone(), reader, false));

    let app_for_wait = app.clone();
    thread::spawn(move || {
        let state_for_wait = app_for_wait.state::<RepairState>();
        let exit_status = child.wait();
        // Deliver buffered helper output before the final UI state.
        for reader in [stdout_reader, stderr_reader].into_iter().flatten() {
            let _ = reader.join();
        }

        if let Ok(mut guard) = state_for_wait.child_pid.lock() {
            *guard = None;
        }

        if state_for_wait.cancelled.load(Ordering::SeqCst) {
            emit_event(
                &app_for_wait,
                "cancelled",
                None,
                Some(
                    "Repair cancelled. Partial output may remain next to the RSV file.".to_string(),
                ),
                None,
            );
            return;
        }

        match exit_status {
            Ok(status)
                if status.success()
                    && output_path
                        .metadata()
                        .map(|m| m.is_file() && m.len() > 0)
                        .unwrap_or(false) =>
            {
                emit_event(
                    &app_for_wait,
                    "complete",
                    Some(100.0),
                    Some("Repair complete.".to_string()),
                    Some(output_path.to_string_lossy().to_string()),
                )
            }
            Ok(status) if status.success() => emit_event(
                &app_for_wait,
                "failed",
                None,
                Some("Repair ended without producing a video file.".to_string()),
                None,
            ),
            Ok(status) => emit_event(
                &app_for_wait,
                "failed",
                None,
                Some(format!("Repair failed: {}.", describe_exit_status(status))),
                None,
            ),
            Err(error) => emit_event(
                &app_for_wait,
                "failed",
                None,
                Some(format!("Repair process failed: {error}")),
                None,
            ),
        }
    });

    Ok(())
}

#[tauri::command]
fn cancel_repair(state: tauri::State<'_, RepairState>) -> Result<(), String> {
    state.cancelled.store(true, Ordering::SeqCst);

    let pid = {
        let guard = state
            .child_pid
            .lock()
            .map_err(|_| "Repair state is locked")?;
        *guard
    };

    match pid {
        Some(pid) => {
            let status = Command::new("kill")
                .arg("-TERM")
                .arg(pid.to_string())
                .status()
                .map_err(|error| format!("Failed to cancel repair: {error}"))?;

            if status.success() {
                Ok(())
            } else {
                Err(format!(
                    "Failed to cancel repair: kill exited with {status}"
                ))
            }
        }
        None => Err("No repair is running.".to_string()),
    }
}

#[tauri::command]
fn reveal_output(output_path: String) -> Result<(), String> {
    let path = PathBuf::from(output_path);
    if !path.exists() {
        return Err("Output file does not exist yet.".to_string());
    }

    Command::new("open")
        .arg("-R")
        .arg(path)
        .status()
        .map_err(|error| format!("Failed to reveal output: {error}"))?;

    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(RepairState {
            child_pid: Mutex::new(None),
            cancelled: AtomicBool::new(false),
        })
        .setup(|app| {
            app.set_menu(build_app_menu(app.handle(), "en")?)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_repair,
            cancel_repair,
            reveal_output,
            media::probe_media,
            set_language
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Repair RSV");
}

#[tauri::command]
fn set_language(app: AppHandle, language: String) -> Result<(), String> {
    if language != "en" && language != "zh-CN" {
        return Err("Unsupported language.".to_string());
    }
    let menu = build_app_menu(&app, &language).map_err(|error| error.to_string())?;
    app.set_menu(menu).map_err(|error| error.to_string())?;
    Ok(())
}

fn build_app_menu(app: &AppHandle, language: &str) -> tauri::Result<Menu<Wry>> {
    let tr = |en, zh| if language == "zh-CN" { zh } else { en };
    let about = AboutMetadataBuilder::new()
        .name(Some("Repair RSV"))
        .version(Some(env!("CARGO_PKG_VERSION")))
        .copyright(Some("© 2026 Dom Esposito"))
        .credits(Some(tr(
            "Original builder: Dom Esposito\nLicense: GPL-3.0-or-later",
            "原作者：Dom Esposito\n许可证：GPL-3.0-or-later",
        )))
        .build();

    let app_menu = SubmenuBuilder::new(app, "Repair RSV")
        .about_with_text(tr("About Repair RSV", "关于 Repair RSV"), Some(about))
        .separator()
        .services_with_text(tr("Services", "服务"))
        .separator()
        .hide_with_text(tr("Hide Repair RSV", "隐藏 Repair RSV"))
        .hide_others_with_text(tr("Hide Others", "隐藏其他"))
        .show_all_with_text(tr("Show All", "全部显示"))
        .separator()
        .quit_with_text(tr("Quit Repair RSV", "退出 Repair RSV"))
        .build()?;

    let edit_menu = SubmenuBuilder::new(app, tr("Edit", "编辑"))
        .undo_with_text(tr("Undo", "撤销"))
        .redo_with_text(tr("Redo", "重做"))
        .separator()
        .cut_with_text(tr("Cut", "剪切"))
        .copy_with_text(tr("Copy", "复制"))
        .paste_with_text(tr("Paste", "粘贴"))
        .select_all_with_text(tr("Select All", "全选"))
        .build()?;

    let window_menu = SubmenuBuilder::new(app, tr("Window", "窗口"))
        .minimize_with_text(tr("Minimize", "最小化"))
        .fullscreen_with_text(tr("Enter Full Screen", "进入全屏幕"))
        .separator()
        .close_window_with_text(tr("Close Window", "关闭窗口"))
        .build()?;

    MenuBuilder::new(app)
        .items(&[&app_menu, &edit_menu, &window_menu])
        .build()
}

fn spawn_output_reader<R>(app: AppHandle, reader: R, parse_progress: bool) -> thread::JoinHandle<()>
where
    R: std::io::Read + Send + 'static,
{
    thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut buffer = Vec::new();

        loop {
            buffer.clear();
            match reader.read_until(b'\r', &mut buffer) {
                Ok(0) => break,
                Ok(_) => {
                    for part in split_output_chunk(&buffer) {
                        if part.is_empty() {
                            continue;
                        }

                        if parse_progress {
                            if let Some(percent) = parse_percent(&part) {
                                emit_event(&app, "progress", Some(percent), None, None);
                                continue;
                            }
                        }

                        emit_event(&app, "status", None, Some(part), None);
                    }
                }
                Err(error) => {
                    emit_event(
                        &app,
                        "status",
                        None,
                        Some(format!("Output read error: {error}")),
                        None,
                    );
                    break;
                }
            }
        }
    })
}

fn split_output_chunk(buffer: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(buffer)
        .split(['\r', '\n'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_percent(line: &str) -> Option<f64> {
    line.split_whitespace()
        .find_map(|token| token.strip_suffix('%')?.parse::<f64>().ok())
}

fn emit_event(
    app: &AppHandle,
    kind: &str,
    percent: Option<f64>,
    status: Option<String>,
    output_path: Option<String>,
) {
    let _ = app.emit(
        "repair-event",
        RepairEvent {
            kind: kind.to_string(),
            percent,
            status,
            output_path,
        },
    );
}

fn resolve_tool_path(app: &AppHandle) -> Result<PathBuf, String> {
    if let Ok(resource_path) = app
        .path()
        .resolve("bin/untrunc-rsv", BaseDirectory::Resource)
    {
        if resource_path.is_file() {
            return Ok(resource_path);
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "Cannot locate repository root.".to_string())?;
    let dev_path = repo_root.join("bin/untrunc-rsv");

    if dev_path.is_file() {
        Ok(dev_path)
    } else {
        Err("Cannot find bundled untrunc-rsv binary.".to_string())
    }
}

fn expected_output_path(broken_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}_fixed-rsv.MP4", broken_path.to_string_lossy()))
}

fn describe_exit_status(status: std::process::ExitStatus) -> String {
    if let Some(code) = status.code() {
        return format!("exit code {code}");
    }

    #[cfg(unix)]
    if let Some(signal) = status.signal() {
        return format!("terminated by signal {signal}");
    }

    "terminated without an exit code".to_string()
}
