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

/// 检查端口是否仍然被监听
#[allow(dead_code)]
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

/// 检查进程是否仍然存在（Windows：tasklist）
#[cfg(windows)]
pub fn is_process_alive(pid: u32) -> bool {
    let output = crate::windows_command::hidden_command("tasklist")
        .args(["/FI", &format!("PID eq {}", pid), "/NH"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // 如果进程存在，输出包含进程名；如果不存在，输出 "没有运行的任务匹配..."
            // 英文系统: "No tasks are running which match..."
            !stdout.contains("No tasks")
                && !stdout.contains("没有运行")
                && !stdout.trim().is_empty()
        }
        Err(_) => false,
    }
}

/// 检查端口是否仍然被监听（Windows：netstat）
#[allow(dead_code)]
#[cfg(windows)]
pub fn is_port_listening(port: u16) -> bool {
    let output = crate::windows_command::hidden_command("netstat")
        .args(["-ano", "-p", "tcp"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let port_str = format!(":{}", port);
            stdout
                .lines()
                .any(|line| line.contains(&port_str) && line.contains("LISTENING"))
        }
        Err(_) => false,
    }
}
