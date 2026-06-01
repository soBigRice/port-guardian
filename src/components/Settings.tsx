import { useState } from "react";
import { Theme } from "../types";

interface Props {
  theme: Theme;
  onThemeChange: (t: Theme) => void;
  onClose: () => void;
  onCheckUpdate: () => Promise<boolean>;
}

const VERSION = "0.1.0";

export default function Settings({ theme, onThemeChange, onClose, onCheckUpdate }: Props) {
  const [checking, setChecking] = useState(false);
  const [updateStatus, setUpdateStatus] = useState<"idle" | "latest" | "found">("idle");

  async function handleCheck() {
    setChecking(true);
    setUpdateStatus("idle");
    try {
      const found = await onCheckUpdate();
      setUpdateStatus(found ? "found" : "latest");
    } catch {
      setUpdateStatus("idle");
    } finally {
      setChecking(false);
    }
  }
  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog settings-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="settings-titlebar">
          <div className="traffic-lights">
            <span className="traffic-light red" onClick={onClose} title="关闭">&times;</span>
          </div>
          <span className="settings-titlebar-text">设置</span>
          <span className="version-tag">v{VERSION}</span>
        </div>

        <div className="settings-content">
        <div className="settings-section">
          <h4>外观</h4>
          <div className="settings-row">
            <span className="settings-label">颜色主题</span>
            <div className="theme-options">
              {(["dark", "light", "auto"] as Theme[]).map((t) => (
                <button
                  key={t}
                  className={`theme-btn ${theme === t ? "active" : ""}`}
                  onClick={() => onThemeChange(t)}
                >
                  {t === "dark" ? "深色" : t === "light" ? "浅色" : "跟随系统"}
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className="settings-section">
          <h4>关于</h4>
          <div className="settings-row">
            <span className="settings-label">应用名称</span>
            <span className="settings-value">Port Guardian</span>
          </div>
          <div className="settings-row">
            <span className="settings-label">版本</span>
            <span className="settings-value">{VERSION}</span>
          </div>
          <div className="settings-row">
            <span className="settings-label">技术栈</span>
            <span className="settings-value">Tauri 2 + Rust + React</span>
          </div>
          <div className="settings-row">
            <span className="settings-label">平台</span>
            <span className="settings-value">macOS</span>
          </div>
          <div className="settings-row">
            <span className="settings-label">更新</span>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              {updateStatus === "latest" && (
                <span className="settings-value" style={{ color: "var(--safe)", fontSize: 12 }}>
                  已是最新版本
                </span>
              )}
              <button
                className="btn btn-refresh"
                onClick={handleCheck}
                disabled={checking}
                style={{ padding: "4px 12px", fontSize: 12 }}
              >
                {checking ? "检查中..." : "检查更新"}
              </button>
            </div>
          </div>
        </div>

        <div className="settings-section">
          <h4>功能说明</h4>
          <div className="settings-note">
            <p>Port Guardian 帮助你发现、识别和安全清理本机端口服务。</p>
            <ul>
              <li><strong>安全</strong>：开发服务，可直接终止</li>
              <li><strong>谨慎</strong>：数据库/Docker/应用，需二次确认</li>
              <li><strong>危险</strong>：系统服务，禁止终止</li>
              <li><strong>未知</strong>：无法识别，需手动判断</li>
            </ul>
          </div>
        </div>
        </div>

      </div>
    </div>
  );
}
