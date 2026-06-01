import { useState, useEffect } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

interface Props {
  show: boolean;
  updateInfo: Update | null;
  onUpdateFound: (update: Update) => void;
  onClose: () => void;
}

export default function UpdateChecker({ show, updateInfo, onUpdateFound, onClose }: Props) {
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState(0);
  const [totalSize, setTotalSize] = useState(0);
  const [installed, setInstalled] = useState(false);

  // 启动时静默检查
  useEffect(() => {
    (async () => {
      try {
        const update = await check();
        if (update) {
          onUpdateFound(update);
        }
      } catch (err) {
        console.log("更新检查失败:", err);
      }
    })();
  }, []);

  async function handleDownload() {
    if (!updateInfo) return;
    setDownloading(true);
    setProgress(0);

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
      console.error("下载更新失败:", err);
      alert(`更新下载失败: ${err}`);
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

  if (!show || !updateInfo) return null;

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog update-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="update-icon">&#8635;</div>
        <h3>发现新版本</h3>
        <div className="update-version">
          v{updateInfo.version}
        </div>

        {updateInfo.body && (
          <div className="update-changelog">
            <h4>更新内容</h4>
            <p>{updateInfo.body}</p>
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
                : "正在下载..."}
            </span>
          </div>
        )}

        <div className="update-actions">
          {!downloading && !installed && (
            <>
              <button className="btn" onClick={onClose}>
                稍后再说
              </button>
              <button className="btn btn-primary" onClick={handleDownload}>
                立即更新
              </button>
            </>
          )}
          {downloading && (
            <button className="btn" disabled>
              下载中...
            </button>
          )}
          {installed && (
            <button className="btn btn-primary" onClick={handleRestart}>
              重启应用
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
