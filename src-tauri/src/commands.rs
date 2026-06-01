use serde::Serialize;

use crate::port_scanner;
use crate::process_resolver::{self, ProcessInfo};
use crate::process_tree;
use crate::service_classifier::{self, ServiceClassification, ServiceType};
use crate::safety_checker::{self, SafetyJudgment, SafetyLevel};
use crate::terminator::{self, TerminateResult};

#[derive(Debug, Clone, Serialize)]
pub struct PortService {
    pub id: String,
    pub port: u16,
    pub protocol: String,
    pub local_address: String,
    pub state: String,
    pub pid: u32,
    pub process_name: String,
    pub executable_path: String,
    pub command_line: String,
    pub cwd: String,
    pub user: String,
    pub parent_chain: Vec<process_tree::ProcessNode>,
    pub source: String,
    pub service_type: ServiceType,
    pub service_name: String,
    pub safety_level: SafetyLevel,
    pub safety_reason: String,
    pub can_terminate: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessDetail {
    pub process: ProcessInfo,
    pub parent_chain: Vec<process_tree::ProcessNode>,
    pub source: String,
    pub classification: ServiceClassification,
    pub safety: SafetyJudgment,
}

/// 扫描所有监听端口，返回完整的端口服务列表
#[tauri::command]
pub fn scan_ports() -> Result<Vec<PortService>, String> {
    let current_user = whoami::username();
    let ports = port_scanner::scan_listening_ports()?;
    let mut services = Vec::new();

    for port_info in ports {
        let process = match process_resolver::resolve_process(port_info.pid) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let parent_chain = process_tree::build_parent_chain(port_info.pid);
        let source = process_tree::identify_source(&parent_chain);
        let classification =
            service_classifier::classify(&process, &parent_chain, port_info.port, &source);
        let safety = safety_checker::judge(
            &classification.service_type,
            &process.name,
            &process.command_line,
            &process.user,
            &current_user,
        );

        services.push(PortService {
            id: format!("{}-{}", port_info.port, port_info.pid),
            port: port_info.port,
            protocol: port_info.protocol,
            local_address: port_info.local_address,
            state: port_info.state,
            pid: port_info.pid,
            process_name: process.name,
            executable_path: process.executable_path,
            command_line: process.command_line,
            cwd: process.cwd,
            user: process.user,
            parent_chain,
            source,
            service_type: classification.service_type,
            service_name: classification.service_name,
            safety_level: safety.level,
            safety_reason: safety.reason,
            can_terminate: safety.can_terminate,
        });
    }

    Ok(services)
}

/// 获取指定 PID 的详细进程信息
#[tauri::command]
pub fn get_process_detail(pid: u32) -> Result<ProcessDetail, String> {
    let current_user = whoami::username();
    let process = process_resolver::resolve_process(pid)?;
    let parent_chain = process_tree::build_parent_chain(pid);
    let source = process_tree::identify_source(&parent_chain);
    let port = 0; // 进程详情不需要端口信息做分类

    // 尝试从命令行推断端口
    let classification = service_classifier::classify(&process, &parent_chain, port, &source);
    let safety = safety_checker::judge(
        &classification.service_type,
        &process.name,
        &process.command_line,
        &process.user,
        &current_user,
    );

    Ok(ProcessDetail {
        process,
        parent_chain,
        source,
        classification,
        safety,
    })
}

/// 终止指定进程
#[tauri::command]
pub fn terminate_process(pid: u32, force: bool) -> Result<TerminateResult, String> {
    if force {
        Ok(terminator::force_terminate(pid))
    } else {
        let mut result = terminator::terminate(pid);
        if result.success {
            // 等待 500ms 后检查进程是否已退出
            std::thread::sleep(std::time::Duration::from_millis(500));
            if !terminator::is_process_alive(pid) {
                result.port_released = true;
                result.message = format!("进程 {} 已成功终止", pid);
            }
        }
        Ok(result)
    }
}

// 需要在 Cargo.toml 中添加 whoami 依赖
// 临时使用环境变量替代
mod whoami {
    pub fn username() -> String {
        std::env::var("USER").unwrap_or_else(|_| "unknown".to_string())
    }
}
