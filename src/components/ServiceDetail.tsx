import { invoke } from "@tauri-apps/api/core";
import { PortService } from "../types";
import RiskBadge from "./RiskBadge";
import SourceIcon from "./SourceIcon";

interface Props {
  service: PortService;
  onKill: () => void;
}

export default function ServiceDetail({ service, onKill }: Props) {
  const shortCwd = service.cwd
    ? service.cwd.replace(/^\/Users\/[^/]+/, "~")
    : "";

  const handleOpenPath = async (path: string) => {
    try {
      await invoke("open_directory", { path });
    } catch (e) {
      console.error("打开目录失败:", e);
    }
  };

  return (
    <div className="detail-panel">
      <div className="detail-header">
        <h3>端口 {service.port} 详情</h3>
        <RiskBadge level={service.safety_level} />
      </div>

      <div className="detail-section">
        <h4>基础信息</h4>
        <div className="detail-row">
          <span className="detail-label">端口</span>
          <span className="detail-value">{service.port}</span>
        </div>
        <div className="detail-row">
          <span className="detail-label">协议</span>
          <span className="detail-value">{service.protocol}</span>
        </div>
        <div className="detail-row">
          <span className="detail-label">地址</span>
          <span className="detail-value">{service.local_address}</span>
        </div>
        <div className="detail-row">
          <span className="detail-label">状态</span>
          <span className="detail-value">{service.state}</span>
        </div>
        <div className="detail-row">
          <span className="detail-label">PID</span>
          <span className="detail-value">{service.pid}</span>
        </div>
        <div className="detail-row">
          <span className="detail-label">用户</span>
          <span className="detail-value">{service.user}</span>
        </div>
      </div>

      <div className="detail-section">
        <h4>进程信息</h4>
        <div className="detail-row">
          <span className="detail-label">进程名</span>
          <span className="detail-value">{service.process_name}</span>
        </div>
        {service.executable_path && (
          <div className="detail-row">
            <span className="detail-label">可执行文件</span>
            <span
              className="detail-value long clickable-path"
              title="点击打开所在目录"
              onClick={() => handleOpenPath(service.executable_path)}
            >
              {service.executable_path}
            </span>
          </div>
        )}
        <div className="detail-row">
          <span className="detail-label">启动命令</span>
          <span className="detail-value long">{service.command_line}</span>
        </div>
        {shortCwd && (
          <div className="detail-row">
            <span className="detail-label">工作目录</span>
            <span
              className="detail-value long clickable-path"
              title="点击打开目录"
              onClick={() => handleOpenPath(service.cwd)}
            >
              {shortCwd}
            </span>
          </div>
        )}
      </div>

      <div className="detail-section">
        <h4>来源信息</h4>
        <div className="detail-row">
          <span className="detail-label">来源</span>
          <span className="detail-value">
            <SourceIcon source={service.source} size={16} />{service.source}
          </span>
        </div>
        {service.parent_chain.length > 0 && (
          <div style={{ marginTop: 6 }}>
            <span className="detail-label">进程链</span>
            <div className="parent-chain">
              {service.parent_chain.map((node, i) => (
                <span key={node.pid} style={{ display: "flex", alignItems: "center", gap: 4 }}>
                  {i > 0 && <span className="parent-arrow">&#8594;</span>}
                  <span className="parent-node" title={node.command_line}>
                    {node.name}
                  </span>
                </span>
              ))}
            </div>
          </div>
        )}
      </div>

      <div className="detail-section">
        <h4>安全判断</h4>
        <div className="detail-row">
          <span className="detail-label">服务类型</span>
          <span className="detail-value">
            <span className="badge badge-service">
              {service.service_name || service.service_type}
            </span>
          </span>
        </div>
        <div className="detail-row">
          <span className="detail-label">风险等级</span>
          <span className="detail-value">
            <RiskBadge level={service.safety_level} />
          </span>
        </div>
        <div className="detail-row">
          <span className="detail-label">判断依据</span>
          <span className="detail-value long" style={{ color: "var(--text-dim)" }}>
            {service.safety_reason}
          </span>
        </div>
      </div>

      {service.can_terminate && (
        <div style={{ marginTop: 16 }}>
          <button className="btn btn-danger" onClick={onKill} style={{ width: "100%" }}>
            终止该服务
          </button>
        </div>
      )}
    </div>
  );
}
