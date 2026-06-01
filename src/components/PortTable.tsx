import { invoke } from "@tauri-apps/api/core";
import { PortService } from "../types";
import RiskBadge from "./RiskBadge";
import SourceIcon from "./SourceIcon";

interface Props {
  services: PortService[];
  selected: PortService | null;
  loading: boolean;
  scanTotal: number;
  scannedCount: number;
  hasFilter: boolean;
  onSelect: (s: PortService) => void;
  onKill: (s: PortService) => void;
}

export default function PortTable({ services, selected, loading, scanTotal, scannedCount, hasFilter, onSelect, onKill }: Props) {
  const handleOpenCwd = async (e: React.MouseEvent, cwd: string) => {
    e.stopPropagation();
    try {
      await invoke("open_directory", { path: cwd });
    } catch (err) {
      console.error("打开目录失败:", err);
    }
  };

  const progress = scanTotal > 0 ? Math.round((scannedCount / scanTotal) * 100) : 0;

  if (services.length === 0 && !loading) {
    return (
      <div className="empty-state">
        <span className="icon">&#128269;</span>
        <span>{hasFilter ? "没有匹配的服务" : "未发现监听端口"}</span>
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
        <span className="scan-status">正在扫描端口...</span>
        {scanTotal > 0 && (
          <span className="scan-progress">{scannedCount} / {scanTotal}</span>
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
            {scanTotal > 0 ? `${scannedCount}/${scanTotal}` : "扫描中..."}
          </span>
        </div>
      )}
      <table className="port-table">
        <thead>
          <tr>
            <th>端口</th>
            <th>服务类型</th>
            <th>进程</th>
            <th>PID</th>
            <th>来源</th>
            <th>命令</th>
            <th>目录</th>
            <th>风险</th>
            <th>操作</th>
          </tr>
        </thead>
        <tbody>
          {services.map((s) => (
            <tr
              key={s.id}
              className={selected?.id === s.id ? "selected" : ""}
              onClick={() => onSelect(s)}
            >
              <td className="port-num">{s.port}</td>
              <td>
                <span className="badge badge-service">
                  {s.service_name || s.service_type}
                </span>
              </td>
              <td>{s.process_name}</td>
              <td className="pid-num">{s.pid}</td>
              <td><SourceIcon source={s.source} />{s.source}</td>
              <td className="cmd-text" title={s.command_line}>
                {s.command_line}
              </td>
              <td
                className="cwd-text clickable-path"
                title={s.cwd ? `点击打开: ${s.cwd}` : ""}
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
                    终止
                  </button>
                ) : s.safety_level === "danger" ? (
                  <span style={{ color: "var(--text-muted)", fontSize: 11 }}>
                    禁止
                  </span>
                ) : (
                  <button
                    className="btn"
                    onClick={(e) => {
                      e.stopPropagation();
                      onKill(s);
                    }}
                  >
                    查看
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
