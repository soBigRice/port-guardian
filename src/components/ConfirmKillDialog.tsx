import { useState } from "react";
import { PortService } from "../types";
import { useTranslation } from "../i18n";

interface Props {
  service: PortService;
  onConfirm: (force: boolean) => void;
  onCancel: () => void;
}

export default function ConfirmKillDialog({ service, onConfirm, onCancel }: Props) {
  const { t } = useTranslation();
  const [confirmText, setConfirmText] = useState("");
  const isDanger = service.safety_level === "danger";
  const isCaution = service.safety_level === "caution" || service.safety_level === "unknown";

  const shortCwd = service.cwd
    ? service.cwd.replace(/^\/Users\/[^/]+/, "~")
    : "";

  // 危险服务禁止终止
  if (isDanger) {
    return (
      <div className="dialog-overlay" onClick={onCancel}>
        <div className="dialog" onClick={(e) => e.stopPropagation()}>
          <h3>{t("confirmDialog.danger.title")}</h3>
          <div className="dialog-warning danger">
            {t("confirmDialog.danger.warning", { processName: service.process_name })}
          </div>
          <div className="dialog-info">
            <div className="detail-row">
              <span className="detail-label">{t("confirmDialog.field.port")}</span>
              <span className="detail-value">{service.port}</span>
            </div>
            <div className="detail-row">
              <span className="detail-label">{t("confirmDialog.field.process")}</span>
              <span className="detail-value">{service.process_name}</span>
            </div>
            <div className="detail-row">
              <span className="detail-label">{t("confirmDialog.field.pid")}</span>
              <span className="detail-value">{service.pid}</span>
            </div>
            <div className="detail-row">
              <span className="detail-label">{t("confirmDialog.field.description")}</span>
              <span className="detail-value long">{service.safety_reason}</span>
            </div>
          </div>
          <div className="dialog-actions">
            <button className="btn" onClick={onCancel}>
              {t("common.close")}
            </button>
          </div>
        </div>
      </div>
    );
  }

  // 谨慎服务需要输入端口号确认
  if (isCaution) {
    const canConfirm = confirmText === service.port.toString();
    return (
      <div className="dialog-overlay" onClick={onCancel}>
        <div className="dialog" onClick={(e) => e.stopPropagation()}>
          <h3>{t("confirmDialog.caution.title")}</h3>
          <div className="dialog-warning caution">
            {service.safety_reason}
          </div>
          <div className="dialog-info">
            <div className="detail-row">
              <span className="detail-label">{t("confirmDialog.field.port")}</span>
              <span className="detail-value">{service.port}</span>
            </div>
            <div className="detail-row">
              <span className="detail-label">{t("confirmDialog.field.process")}</span>
              <span className="detail-value">{service.process_name}</span>
            </div>
            <div className="detail-row">
              <span className="detail-label">{t("confirmDialog.field.pid")}</span>
              <span className="detail-value">{service.pid}</span>
            </div>
            {shortCwd && (
              <div className="detail-row">
                <span className="detail-label">{t("confirmDialog.field.directory")}</span>
                <span className="detail-value long">{shortCwd}</span>
              </div>
            )}
          </div>
          <div style={{ marginBottom: 16 }}>
            <label style={{ fontSize: 12, color: "var(--text-dim)", display: "block", marginBottom: 6 }}>
              {t("confirmDialog.caution.confirmLabel", { port: service.port })}
            </label>
            <input
              className="search-input"
              style={{ width: "100%" }}
              value={confirmText}
              onChange={(e) => setConfirmText(e.target.value)}
              placeholder={t("confirmDialog.caution.placeholder", { port: service.port })}
              autoFocus
            />
          </div>
          <div className="dialog-actions">
            <button className="btn" onClick={onCancel}>
              {t("common.cancel")}
            </button>
            <button
              className="btn btn-danger"
              disabled={!canConfirm}
              onClick={() => onConfirm(false)}
            >
              {t("common.terminate")}
            </button>
          </div>
        </div>
      </div>
    );
  }

  // 安全服务简单确认
  return (
    <div className="dialog-overlay" onClick={onCancel}>
      <div className="dialog" onClick={(e) => e.stopPropagation()}>
        <h3>{t("confirmDialog.safe.title")}</h3>
        <div className="dialog-warning safe">
          {service.safety_reason}
        </div>
        <div className="dialog-info">
          <div className="detail-row">
            <span className="detail-label">{t("confirmDialog.field.port")}</span>
            <span className="detail-value">{service.port}</span>
          </div>
          <div className="detail-row">
            <span className="detail-label">{t("confirmDialog.field.service")}</span>
            <span className="detail-value">{service.service_name}</span>
          </div>
          <div className="detail-row">
            <span className="detail-label">{t("confirmDialog.field.process")}</span>
            <span className="detail-value">{service.process_name}</span>
          </div>
          <div className="detail-row">
            <span className="detail-label">{t("confirmDialog.field.pid")}</span>
            <span className="detail-value">{service.pid}</span>
          </div>
          <div className="detail-row">
            <span className="detail-label">{t("confirmDialog.field.command")}</span>
            <span className="detail-value long">{service.command_line}</span>
          </div>
          {shortCwd && (
            <div className="detail-row">
              <span className="detail-label">{t("confirmDialog.field.directory")}</span>
              <span className="detail-value long">{shortCwd}</span>
            </div>
          )}
          {service.source !== "Unknown" && (
            <div className="detail-row">
              <span className="detail-label">{t("confirmDialog.field.source")}</span>
              <span className="detail-value">{service.source}</span>
            </div>
          )}
        </div>
        <div className="dialog-actions">
          <button className="btn" onClick={onCancel}>
            {t("common.cancel")}
          </button>
          <button className="btn btn-safe" onClick={() => onConfirm(false)}>
            {t("common.terminate")}
          </button>
        </div>
      </div>
    </div>
  );
}
