import { useState } from "react";
import { Theme } from "../types";

interface Props {
  version: string;
  theme: Theme;
  updateError: string | null;
  onThemeChange: (t: Theme) => void;
  onClose: () => void;
  onCheckUpdate: () => Promise<boolean>;
}

function getPlatform(): string {
  const p = navigator.platform || navigator.userAgent || "";
  if (p.includes("Mac") || p.includes("mac")) return "macOS";
  if (p.includes("Win") || p.includes("win")) return "Windows";
  if (p.includes("Linux") || p.includes("linux")) return "Linux";
  return p;
}

function isWindows(): boolean {
  const p = navigator.platform || navigator.userAgent || "";
  return p.includes("Win") || p.includes("win");
}

export default function Settings({ version, theme, updateError, onThemeChange, onClose, onCheckUpdate }: Props) {
  const [checking, setChecking] = useState(false);
  const [updateStatus, setUpdateStatus] = useState<"idle" | "latest" | "found" | "error">("idle");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  async function handleCheck() {
    setChecking(true);
    setUpdateStatus("idle");
    setErrorMessage(null);
    try {
      const found = await onCheckUpdate();
      setUpdateStatus(found ? "found" : "latest");
    } catch (err) {
      setErrorMessage(err instanceof Error ? err.message : String(err));
      setUpdateStatus("error");
    } finally {
      setChecking(false);
    }
  }
  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog settings-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="settings-titlebar">
          {!isWindows() ? (
            <>
              <div className="traffic-lights">
                <span className="traffic-light red" onClick={onClose} title="关闭">&times;</span>
              </div>
              <span className="settings-titlebar-text">设置</span>
              <span className="version-tag">v{version}</span>
            </>
          ) : (
            <>
              <span className="settings-titlebar-text" style={{ flex: 1 }}>设置</span>
              <span className="version-tag">v{version}</span>
              <button className="settings-close-btn" onClick={onClose} title="关闭">&times;</button>
            </>
          )}
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
            <span className="settings-value">{version}</span>
          </div>
          <div className="settings-row">
            <span className="settings-label">技术栈</span>
            <span className="settings-value">Tauri 2 + Rust + React</span>
          </div>
          <div className="settings-row">
            <span className="settings-label">平台</span>
            <span className="settings-value">{getPlatform()}</span>
          </div>
          <div className="settings-row">
            <span className="settings-label">更新</span>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              {updateStatus === "latest" && (
                <span className="settings-value update-status-message success">
                  已是最新版本
                </span>
              )}
              {updateStatus === "found" && (
                <span className="settings-value update-status-message success">
                  发现新版本
                </span>
              )}
              {updateStatus === "error" && (
                <span className="settings-value update-status-message error" title={errorMessage || updateError || undefined}>
                  {errorMessage || updateError || "更新检查失败"}
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
