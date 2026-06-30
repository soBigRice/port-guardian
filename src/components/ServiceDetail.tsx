import { invoke } from "@tauri-apps/api/core";
import { PortService } from "../types";
import RiskBadge from "./RiskBadge";
import SourceIcon from "./SourceIcon";
import { useTranslation } from "../i18n";

interface Props {
  service: PortService;
  onKill: () => void;
  onClose: () => void;
}

export default function ServiceDetail({ service, onKill, onClose }: Props) {
  const { t } = useTranslation();

  const shortCwd = service.cwd
    ? service.cwd.replace(/^\/Users\/[^/]+/, "~")
    : "";

  const handleOpenPath = async (path: string) => {
    try {
      await invoke("open_directory", { path });
    } catch (e) {
      console.error("打开目录失败:", e);
    }
  };

  return (
    <div className="detail-panel">
      <div className="detail-header">
        <h3>{t("serviceDetail.title", { port: service.port })}</h3>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <RiskBadge level={service.safety_level} />
          <button className="btn btn-icon detail-close" onClick={onClose} title={t("common.close")}>
            &#10005;
          </button>
        </div>
      </div>

      <div className="detail-section">
        <h4>{t("serviceDetail.section.basicInfo")}</h4>
        <div className="detail-row">
          <span className="detail-label">{t("serviceDetail.field.port")}</span>
          <span className="detail-value">{service.port}</span>
        </div>
        <div className="detail-row">
          <span className="detail-label">{t("serviceDetail.field.protocol")}</span>
          <span className="detail-value">{service.protocol}</span>
        </div>
        <div className="detail-row">
          <span className="detail-label">{t("serviceDetail.field.address")}</span>
          <span className="detail-value">{service.local_address}</span>
        </div>
        <div className="detail-row">
          <span className="detail-label">{t("serviceDetail.field.state")}</span>
          <span className="detail-value">{service.state}</span>
        </div>
        <div className="detail-row">
          <span className="detail-label">{t("serviceDetail.field.pid")}</span>
          <span className="detail-value">{service.pid}</span>
        </div>
        <div className="detail-row">
          <span className="detail-label">{t("serviceDetail.field.user")}</span>
          <span className="detail-value">{service.user}</span>
        </div>
      </div>

      <div className="detail-section">
        <h4>{t("serviceDetail.section.processInfo")}</h4>
        <div className="detail-row">
          <span className="detail-label">{t("serviceDetail.field.processName")}</span>
          <span className="detail-value">{service.process_name}</span>
        </div>
        {service.executable_path && (
          <div className="detail-row">
            <span className="detail-label">{t("serviceDetail.field.executable")}</span>
            <span
              className="detail-value long clickable-path"
              title={t("serviceDetail.openDirTitle")}
              onClick={() => handleOpenPath(service.executable_path)}
            >
              {service.executable_path}
            </span>
          </div>
        )}
        <div className="detail-row">
          <span className="detail-label">{t("serviceDetail.field.command")}</span>
          <span className="detail-value long">{service.command_line}</span>
        </div>
        {shortCwd && (
          <div className="detail-row">
            <span className="detail-label">{t("serviceDetail.field.workDir")}</span>
            <span
              className="detail-value long clickable-path"
              title={t("serviceDetail.openDirTitle2")}
              onClick={() => handleOpenPath(service.cwd)}
            >
              {shortCwd}
            </span>
          </div>
        )}
      </div>

      <div className="detail-section">
        <h4>{t("serviceDetail.section.sourceInfo")}</h4>
        <div className="detail-row">
          <span className="detail-label">{t("serviceDetail.field.source")}</span>
          <span className="detail-value">
            <SourceIcon source={service.source} size={16} />{service.source}
          </span>
        </div>
        {service.parent_chain.length > 0 && (
          <div style={{ marginTop: 6 }}>
            <span className="detail-label">{t("serviceDetail.field.processChain")}</span>
            <div className="process-tree">
              {service.parent_chain.map((node, i) => {
                const isLast = i === service.parent_chain.length - 1;
                return (
                  <div key={node.pid} className="tree-node" style={{ paddingLeft: i * 16 }}>
                    <span className="tree-connector">{i === 0 ? "●" : isLast ? "└─" : "├─"}</span>
                    <span className="tree-name" title={node.command_line}>
                      {node.name}
                    </span>
                    <span className="tree-pid">({node.pid})</span>
                  </div>
                );
              })}
              {/* 当前进程 */}
              <div className="tree-node current" style={{ paddingLeft: service.parent_chain.length * 16 }}>
                <span className="tree-connector">└─</span>
                <span className="tree-name">{service.process_name}</span>
                <span className="tree-pid">({service.pid})</span>
                <span className="tree-current-badge">← {t("serviceDetail.currentProcess")}</span>
              </div>
            </div>
          </div>
        )}
      </div>

      <div className="detail-section">
        <h4>{t("serviceDetail.section.security")}</h4>
        <div className="detail-row">
          <span className="detail-label">{t("serviceDetail.field.serviceType")}</span>
          <span className="detail-value">
            <span className="badge badge-service">
              {service.service_name || service.service_type}
            </span>
          </span>
        </div>
        <div className="detail-row">
          <span className="detail-label">{t("serviceDetail.field.riskLevel")}</span>
          <span className="detail-value">
            <RiskBadge level={service.safety_level} />
          </span>
        </div>
        <div className="detail-row">
          <span className="detail-label">{t("serviceDetail.field.basis")}</span>
          <span className="detail-value long" style={{ color: "var(--text-dim)" }}>
            {service.safety_reason}
          </span>
        </div>
      </div>

      {service.can_terminate && (
        <div style={{ marginTop: 16 }}>
          <button className="btn btn-danger" onClick={onKill} style={{ width: "100%" }}>
            {t("serviceDetail.terminateService")}
          </button>
        </div>
      )}
    </div>
  );
}
