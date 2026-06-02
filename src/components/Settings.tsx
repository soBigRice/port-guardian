import { useState } from "react";
import { Theme } from "../types";
import { useTranslation, type Language } from "../i18n";

const PROJECT_URL = "https://github.com/soBigRice/port-guardian";

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
  const { t, language, setLanguage } = useTranslation();
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
                <span className="traffic-light red" onClick={onClose} title={t("common.close")}>&times;</span>
              </div>
              <span className="settings-titlebar-text">{t("settings.title")}</span>
              <span className="version-tag">v{version}</span>
            </>
          ) : (
            <>
              <span className="settings-titlebar-text" style={{ flex: 1 }}>{t("settings.title")}</span>
              <span className="version-tag">v{version}</span>
              <button className="settings-close-btn" onClick={onClose} title={t("common.close")}>&times;</button>
            </>
          )}
        </div>

        <div className="settings-content">
        <div className="settings-section">
          <h4>{t("settings.appearance")}</h4>
          <div className="settings-row">
            <span className="settings-label">{t("settings.colorTheme")}</span>
            <div className="theme-options">
              {(["dark", "light", "auto"] as Theme[]).map((th) => (
                <button
                  key={th}
                  className={`theme-btn ${theme === th ? "active" : ""}`}
                  onClick={() => onThemeChange(th)}
                >
                  {th === "dark" ? t("settings.theme.dark") : th === "light" ? t("settings.theme.light") : t("settings.theme.auto")}
                </button>
              ))}
            </div>
          </div>
          <div className="settings-row">
            <span className="settings-label">{t("settings.language")}</span>
            <div className="theme-options">
              {(["zh", "en"] as Language[]).map((lang) => (
                <button
                  key={lang}
                  className={`theme-btn ${language === lang ? "active" : ""}`}
                  onClick={() => setLanguage(lang)}
                >
                  {lang === "zh" ? t("settings.languageZh") : t("settings.languageEn")}
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className="settings-section">
          <h4>{t("settings.about")}</h4>
          <div className="settings-row">
            <span className="settings-label">{t("settings.appName")}</span>
            <span className="settings-value">Port Guardian</span>
          </div>
          <div className="settings-row">
            <span className="settings-label">{t("settings.version")}</span>
            <span className="settings-value">{version}</span>
          </div>
          <div className="settings-row">
            <span className="settings-label">{t("settings.techStack")}</span>
            <span className="settings-value">Tauri 2 + Rust + React</span>
          </div>
          <div className="settings-row">
            <span className="settings-label">{t("settings.platform")}</span>
            <span className="settings-value">{getPlatform()}</span>
          </div>
          <div className="settings-row">
            <span className="settings-label">{t("settings.projectUrl")}</span>
            <a
              className="settings-link"
              href={PROJECT_URL}
              target="_blank"
              rel="noreferrer"
              title={PROJECT_URL}
            >
              {PROJECT_URL}
            </a>
          </div>
          <div className="settings-row">
            <span className="settings-label">{t("settings.update")}</span>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              {updateStatus === "latest" && (
                <span className="settings-value update-status-message success">
                  {t("settings.updateStatus.latest")}
                </span>
              )}
              {updateStatus === "found" && (
                <span className="settings-value update-status-message success">
                  {t("settings.updateStatus.found")}
                </span>
              )}
              {updateStatus === "error" && (
                <span className="settings-value update-status-message error" title={errorMessage || updateError || undefined}>
                  {errorMessage || updateError || t("settings.updateStatus.checkFailed")}
                </span>
              )}
              <button
                className="btn btn-refresh"
                onClick={handleCheck}
                disabled={checking}
                style={{ padding: "4px 12px", fontSize: 12 }}
              >
                {checking ? t("settings.updateChecking") : t("settings.updateCheckButton")}
              </button>
            </div>
          </div>
        </div>

        <div className="settings-section">
          <h4>{t("settings.features")}</h4>
          <div className="settings-note">
            <p>{t("settings.featureDesc")}</p>
            <ul>
              <li><strong>{t("settings.feature.safe")}</strong>{t("settings.feature.safeDesc")}</li>
              <li><strong>{t("settings.feature.caution")}</strong>{t("settings.feature.cautionDesc")}</li>
              <li><strong>{t("settings.feature.danger")}</strong>{t("settings.feature.dangerDesc")}</li>
              <li><strong>{t("settings.feature.unknown")}</strong>{t("settings.feature.unknownDesc")}</li>
            </ul>
          </div>
        </div>
        </div>

      </div>
    </div>
  );
}
