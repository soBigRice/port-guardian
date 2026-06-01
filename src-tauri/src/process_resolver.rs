use serde::Serialize;
use std::process::Command;

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

/// 通过 ps 获取进程基础信息
fn get_ps_info(pid: u32) -> Result<(u32, String, String, String), String> {
    let output = Command::new("ps")
        .args([
            "-p",
            &pid.to_string(),
            "-o",
            "pid=,ppid=,user=,comm=,args=",
        ])
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

    Ok((ppid, user.trim().to_string(), name.trim().to_string(), command_line))
}

/// 从 ps 输出中提取一个字段（跳过前导空格，取到下一个空白）
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
fn get_cwd(pid: u32) -> String {
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
fn get_executable_path(pid: u32) -> String {
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
