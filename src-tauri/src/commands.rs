use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::port_scanner;
use crate::process_resolver::{self, ProcessInfo};
use crate::process_tree;
use crate::safety_checker::{self, SafetyJudgment, SafetyLevel};
use crate::service_classifier::{self, ServiceClassification, ServiceType};
use crate::terminator::{self, TerminateResult};

#[derive(Debug, Clone, Serialize)]
pub struct PortService {
    pub id: String,
    pub port: u16,
    pub protocol: String,
    pub local_address: String,
    pub state: String,
    pub pid: u32,
    pub process_name: String,
    pub executable_path: String,
    pub command_line: String,
    pub cwd: String,
    pub user: String,
    pub parent_chain: Vec<process_tree::ProcessNode>,
    pub source: String,
    pub service_type: ServiceType,
    pub service_name: String,
    pub safety_level: SafetyLevel,
    pub safety_reason: String,
    pub can_terminate: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessDetail {
    pub process: ProcessInfo,
    pub parent_chain: Vec<process_tree::ProcessNode>,
    pub source: String,
    pub classification: ServiceClassification,
    pub safety: SafetyJudgment,
}

/// 扫描所有监听端口，返回完整的端口服务列表
#[tauri::command]
pub fn scan_ports() -> Result<Vec<PortService>, String> {
    let current_user = whoami::username();

    // Windows: 一次性批量获取所有进程信息（避免逐个调用 PowerShell，大幅提速）
    process_resolver::prefetch_all_processes();

    let ports = port_scanner::scan_listening_ports()?;
    let mut services = Vec::new();

    for port_info in ports {
        let process = match process_resolver::resolve_process(port_info.pid) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let parent_chain = process_tree::build_parent_chain(port_info.pid);
        let source = process_tree::identify_source(&parent_chain);
        let classification =
            service_classifier::classify(&process, &parent_chain, port_info.port, &source);
        let safety = safety_checker::judge(
            &classification.service_type,
            &process.name,
            &process.command_line,
            &process.user,
            &current_user,
        );

        services.push(PortService {
            id: format!("{}-{}", port_info.port, port_info.pid),
            port: port_info.port,
            protocol: port_info.protocol,
            local_address: port_info.local_address,
            state: port_info.state,
            pid: port_info.pid,
            process_name: process.name,
            executable_path: process.executable_path,
            command_line: process.command_line,
            cwd: process.cwd,
            user: process.user,
            parent_chain,
            source,
            service_type: classification.service_type,
            service_name: classification.service_name,
            safety_level: safety.level,
            safety_reason: safety.reason,
            can_terminate: safety.can_terminate,
        });
    }

    Ok(services)
}

/// 流式扫描端口：扫描到一个就通过事件推送给前端
#[tauri::command(async)]
pub async fn scan_ports_stream(app: AppHandle) -> Result<(), String> {
    // Windows: 一次性批量获取所有进程信息（避免逐个调用 PowerShell，大幅提速）
    process_resolver::prefetch_all_processes();

    let ports = port_scanner::scan_listening_ports()?;
    let current_user = whoami::username();
    let total = ports.len();

    let _ = app.emit("scan-start", total);

    for port_info in ports {
        let process = match process_resolver::resolve_process(port_info.pid) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let parent_chain = process_tree::build_parent_chain(port_info.pid);
        let source = process_tree::identify_source(&parent_chain);
        let classification =
            service_classifier::classify(&process, &parent_chain, port_info.port, &source);
        let safety = safety_checker::judge(
            &classification.service_type,
            &process.name,
            &process.command_line,
            &process.user,
            &current_user,
        );

        let service = PortService {
            id: format!("{}-{}", port_info.port, port_info.pid),
            port: port_info.port,
            protocol: port_info.protocol,
            local_address: port_info.local_address,
            state: port_info.state,
            pid: port_info.pid,
            process_name: process.name,
            executable_path: process.executable_path,
            command_line: process.command_line,
            cwd: process.cwd,
            user: process.user,
            parent_chain,
            source,
            service_type: classification.service_type,
            service_name: classification.service_name,
            safety_level: safety.level,
            safety_reason: safety.reason,
            can_terminate: safety.can_terminate,
        };

        let _ = app.emit("port-found", service);
    }

    let _ = app.emit("scan-complete", ());
    Ok(())
}

/// 获取指定 PID 的详细进程信息
#[tauri::command]
pub fn get_process_detail(pid: u32) -> Result<ProcessDetail, String> {
    let current_user = whoami::username();

    // Windows: 预取进程信息（如果还没有缓存的话）
    process_resolver::prefetch_all_processes();
    let process = process_resolver::resolve_process(pid)?;
    let parent_chain = process_tree::build_parent_chain(pid);
    let source = process_tree::identify_source(&parent_chain);
    let port = 0; // 进程详情不需要端口信息做分类

    // 尝试从命令行推断端口
    let classification = service_classifier::classify(&process, &parent_chain, port, &source);
    let safety = safety_checker::judge(
        &classification.service_type,
        &process.name,
        &process.command_line,
        &process.user,
        &current_user,
    );

    Ok(ProcessDetail {
        process,
        parent_chain,
        source,
        classification,
        safety,
    })
}

/// 终止指定进程
#[tauri::command]
pub fn terminate_process(pid: u32, force: bool) -> Result<TerminateResult, String> {
    if force {
        let result = terminator::force_terminate(pid);
        if result.success {
            // 等待进程真正退出（最多 3 秒），替代硬编码 sleep
            terminator::wait_for_process_exit(pid, 3000);
            Ok(TerminateResult {
                port_released: !terminator::is_process_alive(pid),
                ..result
            })
        } else {
            Ok(result)
        }
    } else {
        let result = terminator::terminate(pid);
        if result.success {
            // 等待进程真正退出（最多 2 秒）
            terminator::wait_for_process_exit(pid, 2000);
            Ok(TerminateResult {
                port_released: !terminator::is_process_alive(pid),
                message: format!("进程 {} 已成功终止", pid),
                ..result
            })
        } else {
            // Windows: 普通 taskkill 经常失败（WM_CLOSE 对控制台进程无效），
            // 自动回退到强制终止 taskkill /F
            let force_result = terminator::force_terminate(pid);
            if force_result.success {
                // 等待进程真正退出（最多 3 秒）
                terminator::wait_for_process_exit(pid, 3000);
                Ok(TerminateResult {
                    port_released: !terminator::is_process_alive(pid),
                    message: format!("进程 {} 已强制终止", pid),
                    ..force_result
                })
            } else {
                // 两次都失败，返回更有用的错误提示
                let mut msg = force_result.message;
                if msg.contains("Access") || msg.contains("拒绝") || msg.contains("denied") {
                    msg = format!("{}（请尝试以管理员身份运行本工具）", msg);
                }
                Ok(TerminateResult {
                    success: false,
                    message: msg,
                    port_released: false,
                })
            }
        }
    }
}

/// 检查指定端口是否仍在监听（轻量级，单端口查询）
#[tauri::command]
pub fn check_port_listening(port: u16) -> bool {
    terminator::is_port_listening(port)
}

/// 在文件管理器中打开指定路径（目录或文件所在目录）
#[tauri::command]
pub fn open_directory(path: String) -> Result<String, String> {
    let target = std::path::Path::new(&path);
    let open_path = if target.is_dir() {
        path.clone()
    } else if target.exists() {
        target
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone())
    } else {
        return Err(format!("路径不存在: {}", path));
    };

    #[cfg(unix)]
    {
        std::process::Command::new("open")
            .arg(&open_path)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }

    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(&open_path)
            .spawn()
            .map_err(|e| format!("打开目录失败: {}", e))?;
    }

    Ok(open_path)
}

// ═══════════════════════════════════════════════════════════════
// macOS .app 图标功能（仅 Unix）
// ═══════════════════════════════════════════════════════════════

/// 来源名 → .app 路径映射（仅 macOS）
#[cfg(unix)]
fn source_to_app_path(source: &str) -> Option<String> {
    let path = match source {
        // AI / IDE
        "Cursor" => "/Applications/Cursor.app",
        "Codex" => "/Applications/Codex.app",
        "Windsurf" => "/Applications/Windsurf.app",
        "VSCode" => "/Applications/Visual Studio Code.app",
        "JetBrains" => "/Applications/IntelliJ IDEA.app",
        "Xcode" => "/Applications/Xcode.app",
        "Android Studio" => "/Applications/Android Studio.app",
        "Sublime Text" => "/Applications/Sublime Text.app",
        "Zed" => "/Applications/Zed.app",
        "Vim/Neovim" => "/Applications/MacVim.app",
        "Emacs" => "/Applications/Emacs.app",
        "Claude" => "/Applications/Claude.app",
        "ChatGPT" => "/Applications/ChatGPT.app",
        // 终端
        "iTerm2" => "/Applications/iTerm.app",
        "Terminal" => "/System/Applications/Utilities/Terminal.app",
        "Warp" => "/Applications/Warp.app",
        "Alacritty" => "/Applications/Alacritty.app",
        "Kitty" => "/Applications/kitty.app",
        "Hyper" => "/Applications/Hyper.app",
        "Tabby" => "/Applications/Tabby.app",
        "WezTerm" => "/Applications/WezTerm.app",
        "Ghostty" => "/Applications/Ghostty.app",
        // 浏览器
        "Chrome" => "/Applications/Google Chrome.app",
        "Firefox" => "/Applications/Firefox.app",
        "Safari" => "/Applications/Safari.app",
        "Arc" => "/Applications/Arc.app",
        "Edge" => "/Applications/Microsoft Edge.app",
        "Brave" => "/Applications/Brave Browser.app",
        "Opera" => "/Applications/Opera.app",
        "Vivaldi" => "/Applications/Vivaldi.app",
        // 通讯 / 协作
        "Slack" => "/Applications/Slack.app",
        "Discord" => "/Applications/Discord.app",
        "Telegram" => "/Applications/Telegram.app",
        "WhatsApp" => "/Applications/WhatsApp.app",
        "Zoom" => "/Applications/zoom.us.app",
        "Teams" => "/Applications/Microsoft Teams.app",
        "Notion" => "/Applications/Notion.app",
        "Obsidian" => "/Applications/Obsidian.app",
        "飞书" => "/Applications/Lark.app",
        "钉钉" => "/Applications/DingTalk.app",
        "微信" => "/Applications/WeChat.app",
        "QQ" => "/Applications/QQ.app",
        "Raycast" => "/Applications/Raycast.app",
        "Alfred" => "/Applications/Alfred.app",
        // 其他
        "Docker" => "/Applications/Docker.app",
        "Homebrew" => return None,
        _ => return None,
    };
    Some(path.to_string())
}

/// 获取来源对应的应用图标（base64 PNG data URL）
/// 使用 macOS sips 命令从 .app 包中提取真实图标
#[cfg(unix)]
#[tauri::command]
pub fn get_source_icon(source: String, _executable_path: Option<String>) -> Option<String> {
    use std::collections::HashMap;
    use std::io::Read;
    use std::sync::Mutex;

    static ICON_CACHE: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

    // 检查缓存
    {
        let cache = ICON_CACHE.lock().unwrap();
        if let Some(ref map) = *cache {
            if let Some(cached) = map.get(&source) {
                return Some(cached.clone());
            }
        }
    }

    let app_path = source_to_app_path(&source)?;

    // 读取 Info.plist 获取图标文件名
    let plist_path = format!("{}/Contents/Info.plist", app_path);
    let plist_path = std::path::Path::new(&plist_path);
    if !plist_path.exists() {
        return None;
    }

    // 使用 PlistBuddy 读取 CFBundleIconFile
    let icon_name = std::process::Command::new("/usr/libexec/PlistBuddy")
        .args([
            "-c",
            "Print :CFBundleIconFile",
            &plist_path.to_string_lossy(),
        ])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })?;

    // 构造 .icns 路径
    let icns_path = if icon_name.ends_with(".icns") {
        format!("{}/Contents/Resources/{}", app_path, icon_name)
    } else {
        format!("{}/Contents/Resources/{}.icns", app_path, icon_name)
    };

    if !std::path::Path::new(&icns_path).exists() {
        return None;
    }

    // 使用 sips 转换为 PNG 到临时文件
    let tmp_path = format!(
        "/tmp/port-guardian-icon-{}.png",
        source.replace(' ', "_").replace('/', "_")
    );
    let status = std::process::Command::new("sips")
        .args(["-s", "format", "png", &icns_path, "--out", &tmp_path])
        .output();

    match status {
        Ok(o) if o.status.success() => {
            // 读取 PNG 并编码为 base64
            if let Ok(mut file) = std::fs::File::open(&tmp_path) {
                let mut buf = Vec::new();
                if file.read_to_end(&mut buf).is_ok() {
                    use base64::Engine;
                    let b64 = base64::engine::general_purpose::STANDARD.encode(&buf);
                    let data_url = format!("data:image/png;base64,{}", b64);

                    // 写入缓存
                    let mut cache = ICON_CACHE.lock().unwrap();
                    cache
                        .get_or_insert_with(HashMap::new)
                        .insert(source, data_url.clone());

                    // 清理临时文件
                    let _ = std::fs::remove_file(&tmp_path);
                    return Some(data_url);
                }
            }
            let _ = std::fs::remove_file(&tmp_path);
            None
        }
        _ => None,
    }
}

/// Windows 版本：从 .exe 文件提取应用图标
#[cfg(windows)]
#[tauri::command]
pub fn get_source_icon(source: String, executable_path: Option<String>) -> Option<String> {
    use std::collections::HashMap;
    use std::sync::Mutex;

    static ICON_CACHE: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);
    let exe_key_part = executable_path.as_deref().unwrap_or_default();
    let cache_key = format!("{}::{}", source, exe_key_part);

    // 检查缓存
    {
        let cache = ICON_CACHE.lock().unwrap();
        if let Some(ref map) = *cache {
            if let Some(cached) = map.get(&cache_key) {
                return if cached.is_empty() {
                    None
                } else {
                    Some(cached.clone())
                };
            }
        }
    }

    // 优先用扫描结果里的真实可执行文件路径，失败时回退到 source->安装路径映射
    let exe_path = executable_path
        .as_deref()
        .and_then(normalize_windows_exe_path)
        .or_else(|| find_windows_exe_path(&source));

    let data_url = exe_path
        .as_deref()
        .and_then(|path| extract_windows_icon_data_url(path, &cache_key));

    // 写入缓存
    let mut cache = ICON_CACHE.lock().unwrap();
    cache
        .get_or_insert_with(HashMap::new)
        .insert(cache_key, data_url.clone().unwrap_or_default());

    data_url
}

#[cfg(windows)]
fn normalize_windows_exe_path(path: &str) -> Option<String> {
    let trimmed = path.trim().trim_matches('"');
    if trimmed.is_empty() {
        return None;
    }

    let p = std::path::Path::new(trimmed);
    if p.is_file() {
        Some(trimmed.to_string())
    } else {
        None
    }
}

#[cfg(windows)]
fn extract_windows_icon_data_url(exe_path: &str, cache_key: &str) -> Option<String> {
    use crate::windows_command::hidden_command;

    // 用 PowerShell 从 .exe 提取图标，通过临时文件中转（避免管道编码问题）
    let tmp_dir = std::env::temp_dir();
    let safe_name: String = cache_key
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let tmp_png = tmp_dir.join(format!("port-guardian-icon-{}.png", safe_name));
    let tmp_ps1 = tmp_dir.join(format!("port-guardian-icon-{}.ps1", safe_name));

    // 写 PowerShell 脚本文件（避免命令行转义问题）
    let ps_content = format!(
        "$ErrorActionPreference = 'SilentlyContinue'\r\n\
         Add-Type -AssemblyName System.Drawing\r\n\
         $icon = [System.Drawing.Icon]::ExtractAssociatedIcon('{exe}')\r\n\
         if ($icon) {{\r\n\
           $bmp = $icon.ToBitmap()\r\n\
           $bmp.Save('{png}', [System.Drawing.Imaging.ImageFormat]::Png)\r\n\
           Write-Output 'OK'\r\n\
         }} else {{\r\n\
           Write-Output 'FAIL'\r\n\
         }}",
        exe = exe_path.replace('\'', "''"),
        png = tmp_png.to_string_lossy().replace('\'', "''"),
    );
    let _ = std::fs::write(&tmp_ps1, &ps_content);

    let output = hidden_command("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &tmp_ps1.to_string_lossy(),
        ])
        .output()
        .ok()?;

    // 清理脚本文件
    let _ = std::fs::remove_file(&tmp_ps1);

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() || !stdout.contains("OK") {
        let _ = std::fs::remove_file(&tmp_png);
        return None;
    }

    // 读取 PNG 文件并转 base64
    let png_data = std::fs::read(&tmp_png).ok()?;
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png_data);
    let _ = std::fs::remove_file(&tmp_png);

    Some(format!("data:image/png;base64,{}", b64))
}

/// 解析 exe 路径，支持通配符（如 Discord 的 app-* 目录）
#[cfg(windows)]
fn resolve_exe_path(path: &str) -> Option<String> {
    if !path.contains('*') {
        return if std::path::Path::new(path).exists() {
            Some(path.to_string())
        } else {
            None
        };
    }

    // 通配符路径：找到通配符所在层级的父目录，列出子目录匹配
    // 例如: C:\Users\xxx\AppData\Local\Discord\app-*\Discord.exe
    let path_obj = std::path::Path::new(path);
    let segments: Vec<_> = path_obj.components().collect();

    // 找到包含 * 的段
    let star_idx = segments
        .iter()
        .position(|c| c.as_os_str().to_string_lossy().contains('*'))?;

    // 构建通配符之前的目录路径
    let parent: std::path::PathBuf = segments[..star_idx].iter().collect();
    let rest: std::path::PathBuf = segments[star_idx + 1..].iter().collect();
    let star_prefix = segments[star_idx].as_os_str().to_string_lossy();
    let prefix = star_prefix.split('*').next().unwrap_or("");

    let entries = std::fs::read_dir(&parent).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(prefix) {
            let exe_path = entry.path().join(&rest);
            if exe_path.exists() {
                return Some(exe_path.to_string_lossy().to_string());
            }
        }
    }
    None
}

/// 根据来源名查找 Windows 可执行文件路径
#[cfg(windows)]
fn find_windows_exe_path(source: &str) -> Option<String> {
    let local_app = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let prog_files = std::env::var("ProgramFiles").unwrap_or_default();
    let prog_files_x86 = std::env::var("ProgramFiles(x86)").unwrap_or_default();

    let candidates: Vec<String> = match source {
        // AI / IDE
        "Cursor" => vec![format!(r"{}\Programs\Cursor\Cursor.exe", local_app)],
        "VSCode" => vec![format!(
            r"{}\Programs\Microsoft VS Code\Code.exe",
            local_app
        )],
        "Windsurf" => vec![format!(r"{}\Programs\Windsurf\Windsurf.exe", local_app)],
        "JetBrains" => vec![
            format!(r"{}\JetBrains\IntelliJ IDEA\bin\idea64.exe", prog_files),
            format!(r"{}\JetBrains\IntelliJ IDEA\bin\idea.exe", prog_files),
        ],
        "Android Studio" => vec![format!(
            r"{}\Android\Android Studio\bin\studio64.exe",
            prog_files
        )],
        "Sublime Text" => vec![format!(r"{}\Sublime Text\sublime_text.exe", prog_files)],
        "Zed" => vec![format!(r"{}\Zed\zed.exe", local_app)],
        "Claude" => vec![format!(r"{}\Claude\Claude.exe", local_app)],
        // 浏览器
        "Chrome" => vec![
            format!(r"{}\Google\Chrome\Application\chrome.exe", prog_files),
            format!(r"{}\Google\Chrome\Application\chrome.exe", prog_files_x86),
        ],
        "Firefox" => vec![format!(r"{}\Mozilla Firefox\firefox.exe", prog_files)],
        "Edge" => vec![format!(
            r"{}\Microsoft\Edge\Application\msedge.exe",
            prog_files_x86
        )],
        "Brave" => vec![format!(
            r"{}\BraveSoftware\Brave-Browser\Application\brave.exe",
            local_app
        )],
        "Opera" => vec![format!(r"{}\Opera\opera.exe", local_app)],
        // 通讯
        "Slack" => vec![format!(r"{}\Slack\slack.exe", local_app)],
        "Discord" => vec![format!(r"{}\Discord\app-*\Discord.exe", local_app)],
        "Telegram" => vec![format!(r"{}\Telegram Desktop\Telegram.exe", prog_files)],
        "Zoom" => vec![
            format!(r"{}\Zoom\bin\Zoom.exe", local_app),
            format!(r"{}\Zoom\bin\Zoom.exe", prog_files),
        ],
        "Teams" => vec![format!(r"{}\Microsoft\Teams\current\Teams.exe", local_app)],
        "Notion" => vec![format!(r"{}\Notion\Notion.exe", local_app)],
        "Obsidian" => vec![format!(r"{}\Obsidian\Obsidian.exe", local_app)],
        "WezTerm" => vec![format!(r"{}\WezTerm\wezterm-gui.exe", local_app)],
        "Docker" => vec![format!(r"{}\Docker\Docker\Docker Desktop.exe", prog_files)],
        "Postman" => vec![format!(r"{}\Postman\Postman.exe", local_app)],
        "Figma" => vec![format!(r"{}\Figma\Figma.exe", local_app)],
        "Spotify" => vec![format!(r"{}\Spotify\Spotify.exe", local_app)],
        _ => return None,
    };

    for path in &candidates {
        if let Some(exe) = resolve_exe_path(path) {
            return Some(exe);
        }
    }
    None
}

// ═══════════════════════════════════════════════════════════════
// 获取当前用户名（跨平台）
// ═══════════════════════════════════════════════════════════════

mod whoami {
    pub fn username() -> String {
        #[cfg(unix)]
        {
            std::env::var("USER").unwrap_or_else(|_| "unknown".to_string())
        }
        #[cfg(windows)]
        {
            std::env::var("USERNAME").unwrap_or_else(|_| "unknown".to_string())
        }
    }
}
