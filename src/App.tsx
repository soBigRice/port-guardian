import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { PortService, Theme, TerminateResult } from "./types";
import { usePortScan, isTauriRuntime } from "./hooks/usePortScan";
import { groupServices, matchesSearch, sortServices, uniqueProcesses, type SortKey } from "./utils/scanState";
import ConfirmBatchKillDialog from "./components/ConfirmBatchKillDialog";
import PortTable from "./components/PortTable";
import ConfirmKillDialog from "./components/ConfirmKillDialog";
import SearchBar from "./components/SearchBar";
import Settings from "./components/Settings";
import UpdateChecker from "./components/UpdateChecker";
import { formatUpdateError } from "./utils/updateErrors";
import { useTranslation } from "./i18n";
import { CircleIcon, CloseIcon, ExportIcon, FilterIcon, ListIcon, RefreshIcon, SettingsIcon, StopIcon } from "./components/icons";

const FALLBACK_VERSION = "0.1.0";

type FilterKey =
  | "all"
  | "safe"
  | "caution"
  | "danger"
  | "dev-service"
  | "web-server"
  | "database-service"
  | "infra-service"
  | "docker-service"
  | "system-service"
  | "app-service";


function matchesFilter(service: PortService, filter: FilterKey) {
  switch (filter) {
    case "all":
      return true;
    case "safe":
      return service.safety_level === "safe";
    case "caution":
      return service.safety_level === "caution" || service.safety_level === "unknown";
    case "danger":
      return service.safety_level === "danger";
    case "dev-service":
      return service.service_type === "dev-service" || service.service_type === "ai-dev-service";
    default:
      return service.service_type === filter;
  }
}

function getInitialTheme(): Theme {
  const saved = localStorage.getItem("pg-theme") as Theme;
  if (saved && ["dark", "light", "auto"].includes(saved)) return saved;
  return "light";
}

function applyTheme(theme: Theme) {
  const root = document.documentElement;
  if (theme === "auto") {
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    root.setAttribute("data-theme", prefersDark ? "dark" : "light");
  } else {
    root.setAttribute("data-theme", theme);
  }
}

function App() {
  const { t } = useTranslation();

  const FILTER_OPTIONS: { key: FilterKey; label: string }[] = useMemo(() => [
    { key: "all", label: t("app.filter.all") },
    { key: "safe", label: t("app.filter.safe") },
    { key: "caution", label: t("app.filter.caution") },
    { key: "danger", label: t("app.filter.danger") },
    { key: "dev-service", label: t("app.filter.devService") },
    { key: "web-server", label: t("app.filter.webServer") },
    { key: "database-service", label: t("app.filter.database") },
    { key: "infra-service", label: t("app.filter.infra") },
    { key: "docker-service", label: t("app.filter.docker") },
    { key: "system-service", label: t("app.filter.system") },
    { key: "app-service", label: t("app.filter.app") },
  ], [t]);
  const { services, loading, scanning, scanTotal, scannedCount, lastRefresh, durationMs, issue, refresh, removeProcess } = usePortScan();
  const [selected, setSelected] = useState<PortService | null>(null);
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<FilterKey>("all");
  const [killTarget, setKillTarget] = useState<PortService | null>(null);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [batchKilling, setBatchKilling] = useState(false);
  const [bookmarkedPorts, setBookmarkedPorts] = useState<Set<number>>(() => {
    try {
      const saved = localStorage.getItem("pg-bookmarks");
      return saved ? new Set(JSON.parse(saved)) : new Set();
    } catch { return new Set(); }
  });
  const [showSettings, setShowSettings] = useState(false);
  const [theme, setTheme] = useState<Theme>(getInitialTheme);
  const [appVersion, setAppVersion] = useState(FALLBACK_VERSION);
  const [updateInfo, setUpdateInfo] = useState<Update | null>(null);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const [showUpdate, setShowUpdate] = useState(false);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [protocolFilter, setProtocolFilter] = useState("all");
  const [sort, setSort] = useState<SortKey>("port");
  const [showTechnicalColumns, setShowTechnicalColumns] = useState(false);
  const [byProject, setByProject] = useState(true);
  const [showBatchConfirm, setShowBatchConfirm] = useState(false);
  const [terminatingPid, setTerminatingPid] = useState<number | null>(null);
  const [actionNotice, setActionNotice] = useState<string | null>(null);
  const operationRef = useRef(false);

  // 持久化收藏端口
  useEffect(() => {
    localStorage.setItem("pg-bookmarks", JSON.stringify([...bookmarkedPorts]));
  }, [bookmarkedPorts]);

  // 应用主题
  useEffect(() => {
    applyTheme(theme);
    localStorage.setItem("pg-theme", theme);

    // 监听系统主题变化（auto 模式）
    if (theme === "auto") {
      const mq = window.matchMedia("(prefers-color-scheme: dark)");
      const handler = () => applyTheme("auto");
      mq.addEventListener("change", handler);
      return () => mq.removeEventListener("change", handler);
    }
  }, [theme]);

  useEffect(() => {
    const dismissMenus = (event: PointerEvent) => {
      const targetMenu = event.target instanceof Element ? event.target.closest(".workspace-menu") : null;
      document.querySelectorAll<HTMLDetailsElement>(".workspace-menu[open]").forEach((menu) => {
        if (menu !== targetMenu) menu.open = false;
      });
    };
    document.addEventListener("pointerdown", dismissMenus);
    return () => document.removeEventListener("pointerdown", dismissMenus);
  }, []);

  // 从 Tauri 读取真实应用版本，避免界面版本号写死
  useEffect(() => {
    if (!isTauriRuntime()) return;
    (async () => {
      try {
        const runtimeVersion = await getVersion();
        setAppVersion(runtimeVersion);
      } catch (err) {
        console.warn("读取应用版本失败，使用默认版本号:", err);
      }
    })();
  }, []);

  // 键盘快捷键
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement).tagName;
      const isInput = ["INPUT", "TEXTAREA", "SELECT"].includes(tag) || (e.target as HTMLElement).isContentEditable;

      // Escape: 逐层关闭面板
      if (e.key === "Escape") {
        if (showBatchConfirm) setShowBatchConfirm(false);
        else if (killTarget) setKillTarget(null);
        else if (showSettings) setShowSettings(false);
        else if (document.querySelector(".workspace-menu[open]")) document.querySelectorAll<HTMLDetailsElement>(".workspace-menu[open]").forEach((menu) => { menu.open = false; });
        else if (selected) setSelected(null);
        else if (isInput) (e.target as HTMLElement).blur();
        return;
      }

      // 弹窗接管键盘；不让方向键和终止快捷键操作背后的列表。
      if (isInput || killTarget || showBatchConfirm || showSettings || document.querySelector(".workspace-menu[open]")) return;

      // R / F5 → 刷新
      if (e.key === "r" || e.key === "F5") {
        e.preventDefault();
        refresh();
        return;
      }

      // / 或 Ctrl+K / Cmd+K → 聚焦搜索
      if (e.key === "/" || ((e.ctrlKey || e.metaKey) && e.key === "k")) {
        e.preventDefault();
        searchInputRef.current?.focus();
        return;
      }

      // ↑ / ↓ → 切换选中行
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        const visible = groupServices(sortServices(services.filter((s) => matchesSearch(s, search) && matchesFilter(s, filter) && (protocolFilter === "all" || s.protocol === protocolFilter)), bookmarkedPorts, sort), byProject).flatMap((group) => group.services);
        if (visible.length === 0) return;

        const idx = selected ? visible.findIndex((s) => s.id === selected.id) : -1;
        let nextIdx: number;
        if (e.key === "ArrowDown") {
          nextIdx = idx < visible.length - 1 ? idx + 1 : 0;
        } else {
          nextIdx = idx > 0 ? idx - 1 : visible.length - 1;
        }
        setSelected(visible[nextIdx]);
        return;
      }

      // K / Delete → 终止选中服务
      if ((e.key === "k" || e.key === "Delete") && selected && !operationRef.current) {
        e.preventDefault();
        setKillTarget(selected);
        return;
      }

      // 数字键 1-9 → 切换筛选器
      if (e.key >= "1" && e.key <= "9") {
        const idx = parseInt(e.key) - 1;
        if (idx < FILTER_OPTIONS.length) {
          setFilter(FILTER_OPTIONS[idx].key);
        }
        return;
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [selected, killTarget, showSettings, showBatchConfirm, search, filter, refresh, protocolFilter, bookmarkedPorts, sort, byProject, FILTER_OPTIONS]);

  // 切换收藏
  const toggleBookmark = (port: number) => {
    setBookmarkedPorts((prev) => {
      const next = new Set(prev);
      if (next.has(port)) next.delete(port);
      else next.add(port);
      return next;
    });
  };

  const filtered = useMemo(() => sortServices(
    services.filter((service) => matchesSearch(service, search) && matchesFilter(service, filter)
      && (protocolFilter === "all" || service.protocol === protocolFilter)), bookmarkedPorts, sort,
  ), [services, search, filter, protocolFilter, bookmarkedPorts, sort]);

  useEffect(() => {
    setSelected((current) => current ? filtered.find((service) => service.id === current.id) ?? null : null);
    setKillTarget((current) => current ? services.find((service) => service.id === current.id) ?? null : null);
    setSelectedIds((prev) => {
      const valid = new Set(services.filter((service) => service.can_terminate).map((service) => service.id));
      const next = new Set([...prev].filter((id) => valid.has(id)));
      return next.size === prev.size ? prev : next;
    });
  }, [filtered, services]);

  const handleSearchChange = (value: string) => setSearch(value);
  const handleFilterChange = (nextFilter: FilterKey) => setFilter(nextFilter);
  const terminateService = (service: PortService, force: boolean) => invoke<TerminateResult>("terminate_process", {
    pid: service.pid, force, port: service.port, protocol: service.protocol,
    expectedExecutablePath: service.executable_path, expectedCommandLine: service.command_line,
  });
  const handleKill = async (service: PortService, force: boolean) => {
    if (operationRef.current) return;
    operationRef.current = true;
    setKillTarget(null);
    setTerminatingPid(service.pid);
    setActionNotice(null);
    try {
      const result = await terminateService(service, force);
      if (result.success) {
        removeProcess(service.pid);
        setActionNotice(t(result.port_released ? "app.processExited" : "app.portStillOccupied", {pid: service.pid, port: service.port}));
      } else {
        setActionNotice(`${t("app.terminateFailed")} ${result.message}`);
        void refresh(true);
      }
    } catch (error) {
      setActionNotice(`${t("app.terminateFailed")} ${String(error)}`);
      void refresh(true);
    } finally {
      operationRef.current = false;
      setTerminatingPid(null);
    }
  };

  // 多选切换
  const toggleSelect = (id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  // 全选/取消全选（仅限当前可见列表中 can_terminate 的服务）
  const toggleSelectAll = () => {
    const terminable = filtered.filter((s) => s.can_terminate);
    const allSelected = terminable.every((s) => selectedIds.has(s.id));
    if (allSelected) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(terminable.map((s) => s.id)));
    }
  };

  const batchTargets = uniqueProcesses(services.filter((service) => selectedIds.has(service.id) && service.can_terminate));
  const handleBatchKill = async (force: boolean) => {
    if (operationRef.current) return;
    const targets = batchTargets;
    setShowBatchConfirm(false);
    if (!targets.length) return;
    operationRef.current = true;
    setBatchKilling(true);
    setActionNotice(null);
    let successCount = 0;
    const failures: string[] = [];
    for (const service of targets) {
      setTerminatingPid(service.pid);
      try {
        const result = await terminateService(service, force);
        if (result.success) { successCount++; removeProcess(service.pid); }
        else failures.push(`${service.port}: ${result.message}`);
      } catch (error) { failures.push(`${service.port}: ${String(error)}`); }
    }
    setSelectedIds(new Set());
    setBatchKilling(false);
    setTerminatingPid(null);
    operationRef.current = false;
    setActionNotice(`${t("app.batchResult", {success: successCount, failed: failures.length})}${failures.length ? ` · ${failures.join("; ")}` : ""}`);
    void refresh(true, true);
  };

  // 导出端口列表
  const handleExport = (format: "csv" | "json") => {
    document.querySelectorAll<HTMLDetailsElement>(".workspace-menu[open]").forEach((menu) => { menu.open = false; });
    const data = services.map((s) => ({
      port: s.port,
      protocol: s.protocol,
      process: s.process_name,
      pid: s.pid,
      service: s.service_name || s.service_type,
      safety: s.safety_level,
      source: s.source,
      command: s.command_line,
      directory: s.cwd,
    }));

    let content: string;
    let mime: string;
    let ext: string;

    if (format === "csv") {
      const headers = Object.keys(data[0] || {});
      const rows = data.map((row) =>
        headers.map((h) => `"${String((row as any)[h]).replace(/"/g, '""')}"`).join(",")
      );
      content = [headers.join(","), ...rows].join("\n");
      mime = "text/csv";
      ext = "csv";
    } else {
      content = JSON.stringify(data, null, 2);
      mime = "application/json";
      ext = "json";
    }

    const blob = new Blob([content], { type: `${mime};charset=utf-8` });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `port-guardian-${new Date().toISOString().slice(0, 10)}.${ext}`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const safeCount = services.filter((s) => s.safety_level === "safe").length;
  const cautionCount = services.filter(
    (s) => s.safety_level === "caution" || s.safety_level === "unknown"
  ).length;
  const dangerCount = services.filter((s) => s.safety_level === "danger").length;

  const isTauri = isTauriRuntime();

  const handleCheckUpdate = useCallback(async () => {
    if (!isTauri) {
      const message = t("app.browserUpdateError");
      setUpdateError(message);
      throw new Error(message);
    }

    setUpdateError(null);
    try {
      const update = await check();
      if (update) {
        setUpdateInfo(update);
        setShowUpdate(true);
      } else {
        setUpdateInfo(null);
      }
      return !!update;
    } catch (err) {
      const message = formatUpdateError(err, t);
      setUpdateError(message);
      console.error("检查更新失败:", err);
      throw new Error(message);
    }
  }, [isTauri]);

  return (
    <div className={`app workspace ${isTauri && /Mac/.test(navigator.platform) ? "native-mac" : ""}`}>
      {!isTauri && (
        <div style={{
          padding: "12px 20px",
          background: "var(--caution-bg)",
          color: "var(--caution)",
          borderBottom: "1px solid var(--caution)",
          fontSize: 13,
          textAlign: "center"
        }}>
          {t("app.browserWarningBefore")} <code>npm run tauri dev</code> {t("app.browserWarningAfter")}
        </div>
      )}
      <header className="workspace-header" data-tauri-drag-region>
        <div className="workspace-brand" data-tauri-drag-region><h1 data-tauri-drag-region>Port Guardian</h1><span data-tauri-drag-region>{t("app.brandSubtitle")}</span></div>
        <div className="workspace-header-actions">
          {lastRefresh && <time className="workspace-update-time" dateTime={lastRefresh.toISOString()}>{lastRefresh.toLocaleString(undefined, {year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit", hour12: false})}</time>}
          <button className="rescan-button" onClick={() => void refresh()} disabled={loading}><RefreshIcon size={18} className={scanning ? "spinning" : ""} />{loading ? t("common.scanning") : t("app.refresh")}</button>
          {services.length > 0 && <details className="workspace-menu export-options"><summary className="icon-button" aria-label={t("app.export")} title={t("app.export")}><ExportIcon size={18} /></summary><div className="workspace-popover"><button onClick={() => handleExport("csv")}>CSV</button><button onClick={() => handleExport("json")}>JSON</button></div></details>}
          <button className="icon-button settings-button" onClick={() => setShowSettings(true)} aria-label={t("common.settings")} title={t("common.settings")}><SettingsIcon size={19} /></button>
        </div>
      </header>

      <div className="workspace-toolbar">
        <SearchBar ref={searchInputRef} value={search} onChange={handleSearchChange} />
        <div className="protocol-switch" role="group" aria-label={t("app.protocolLabel")}>
          {["all", "TCP", "UDP"].map((protocol) => <button key={protocol} aria-pressed={protocolFilter === protocol} onClick={() => setProtocolFilter(protocol)}>{protocol === "all" ? t("app.filter.all") : protocol}</button>)}
        </div>
        <label className="grouping-select"><ListIcon size={18} aria-hidden="true" /><select aria-label={t("app.listView")} value={byProject ? "project" : "flat"} onChange={(event) => setByProject(event.target.value === "project")}><option value="project">{t("app.groupByProject")}</option><option value="flat">{t("app.flatList")}</option></select></label>
        <details className="workspace-menu filter-options">
          <summary className={"icon-button " + (filter !== "all" || showTechnicalColumns ? "has-filter" : "")} aria-label={t("app.filterAndDisplay")} title={t("app.filterAndDisplay")}><FilterIcon size={18} /></summary>
          <div className="workspace-popover">
            <label>{t("app.filterLabel")}<select aria-label={t("app.filterLabel")} value={filter} onChange={(event) => handleFilterChange(event.target.value as FilterKey)}>{FILTER_OPTIONS.map((option) => <option key={option.key} value={option.key}>{option.label}{option.key === "safe" ? " · " + safeCount : option.key === "caution" ? " · " + cautionCount : option.key === "danger" ? " · " + dangerCount : ""}</option>)}</select></label>
            <label>{t("app.sortLabel")}<select aria-label={t("app.sortLabel")} value={sort} onChange={(event) => setSort(event.target.value as SortKey)}><option value="port">{t("app.sort.port")}</option><option value="project">{t("app.sort.project")}</option><option value="process">{t("app.sort.process")}</option><option value="pid">PID</option></select></label>
            <label className="technical-toggle"><input type="checkbox" checked={showTechnicalColumns} onChange={(event) => setShowTechnicalColumns(event.target.checked)} />{t("app.technicalColumns")}</label>
          </div>
        </details>
        {selectedIds.size > 0 && <button className="batch-stop-button" onClick={() => setShowBatchConfirm(true)} disabled={batchKilling || terminatingPid !== null}><StopIcon size={16} />{batchKilling ? t("app.batchKilling") : t("app.batchKill", {count: batchTargets.length})}</button>}
      </div>

      {actionNotice && <div className="workspace-action-notice" role="status"><span>{actionNotice}</span><button className="icon-button" aria-label={t("common.close")} onClick={() => setActionNotice(null)}><CloseIcon size={16} /></button></div>}
      <main className="workspace-main">
        <PortTable services={filtered} selected={selected} loading={loading} scanTotal={scanTotal} scannedCount={scannedCount}
          hasFilter={search !== "" || filter !== "all" || protocolFilter !== "all"} scanFailed={issue?.kind === "failed" || issue?.kind === "timeout"}
          showTechnicalColumns={showTechnicalColumns} byProject={byProject} search={search}
          terminatingPid={terminatingPid} actionsDisabled={terminatingPid !== null} selectedIds={selectedIds} bookmarkedPorts={bookmarkedPorts}
          onSelect={setSelected} onClose={() => setSelected(null)} onKill={(service) => {if (!operationRef.current) setKillTarget(service);}}
          onToggleSelect={toggleSelect} onToggleSelectAll={toggleSelectAll} onToggleBookmark={toggleBookmark} />
      </main>
      <footer className={"workspace-status " + (issue ? "has-issue" : "")} role="status" aria-live="polite">
        <span className="scan-state">{scanning ? <RefreshIcon className="spinning" size={13} /> : <CircleIcon size={9} weight="fill" aria-hidden="true" />}
          {issue ? t("app.scanIssue." + issue.kind as "app.scanIssue.failed" | "app.scanIssue.timeout" | "app.scanIssue.partial", {count: issue.skipped ?? 0}) : scanning
            ? scanTotal ? t("app.scanProgress", {count: scannedCount, total: scanTotal}) : t("portTable.scanningPorts")
            : t("app.scanComplete")}</span>
        {(!issue || issue.kind === "partial") && <span className="workspace-result-count">{t("app.resultCount", {shown: filtered.length, total: services.length})}</span>}
        {issue?.message && <span className="workspace-error-detail" title={issue.message}>{issue.message}</span>}
        {!scanning && durationMs !== null && <span className="workspace-duration">{t("app.scanDuration", {seconds: (durationMs / 1000).toFixed(2)})}</span>}
      </footer>

      {showBatchConfirm && (
        <ConfirmBatchKillDialog services={batchTargets} onConfirm={handleBatchKill} onCancel={() => setShowBatchConfirm(false)} />
      )}
      {killTarget && (
        <ConfirmKillDialog
          service={killTarget}
          onConfirm={(force) => handleKill(killTarget, force)}
          onCancel={() => setKillTarget(null)}
        />
      )}

        {showSettings && (
          <Settings
            version={appVersion}
            theme={theme}
            updateError={updateError}
            onThemeChange={setTheme}
            onClose={() => setShowSettings(false)}
            onCheckUpdate={handleCheckUpdate}
        />
      )}

      {isTauri && (
        <UpdateChecker
          show={showUpdate}
          updateInfo={updateInfo}
          onAutoCheck={handleCheckUpdate}
          onClose={() => setShowUpdate(false)}
        />
      )}
    </div>
  );
}

export default App;
