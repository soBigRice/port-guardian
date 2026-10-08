import { Fragment, useEffect, useMemo, useState } from "react";
import type { PortService } from "../types";
import { groupServices, projectPath } from "../utils/scanState";
import RiskBadge from "./RiskBadge";
import SourceIcon from "./SourceIcon";
import ServiceDetail from "./ServiceDetail";
import { useTranslation } from "../i18n";
import { CaretDownIcon, CaretRightIcon, FolderIcon, MoreIcon, RefreshIcon, SearchIcon, StarIcon } from "./icons";

interface Props {
  services: PortService[];
  selected: PortService | null;
  loading: boolean;
  scanTotal: number;
  scannedCount: number;
  hasFilter: boolean;
  scanFailed: boolean;
  showTechnicalColumns: boolean;
  byProject: boolean;
  search: string;
  terminatingPid: number | null;
  actionsDisabled: boolean;
  selectedIds: Set<string>;
  bookmarkedPorts: Set<number>;
  onSelect: (s: PortService) => void;
  onClose: () => void;
  onKill: (s: PortService) => void;
  onToggleSelect: (id: string) => void;
  onToggleSelectAll: () => void;
  onToggleBookmark: (port: number) => void;
}

export default function PortTable({services, selected, loading, scanTotal, scannedCount, hasFilter, scanFailed, showTechnicalColumns, byProject, search, terminatingPid, actionsDisabled, selectedIds, bookmarkedPorts, onSelect, onClose, onKill, onToggleSelect, onToggleSelectAll, onToggleBookmark}: Props) {
  const { t } = useTranslation();
  const groups = useMemo(() => groupServices(services, byProject), [services, byProject]);
  const [collapsed, setCollapsed] = useState(new Set<string>());
  const columnCount = showTechnicalColumns ? 9 : 6;
  const terminable = services.filter((service) => service.can_terminate);

  useEffect(() => {
    // 新查询展开匹配组，避免之前折叠的项目隐藏搜索结果；仍可手动折叠当前结果。
    if (search.trim()) setCollapsed(new Set());
  }, [search]);

  useEffect(() => {
    if (!selected) return;
    const key = projectPath(selected) ? "project:" + projectPath(selected) : "other";
    setCollapsed((prev) => {
      if (!prev.has(key)) return prev;
      const next = new Set(prev); next.delete(key); return next;
    });
    document.getElementById("service-" + selected.id)?.scrollIntoView({block: "nearest"});
  }, [selected?.id, selected?.cwd]);

  const openService = (service: PortService) => selected?.id === service.id ? onClose() : onSelect(service);
  const toggleGroup = (key: string, containsSelected: boolean) => {
    if (containsSelected) onClose();
    setCollapsed((prev) => { const next = new Set(prev); if (next.has(key)) next.delete(key); else next.add(key); return next; });
  };

  if (!services.length) return <div className="workspace-empty">
    {loading ? <RefreshIcon className="spinning" size={28} /> : <SearchIcon size={28} />}
    <p>{loading ? t("portTable.scanningPorts") : hasFilter ? t("portTable.empty.noMatch") : scanFailed ? t("portTable.empty.scanFailed") : t("portTable.empty.noPorts")}</p>
    {loading && scanTotal > 0 && <span>{Math.min(scannedCount, scanTotal)} / {scanTotal}</span>}
  </div>;

  return <div className="service-list" onClick={(event) => { if (event.target === event.currentTarget) onClose(); }}>
    <table className={"service-table " + (showTechnicalColumns ? "technical-view" : "")}>
      <colgroup><col className="check-col" /><col className="port-col" /><col />{showTechnicalColumns && <col className="pid-col" />}<col className="source-col" />{showTechnicalColumns && <><col className="command-col" /><col className="directory-col" /></>}<col className="risk-col" /><col className="more-col" /></colgroup>
      <thead><tr>
        <th className="selection-cell"><input type="checkbox" aria-label={t("portTable.selectAll")} disabled={actionsDisabled || !terminable.length} checked={terminable.length > 0 && terminable.every((service) => selectedIds.has(service.id))} onChange={onToggleSelectAll} /></th>
        <th>{t("portTable.header.port")}</th><th>{t("portTable.processAndService")}</th>{showTechnicalColumns && <th>PID</th>}<th>{t("portTable.header.source")}</th>{showTechnicalColumns && <><th>{t("portTable.header.command")}</th><th>{t("portTable.header.directory")}</th></>}<th>{t("portTable.header.risk")}</th><th><span className="sr-only">{t("portTable.header.actions")}</span></th>
      </tr></thead>
      {groups.map((group) => <tbody key={group.key}>
        {byProject && <tr className="project-heading"><td colSpan={columnCount}><button className="project-toggle" aria-expanded={!collapsed.has(group.key)} onClick={() => toggleGroup(group.key, group.services.some((service) => service.id === selected?.id))}>
          <FolderIcon size={20} weight="duotone" aria-hidden="true" /><strong>{group.path ? group.name : t("portTable.otherGroup")}</strong>{group.path && <span className="project-path" title={group.path}>{group.path.replace(/^\/Users\/[^/]+/, "~")}</span>}<span className="project-count">{t("portTable.groupCount", {count: group.services.length})}</span>{collapsed.has(group.key) ? <CaretRightIcon className="project-caret" size={16} /> : <CaretDownIcon className="project-caret" size={16} />}
        </button></td></tr>}
        {(!byProject || !collapsed.has(group.key)) && group.services.map((service) => <Fragment key={service.id}>
          <tr id={"service-" + service.id} className={"service-row " + (selected?.id === service.id ? "is-selected " : "") + (selectedIds.has(service.id) ? "is-checked" : "")} onClick={() => openService(service)}>
            <td className="selection-cell" onClick={(event) => event.stopPropagation()}>{service.can_terminate && <input type="checkbox" aria-label={t("portTable.selectPort", {port: service.port, pid: service.pid})} disabled={actionsDisabled} checked={selectedIds.has(service.id)} onChange={() => onToggleSelect(service.id)} />}</td>
            <td><div className="port-identity"><button className={"bookmark-button " + (bookmarkedPorts.has(service.port) ? "is-bookmarked" : "")} aria-label={t(bookmarkedPorts.has(service.port) ? "portTable.removeBookmark" : "portTable.bookmark", {port: service.port})} aria-pressed={bookmarkedPorts.has(service.port)} onClick={(event) => {event.stopPropagation(); onToggleBookmark(service.port);}}><StarIcon size={17} weight={bookmarkedPorts.has(service.port) ? "fill" : "regular"} /></button><button className="port-number" aria-label={t("serviceDetail.title", {port: service.port})} aria-expanded={selected?.id === service.id} onClick={(event) => {event.stopPropagation(); openService(service);}}>{service.port}</button><span className={"protocol-badge " + service.protocol.toLowerCase()}>{service.protocol}</span></div></td>
            <td><div className="process-name" title={service.process_name + " " + service.service_name}>{service.process_name}{service.service_name && service.service_name !== service.process_name && <span> ({service.service_name})</span>}</div>{!showTechnicalColumns && <span className="process-pid">PID {service.pid}</span>}</td>
            {showTechnicalColumns && <td className="mono">{service.pid}</td>}
            <td><span className="source-cell"><SourceIcon source={service.source} executablePath={service.executable_path} size={16} />{service.source}</span></td>
            {showTechnicalColumns && <><td className="technical-cell" title={service.command_line}>{service.command_line}</td><td className="technical-cell" title={service.cwd}>{service.cwd || t("portTable.unknownProject")}</td></>}
            <td><RiskBadge level={service.safety_level} /></td><td><button className="icon-button row-more" aria-label={t("portTable.viewDetails", {port: service.port})} aria-expanded={selected?.id === service.id} onClick={(event) => {event.stopPropagation(); openService(service);}}><MoreIcon size={20} weight="bold" /></button></td>
          </tr>
          {selected?.id === service.id && <tr className="inspector-row"><td colSpan={columnCount}><ServiceDetail service={service} disabled={actionsDisabled} terminating={terminatingPid === service.pid} onClose={onClose} onKill={() => onKill(service)} /></td></tr>}
        </Fragment>)}
      </tbody>)}
    </table>
  </div>;
}
