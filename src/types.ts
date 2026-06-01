/// PortInfo - 端口扫描结果
export interface PortInfo {
  port: number;
  protocol: string;
  local_address: string;
  state: string;
  pid: number;
}

/// ProcessInfo - 进程详细信息
export interface ProcessInfo {
  pid: number;
  ppid: number;
  name: string;
  user: string;
  command_line: string;
  cwd: string;
  executable_path: string;
}

/// ProcessNode - 进程树节点
export interface ProcessNode {
  pid: number;
  name: string;
  command_line: string;
}

/// ServiceType - 服务类型
export type ServiceType =
  | "dev-service"
  | "ai-dev-service"
  | "docker-service"
  | "database-service"
  | "web-server"
  | "system-service"
  | "infra-service"
  | "app-service"
  | "unknown";

/// SafetyLevel - 安全等级
export type SafetyLevel = "safe" | "caution" | "danger" | "unknown";

/// Theme - 颜色主题
export type Theme = "dark" | "light" | "auto";

/// PortService - 完整端口服务信息（前端展示用）
export interface PortService {
  id: string;
  port: number;
  protocol: string;
  local_address: string;
  state: string;
  pid: number;
  process_name: string;
  executable_path: string;
  command_line: string;
  cwd: string;
  user: string;
  parent_chain: ProcessNode[];
  source: string;
  service_type: ServiceType;
  service_name: string;
  safety_level: SafetyLevel;
  safety_reason: string;
  can_terminate: boolean;
}

/// TerminateResult - 终止结果
export interface TerminateResult {
  success: boolean;
  message: string;
  port_released: boolean;
}
