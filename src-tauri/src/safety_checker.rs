use serde::Serialize;

use crate::service_classifier::ServiceType;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SafetyLevel {
    Safe,
    Caution,
    Danger,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct SafetyJudgment {
    pub level: SafetyLevel,
    pub reason: String,
    pub can_terminate: bool,
    pub require_confirm: bool,
}

/// 根据服务类型和进程信息判断安全等级
pub fn judge(
    service_type: &ServiceType,
    process_name: &str,
    command_line: &str,
    user: &str,
    current_user: &str,
) -> SafetyJudgment {
    let name_lower = process_name.to_lowercase();
    let cmd_lower = command_line.to_lowercase();

    // 非当前用户进程 → caution
    if !user.is_empty() && user != current_user && user != "root" {
        return SafetyJudgment {
            level: SafetyLevel::Caution,
            reason: format!("该进程由用户 {} 启动，非当前用户", user),
            can_terminate: false,
            require_confirm: true,
        };
    }

    match service_type {
        ServiceType::SystemService => SafetyJudgment {
            level: SafetyLevel::Danger,
            reason: format!("{} 是系统服务，禁止终止", process_name),
            can_terminate: false,
            require_confirm: false,
        },

        ServiceType::DockerService => SafetyJudgment {
            level: SafetyLevel::Caution,
            reason: "Docker 服务，建议停止容器而非杀进程".to_string(),
            can_terminate: false,
            require_confirm: true,
        },

        ServiceType::DatabaseService => SafetyJudgment {
            level: SafetyLevel::Caution,
            reason: format!(
                "{} 是数据库服务，终止后会影响依赖该数据库的项目",
                process_name
            ),
            can_terminate: false,
            require_confirm: true,
        },

        ServiceType::InfraService => SafetyJudgment {
            level: SafetyLevel::Caution,
            reason: format!(
                "{} 是基础设施服务，终止前请确认是否有项目正在使用",
                process_name
            ),
            can_terminate: false,
            require_confirm: true,
        },

        ServiceType::AppService => SafetyJudgment {
            level: SafetyLevel::Caution,
            reason: format!(
                "{} 是用户应用程序，终止可能影响正在使用的功能",
                process_name
            ),
            can_terminate: false,
            require_confirm: true,
        },

        ServiceType::WebServer => {
            if is_user_web_server(&name_lower, &cmd_lower, user, current_user) {
                SafetyJudgment {
                    level: SafetyLevel::Safe,
                    reason: "用户启动的 Web 开发服务".to_string(),
                    can_terminate: true,
                    require_confirm: true,
                }
            } else {
                SafetyJudgment {
                    level: SafetyLevel::Caution,
                    reason: format!("{} 可能是系统级 Web 服务", process_name),
                    can_terminate: false,
                    require_confirm: true,
                }
            }
        }

        ServiceType::DevService => SafetyJudgment {
            level: SafetyLevel::Safe,
            reason: "开发服务，可以安全终止".to_string(),
            can_terminate: true,
            require_confirm: true,
        },

        ServiceType::AiDevService => SafetyJudgment {
            level: SafetyLevel::Safe,
            reason: "AI 开发工具启动的服务，可以安全终止".to_string(),
            can_terminate: true,
            require_confirm: true,
        },

        ServiceType::Unknown => {
            if user == "root" {
                SafetyJudgment {
                    level: SafetyLevel::Danger,
                    reason: "root 进程，禁止终止".to_string(),
                    can_terminate: false,
                    require_confirm: false,
                }
            } else {
                SafetyJudgment {
                    level: SafetyLevel::Unknown,
                    reason: "无法识别该服务类型，请手动确认".to_string(),
                    can_terminate: true,
                    require_confirm: true,
                }
            }
        }
    }
}

fn is_user_web_server(name: &str, cmd: &str, user: &str, current_user: &str) -> bool {
    // 用户自己启动的 nginx/caddy（非系统服务）
    if user == current_user && (name == "nginx" || name == "caddy") {
        return true;
    }
    cmd.contains("dev") || cmd.contains("serve")
}
