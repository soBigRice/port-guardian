use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PortInfo {
    pub port: u16,
    pub protocol: String,
    pub local_address: String,
    pub state: String,
    pub pid: u32,
}

/// 扫描本机 TCP 监听端口
#[cfg(unix)]
pub fn scan_listening_ports() -> Result<Vec<PortInfo>, String> {
    use std::process::Command;

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
        if ports
            .iter()
            .any(|p: &PortInfo| p.pid == pid && p.port == port)
        {
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

/// 扫描本机 TCP 监听端口（Windows 版本，使用 netstat2 调用 GetExtendedTcpTable API）
#[cfg(windows)]
pub fn scan_listening_ports() -> Result<Vec<PortInfo>, String> {
    use netstat2::{
        get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState,
    };

    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let proto_flags = ProtocolFlags::TCP;
    let sockets =
        get_sockets_info(af_flags, proto_flags).map_err(|e| format!("GetExtendedTcpTable: {}", e))?;

    let mut ports = Vec::new();

    for si in &sockets {
        if let ProtocolSocketInfo::Tcp(tcp_si) = &si.protocol_socket_info {
            // 只要 LISTEN 状态
            if tcp_si.state != TcpState::Listen {
                continue;
            }

            let port = tcp_si.local_port;
            let addr = tcp_si.local_addr.to_string();

            for &pid in &si.associated_pids {
                // 去重（同一进程同一端口可能有 IPv4 和 IPv6 两条记录）
                if ports.iter().any(|p: &PortInfo| p.pid == pid && p.port == port) {
                    continue;
                }

                ports.push(PortInfo {
                    port,
                    protocol: "TCP".to_string(),
                    local_address: addr.clone(),
                    state: "LISTEN".to_string(),
                    pid,
                });
            }
        }
    }

    Ok(ports)
}

/// 解析 lsof NAME 字段，返回 (地址, 端口)
#[cfg(unix)]
fn parse_address_port(name: &str) -> (String, Option<u16>) {
    // 去掉尾部的 (LISTEN)
    let cleaned = name.replace(" (LISTEN)", "").replace("(LISTEN)", "");
    let cleaned = cleaned.trim();

    // 格式: *:5173 或 127.0.0.1:5173 或 [::1]:5173
    if let Some(idx) = cleaned.rfind(':') {
        let addr = &cleaned[..idx];
        let port_str = &cleaned[idx + 1..];
        let addr = addr.trim_matches(|c| c == '[' || c == ']');
        let addr = if addr.is_empty() || addr == "*" {
            "0.0.0.0"
        } else {
            addr
        };
        if let Ok(port) = port_str.parse::<u16>() {
            return (addr.to_string(), Some(port));
        }
    }
    (cleaned.to_string(), None)
}
