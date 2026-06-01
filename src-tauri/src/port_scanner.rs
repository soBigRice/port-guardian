use serde::Serialize;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct PortInfo {
    pub port: u16,
    pub protocol: String,
    pub local_address: String,
    pub state: String,
    pub pid: u32,
}

/// 扫描本机 TCP 监听端口
pub fn scan_listening_ports() -> Result<Vec<PortInfo>, String> {
    let output = Command::new("lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN"])
        .output()
        .map_err(|e| format!("Failed to run lsof: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut ports = Vec::new();

    for line in stdout.lines().skip(1) {
        // lsof 输出格式:
        // COMMAND   PID   USER   FD   TYPE   DEVICE   SIZE/OFF   NODE   NAME
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 9 {
            continue;
        }

        let pid: u32 = match parts[1].parse() {
            Ok(v) => v,
            Err(_) => continue,
        };

        let name = parts[8..].join(" ");
        // NAME 格式: *:<port> (LISTEN) 或 <addr>:<port> (LISTEN)
        let (addr, port) = parse_address_port(&name);
        let port = match port {
            Some(p) => p,
            None => continue,
        };

        // 去重（同一进程同一端口可能有 IPv4 和 IPv6 两条记录）
        if ports.iter().any(|p: &PortInfo| p.pid == pid && p.port == port) {
            continue;
        }

        ports.push(PortInfo {
            port,
            protocol: "TCP".to_string(),
            local_address: addr,
            state: "LISTEN".to_string(),
            pid,
        });
    }

    Ok(ports)
}

/// 解析 lsof NAME 字段，返回 (地址, 端口)
fn parse_address_port(name: &str) -> (String, Option<u16>) {
    // 去掉尾部的 (LISTEN)
    let cleaned = name.replace(" (LISTEN)", "").replace("(LISTEN)", "");
    let cleaned = cleaned.trim();

    // 格式: *:5173 或 127.0.0.1:5173 或 [::1]:5173
    if let Some(idx) = cleaned.rfind(':') {
        let addr = &cleaned[..idx];
        let port_str = &cleaned[idx + 1..];
        let addr = addr.trim_matches(|c| c == '[' || c == ']');
        let addr = if addr.is_empty() || addr == "*" { "0.0.0.0" } else { addr };
        if let Ok(port) = port_str.parse::<u16>() {
            return (addr.to_string(), Some(port));
        }
    }
    (cleaned.to_string(), None)
}
