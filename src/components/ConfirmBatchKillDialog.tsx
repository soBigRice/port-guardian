import { useState } from "react";
import type { PortService } from "../types";
import { useTranslation } from "../i18n";

interface Props {
  services: PortService[];
  onConfirm: (force: boolean) => void;
  onCancel: () => void;
}

export default function ConfirmBatchKillDialog({services, onConfirm, onCancel}: Props) {
  const {t} = useTranslation();
  const [force, setForce] = useState(false);
  const [confirmation, setConfirmation] = useState("");
  const cautionPorts = [...new Set(services.filter((s) => s.safety_level !== "safe").map((s) => s.port))].sort((a, b) => a - b).join(",");
  const canConfirm = services.length > 0 && (!cautionPorts || confirmation.replace(/\s/g, "") === cautionPorts);
  return <div className="dialog-overlay" onClick={onCancel}>
    <div className="dialog" role="dialog" aria-modal="true" aria-labelledby="batch-confirm-title" onClick={(event) => event.stopPropagation()}>
      <h3 id="batch-confirm-title">{t("confirmDialog.batch.title", {count: services.length})}</h3>
      <p className="dialog-warning caution">{t("confirmDialog.processScope")}</p>
      <ul className="batch-targets">
        {services.map((service) => <li key={service.pid}><strong>{service.port} · {service.process_name}</strong><span>PID {service.pid}</span></li>)}
      </ul>
      {cautionPorts && <label className="batch-confirm-input">{t("confirmDialog.batch.confirm", {ports: cautionPorts})}
        <input className="search-input" value={confirmation} onChange={(event) => setConfirmation(event.target.value)} placeholder={cautionPorts} autoFocus />
      </label>}
      <label className="termination-mode"><input type="checkbox" checked={force} onChange={(event) => setForce(event.target.checked)} />{t("confirmDialog.force")}</label>
      <div className="dialog-actions">
        <button className="btn" onClick={onCancel}>{t("common.cancel")}</button>
        <button className="btn btn-danger" disabled={!canConfirm} onClick={() => onConfirm(force)}>{t("common.terminate")}</button>
      </div>
    </div>
  </div>;
}
