use std::collections::HashMap;
use std::io::Read;
use std::sync::Mutex;

use serde::Serialize;

use crate::port_scanner;
use crate::process_resolver::{self, ProcessInfo};
use crate::process_tree;
use crate::service_classifier::{self, ServiceClassification, ServiceType};
use crate::safety_checker::{self, SafetyJudgment, SafetyLevel};
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

/// 获取指定 PID 的详细进程信息
#[tauri::command]
pub fn get_process_detail(pid: u32) -> Result<ProcessDetail, String> {
    let current_user = whoami::username();
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
        Ok(terminator::force_terminate(pid))
    } else {
        let mut result = terminator::terminate(pid);
        if result.success {
            // 等待 500ms 后检查进程是否已退出
            std::thread::sleep(std::time::Duration::from_millis(500));
            if !terminator::is_process_alive(pid) {
                result.port_released = true;
                result.message = format!("进程 {} 已成功终止", pid);
            }
        }
        Ok(result)
    }
}

/// 在 Finder 中打开指定路径（目录或文件所在目录）
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

    std::process::Command::new("open")
        .arg(&open_path)
        .spawn()
        .map_err(|e| format!("打开目录失败: {}", e))?;

    Ok(open_path)
}

/// 来源名 → .app 路径映射
fn source_to_app_path(source: &str) -> Option<String> {
    let path = match source {
        // AI / IDE
        "Cursor"            => "/Applications/Cursor.app",
        "Codex"             => "/Applications/Codex.app",
        "Windsurf"          => "/Applications/Windsurf.app",
        "VSCode"            => "/Applications/Visual Studio Code.app",
        "JetBrains"         => "/Applications/IntelliJ IDEA.app",
        "Xcode"             => "/Applications/Xcode.app",
        "Android Studio"    => "/Applications/Android Studio.app",
        "Sublime Text"      => "/Applications/Sublime Text.app",
        "Zed"               => "/Applications/Zed.app",
        "Vim/Neovim"        => "/Applications/MacVim.app",
        "Emacs"             => "/Applications/Emacs.app",
        "Claude"            => "/Applications/Claude.app",
        "ChatGPT"           => "/Applications/ChatGPT.app",
        // 终端
        "iTerm2"            => "/Applications/iTerm.app",
        "Terminal"          => "/System/Applications/Utilities/Terminal.app",
        "Warp"              => "/Applications/Warp.app",
        "Alacritty"         => "/Applications/Alacritty.app",
        "Kitty"             => "/Applications/kitty.app",
        "Hyper"             => "/Applications/Hyper.app",
        "Tabby"             => "/Applications/Tabby.app",
        "WezTerm"           => "/Applications/WezTerm.app",
        "Ghostty"           => "/Applications/Ghostty.app",
        // 浏览器
        "Chrome"            => "/Applications/Google Chrome.app",
        "Firefox"           => "/Applications/Firefox.app",
        "Safari"            => "/Applications/Safari.app",
        "Arc"               => "/Applications/Arc.app",
        "Edge"              => "/Applications/Microsoft Edge.app",
        "Brave"             => "/Applications/Brave Browser.app",
        "Opera"             => "/Applications/Opera.app",
        "Vivaldi"           => "/Applications/Vivaldi.app",
        // 通讯 / 协作
        "Slack"             => "/Applications/Slack.app",
        "Discord"           => "/Applications/Discord.app",
        "Telegram"          => "/Applications/Telegram.app",
        "WhatsApp"          => "/Applications/WhatsApp.app",
        "Zoom"              => "/Applications/zoom.us.app",
        "Teams"             => "/Applications/Microsoft Teams.app",
        "Notion"            => "/Applications/Notion.app",
        "Obsidian"          => "/Applications/Obsidian.app",
        "飞书"              => "/Applications/Lark.app",
        "钉钉"              => "/Applications/DingTalk.app",
        "微信"              => "/Applications/WeChat.app",
        "QQ"                => "/Applications/QQ.app",
        "Raycast"           => "/Applications/Raycast.app",
        "Alfred"            => "/Applications/Alfred.app",
        // 其他
        "Docker"            => "/Applications/Docker.app",
        "Homebrew"          => return None,
        _                   => return None,
    };
    Some(path.to_string())
}

/// 获取来源对应的应用图标（base64 PNG data URL）
/// 使用 macOS sips 命令从 .app 包中提取真实图标
#[tauri::command]
pub fn get_source_icon(source: String) -> Option<String> {
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
        .args(["-c", "Print :CFBundleIconFile", &plist_path.to_string_lossy()])
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
    let tmp_path = format!("/tmp/port-guardian-icon-{}.png", source.replace(' ', "_").replace('/', "_"));
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

// 需要在 Cargo.toml 中添加 whoami 依赖
// 临时使用环境变量替代
mod whoami {
    pub fn username() -> String {
        std::env::var("USER").unwrap_or_else(|_| "unknown".to_string())
    }
}
