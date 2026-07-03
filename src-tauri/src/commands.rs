use std::collections::HashMap;

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

#[derive(Debug, Clone)]
struct ProcessScanContext {
    process: ProcessInfo,
    parent_chain: Vec<process_tree::ProcessNode>,
    source: String,
}

fn scan_port_infos_and_user() -> Result<(Vec<port_scanner::PortInfo>, String), String> {
    // Windows: 一次性批量获取所有进程信息；Windows 内部带 TTL，避免静默轮询反复触发 WMI。
    process_resolver::prefetch_all_processes();

    let ports = port_scanner::scan_listening_ports()?;
    let current_user = whoami::username();
    Ok((ports, current_user))
}

fn resolve_scan_context(
    pid: u32,
    process_cache: &mut HashMap<u32, ProcessScanContext>,
) -> Result<ProcessScanContext, String> {
    if let Some(context) = process_cache.get(&pid) {
        return Ok(context.clone());
    }

    // 同一 PID 经常监听多个 TCP/UDP 端口；每轮只解析一次进程和父进程链。
    let process = process_resolver::resolve_process(pid)?;
    let parent_chain = process_tree::build_parent_chain(pid);
    let source = process_tree::identify_source(&parent_chain);
    let context = ProcessScanContext {
        process,
        parent_chain,
        source,
    };
    process_cache.insert(pid, context.clone());
    Ok(context)
}

fn build_port_service(
    port_info: port_scanner::PortInfo,
    context: &ProcessScanContext,
    current_user: &str,
) -> PortService {
    let process = &context.process;
    let classification = service_classifier::classify(
        process,
        &context.parent_chain,
        port_info.port,
        &context.source,
    );
    let safety = safety_checker::judge(
        &classification.service_type,
        &process.name,
        &process.command_line,
        &process.user,
        current_user,
    );

    PortService {
        id: format!(
            "{}-{}-{}",
            port_info.protocol, port_info.port, port_info.pid
        ),
        port: port_info.port,
        protocol: port_info.protocol,
        local_address: port_info.local_address,
        state: port_info.state,
        pid: port_info.pid,
        process_name: process.name.clone(),
        executable_path: process.executable_path.clone(),
        command_line: process.command_line.clone(),
        cwd: process.cwd.clone(),
        user: process.user.clone(),
        parent_chain: context.parent_chain.clone(),
        source: context.source.clone(),
        service_type: classification.service_type,
        service_name: classification.service_name,
        safety_level: safety.level,
        safety_reason: safety.reason,
        can_terminate: safety.can_terminate,
    }
}

// 端口已经由系统 API 扫到，但进程详情可能因为权限、进程退出或系统保护读取失败。
// 这种情况下仍展示端口，避免用户误以为“只扫到一部分”；同时禁止终止，保持安全优先。
fn build_unresolved_port_service(port_info: port_scanner::PortInfo, reason: &str) -> PortService {
    PortService {
        id: format!(
            "{}-{}-{}",
            port_info.protocol, port_info.port, port_info.pid
        ),
        port: port_info.port,
        protocol: port_info.protocol,
        local_address: port_info.local_address,
        state: port_info.state,
        pid: port_info.pid,
        process_name: if port_info.pid == 0 {
            "Unknown Process".to_string()
        } else {
            format!("PID {}", port_info.pid)
        },
        executable_path: String::new(),
        command_line: String::new(),
        cwd: String::new(),
        user: String::new(),
        parent_chain: Vec::new(),
        source: "Unknown".to_string(),
        service_type: ServiceType::Unknown,
        service_name: "Unresolved".to_string(),
        safety_level: SafetyLevel::Danger,
        safety_reason: format!("无法解析该端口对应进程，已禁止终止：{}", reason),
        can_terminate: false,
    }
}

/// 扫描所有监听端口，返回完整的端口服务列表
#[tauri::command]
pub fn scan_ports() -> Result<Vec<PortService>, String> {
    let (ports, current_user) = scan_port_infos_and_user()?;
    let mut services = Vec::with_capacity(ports.len());
    let mut process_cache = HashMap::new();

    for port_info in ports {
        match resolve_scan_context(port_info.pid, &mut process_cache) {
            Ok(context) => services.push(build_port_service(port_info, &context, &current_user)),
            Err(err) => services.push(build_unresolved_port_service(port_info, &err)),
        }
    }

    Ok(services)
}

/// 流式扫描端口：扫描到一个就通过事件推送给前端
#[tauri::command(async)]
pub async fn scan_ports_stream(app: AppHandle, stream_results: bool) -> Result<(), String> {
    let (ports, current_user) = scan_port_infos_and_user()?;
    let total = ports.len();
    let mut process_cache = HashMap::new();

    let _ = app.emit("scan-start", total);

    if !stream_results {
        let mut services = Vec::with_capacity(total);
        for port_info in ports {
            match resolve_scan_context(port_info.pid, &mut process_cache) {
                Ok(context) => {
                    services.push(build_port_service(port_info, &context, &current_user))
                }
                Err(err) => services.push(build_unresolved_port_service(port_info, &err)),
            }
        }

        // 非流式刷新走批量事件，避免大量 port-found IPC 和前端逐条处理。
        let _ = app.emit("scan-results", services);
        let _ = app.emit("scan-complete", ());
        return Ok(());
    }

    for port_info in ports {
        let service = match resolve_scan_context(port_info.pid, &mut process_cache) {
            Ok(context) => build_port_service(port_info, &context, &current_user),
            Err(err) => build_unresolved_port_service(port_info, &err),
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
        if let Some(ref map) = *cache
            && let Some(cached) = map.get(&cache_key)
        {
            return if cached.is_empty() {
                None
            } else {
                Some(cached.clone())
            };
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
fn extract_windows_icon_data_url(exe_path: &str, _cache_key: &str) -> Option<String> {
    // Windows 图标提取走 Rust 原生库，避免每个未缓存图标都启动 PowerShell 子进程。
    // 这里传真实 exe 绝对路径；systemicons 在 Windows 下会从可执行文件资源里读取图标并返回 PNG bytes。
    let png_data = systemicons::get_icon(exe_path, 32).ok()?;
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png_data);

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
