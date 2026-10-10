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

/// 将 ps / PowerShell 可能返回的可执行路径收敛为进程名。
/// macOS 的 comm 字段可能被截成 /Users/superrice 这类无效片段，所以优先用真实可执行路径，其次用命令行首个可执行项。
pub(crate) fn normalize_process_name(
    raw_name: &str,
    command_line: &str,
    executable_path: &str,
) -> String {
    let executable_path = executable_path.trim();
    if !executable_path.is_empty() {
        return file_name_or_original(executable_path);
    }

    let trimmed = raw_name.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if trimmed.starts_with('/')
        && let Some(first_arg) = command_line.split_whitespace().next()
    {
        let first_arg_name = file_name_or_original(first_arg);
        if !first_arg_name.is_empty() {
            return first_arg_name;
        }
    }

    file_name_or_original(trimmed)
}

fn file_name_or_original(value: &str) -> String {
    std::path::Path::new(value)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(value)
        .to_string()
}

/// 解码命令行中的百分号编码路径，例如 file:///.../%E5%B0%8F...，让中文目录按可读文本展示和搜索。
#[cfg_attr(all(windows, not(test)), allow(dead_code))]
pub(crate) fn decode_percent_encoded_utf8(input: &str) -> String {
    let bytes = input.as_bytes();
    if !bytes.contains(&b'%') {
        return input.to_string();
    }

    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
        {
            decoded.push((high << 4) | low);
            index += 3;
            continue;
        }

        decoded.push(bytes[index]);
        index += 1;
    }

    String::from_utf8(decoded).unwrap_or_else(|_| input.to_string())
}

#[cfg_attr(all(windows, not(test)), allow(dead_code))]
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// 根据 PID 获取进程详细信息
pub fn resolve_process(pid: u32) -> Result<ProcessInfo, String> {
    // Windows: 先检查缓存
    #[cfg(windows)]
    {
        if let Some(info) = get_cached(pid) {
            return Ok(hydrate_cached_process_info(pid, info));
        }
    }

    let (ppid, user, name, command_line) = get_ps_info(pid)?;
    let cwd = get_cwd(pid);
    let executable_path = get_executable_path(pid);
    let name = normalize_process_name(&name, &command_line, &executable_path);

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

// 执行前不能使用 Windows 扫描 TTL 缓存，防止 PID 被复用后仍拿到旧身份。
#[cfg(unix)]
pub fn resolve_process_for_termination(pid: u32) -> Result<ProcessInfo, String> {
    resolve_process(pid)
}

#[cfg(windows)]
pub fn resolve_process_for_termination(pid: u32) -> Result<ProcessInfo, String> {
    let (ppid, user, name, command_line) = get_ps_info_uncached(pid)?;
    let executable_path = get_executable_path_uncached(pid);
    let name = normalize_process_name(&name, &command_line, &executable_path);
    Ok(ProcessInfo {
        pid,
        ppid,
        name,
        user: if user.is_empty() {
            "unknown".into()
        } else {
            user
        },
        command_line,
        cwd: get_cwd(pid),
        executable_path,
    })
}

// ═══════════════════════════════════════════════════════════════
// Windows 进程缓存 — 批量获取，避免逐个调用 PowerShell
// ═══════════════════════════════════════════════════════════════

#[cfg(windows)]
use crate::windows_command::powershell_output;
#[cfg(windows)]
use std::collections::HashMap;
#[cfg(windows)]
use std::sync::Mutex;
#[cfg(windows)]
use std::time::{Duration, Instant};

#[cfg(windows)]
static PROCESS_CACHE: Mutex<Option<HashMap<u32, ProcessInfo>>> = Mutex::new(None);
#[cfg(windows)]
static PROCESS_CACHE_UPDATED: Mutex<Option<Instant>> = Mutex::new(None);
#[cfg(windows)]
const PROCESS_CACHE_TTL: Duration = Duration::from_secs(30);

/// 从缓存中获取进程信息
#[cfg(windows)]
fn get_cached(pid: u32) -> Option<ProcessInfo> {
    let cache = PROCESS_CACHE.lock().ok()?;
    let map = cache.as_ref()?;
    map.get(&pid).cloned()
}

/// Windows 父进程链只需要 ppid/name/command_line；这里故意不补齐 owner/cwd，
/// 避免扫描每一层父进程时触发较重的权限和目录读取。
#[cfg(windows)]
pub(crate) fn get_cached_process_brief(pid: u32) -> Option<(u32, String, String, String)> {
    let info = get_cached(pid)?;
    let name = normalize_process_name(&info.name, &info.command_line, &info.executable_path);
    Some((info.ppid, info.user, name, info.command_line))
}

#[cfg(windows)]
fn update_cached(pid: u32, info: ProcessInfo) {
    if let Ok(mut cache) = PROCESS_CACHE.lock()
        && let Some(map) = cache.as_mut()
    {
        map.insert(pid, info);
    }
}

/// Windows 批量缓存为了速度不主动读取 owner/cwd；命中缓存时按需补齐这些安全相关字段。
/// owner 读取失败时写入 unknown，让安全判断走“非当前用户/不可直接终止”的保守分支。
#[cfg(windows)]
fn hydrate_cached_process_info(pid: u32, mut info: ProcessInfo) -> ProcessInfo {
    let original = info.clone();

    if info.user.is_empty() {
        let user = get_windows_process_user(pid);
        info.user = if user.is_empty() {
            "unknown".to_string()
        } else {
            user
        };
    }

    if info.cwd.is_empty() {
        info.cwd = get_cwd(pid);
    }

    let normalized_name =
        normalize_process_name(&info.name, &info.command_line, &info.executable_path);
    if info.name != normalized_name {
        info.name = normalized_name;
    }

    if info.user != original.user || info.cwd != original.cwd || info.name != original.name {
        update_cached(pid, info.clone());
    }

    info
}

/// 一次性批量获取所有进程信息（Windows 专用，启动时调用一次）
/// 使用单次 PowerShell 调用，避免每个 PID 单独启动 PowerShell
#[cfg(windows)]
pub fn prefetch_all_processes() -> Result<(), String> {
    // 静默轮询会频繁调用扫描；30 秒内复用全量进程快照，避免持续唤起 WMI/PowerShell。
    // 如果出现新 PID，resolve_process 仍会走单 PID 查询兜底，不会等到 TTL 过期才显示。
    let cache_is_fresh = PROCESS_CACHE_UPDATED
        .lock()
        .ok()
        .and_then(|updated| *updated)
        .is_some_and(|updated| updated.elapsed() < PROCESS_CACHE_TTL);
    let cache_has_data = PROCESS_CACHE
        .lock()
        .ok()
        .and_then(|cache| cache.as_ref().map(|map| !map.is_empty()))
        .unwrap_or(false);
    if cache_is_fresh && cache_has_data {
        return Ok(());
    }

    let ps_script = "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
                     $OutputEncoding = [System.Text.Encoding]::UTF8; \
                     Get-CimInstance -ClassName Win32_Process -OperationTimeoutSec 20 -ErrorAction Stop | \
                     ForEach-Object { \
                       Write-Output ('PID:' + $_.ProcessId); \
                       Write-Output ('PPID:' + $_.ParentProcessId); \
                       Write-Output ('NAME:' + $_.Name); \
                       Write-Output ('EPATH:' + [string]$_.ExecutablePath); \
                       Write-Output ('CMD:' + [string]$_.CommandLine); \
                       Write-Output '---'; \
                     }";

    // 失败不能静默变成缓存未命中，否则会对每个端口及父进程反复启动查询。
    let output = powershell_output(ps_script, "批量查询 Windows 进程")?;

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

    if map.is_empty() {
        return Err("批量查询 Windows 进程未返回可识别记录，请检查本机 WMI 状态".into());
    }

    // 写入缓存。刷新全量进程快照时，如果同一 PID 的命令和可执行路径没变，
    // 只保留上一轮已补齐的 owner，避免每 30 秒重复调用 PowerShell。
    // cwd 通过原生 API 读取成本较低，刷新后重新读取，避免长期保留旧工作目录。
    if let Ok(mut cache) = PROCESS_CACHE.lock() {
        if let Some(old_map) = cache.as_ref() {
            for (pid, info) in &mut map {
                if let Some(old) = old_map.get(pid) {
                    let same_process = old.name == info.name
                        && old.command_line == info.command_line
                        && old.executable_path == info.executable_path;
                    if same_process && info.user.is_empty() && !old.user.is_empty() {
                        info.user = old.user.clone();
                    }
                }
            }
        }
        *cache = Some(map);
    }
    if let Ok(mut updated) = PROCESS_CACHE_UPDATED.lock() {
        *updated = Some(Instant::now());
    }
    Ok(())
}

/// Unix: 无操作的预取占位
#[cfg(unix)]
pub fn prefetch_all_processes() -> Result<(), String> {
    // Unix 不需要预取；ps 和系统 /proc API 单次调用成本较低。
    Ok(())
}

// ═══════════════════════════════════════════════════════════════
// Unix (macOS / Linux) 实现 — 使用 ps、proc_pidinfo/proc_pidpath 和 /proc
// ═══════════════════════════════════════════════════════════════

/// 通过 ps 获取进程基础信息
#[cfg(unix)]
fn get_ps_info(pid: u32) -> Result<(u32, String, String, String), String> {
    use std::process::Command;

    let output = Command::new("ps")
        // Finder 启动的 macOS 打包版可能没有继承终端的 UTF-8 locale；
        // 这里显式指定，避免中文路径在 ps 的 args 输出中被转成 M-xx 转义形式。
        .env("LC_ALL", "en_US.UTF-8")
        .env("LANG", "en_US.UTF-8")
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

    let (raw_name, rest) = next_field(rest);
    let command_line = rest.trim();
    let command_line = if command_line.is_empty() {
        raw_name.trim().to_string()
    } else {
        decode_percent_encoded_utf8(command_line)
    };
    let name = normalize_process_name(raw_name, &command_line, "");

    Ok((ppid, user.trim().to_string(), name, command_line))
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
/// macOS: 使用 proc_pidinfo 系统调用，不依赖 lsof 子进程，打包后也能正常工作
/// Linux: 读取 /proc/<pid>/cwd 符号链接
#[cfg(unix)]
fn get_cwd(pid: u32) -> String {
    #[cfg(target_os = "macos")]
    {
        return get_cwd_macos(pid);
    }

    #[cfg(target_os = "linux")]
    {
        return get_cwd_linux(pid);
    }
}

/// macOS: 通过 proc_pidinfo(PROC_PIDVNODEPATHINFO) 获取进程当前目录
#[cfg(target_os = "macos")]
fn get_cwd_macos(pid: u32) -> String {
    // 使用 libc 的系统 ABI 类型，避免手写布局遗漏字段导致 proc_pidinfo 拒绝缓冲区。
    let mut info = core::mem::MaybeUninit::<libc::proc_vnodepathinfo>::zeroed();
    let size = core::mem::size_of::<libc::proc_vnodepathinfo>() as i32;

    let ret = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            libc::PROC_PIDVNODEPATHINFO,
            0,
            info.as_mut_ptr() as *mut libc::c_void,
            size,
        )
    };

    if ret != size {
        return String::new();
    }

    let info = unsafe { info.assume_init() };
    let path_bytes: Vec<u8> = info
        .pvi_cdir
        .vip_path
        .iter()
        .flatten()
        .map(|&b| b as u8)
        .collect();
    let len = path_bytes
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(path_bytes.len());
    String::from_utf8_lossy(&path_bytes[..len]).to_string()
}

/// Linux: 读取 /proc/<pid>/cwd 符号链接
#[cfg(target_os = "linux")]
fn get_cwd_linux(pid: u32) -> String {
    std::fs::read_link(format!("/proc/{}/cwd", pid))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// 获取进程可执行文件路径
/// macOS: 使用 proc_pidpath 系统调用，打包后也能正常工作
/// Linux: 读取 /proc/<pid>/exe 符号链接
#[cfg(unix)]
fn get_executable_path(pid: u32) -> String {
    #[cfg(target_os = "macos")]
    {
        // proc_pidpath: 从进程 PID 获取可执行文件路径
        unsafe extern "C" {
            fn proc_pidpath(pid: libc::c_int, buf: *mut libc::c_void, bufsize: u32) -> libc::c_int;
        }
        let mut buf = [0u8; 1024];
        let ret = unsafe { proc_pidpath(pid as i32, buf.as_mut_ptr() as *mut libc::c_void, 1024) };
        if ret <= 0 {
            return String::new();
        }
        let len = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
        return String::from_utf8_lossy(&buf[..len]).to_string();
    }

    #[cfg(target_os = "linux")]
    {
        return std::fs::read_link(format!("/proc/{}/exe", pid))
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_percent_encoded_utf8, normalize_process_name};

    #[cfg(target_os = "macos")]
    #[test]
    fn reads_actual_working_directory_with_system_abi() {
        let expected = std::env::current_dir().unwrap();
        assert_eq!(
            std::path::PathBuf::from(super::get_cwd_macos(std::process::id())),
            expected
        );
    }

    #[cfg(windows)]
    #[test]
    fn termination_identity_bypasses_scan_cache() {
        let pid = std::process::id();
        let stale = super::ProcessInfo {
            pid,
            ppid: 0,
            name: "stale-process".into(),
            user: "stale-user".into(),
            command_line: "stale-command".into(),
            cwd: String::new(),
            executable_path: "stale-executable".into(),
        };
        super::PROCESS_CACHE
            .lock()
            .unwrap()
            .get_or_insert_with(std::collections::HashMap::new)
            .insert(pid, stale);
        let result = super::resolve_process_for_termination(pid);
        super::PROCESS_CACHE
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .remove(&pid);
        let actual = result.expect("read the running test process");
        assert_ne!(actual.command_line, "stale-command");
        assert_ne!(actual.executable_path, "stale-executable");
        assert_ne!(actual.user, "stale-user");
        assert!(!actual.executable_path.is_empty());
    }

    #[test]
    fn decodes_utf8_percent_encoded_command_paths() {
        let raw = "node --import file:///Users/superrice/CodePublic/%E5%B0%8F%E9%BB%91%E7%B1%B3/node_modules/tsx/dist/loader.mjs";
        let decoded = decode_percent_encoded_utf8(raw);

        assert!(decoded.contains("/小黑米/node_modules/tsx/dist/loader.mjs"));
    }

    #[test]
    fn keeps_invalid_percent_sequences_unchanged() {
        let raw = "node --flag 100% --name %ZZ";

        assert_eq!(decode_percent_encoded_utf8(raw), raw);
    }

    #[test]
    fn normalizes_full_executable_path_to_file_name() {
        assert_eq!(
            normalize_process_name(
                "/Users/superrice/.nvm/versions/node/v23.10.0/bin/node",
                "",
                ""
            ),
            "node"
        );
    }

    #[test]
    fn normalizes_truncated_comm_from_command_line() {
        assert_eq!(
            normalize_process_name(
                "/Users/superrice",
                "/Users/superrice/.nvm/versions/node/v23.10.0/bin/node --require preflight.cjs",
                ""
            ),
            "node"
        );
    }

    #[test]
    fn prefers_executable_path_when_available() {
        assert_eq!(
            normalize_process_name(
                "/Users/superrice",
                "/Users/superrice/.nvm/versions/node/v23.10.0/bin/node --require preflight.cjs",
                "/Users/superrice/.nvm/versions/node/v23.10.0/bin/node"
            ),
            "node"
        );
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
    get_ps_info_uncached(pid)
}

#[cfg(windows)]
fn get_ps_info_uncached(pid: u32) -> Result<(u32, String, String, String), String> {
    let ps_script = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
         $OutputEncoding = [System.Text.Encoding]::UTF8; \
         $p = Get-CimInstance -ClassName Win32_Process -Filter 'ProcessId={}' -OperationTimeoutSec 20 -ErrorAction Stop; \
         if ($p) {{ \
           Write-Output ('PPID:' + $p.ParentProcessId); \
           Write-Output ('NAME:' + $p.Name); \
           Write-Output ('CMD:' + [string]$p.CommandLine); \
         }}",
        pid
    );

    let output = powershell_output(&ps_script, &format!("查询 Windows 进程 PID {pid}"))?;

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
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::TOKEN_QUERY;
    use windows::Win32::System::Threading::{
        OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    unsafe {
        let process_handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(handle) => handle,
            Err(_) => return String::new(),
        };

        let mut token_handle = HANDLE::default();
        if OpenProcessToken(process_handle, TOKEN_QUERY, &mut token_handle).is_err() {
            let _ = CloseHandle(process_handle);
            return String::new();
        }

        // 原生 token 查询比 PowerShell GetOwner 快得多，也不会因为 WMI 卡住扫描链路。
        let result = get_token_user_name(token_handle);
        let _ = CloseHandle(token_handle);
        let _ = CloseHandle(process_handle);
        result
    }
}

#[cfg(windows)]
unsafe fn get_token_user_name(token_handle: windows::Win32::Foundation::HANDLE) -> String {
    use windows::Win32::Security::{
        GetTokenInformation, LookupAccountSidW, SID_NAME_USE, TOKEN_USER, TokenUser,
    };
    use windows::core::{PCWSTR, PWSTR};

    let mut needed = 0u32;
    let _ = unsafe { GetTokenInformation(token_handle, TokenUser, None, 0, &mut needed) };
    if needed == 0 {
        return String::new();
    }

    let mut token_buf = vec![0u8; needed as usize];
    if unsafe {
        GetTokenInformation(
            token_handle,
            TokenUser,
            Some(token_buf.as_mut_ptr() as *mut core::ffi::c_void),
            needed,
            &mut needed,
        )
    }
    .is_err()
    {
        return String::new();
    }

    let token_user = unsafe { &*(token_buf.as_ptr() as *const TOKEN_USER) };
    let sid = token_user.User.Sid;

    let mut name_len = 0u32;
    let mut domain_len = 0u32;
    let mut sid_use = SID_NAME_USE::default();
    let _ = unsafe {
        LookupAccountSidW(
            PCWSTR::null(),
            sid,
            PWSTR::null(),
            &mut name_len,
            PWSTR::null(),
            &mut domain_len,
            &mut sid_use,
        )
    };
    if name_len == 0 {
        return String::new();
    }

    let mut name_buf = vec![0u16; name_len as usize];
    let mut domain_buf = vec![0u16; domain_len.max(1) as usize];
    if unsafe {
        LookupAccountSidW(
            PCWSTR::null(),
            sid,
            PWSTR(name_buf.as_mut_ptr()),
            &mut name_len,
            PWSTR(domain_buf.as_mut_ptr()),
            &mut domain_len,
            &mut sid_use,
        )
    }
    .is_err()
    {
        return String::new();
    }

    let name = utf16_buffer_to_string(&name_buf, name_len);
    let domain = utf16_buffer_to_string(&domain_buf, domain_len);
    if name.is_empty() {
        String::new()
    } else if domain.is_empty() {
        name
    } else {
        format!("{}\\{}", domain, name)
    }
}

#[cfg(windows)]
fn utf16_buffer_to_string(buf: &[u16], len: u32) -> String {
    let end = (len as usize).min(buf.len());
    let slice = &buf[..end];
    let end = slice.iter().position(|&ch| ch == 0).unwrap_or(slice.len());
    String::from_utf16_lossy(&slice[..end])
}

/// 获取进程工作目录（Windows：通过 NT API 读取进程 PEB，支持中文路径）
#[cfg(windows)]
fn get_cwd(pid: u32) -> String {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
    };

    // ── NT API FFI 定义 ──
    #[repr(C)]
    struct ProcessBasicInformation {
        reserved1: *mut core::ffi::c_void,
        peb_base_address: *mut core::ffi::c_void,
        reserved2: [usize; 2],
        unique_process_id: usize,
        reserved3: usize,
    }

    // PEB 64-bit 布局：ProcessParameters 在偏移 0x20
    // RTL_USER_PROCESS_PARAMETERS：CurrentDirectory 在偏移 0x38
    // CurrentDirectory.Buffer 在 CurrentDirectory 起始 +0x40

    unsafe extern "system" {
        fn NtQueryInformationProcess(
            process_handle: HANDLE,
            process_information_class: u32,
            process_information: *mut core::ffi::c_void,
            process_information_length: u32,
            return_length: *mut u32,
        ) -> i32;
    }

    unsafe {
        // 打开进程：需要查询信息 + 读内存权限
        let handle = match OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) {
            Ok(h) => h,
            Err(_) => return String::new(),
        };

        // 1. 查询 PEB 基址
        let mut pbi = ProcessBasicInformation {
            reserved1: core::ptr::null_mut(),
            peb_base_address: core::ptr::null_mut(),
            reserved2: [0; 2],
            unique_process_id: 0,
            reserved3: 0,
        };
        let status = NtQueryInformationProcess(
            handle,
            0, // ProcessBasicInformation
            &mut pbi as *mut _ as *mut core::ffi::c_void,
            core::mem::size_of::<ProcessBasicInformation>() as u32,
            core::ptr::null_mut(),
        );
        if status != 0 {
            let _ = CloseHandle(handle);
            return String::new();
        }

        // 2. 从 PEB 读取 ProcessParameters 指针（偏移 0x20）
        let mut params_ptr: usize = 0;
        let mut bytes_read = 0usize;
        let ok = windows::Win32::System::Diagnostics::Debug::ReadProcessMemory(
            handle,
            (pbi.peb_base_address as usize + 0x20) as *const core::ffi::c_void,
            &mut params_ptr as *mut _ as *mut core::ffi::c_void,
            core::mem::size_of::<usize>(),
            Some(&mut bytes_read),
        );
        if ok.is_err() || params_ptr == 0 {
            let _ = CloseHandle(handle);
            return String::new();
        }

        // 3. 读取 CurrentDirectory 缓冲区长度（偏移 0x3C，u16）和地址（偏移 0x40，usize）
        let mut buf_len: u16 = 0;
        let mut buf_addr: usize = 0;
        let _ = windows::Win32::System::Diagnostics::Debug::ReadProcessMemory(
            handle,
            (params_ptr + 0x3C) as *const core::ffi::c_void,
            &mut buf_len as *mut _ as *mut core::ffi::c_void,
            2,
            Some(&mut bytes_read),
        );
        let _ = windows::Win32::System::Diagnostics::Debug::ReadProcessMemory(
            handle,
            (params_ptr + 0x40) as *const core::ffi::c_void,
            &mut buf_addr as *mut _ as *mut core::ffi::c_void,
            core::mem::size_of::<usize>(),
            Some(&mut bytes_read),
        );
        if buf_addr == 0 || buf_len == 0 {
            let _ = CloseHandle(handle);
            return String::new();
        }

        // 4. 读取 UTF-16 路径字符串
        let char_count = (buf_len / 2) as usize;
        let mut buf = vec![0u16; char_count];
        let ok = windows::Win32::System::Diagnostics::Debug::ReadProcessMemory(
            handle,
            buf_addr as *const core::ffi::c_void,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            buf_len as usize,
            Some(&mut bytes_read),
        );
        let _ = CloseHandle(handle);

        if ok.is_err() {
            return String::new();
        }

        String::from_utf16_lossy(&buf)
    }
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
    get_executable_path_uncached(pid)
}

#[cfg(windows)]
fn get_executable_path_uncached(pid: u32) -> String {
    let ps_script = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
         $OutputEncoding = [System.Text.Encoding]::UTF8; \
         $p = Get-CimInstance -ClassName Win32_Process -Filter 'ProcessId={}' -OperationTimeoutSec 20 -ErrorAction Stop; \
         if ($p) {{ \
           $ep = [string]$p.ExecutablePath; \
           if ($ep) {{ Write-Output $ep }} \
           else {{ Write-Output $p.Name }} \
         }}",
        pid
    );

    let output = powershell_output(&ps_script, &format!("查询 Windows 可执行路径 PID {pid}"));

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.lines().next().unwrap_or("").trim().to_string()
        }
        Err(_) => String::new(),
    }
}
