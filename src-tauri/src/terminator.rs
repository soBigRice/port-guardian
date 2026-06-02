use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TerminateResult {
    pub success: bool,
    pub message: String,
    pub port_released: bool,
}

// ═══════════════════════════════════════════════════════════════
// Unix (macOS / Linux) 实现 — 使用 kill 和 lsof
// ═══════════════════════════════════════════════════════════════

/// 普通终止进程 (SIGTERM)
#[cfg(unix)]
pub fn terminate(pid: u32) -> TerminateResult {
    let output = std::process::Command::new("kill")
        .args([&pid.to_string()])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                TerminateResult {
                    success: true,
                    message: format!("已发送终止信号给进程 {}", pid),
                    port_released: false,
                }
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                TerminateResult {
                    success: false,
                    message: format!("终止失败: {}", stderr.trim()),
                    port_released: false,
                }
            }
        }
        Err(e) => TerminateResult {
            success: false,
            message: format!("执行 kill 命令失败: {}", e),
            port_released: false,
        },
    }
}

/// 强制终止进程 (SIGKILL)
#[cfg(unix)]
pub fn force_terminate(pid: u32) -> TerminateResult {
    let output = std::process::Command::new("kill")
        .args(["-9", &pid.to_string()])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                TerminateResult {
                    success: true,
                    message: format!("已强制终止进程 {}", pid),
                    port_released: false,
                }
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                TerminateResult {
                    success: false,
                    message: format!("强制终止失败: {}", stderr.trim()),
                    port_released: false,
                }
            }
        }
        Err(e) => TerminateResult {
            success: false,
            message: format!("执行 kill -9 命令失败: {}", e),
            port_released: false,
        },
    }
}

/// 检查进程是否仍然存在
#[cfg(unix)]
pub fn is_process_alive(pid: u32) -> bool {
    let output = std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output();

    match output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

/// 等待进程退出（Unix：轮询 kill -0）
/// 返回 true 表示进程已退出，false 表示超时
#[cfg(unix)]
pub fn wait_for_process_exit(pid: u32, timeout_ms: u32) -> bool {
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms as u64);

    while start.elapsed() < timeout {
        if !is_process_alive(pid) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    !is_process_alive(pid)
}

/// 检查端口是否仍然被监听
#[cfg(unix)]
pub fn is_port_listening(port: u16) -> bool {
    let output = std::process::Command::new("lsof")
        .args(["-nP", "-iTCP", &format!(":{}", port), "-sTCP:LISTEN"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.lines().count() > 1 // 第一行是表头
        }
        Err(_) => false,
    }
}

// ═══════════════════════════════════════════════════════════════
// Windows 实现 — 使用 taskkill 和 netstat
// ═══════════════════════════════════════════════════════════════

/// 普通终止进程（Windows：taskkill）
#[cfg(windows)]
pub fn terminate(pid: u32) -> TerminateResult {
    let output = crate::windows_command::hidden_command("taskkill")
        .args(["/PID", &pid.to_string()])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                TerminateResult {
                    success: true,
                    message: format!("已发送终止信号给进程 {}", pid),
                    port_released: false,
                }
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let stdout = String::from_utf8_lossy(&out.stdout);
                let msg = if !stderr.trim().is_empty() {
                    stderr.trim().to_string()
                } else {
                    stdout.trim().to_string()
                };
                TerminateResult {
                    success: false,
                    message: format!("终止失败: {}", msg),
                    port_released: false,
                }
            }
        }
        Err(e) => TerminateResult {
            success: false,
            message: format!("执行 taskkill 命令失败: {}", e),
            port_released: false,
        },
    }
}

/// 强制终止进程（Windows：taskkill /F）
#[cfg(windows)]
pub fn force_terminate(pid: u32) -> TerminateResult {
    let output = crate::windows_command::hidden_command("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                TerminateResult {
                    success: true,
                    message: format!("已强制终止进程 {}", pid),
                    port_released: false,
                }
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let stdout = String::from_utf8_lossy(&out.stdout);
                let msg = if !stderr.trim().is_empty() {
                    stderr.trim().to_string()
                } else {
                    stdout.trim().to_string()
                };
                TerminateResult {
                    success: false,
                    message: format!("强制终止失败: {}", msg),
                    port_released: false,
                }
            }
        }
        Err(e) => TerminateResult {
            success: false,
            message: format!("执行 taskkill /F 命令失败: {}", e),
            port_released: false,
        },
    }
}

/// 检查进程是否仍然存在（Windows：OpenProcess API，比 tasklist 快得多）
#[cfg(windows)]
pub fn is_process_alive(pid: u32) -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    unsafe {
        match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(handle) => {
                let _ = CloseHandle(handle);
                true
            }
            Err(_) => false,
        }
    }
}

/// 等待进程退出（Windows：OpenProcess + WaitForSingleObject）
/// 返回 true 表示进程已退出，false 表示超时
#[cfg(windows)]
pub fn wait_for_process_exit(pid: u32, timeout_ms: u32) -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, INFINITE, PROCESS_SYNCHRONIZE,
    };

    unsafe {
        let handle = match OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            Ok(h) => h,
            Err(_) => return true, // 进程已不存在，视为退出
        };

        let timeout = if timeout_ms == 0 { INFINITE } else { timeout_ms };
        let result = WaitForSingleObject(handle, timeout);
        let _ = CloseHandle(handle);

        // WAIT_OBJECT_0 (0) 表示进程已退出
        result.0 == 0
    }
}

/// 检查端口是否仍然被监听（Windows：netstat2 API，毫秒级）
#[cfg(windows)]
pub fn is_port_listening(port: u16) -> bool {
    use netstat2::{
        get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState,
    };

    let sockets = get_sockets_info(
        AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6,
        ProtocolFlags::TCP,
    );

    match sockets {
        Ok(sockets) => sockets.iter().any(|si| {
            if let ProtocolSocketInfo::Tcp(tcp) = &si.protocol_socket_info {
                tcp.local_port == port && tcp.state == TcpState::Listen
            } else {
                false
            }
        }),
        Err(_) => false,
    }
}
