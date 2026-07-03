use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PortInfo {
    pub port: u16,
    pub protocol: String,
    pub local_address: String,
    pub state: String,
    pub pid: u32,
}

/// 扫描本机 TCP 监听端口和 UDP 绑定端口（全平台统一使用 netstat2 API）
/// - macOS: 通过 libproc 调用系统 API，比 lsof 子进程快 10 倍以上
/// - Windows: 通过 GetExtendedTcpTable / GetExtendedUdpTable API
/// - Linux: 通过 /proc/net/tcp 和 /proc/net/udp
pub fn scan_listening_ports() -> Result<Vec<PortInfo>, String> {
    use std::collections::HashSet;

    use netstat2::{
        AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState, get_sockets_info,
    };

    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let mut ports = Vec::new();
    let mut seen = HashSet::new();

    // ── TCP: 只要 LISTEN 状态 ──
    let tcp_sockets = get_sockets_info(af_flags, ProtocolFlags::TCP)
        .map_err(|e| format!("Failed to get TCP socket info: {}", e))?;

    for si in &tcp_sockets {
        if let ProtocolSocketInfo::Tcp(tcp_si) = &si.protocol_socket_info {
            if tcp_si.state != TcpState::Listen {
                continue;
            }

            let port = tcp_si.local_port;
            let addr = tcp_si.local_addr.to_string();

            for &pid in &si.associated_pids {
                // 同一 PID/协议/端口可能同时有 IPv4/IPv6 记录；用 HashSet 去重，避免端口多时反复线性扫描。
                if !seen.insert((pid, "TCP", port)) {
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

    // ── UDP: 所有绑定端口（UDP 无连接状态，统一标记为 UNCONN） ──
    let udp_sockets = get_sockets_info(af_flags, ProtocolFlags::UDP)
        .map_err(|e| format!("Failed to get UDP socket info: {}", e))?;

    for si in &udp_sockets {
        if let ProtocolSocketInfo::Udp(udp_si) = &si.protocol_socket_info {
            let port = udp_si.local_port;
            let addr = udp_si.local_addr.to_string();

            for &pid in &si.associated_pids {
                // UDP 同样可能出现多地址记录；保持原去重语义，只把复杂度从 O(n²) 降到 O(n)。
                if !seen.insert((pid, "UDP", port)) {
                    continue;
                }
                ports.push(PortInfo {
                    port,
                    protocol: "UDP".to_string(),
                    local_address: addr.clone(),
                    state: "UNCONN".to_string(),
                    pid,
                });
            }
        }
    }

    Ok(ports)
}
