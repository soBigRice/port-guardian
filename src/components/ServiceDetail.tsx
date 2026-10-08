import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { PortService } from "../types";
import SourceIcon from "./SourceIcon";
import { useTranslation } from "../i18n";
import { CaretUpIcon, CheckIcon, CopyIcon, StopIcon } from "./icons";

interface Props {
  service: PortService;
  disabled: boolean;
  terminating: boolean;
  onKill: () => void;
  onClose: () => void;
}

export default function ServiceDetail({service, disabled, terminating, onKill, onClose}: Props) {
  const { t } = useTranslation();
  const [advanced, setAdvanced] = useState(false);
  const [feedback, setFeedback] = useState<string | null>(null);
  const feedbackTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => () => { if (feedbackTimer.current) clearTimeout(feedbackTimer.current); }, []);
  const dangerous = service.safety_level === "danger";
  const advancedId = "process-info-" + service.id;

  const openPath = async (path: string) => {
    try { await invoke("open_directory", {path}); }
    catch (error) { setFeedback(t("serviceDetail.pathFailed") + " " + String(error)); }
  };
  const copyCommand = async () => {
    if (feedbackTimer.current) clearTimeout(feedbackTimer.current);
    try {
      await navigator.clipboard.writeText(service.command_line);
      setFeedback(t("serviceDetail.copied"));
    } catch { setFeedback(t("serviceDetail.copyFailed")); }
    feedbackTimer.current = setTimeout(() => setFeedback(null), 2500);
  };

  return <section className="inline-inspector" aria-label={t("serviceDetail.title", {port: service.port})}>
    <div className="inspector-main">
      <dl className="inspector-facts">
        <dt>{t("portTable.projectDirectory")}</dt>
        <dd>{service.cwd ? <button className="text-path" title={service.cwd} onClick={() => void openPath(service.cwd)}>{service.cwd}</button> : t("portTable.unknownProject")}</dd>
        <dt>{t("serviceDetail.processPid")}</dt><dd className="mono">{service.pid}</dd>
        <dt>{t("serviceDetail.field.protocol")}</dt><dd>{service.protocol}</dd>
        <dt>{t("serviceDetail.field.address")}</dt><dd className="mono">{service.local_address}</dd>
      </dl>
      <div className="inspector-command">
        <dl className="inspector-facts">
          <dt>{t("serviceDetail.launchSource")}</dt><dd className="source-cell"><SourceIcon source={service.source} executablePath={service.executable_path} size={16} />{service.source}</dd>
          <dt>{t("serviceDetail.field.command")}</dt>
          <dd className="command-box"><code>{service.command_line || t("serviceDetail.unavailable")}</code>
            <button className="icon-button" aria-label={t("serviceDetail.copyCommand")} disabled={!service.command_line} onClick={() => void copyCommand()}>{feedback === t("serviceDetail.copied") ? <CheckIcon size={16} /> : <CopyIcon size={16} />}</button>
          </dd>
        </dl>
        {feedback && <p className="inspector-feedback" role="status">{feedback}</p>}
      </div>
      <div className="inspector-actions">
        <button className="stop-process" disabled={disabled || dangerous} title={dangerous ? service.safety_reason : undefined} onClick={onKill}><StopIcon size={17} />{dangerous ? t("common.forbidden") : terminating ? t("app.batchKilling") : t("serviceDetail.terminateProcess")}</button>
        <p>{dangerous ? service.safety_reason : t("serviceDetail.processScope")}</p>
        <div className="inspector-links"><button aria-expanded={advanced} aria-controls={advancedId} onClick={() => setAdvanced(!advanced)}>{t("serviceDetail.moreInfo")}</button><button onClick={onClose} aria-label={t("serviceDetail.collapse")}><CaretUpIcon size={13} />{t("serviceDetail.collapse")}</button></div>
      </div>
    </div>
    {advanced && <div id={advancedId} className="inspector-advanced">
      <dl className="inspector-facts">
        <dt>{t("serviceDetail.field.executable")}</dt><dd>{service.executable_path ? <button className="text-path" onClick={() => void openPath(service.executable_path)}>{service.executable_path}</button> : t("serviceDetail.unavailable")}</dd>
        <dt>{t("serviceDetail.field.state")}</dt><dd>{service.state}</dd>
        <dt>{t("serviceDetail.field.user")}</dt><dd>{service.user || t("serviceDetail.unavailable")}</dd>
        <dt>{t("serviceDetail.field.serviceType")}</dt><dd>{service.service_name || service.service_type}</dd>
        <dt>{t("serviceDetail.field.basis")}</dt><dd>{service.safety_reason}</dd>
      </dl>
      {service.parent_chain.length > 0 && <div><h4>{t("serviceDetail.field.processChain")}</h4><ol className="inspector-process-chain">{service.parent_chain.map((node) => <li key={node.pid} title={node.command_line}><span>{node.name}</span><span className="mono">PID {node.pid}</span>{node.pid === service.pid && <small>{t("serviceDetail.currentProcess")}</small>}</li>)}</ol></div>}
    </div>}
  </section>;
}
