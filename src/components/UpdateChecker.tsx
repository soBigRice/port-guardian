import { useState, useEffect, useMemo } from "react";
import { type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { marked } from "marked";
import { formatUpdateError } from "../utils/updateErrors";
import { useTranslation } from "../i18n";

interface Props {
  show: boolean;
  updateInfo: Update | null;
  onAutoCheck: () => Promise<boolean>;
  onClose: () => void;
}

export default function UpdateChecker({ show, updateInfo, onAutoCheck, onClose }: Props) {
  const { t } = useTranslation();
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState(0);
  const [totalSize, setTotalSize] = useState(0);
  const [installed, setInstalled] = useState(false);
  const [downloadError, setDownloadError] = useState<string | null>(null);

  // 启动时静默检查
  useEffect(() => {
    void onAutoCheck().catch((err) => {
      console.log("更新检查失败:", err);
    });
  }, [onAutoCheck]);

  async function handleDownload() {
    if (!updateInfo) return;
    setDownloading(true);
    setProgress(0);
    setTotalSize(0);
    setDownloadError(null);

    try {
      await updateInfo.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            if (event.data.contentLength) {
              setTotalSize(event.data.contentLength);
            }
            break;
          case "Progress":
            setProgress((prev) => prev + event.data.chunkLength);
            break;
          case "Finished":
            break;
        }
      });
      setInstalled(true);
    } catch (err) {
      const message = formatUpdateError(err, t);
      setDownloadError(message);
      console.error("下载更新失败:", err);
    } finally {
      setDownloading(false);
    }
  }

  async function handleRestart() {
    try {
      await relaunch();
    } catch (err) {
      console.error("重启失败:", err);
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return bytes + " B";
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
    return (bytes / (1024 * 1024)).toFixed(1) + " MB";
  }

  const changelogHtml = useMemo(() => {
    if (!updateInfo?.body) return "";
    const raw = marked.parse(updateInfo.body, { async: false });
    return typeof raw === "string" ? raw : "";
  }, [updateInfo?.body]);

  if (!show || !updateInfo) return null;

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog update-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="update-icon">&#8635;</div>
        <h3>{t("updateChecker.title")}</h3>
        <div className="update-version">
          v{updateInfo.version}
        </div>

        {changelogHtml && (
          <div className="update-changelog">
            <h4>{t("updateChecker.changelog")}</h4>
            <div
              className="changelog-content"
              dangerouslySetInnerHTML={{ __html: changelogHtml }}
            />
          </div>
        )}

        {downloading && (
          <div className="update-progress">
            <div className="progress-bar">
              <div
                className="progress-fill"
                style={{
                  width: totalSize > 0 ? `${(progress / totalSize) * 100}%` : "50%",
                }}
              />
            </div>
            <span className="progress-text">
              {totalSize > 0
                ? `${formatBytes(progress)} / ${formatBytes(totalSize)}`
                : t("updateChecker.downloading")}
            </span>
          </div>
        )}

        {downloadError && (
          <div className="update-error" title={downloadError}>
            {downloadError}
          </div>
        )}

        <div className="update-actions">
          {!downloading && !installed && (
            <>
              <button className="btn" onClick={onClose}>
                {t("updateChecker.later")}
              </button>
              <button className="btn btn-primary" onClick={handleDownload}>
                {t("updateChecker.updateNow")}
              </button>
            </>
          )}
          {downloading && (
            <button className="btn" disabled>
              {t("updateChecker.downloadingBtn")}
            </button>
          )}
          {installed && (
            <button className="btn btn-primary" onClick={handleRestart}>
              {t("updateChecker.restartApp")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
