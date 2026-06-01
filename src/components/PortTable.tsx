import { PortService } from "../types";
import RiskBadge from "./RiskBadge";

interface Props {
  services: PortService[];
  selected: PortService | null;
  loading: boolean;
  hasFilter: boolean;
  onSelect: (s: PortService) => void;
  onKill: (s: PortService) => void;
}

export default function PortTable({ services, selected, loading, hasFilter, onSelect, onKill }: Props) {
  if (services.length === 0) {
    return (
      <div className="empty-state">
        <span className="icon">{loading ? "&#9881;" : "&#128269;"}</span>
        <span>{loading ? "正在扫描端口..." : hasFilter ? "没有匹配的服务" : "未发现监听端口"}</span>
      </div>
    );
  }

  return (
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
            <td>{s.source}</td>
            <td className="cmd-text" title={s.command_line}>
              {s.command_line}
            </td>
            <td className="cwd-text" title={s.cwd}>
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
  );
}
