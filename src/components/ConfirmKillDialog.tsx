import { useState } from "react";
import { PortService } from "../types";

interface Props {
  service: PortService;
  onConfirm: (force: boolean) => void;
  onCancel: () => void;
}

export default function ConfirmKillDialog({ service, onConfirm, onCancel }: Props) {
  const [confirmText, setConfirmText] = useState("");
  const isDanger = service.safety_level === "danger";
  const isCaution = service.safety_level === "caution" || service.safety_level === "unknown";
  const isSafe = service.safety_level === "safe";

  const shortCwd = service.cwd
    ? service.cwd.replace(/^\/Users\/[^/]+/, "~")
    : "";

  // 危险服务禁止终止
  if (isDanger) {
    return (
      <div className="dialog-overlay" onClick={onCancel}>
        <div className="dialog" onClick={(e) => e.stopPropagation()}>
          <h3>禁止终止该服务</h3>
          <div className="dialog-warning danger">
            {service.process_name} 是系统或关键服务，不建议通过本工具终止。
          </div>
          <div className="dialog-info">
            <div className="detail-row">
              <span className="detail-label">端口</span>
              <span className="detail-value">{service.port}</span>
            </div>
            <div className="detail-row">
              <span className="detail-label">进程</span>
              <span className="detail-value">{service.process_name}</span>
            </div>
            <div className="detail-row">
              <span className="detail-label">PID</span>
              <span className="detail-value">{service.pid}</span>
            </div>
            <div className="detail-row">
              <span className="detail-label">说明</span>
              <span className="detail-value long">{service.safety_reason}</span>
            </div>
          </div>
          <div className="dialog-actions">
            <button className="btn" onClick={onCancel}>
              关闭
            </button>
          </div>
        </div>
      </div>
    );
  }

  // 谨慎服务需要输入端口号确认
  if (isCaution) {
    const canConfirm = confirmText === service.port.toString();
    return (
      <div className="dialog-overlay" onClick={onCancel}>
        <div className="dialog" onClick={(e) => e.stopPropagation()}>
          <h3>谨慎操作</h3>
          <div className="dialog-warning caution">
            {service.safety_reason}
          </div>
          <div className="dialog-info">
            <div className="detail-row">
              <span className="detail-label">端口</span>
              <span className="detail-value">{service.port}</span>
            </div>
            <div className="detail-row">
              <span className="detail-label">进程</span>
              <span className="detail-value">{service.process_name}</span>
            </div>
            <div className="detail-row">
              <span className="detail-label">PID</span>
              <span className="detail-value">{service.pid}</span>
            </div>
            {shortCwd && (
              <div className="detail-row">
                <span className="detail-label">目录</span>
                <span className="detail-value long">{shortCwd}</span>
              </div>
            )}
          </div>
          <div style={{ marginBottom: 16 }}>
            <label style={{ fontSize: 12, color: "var(--text-dim)", display: "block", marginBottom: 6 }}>
              请输入端口号 <strong style={{ color: "var(--caution)" }}>{service.port}</strong> 以确认终止：
            </label>
            <input
              className="search-input"
              style={{ width: "100%" }}
              value={confirmText}
              onChange={(e) => setConfirmText(e.target.value)}
              placeholder={`输入 ${service.port} 确认`}
              autoFocus
            />
          </div>
          <div className="dialog-actions">
            <button className="btn" onClick={onCancel}>
              取消
            </button>
            <button
              className="btn btn-danger"
              disabled={!canConfirm}
              onClick={() => onConfirm(false)}
            >
              终止
            </button>
          </div>
        </div>
      </div>
    );
  }

  // 安全服务简单确认
  return (
    <div className="dialog-overlay" onClick={onCancel}>
      <div className="dialog" onClick={(e) => e.stopPropagation()}>
        <h3>确认终止该开发服务？</h3>
        <div className="dialog-warning safe">
          {service.safety_reason}
        </div>
        <div className="dialog-info">
          <div className="detail-row">
            <span className="detail-label">端口</span>
            <span className="detail-value">{service.port}</span>
          </div>
          <div className="detail-row">
            <span className="detail-label">服务</span>
            <span className="detail-value">{service.service_name}</span>
          </div>
          <div className="detail-row">
            <span className="detail-label">进程</span>
            <span className="detail-value">{service.process_name}</span>
          </div>
          <div className="detail-row">
            <span className="detail-label">PID</span>
            <span className="detail-value">{service.pid}</span>
          </div>
          <div className="detail-row">
            <span className="detail-label">命令</span>
            <span className="detail-value long">{service.command_line}</span>
          </div>
          {shortCwd && (
            <div className="detail-row">
              <span className="detail-label">目录</span>
              <span className="detail-value long">{shortCwd}</span>
            </div>
          )}
          {service.source !== "Unknown" && (
            <div className="detail-row">
              <span className="detail-label">来源</span>
              <span className="detail-value">{service.source}</span>
            </div>
          )}
        </div>
        <div className="dialog-actions">
          <button className="btn" onClick={onCancel}>
            取消
          </button>
          <button className="btn btn-safe" onClick={() => onConfirm(false)}>
            终止
          </button>
        </div>
      </div>
    </div>
  );
}
