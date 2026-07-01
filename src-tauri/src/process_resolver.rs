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
use crate::windows_command::hidden_command;
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
    let ps_script = "chcp 65001 > $null; \
                     [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
                     $OutputEncoding = [System.Text.Encoding]::UTF8; \
                     Get-CimInstance -ClassName Win32_Process -ErrorAction SilentlyContinue | \
                     ForEach-Object { \
                       Write-Output ('PID:' + $_.ProcessId); \
                       Write-Output ('PPID:' + $_.ParentProcessId); \
                       Write-Output ('NAME:' + $_.Name); \
                       Write-Output ('EPATH:' + [string]$_.ExecutablePath); \
                       Write-Output ('CMD:' + [string]$_.CommandLine); \
                       Write-Output '---'; \
                     }";

    let output = match hidden_command("powershell")
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
    // PROC_PIDVNODEPATHINFO = 9
    const PROC_PIDVNODEPATHINFO: u32 = 9;

    // 匹配 macOS 内核头文件中的结构体布局
    #[repr(C)]
    struct VnodeInfo {
        vi_type: i32,
        vi_fsid: u32,
        vi_dev: u32,
        vi_mode: u16,
        _pad1: u16,
        vi_nlink: u32,
        vi_ino: u64,
        vi_user: u64,
        vi_group: u64,
        vi_atime: i64,
        vi_atime_nsec: i64,
        vi_mtime: i64,
        vi_mtime_nsec: i64,
        vi_ctime: i64,
        vi_ctime_nsec: i64,
        vi_birthtime: i64,
        vi_birthtime_nsec: i64,
        vi_size: i64,
        vi_blocks: i64,
        vi_blocksize: i32,
        _pad2: i32,
        vi_flags: u32,
        _pad3: u32,
    }

    #[repr(C)]
    struct VnodeInfoWithPath {
        vip_vi: VnodeInfo,
        vip_path: [u8; 1024],
    }

    #[repr(C)]
    struct ProcVnodePathInfo {
        pvi_cdir: VnodeInfoWithPath,
        pvi_rdir: VnodeInfoWithPath,
    }

    let mut info = core::mem::MaybeUninit::<ProcVnodePathInfo>::uninit();
    let size = core::mem::size_of::<ProcVnodePathInfo>() as i32;

    let ret = unsafe {
        libc::proc_pidinfo(
            pid as i32,
            PROC_PIDVNODEPATHINFO as i32,
            0,
            info.as_mut_ptr() as *mut libc::c_void,
            size,
        )
    };

    if ret <= 0 {
        return String::new();
    }

    let info = unsafe { info.assume_init() };
    let path_bytes = &info.pvi_cdir.vip_path;
    // 找到 null 终止符
    let len = path_bytes.iter().position(|&b| b == 0).unwrap_or(path_bytes.len());
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
        let ret = unsafe {
            proc_pidpath(pid as i32, buf.as_mut_ptr() as *mut libc::c_void, 1024)
        };
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
        "chcp 65001 > $null; \
         [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
         $OutputEncoding = [System.Text.Encoding]::UTF8; \
         $p = Get-CimInstance -ClassName Win32_Process -Filter 'ProcessId={}' -ErrorAction SilentlyContinue; \
         if ($p) {{ \
           Write-Output ('PPID:' + $p.ParentProcessId); \
           Write-Output ('NAME:' + $p.Name); \
           Write-Output ('CMD:' + [string]$p.CommandLine); \
         }}",
        pid
    );

    let output = hidden_command("powershell")
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
        "chcp 65001 > $null; \
         [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
         $OutputEncoding = [System.Text.Encoding]::UTF8; \
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

    let output = hidden_command("powershell")
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
        let handle = match OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false,
            pid,
        ) {
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

    let ps_script = format!(
        "chcp 65001 > $null; \
         [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
         $OutputEncoding = [System.Text.Encoding]::UTF8; \
         $p = Get-CimInstance -ClassName Win32_Process -Filter 'ProcessId={}' -ErrorAction SilentlyContinue; \
         if ($p) {{ \
           $ep = [string]$p.ExecutablePath; \
           if ($ep) {{ Write-Output $ep }} \
           else {{ Write-Output $p.Name }} \
         }}",
        pid
    );

    let output = hidden_command("powershell")
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
