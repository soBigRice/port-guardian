use serde::Serialize;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct TerminateResult {
    pub success: bool,
    pub message: String,
    pub port_released: bool,
}

/// 普通终止进程 (SIGTERM)
pub fn terminate(pid: u32) -> TerminateResult {
    let output = Command::new("kill")
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
pub fn force_terminate(pid: u32) -> TerminateResult {
    let output = Command::new("kill")
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
pub fn is_process_alive(pid: u32) -> bool {
    let output = Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output();

    match output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}


