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
/// - macOS: 通过 libproc 调用系统 API
/// - Windows: 通过 GetExtendedTcpTable / GetExtendedUdpTable API
/// - Linux: 通过 /proc/net/tcp 和 /proc/net/udp
pub fn scan_listening_ports() -> Result<Vec<PortInfo>, String> {
    use netstat2::{
        get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState,
    };

    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let mut ports = Vec::new();

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
                add_port(
                    &mut ports,
                    PortInfo {
                        port,
                        protocol: "TCP".to_string(),
                        local_address: addr.clone(),
                        state: "LISTEN".to_string(),
                        pid,
                    },
                );
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
                add_port(
                    &mut ports,
                    PortInfo {
                        port,
                        protocol: "UDP".to_string(),
                        local_address: addr.clone(),
                        state: "UNCONN".to_string(),
                        pid,
                    },
                );
            }
        }
    }

    Ok(ports)
}

// 服务仍按协议、端口和 PID 合并，但保留全部绑定地址（包括 IPv4/IPv6）。
fn add_port(ports: &mut Vec<PortInfo>, incoming: PortInfo) {
    // 系统也会返回尚未绑定的 UDP 套接字；端口 0 不代表可释放的占用端口。
    if incoming.port == 0 {
        return;
    }
    if let Some(existing) = ports.iter_mut().find(|p| {
        p.pid == incoming.pid && p.port == incoming.port && p.protocol == incoming.protocol
    }) {
        let mut addresses: Vec<_> = existing
            .local_address
            .split(", ")
            .map(str::to_owned)
            .collect();
        if !addresses.contains(&incoming.local_address) {
            addresses.push(incoming.local_address);
            addresses.sort();
            existing.local_address = addresses.join(", ");
        }
    } else {
        ports.push(incoming);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn excludes_unbound_sockets() {
        let mut ports = Vec::new();
        add_port(
            &mut ports,
            PortInfo {
                port: 0,
                protocol: "UDP".into(),
                local_address: "0.0.0.0".into(),
                state: "UNCONN".into(),
                pid: 123,
            },
        );
        assert!(ports.is_empty());
    }
    #[test]
    fn preserves_both_addresses_without_duplicate_rows() {
        let mut ports = Vec::new();
        for address in ["127.0.0.1", "::1", "127.0.0.1"] {
            add_port(
                &mut ports,
                PortInfo {
                    port: 3000,
                    protocol: "TCP".into(),
                    local_address: address.into(),
                    state: "LISTEN".into(),
                    pid: 123,
                },
            );
        }
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].local_address, "127.0.0.1, ::1");
        add_port(
            &mut ports,
            PortInfo {
                port: 3000,
                protocol: "UDP".into(),
                local_address: "127.0.0.1".into(),
                state: "UNCONN".into(),
                pid: 123,
            },
        );
        assert_eq!(ports.len(), 2);
    }
}
