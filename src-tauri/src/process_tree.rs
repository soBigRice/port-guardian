use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ProcessNode {
    pub pid: u32,
    pub name: String,
    pub command_line: String,
}

/// 构建父进程链（从当前进程向上追溯到 init/launchd/Windows System）
pub fn build_parent_chain(pid: u32) -> Vec<ProcessNode> {
    let mut chain = Vec::new();
    let mut current_pid = pid;
    let mut seen = std::collections::HashSet::new();

    for _ in 0..20 {
        // Unix: PID 0 (kernel) 和 PID 1 (init/launchd) 是根进程
        // Windows: PID 0 (System Idle) 和 PID 4 (System) 是根进程
        #[cfg(unix)]
        {
            if current_pid == 0 || current_pid == 1 {
                break;
            }
        }
        #[cfg(windows)]
        {
            if current_pid == 0 || current_pid == 4 {
                break;
            }
        }

        if !seen.insert(current_pid) {
            break;
        }

        if let Ok((ppid, _user, name, command_line)) = get_process_brief(current_pid) {
            chain.push(ProcessNode {
                pid: current_pid,
                name,
                command_line,
            });
            current_pid = ppid;
        } else {
            break;
        }
    }

    chain
}

/// 识别进程来源（IDE / 终端 / 浏览器 / 系统等）
pub fn identify_source(chain: &[ProcessNode]) -> String {
    for node in chain {
        let name = node.name.as_str();
        let name_lower = name.to_lowercase();
        let cmd = node.command_line.as_str();
        let cmd_lower = cmd.to_lowercase();

        // ── AI / IDE 工具 ──────────────────────────────────
        if name_lower.contains("cursor")
            || cmd_lower.contains("cursor.app")
            || cmd_lower.contains("cursor.exe")
        {
            return "Cursor".into();
        }
        if name_lower == "codex" || cmd_lower.contains("codex") {
            return "Codex".into();
        }
        if name_lower.contains("windsurf") || cmd_lower.contains("windsurf") {
            return "Windsurf".into();
        }
        // VSCode: macOS 进程名可能是 "Code"，也可能是 "Electron" + vscode 路径
        if (name_lower == "code" || name_lower == "code-insiders")
            || (name_lower == "electron" && cmd_lower.contains("visual studio code"))
            || cmd_lower.contains("/applications/visual studio code")
            || cmd_lower.contains("microsoft vs code")
            || cmd_lower.contains("\\microsoft vs code\\")
        {
            return "VSCode".into();
        }
        // JetBrains
        if cmd_lower.contains("jetbrains")
            || cmd_lower.contains("/idea")
            || cmd_lower.contains("webstorm")
            || cmd_lower.contains("pycharm")
            || cmd_lower.contains("goland")
            || cmd_lower.contains("clion")
            || name_lower.contains("idea")
            || name_lower.contains("webstorm")
        {
            return "JetBrains".into();
        }
        // Xcode
        if name_lower == "xcode" || name_lower.contains("xcode") {
            return "Xcode".into();
        }
        // Android Studio
        if name_lower.contains("android-studio") || cmd_lower.contains("android studio") {
            return "Android Studio".into();
        }
        // Sublime Text
        if name_lower.contains("sublime") || name_lower.contains("subl") {
            return "Sublime Text".into();
        }
        // Zed
        if name_lower == "zed" || cmd_lower.contains("/zed.app") || cmd_lower.contains("zed.exe") {
            return "Zed".into();
        }
        // Vim / Neovim
        if name_lower == "nvim" || name_lower == "vim" || name_lower == "mvim" {
            return "Vim/Neovim".into();
        }
        // Emacs
        if name_lower == "emacs" || name_lower.contains("emacs") {
            return "Emacs".into();
        }
        // Claude Desktop / Claude Code
        if name_lower.contains("claude") || cmd_lower.contains("claude") {
            return "Claude".into();
        }
        // ChatGPT desktop
        if name_lower.contains("chatgpt") || cmd_lower.contains("chatgpt") {
            return "ChatGPT".into();
        }

        // ── 终端模拟器 ────────────────────────────────────
        if name_lower == "iterm2" || name_lower == "iterm" || cmd_lower.contains("iterm") {
            return "iTerm2".into();
        }
        if name_lower == "terminal" && cmd_lower.contains("terminal.app") {
            return "Terminal".into();
        }
        if name_lower == "warp" || cmd_lower.contains("warp.app") {
            return "Warp".into();
        }
        if name_lower == "alacritty" {
            return "Alacritty".into();
        }
        if name_lower == "kitty" || cmd_lower.contains("kitty.app") {
            return "Kitty".into();
        }
        if name_lower == "hyper" || cmd_lower.contains("hyper.app") {
            return "Hyper".into();
        }
        if name_lower == "tabby" || cmd_lower.contains("tabby") {
            return "Tabby".into();
        }
        if name_lower == "wezterm" || name_lower == "wezterm-gui" {
            return "WezTerm".into();
        }
        if name_lower == "ghostty" || cmd_lower.contains("ghostty") {
            return "Ghostty".into();
        }
        // Windows Terminal
        if name_lower == "windowsterminal" || cmd_lower.contains("windowsterminal") {
            return "Windows Terminal".into();
        }
        // PowerShell
        if name_lower == "powershell" || name_lower == "pwsh" {
            // PowerShell 不是最终来源，继续向上找
            continue;
        }
        // cmd.exe
        if name_lower == "cmd" || name_lower == "cmd.exe" {
            continue;
        }
        // tmux
        if name_lower == "tmux" || name_lower == "tmux: server" {
            return "tmux".into();
        }
        // screen
        if name_lower == "screen" {
            return "screen".into();
        }

        // ── Shell（zsh / bash / fish）─────────────────────
        if name_lower == "zsh" || name_lower == "bash" || name_lower == "fish" || name_lower == "sh"
        {
            // Shell 不是最终来源，继续向上找
            continue;
        }

        // ── SSH ───────────────────────────────────────────
        if name_lower == "sshd" {
            return "SSH".into();
        }

        // ── 浏览器 ───────────────────────────────────────
        if name_lower.contains("google chrome") || cmd_lower.contains("google chrome") {
            return "Chrome".into();
        }
        if name_lower.contains("firefox") || cmd_lower.contains("firefox") {
            return "Firefox".into();
        }
        if name_lower.contains("safari") && !cmd_lower.contains("safariview") {
            return "Safari".into();
        }
        if name_lower.contains("arc") || cmd_lower.contains("arc.app") {
            return "Arc".into();
        }
        if name_lower.contains("microsoft edge") || cmd_lower.contains("microsoft edge") {
            return "Edge".into();
        }
        if name_lower.contains("brave") || cmd_lower.contains("brave") {
            return "Brave".into();
        }
        if name_lower.contains("opera") || cmd_lower.contains("opera") {
            return "Opera".into();
        }
        if name_lower.contains("vivaldi") || cmd_lower.contains("vivaldi") {
            return "Vivaldi".into();
        }

        // ── 通讯 / 协作工具 ──────────────────────────────
        if name_lower.contains("slack") || cmd_lower.contains("slack") {
            return "Slack".into();
        }
        if name_lower.contains("discord") || cmd_lower.contains("discord") {
            return "Discord".into();
        }
        if name_lower.contains("telegram") || cmd_lower.contains("telegram") {
            return "Telegram".into();
        }
        if name_lower.contains("whatsapp") || cmd_lower.contains("whatsapp") {
            return "WhatsApp".into();
        }
        if name_lower.contains("zoom") || cmd_lower.contains("zoom.us") {
            return "Zoom".into();
        }
        if name_lower.contains("teams") || cmd_lower.contains("microsoft teams") {
            return "Teams".into();
        }
        if name_lower.contains("notion") || cmd_lower.contains("notion") {
            return "Notion".into();
        }
        if name_lower.contains("obsidian") || cmd_lower.contains("obsidian") {
            return "Obsidian".into();
        }
        if name_lower.contains("feishu")
            || name_lower.contains("lark")
            || cmd_lower.contains("feishu")
        {
            return "飞书".into();
        }
        if name_lower.contains("dingtalk") || cmd_lower.contains("dingtalk") {
            return "钉钉".into();
        }
        if name_lower.contains("wechat") || cmd_lower.contains("wechat") {
            return "微信".into();
        }
        if name_lower.contains("qq") || cmd_lower.contains("qq.app") {
            return "QQ".into();
        }
        if name_lower.contains("raycast") || cmd_lower.contains("raycast") {
            return "Raycast".into();
        }
        if name_lower.contains("alfred") || cmd_lower.contains("alfred") {
            return "Alfred".into();
        }

        // ── Docker ───────────────────────────────────────
        if name_lower.contains("docker") || cmd_lower.contains("docker") {
            return "Docker".into();
        }

        // ── macOS 系统服务 ───────────────────────────────
        if name_lower == "launchd" || name_lower == "systemd" || name_lower == "init" {
            return "System".into();
        }
        if name_lower == "kernel_task" || name_lower == "windowserver" {
            return "System".into();
        }
        if name_lower == "loginwindow" || name_lower == "controlcenter" {
            return "System".into();
        }
        if name_lower == "rapportd" || name_lower == "sharingd" || name_lower == "nsurlsessiond" {
            return "System".into();
        }
        if name_lower == "coreaudiod" || name_lower == "coreaudiod" {
            return "System Audio".into();
        }
        if name_lower == "bluetoothd" || name_lower == "wifiagent" {
            return "System Network".into();
        }
        if name_lower == "mds" || name_lower == "mds_stores" || name_lower == "spotlight" {
            return "Spotlight".into();
        }
        if name_lower == "cfprefsd" || name_lower == "distnoted" || name_lower == "usernoted" {
            return "System Service".into();
        }
        if name_lower == "apsd" || name_lower == "sharingd" {
            return "System Service".into();
        }
        if name_lower.contains("com.apple") || cmd_lower.contains("com.apple") {
            return "Apple Service".into();
        }

        // ── Windows 系统服务 ─────────────────────────────
        if name_lower == "system" || name_lower == "registry" {
            return "System".into();
        }
        if name_lower == "smss" || name_lower == "smss.exe" {
            return "Session Manager".into();
        }
        if name_lower == "csrss" || name_lower == "csrss.exe" {
            return "Client Server Runtime".into();
        }
        if name_lower == "wininit" || name_lower == "wininit.exe" {
            return "Windows Init".into();
        }
        if name_lower == "lsass" || name_lower == "lsass.exe" {
            return "Local Security Authority".into();
        }
        if name_lower == "services" || name_lower == "services.exe" {
            return "Service Control Manager".into();
        }
        if name_lower == "svchost" || name_lower == "svchost.exe" {
            return "Service Host".into();
        }
        if name_lower == "dwm" || name_lower == "dwm.exe" {
            return "Desktop Window Manager".into();
        }
        if name_lower == "winlogon" || name_lower == "winlogon.exe" {
            return "Windows Logon".into();
        }
        if name_lower == "searchhost" || name_lower == "searchapp" {
            return "Windows Search".into();
        }
        if name_lower == "startmenuexperiencehost" {
            return "Start Menu".into();
        }
        if name_lower == "textinputhost" {
            return "Text Input".into();
        }
        if name_lower == "runtimebroker" || name_lower == "runtimebroker.exe" {
            return "Runtime Broker".into();
        }
        if name_lower == "wmiprvse" || name_lower == "wmiprvse.exe" {
            return "WMI Provider".into();
        }
        if name_lower == "dllhost" || name_lower == "dllhost.exe" {
            return "DLL Host".into();
        }
        if name_lower == "conhost" || name_lower == "conhost.exe" {
            return "Console Host".into();
        }
        if name_lower == "spoolsv" || name_lower == "spoolsv.exe" {
            return "Print Spooler".into();
        }
        if name_lower == "msdtc" || name_lower == "msdtc.exe" {
            return "Distributed Transaction Coordinator".into();
        }
        if name_lower == "securityhealthservice" {
            return "Windows Security".into();
        }
        if name_lower == "sihost" || name_lower == "sihost.exe" {
            return "Shell Infrastructure".into();
        }

        // ── 开发工具 / 包管理器 ──────────────────────────
        if name_lower == "homebrew"
            || cmd_lower.contains("homebrew")
            || cmd_lower.contains("/opt/homebrew")
        {
            return "Homebrew".into();
        }
        if name_lower == "node"
            || name_lower == "npm"
            || name_lower == "pnpm"
            || name_lower == "yarn"
            || name_lower == "bun"
        {
            // 继续向上找来源（可能是从 IDE 或终端启动的）
            continue;
        }
        if name_lower == "python" || name_lower == "python3" || name_lower == "pip" {
            continue;
        }
        if name_lower == "ruby" || name_lower == "gem" {
            continue;
        }
        if name_lower == "cargo" || name_lower == "rustc" || name_lower == "rustup" {
            continue;
        }
        if name_lower == "go" {
            continue;
        }
        if name_lower == "java" {
            continue;
        }

        // ── Electron 应用（通用）──────────────────────────
        if name_lower == "electron" {
            // Electron 应用，继续向上找
            continue;
        }
    }

    "Unknown".into()
}

// ═══════════════════════════════════════════════════════════════
// Unix (macOS / Linux) 实现 — 使用 ps
// ═══════════════════════════════════════════════════════════════

/// 获取进程简要信息 (ppid, user, name, command_line)
#[cfg(unix)]
fn get_process_brief(pid: u32) -> Result<(u32, String, String, String), String> {
    use std::process::Command;

    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "pid=,ppid=,user=,comm=,args="])
        .output()
        .map_err(|e| format!("ps failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .next()
        .ok_or_else(|| format!("Process {} not found", pid))?;

    parse_ps_line(line)
}

/// 解析 ps 输出行
/// 格式: PID PPID USER COMM ARGS
/// PID/PPID 是数字，USER 是用户名，COMM 是进程名，ARGS 是完整命令行
#[cfg(unix)]
fn parse_ps_line(line: &str) -> Result<(u32, String, String, String), String> {
    let line = line.trim();
    if line.is_empty() {
        return Err("empty ps output".into());
    }

    // ps -o pid=,ppid=,user=,comm=,args= 的输出按空格分隔
    let mut remaining = line;

    // 提取 PID
    let (pid_str, rest) = extract_field(remaining);
    let _pid: u32 = pid_str.trim().parse().map_err(|_| "bad pid")?;
    remaining = rest;

    // 提取 PPID
    let (ppid_str, rest) = extract_field(remaining);
    let ppid: u32 = ppid_str.trim().parse().map_err(|_| "bad ppid")?;
    remaining = rest;

    // 提取 USER
    let (user, rest) = extract_field(remaining);
    remaining = rest;

    // 提取 COMM（进程名，不含空格）
    let (name, rest) = extract_field(remaining);

    // 剩余部分是 ARGS
    let command_line = rest.trim().to_string();
    let command_line = if command_line.is_empty() {
        name.to_string()
    } else {
        command_line
    };

    Ok((
        ppid,
        user.trim().to_string(),
        name.trim().to_string(),
        command_line,
    ))
}

/// 从 ps 输出中提取一个字段（跳过前导空格，取到下一个空格）
#[cfg(unix)]
fn extract_field(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    if s.is_empty() {
        return ("", "");
    }
    match s.find(char::is_whitespace) {
        Some(pos) => (&s[..pos], &s[pos..]),
        None => (s, ""),
    }
}

// ═══════════════════════════════════════════════════════════════
// Windows 实现 — 使用 PowerShell Get-CimInstance
// ═══════════════════════════════════════════════════════════════

/// 获取进程简要信息 (ppid, user, name, command_line) — Windows
/// 优先使用 process_resolver 的缓存，缓存未命中时才调用 PowerShell
#[cfg(windows)]
fn get_process_brief(pid: u32) -> Result<(u32, String, String, String), String> {
    use crate::process_resolver;

    // 先通过缓存获取
    if let Ok(info) = process_resolver::resolve_process(pid) {
        return Ok((info.ppid, info.user, info.name, info.command_line));
    }

    // 缓存未命中，单独调用 PowerShell
    let ps_script = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
         $p = Get-CimInstance -ClassName Win32_Process -Filter 'ProcessId={}' -ErrorAction SilentlyContinue; \
         if ($p) {{ \
           Write-Output ('PPID:' + $p.ParentProcessId); \
           Write-Output ('NAME:' + $p.Name); \
           Write-Output ('CMD:' + [string]$p.CommandLine); \
         }}",
        pid
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .output()
        .map_err(|e| format!("PowerShell failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut ppid: u32 = 0;
    let mut name = String::new();
    let mut command_line = String::new();

    for line in stdout.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("PPID:") {
            ppid = val.trim().parse().unwrap_or(0);
        } else if let Some(val) = line.strip_prefix("NAME:") {
            name = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("CMD:") {
            command_line = val.trim().to_string();
        }
    }

    if name.is_empty() {
        return Err(format!("Process {} not found", pid));
    }

    if command_line.is_empty() {
        command_line = name.clone();
    }

    Ok((ppid, String::new(), name, command_line))
}
