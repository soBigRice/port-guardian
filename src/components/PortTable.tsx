import { invoke } from "@tauri-apps/api/core";
import { PortService } from "../types";
import RiskBadge from "./RiskBadge";
import SourceIcon from "./SourceIcon";
import { useTranslation } from "../i18n";

interface Props {
  services: PortService[];
  selected: PortService | null;
  loading: boolean;
  scanTotal: number;
  scannedCount: number;
  hasFilter: boolean;
  selectedIds: Set<string>;
  bookmarkedPorts: Set<number>;
  onSelect: (s: PortService) => void;
  onKill: (s: PortService) => void;
  onToggleSelect: (id: string) => void;
  onToggleSelectAll: () => void;
  onToggleBookmark: (port: number) => void;
}

export default function PortTable({ services, selected, loading, scanTotal, scannedCount, hasFilter, selectedIds, bookmarkedPorts, onSelect, onKill, onToggleSelect, onToggleSelectAll, onToggleBookmark }: Props) {
  const { t } = useTranslation();

  const handleOpenCwd = async (e: React.MouseEvent, cwd: string) => {
    e.stopPropagation();
    try {
      await invoke("open_directory", { path: cwd });
    } catch (err) {
      console.error("打开目录失败:", err);
    }
  };

  const displayedScannedCount = scanTotal > 0 ? Math.min(scannedCount, scanTotal) : scannedCount;
  const progress = scanTotal > 0 ? Math.round((displayedScannedCount / scanTotal) * 100) : 0;

  if (services.length === 0 && !loading) {
    return (
      <div className="empty-state">
        <span className="icon">&#128269;</span>
        <span>{hasFilter ? t("portTable.empty.noMatch") : t("portTable.empty.noPorts")}</span>
      </div>
    );
  }

  if (services.length === 0 && loading) {
    return (
      <div className="empty-state">
        <div className="scan-radar">
          <div className="scan-radar-ring" />
          <div className="scan-radar-ring" />
          <div className="scan-radar-ring" />
          <div className="scan-radar-dot" />
        </div>
        <span className="scan-status">{t("portTable.scanningPorts")}</span>
        {scanTotal > 0 && (
          <span className="scan-progress">{displayedScannedCount} / {scanTotal}</span>
        )}
      </div>
    );
  }

  return (
    <div className="table-wrapper">
      {loading && (
        <div className="scanning-overlay">
          <div className="scanning-progress-bar">
            <div
              className="scanning-progress-fill"
              style={{ width: scanTotal > 0 ? `${progress}%` : "100%" }}
            />
          </div>
          <span className="scanning-badge">
            {scanTotal > 0 ? `${displayedScannedCount}/${scanTotal}` : t("common.scanning")}
          </span>
        </div>
      )}
      <table className="port-table">
        <thead>
          <tr>
            <th className="col-check">
              <input
                type="checkbox"
                className="row-check"
                checked={services.filter((s) => s.can_terminate).length > 0 && services.filter((s) => s.can_terminate).every((s) => selectedIds.has(s.id))}
                onChange={onToggleSelectAll}
              />
            </th>
            <th>{t("portTable.header.port")}</th>
            <th>{t("portTable.header.serviceType")}</th>
            <th>{t("portTable.header.process")}</th>
            <th>{t("portTable.header.pid")}</th>
            <th>{t("portTable.header.source")}</th>
            <th>{t("portTable.header.command")}</th>
            <th>{t("portTable.header.directory")}</th>
            <th>{t("portTable.header.risk")}</th>
            <th>{t("portTable.header.actions")}</th>
          </tr>
        </thead>
        <tbody>
          {services.map((s) => (
            <tr
              key={s.id}
              className={`${selected?.id === s.id ? "selected" : ""} ${selectedIds.has(s.id) ? "checked" : ""}`}
              onClick={() => onSelect(s)}
            >
              <td className="col-check" onClick={(e) => e.stopPropagation()}>
                {s.can_terminate && (
                  <input
                    type="checkbox"
                    className="row-check"
                    checked={selectedIds.has(s.id)}
                    onChange={() => onToggleSelect(s.id)}
                  />
                )}
              </td>
              <td className="port-num">
                <span
                  className={`bookmark-star ${bookmarkedPorts.has(s.port) ? "active" : ""}`}
                  onClick={(e) => { e.stopPropagation(); onToggleBookmark(s.port); }}
                  title={bookmarkedPorts.has(s.port) ? "取消收藏" : "收藏"}
                >
                  {bookmarkedPorts.has(s.port) ? "★" : "☆"}
                </span>
                {s.port}
                {s.protocol === "UDP" && <span className="badge badge-udp">UDP</span>}
              </td>
              <td>
                <span className="badge badge-service">
                  {s.service_name || s.service_type}
                </span>
              </td>
              <td>{s.process_name}</td>
              <td className="pid-num">{s.pid}</td>
              <td><SourceIcon source={s.source} executablePath={s.executable_path} />{s.source}</td>
              <td className="cmd-text" title={s.command_line}>
                {s.command_line}
              </td>
              <td
                className="cwd-text clickable-path"
                title={s.cwd ? `${t("portTable.cwdTooltip")} ${s.cwd}` : ""}
                onClick={s.cwd ? (e) => handleOpenCwd(e, s.cwd) : undefined}
              >
                {s.cwd ? s.cwd.replace(/^\/Users\/[^/]+/, "~") : ""}
              </td>
              <td>
                <RiskBadge level={s.safety_level} />
              </td>
              <td>
                {s.can_terminate ? (
                  <button
                    className="btn btn-danger"
                    onClick={(e) => {
                      e.stopPropagation();
                      onKill(s);
                    }}
                  >
                    {t("common.terminate")}
                  </button>
                ) : s.safety_level === "danger" ? (
                  <span style={{ color: "var(--text-muted)", fontSize: 11 }}>
                    {t("common.forbidden")}
                  </span>
                ) : (
                  <button
                    className="btn"
                    onClick={(e) => {
                      e.stopPropagation();
                      onKill(s);
                    }}
                  >
                    {t("common.view")}
                  </button>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
