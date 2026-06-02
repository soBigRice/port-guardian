use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub user: String,
    pub command_line: String,
    pub cwd: String,
    pub executable_path: String,
}

/// 根据 PID 获取进程详细信息
pub fn resolve_process(pid: u32) -> Result<ProcessInfo, String> {
    // Windows: 先检查缓存
    #[cfg(windows)]
    {
        if let Some(info) = get_cached(pid) {
            return Ok(info);
        }
    }

    let (ppid, user, name, command_line) = get_ps_info(pid)?;
    let cwd = get_cwd(pid);
    let executable_path = get_executable_path(pid);

    Ok(ProcessInfo {
        pid,
        ppid,
        name,
        user,
        command_line,
        cwd,
        executable_path,
    })
}

// ═══════════════════════════════════════════════════════════════
// Windows 进程缓存 — 批量获取，避免逐个调用 PowerShell
// ═══════════════════════════════════════════════════════════════

#[cfg(windows)]
use std::collections::HashMap;
#[cfg(windows)]
use std::sync::Mutex;

#[cfg(windows)]
static PROCESS_CACHE: Mutex<Option<HashMap<u32, ProcessInfo>>> = Mutex::new(None);

/// 从缓存中获取进程信息
#[cfg(windows)]
fn get_cached(pid: u32) -> Option<ProcessInfo> {
    let cache = PROCESS_CACHE.lock().ok()?;
    let map = cache.as_ref()?;
    map.get(&pid).cloned()
}

/// 一次性批量获取所有进程信息（Windows 专用，启动时调用一次）
/// 使用单次 PowerShell 调用，避免每个 PID 单独启动 PowerShell
#[cfg(windows)]
pub fn prefetch_all_processes() {
    let ps_script = "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
                     Get-CimInstance -ClassName Win32_Process -ErrorAction SilentlyContinue | \
                     ForEach-Object { \
                       Write-Output ('PID:' + $_.ProcessId); \
                       Write-Output ('PPID:' + $_.ParentProcessId); \
                       Write-Output ('NAME:' + $_.Name); \
                       Write-Output ('EPATH:' + [string]$_.ExecutablePath); \
                       Write-Output ('CMD:' + [string]$_.CommandLine); \
                       Write-Output '---'; \
                     }";

    let output = match std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps_script])
        .output()
    {
        Ok(o) => o,
        Err(_) => return,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut map = HashMap::new();

    let mut pid: u32 = 0;
    let mut ppid: u32 = 0;
    let mut name = String::new();
    let mut executable_path = String::new();
    let mut command_line = String::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line == "---" {
            if pid != 0 && !name.is_empty() {
                let cmd = if command_line.is_empty() {
                    name.clone()
                } else {
                    command_line.clone()
                };
                map.insert(
                    pid,
                    ProcessInfo {
                        pid,
                        ppid,
                        name: name.clone(),
                        user: String::new(), // 批量获取时不查询用户名（太慢），按需查询
                        command_line: cmd,
                        cwd: String::new(),
                        executable_path: executable_path.clone(),
                    },
                );
            }
            pid = 0;
            ppid = 0;
            name.clear();
            executable_path.clear();
            command_line.clear();
        } else if let Some(val) = line.strip_prefix("PID:") {
            pid = val.trim().parse().unwrap_or(0);
        } else if let Some(val) = line.strip_prefix("PPID:") {
            ppid = val.trim().parse().unwrap_or(0);
        } else if let Some(val) = line.strip_prefix("NAME:") {
            name = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("EPATH:") {
            executable_path = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("CMD:") {
            command_line = val.trim().to_string();
        }
    }
    // 处理最后一组（如果没有以 --- 结尾）
    if pid != 0 && !name.is_empty() {
        let cmd = if command_line.is_empty() {
            name.clone()
        } else {
            command_line
        };
        map.insert(
            pid,
            ProcessInfo {
                pid,
                ppid,
                name,
                user: String::new(),
                command_line: cmd,
                cwd: String::new(),
                executable_path,
            },
        );
    }

    // 写入缓存
    if let Ok(mut cache) = PROCESS_CACHE.lock() {
        *cache = Some(map);
    }
}

/// Unix: 无操作的预取占位
#[cfg(unix)]
pub fn prefetch_all_processes() {
    // Unix 不需要预取，ps/lsof 单次调用很快
}

// ═══════════════════════════════════════════════════════════════
// Unix (macOS / Linux) 实现 — 使用 ps 和 lsof
// ═══════════════════════════════════════════════════════════════

/// 通过 ps 获取进程基础信息
#[cfg(unix)]
fn get_ps_info(pid: u32) -> Result<(u32, String, String, String), String> {
    use std::process::Command;

    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "pid=,ppid=,user=,comm=,args="])
        .output()
        .map_err(|e| format!("Failed to run ps: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .next()
        .ok_or_else(|| format!("Process {} not found", pid))?;

    let line = line.trim();
    if line.is_empty() {
        return Err(format!("Process {} not found", pid));
    }

    // 提取 PID、PPID、USER、COMM、ARGS
    let (pid_str, rest) = next_field(line);
    let _pid_val: u32 = pid_str.trim().parse().unwrap_or(0);

    let (ppid_str, rest) = next_field(rest);
    let ppid: u32 = ppid_str.trim().parse().unwrap_or(0);

    let (user, rest) = next_field(rest);

    let (name, rest) = next_field(rest);
    let command_line = rest.trim();
    let command_line = if command_line.is_empty() {
        name.to_string()
    } else {
        command_line.to_string()
    };

    Ok((
        ppid,
        user.trim().to_string(),
        name.trim().to_string(),
        command_line,
    ))
}

/// 从 ps 输出中提取一个字段（跳过前导空格，取到下一个空白）
#[cfg(unix)]
fn next_field(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    if s.is_empty() {
        return ("", "");
    }
    match s.find(char::is_whitespace) {
        Some(pos) => (&s[..pos], &s[pos..]),
        None => (s, ""),
    }
}

/// 获取进程工作目录
#[cfg(unix)]
fn get_cwd(pid: u32) -> String {
    use std::process::Command;

    let output = Command::new("lsof")
        .args(["-p", &pid.to_string(), "-a", "-d", "cwd"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 9 {
                    return parts[8..].join(" ");
                }
            }
            String::new()
        }
        Err(_) => String::new(),
    }
}

/// 获取进程可执行文件路径
#[cfg(unix)]
fn get_executable_path(pid: u32) -> String {
    use std::process::Command;

    let output = Command::new("lsof")
        .args(["-p", &pid.to_string(), "-a", "-d", "txt"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 9 {
                    return parts[8..].join(" ");
                }
            }
            String::new()
        }
        Err(_) => String::new(),
    }
}

// ═══════════════════════════════════════════════════════════════
// Windows 实现 — 使用 PowerShell Get-CimInstance
// ═══════════════════════════════════════════════════════════════

/// 通过 PowerShell 获取进程基础信息（Windows）
/// 优先使用缓存，缓存未命中时才调用 PowerShell
#[cfg(windows)]
fn get_ps_info(pid: u32) -> Result<(u32, String, String, String), String> {
    // 先查缓存
    if let Some(info) = get_cached(pid) {
        return Ok((info.ppid, info.user, info.name, info.command_line));
    }

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
        .map_err(|e| format!("Failed to run PowerShell: {}", e))?;

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

    // 获取用户名（按需，因为比较慢）
    let user = get_windows_process_user(pid);

    Ok((ppid, user, name, command_line))
}

/// 获取 Windows 进程的用户名
#[cfg(windows)]
fn get_windows_process_user(pid: u32) -> String {
    let ps_script = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
         $p = Get-CimInstance -ClassName Win32_Process -Filter 'ProcessId={}' -ErrorAction SilentlyContinue; \
         if ($p) {{ \
           try {{ \
             $owner = Invoke-CimMethod -InputObject $p -MethodName GetOwner -ErrorAction SilentlyContinue; \
             if ($owner.Domain) {{ Write-Output ($owner.Domain + '\\' + $owner.User) }} \
             else {{ Write-Output $owner.User }} \
           }} catch {{ Write-Output '' }} \
         }}",
        pid
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.lines().next().unwrap_or("").trim().to_string()
        }
        Err(_) => String::new(),
    }
}

/// 获取进程工作目录（Windows：不可靠，返回空字符串）
#[cfg(windows)]
fn get_cwd(_pid: u32) -> String {
    // Windows 的 Win32_Process 不可靠地暴露 CWD
    // 返回空字符串，不影响核心功能
    String::new()
}

/// 获取进程可执行文件路径（Windows）
/// 优先使用缓存，缓存未命中时才调用 PowerShell
#[cfg(windows)]
fn get_executable_path(pid: u32) -> String {
    // 先查缓存
    if let Some(info) = get_cached(pid) {
        return if info.executable_path.is_empty() {
            info.name
        } else {
            info.executable_path
        };
    }

    let ps_script = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
         $p = Get-CimInstance -ClassName Win32_Process -Filter 'ProcessId={}' -ErrorAction SilentlyContinue; \
         if ($p) {{ \
           $ep = [string]$p.ExecutablePath; \
           if ($ep) {{ Write-Output $ep }} \
           else {{ Write-Output $p.Name }} \
         }}",
        pid
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.lines().next().unwrap_or("").trim().to_string()
        }
        Err(_) => String::new(),
    }
}
