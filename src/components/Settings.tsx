import { Theme } from "../types";

interface Props {
  theme: Theme;
  onThemeChange: (t: Theme) => void;
  onClose: () => void;
}

const VERSION = "0.1.0";

export default function Settings({ theme, onThemeChange, onClose }: Props) {
  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog settings-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="settings-header">
          <h3>设置</h3>
          <span className="version-tag">v{VERSION}</span>
        </div>

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

        <div className="dialog-actions">
          <button className="btn" onClick={onClose}>关闭</button>
        </div>
      </div>
    </div>
  );
}
