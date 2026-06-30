import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { PortService, Theme } from "./types";
import PortTable from "./components/PortTable";
import ServiceDetail from "./components/ServiceDetail";
import ConfirmKillDialog from "./components/ConfirmKillDialog";
import SearchBar from "./components/SearchBar";
import Settings from "./components/Settings";
import UpdateChecker from "./components/UpdateChecker";
import { formatUpdateError } from "./utils/updateErrors";
import { useTranslation } from "./i18n";

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

function matchesSearch(service: PortService, query: string) {
  if (!query) return true;
  const q = query.toLowerCase();
  return (
    service.port.toString().includes(q) ||
    service.process_name.toLowerCase().includes(q) ||
    service.command_line.toLowerCase().includes(q) ||
    service.cwd.toLowerCase().includes(q) ||
    service.service_name.toLowerCase().includes(q) ||
    service.source.toLowerCase().includes(q)
  );
}

function getInitialTheme(): Theme {
  const saved = localStorage.getItem("pg-theme") as Theme;
  if (saved && ["dark", "light", "auto"].includes(saved)) return saved;
  return "dark";
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
  const [services, setServices] = useState<PortService[]>([]);
  const [selected, setSelected] = useState<PortService | null>(null);
  const [loading, setLoading] = useState(false);
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
  const [lastRefresh, setLastRefresh] = useState<Date | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [theme, setTheme] = useState<Theme>(getInitialTheme);
  const [appVersion, setAppVersion] = useState(FALLBACK_VERSION);
  const [updateInfo, setUpdateInfo] = useState<Update | null>(null);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const [showUpdate, setShowUpdate] = useState(false);
  const [scanTotal, setScanTotal] = useState(0);
  const [scannedCount, setScannedCount] = useState(0);
  const rowClickedRef = useRef(false);
  const pendingServicesRef = useRef<PortService[]>([]);
  const seenServiceIdsRef = useRef<Set<string>>(new Set());
  const scanInFlightRef = useRef(false);
  const scanTotalRef = useRef(0);
  const flushTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

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

  // 从 Tauri 读取真实应用版本，避免界面版本号写死
  useEffect(() => {
    if (!(window as any).__TAURI_INTERNALS__) return;
    (async () => {
      try {
        const runtimeVersion = await getVersion();
        setAppVersion(runtimeVersion);
      } catch (err) {
        console.warn("读取应用版本失败，使用默认版本号:", err);
      }
    })();
  }, []);

  // 将 pendingServices flush 到 state（扫描期间定时批量更新）
  // 用 ref 包一层，避免 useCallback 依赖导致 useEffect 重新订阅
  const flushPendingRef = useRef<() => void>(() => {});
  flushPendingRef.current = () => {
    const pending = pendingServicesRef.current;
    if (pending.length > 0) {
      pendingServicesRef.current = [];
      setServices((prev) => [...prev, ...pending]);
      setScannedCount((prev) => {
        const next = prev + pending.length;
        return scanTotalRef.current > 0 ? Math.min(next, scanTotalRef.current) : next;
      });
    }
  };

  // 是否为静默模式（由 refresh(silent) 控制）
  const silentScanRef = useRef(false);
  // 本轮扫描是否需要流式 flush（仅首次加载为 true，后续为 false）
  const streamFlushRef = useRef(true);

  const finishScanRef = useRef<(completed?: boolean) => void>(() => {});
  finishScanRef.current = (completed = true) => {
    if (flushTimerRef.current) {
      clearInterval(flushTimerRef.current);
      flushTimerRef.current = null;
    }

    const newResults = pendingServicesRef.current;
    const shouldStream = streamFlushRef.current;

    if (shouldStream) {
      // 首次加载：把剩余的 pending 一次性 flush
      flushPendingRef.current();
    } else {
      // 后续刷新 / 静默轮询：增量 diff，无变化不重绘
      const newIds = new Set(newResults.map((s) => s.id));
      const newIdStr = [...newIds].sort().join(",");

      setServices((prev) => {
        const prevIds = new Set(prev.map((s) => s.id));
        const prevIdStr = [...prevIds].sort().join(",");
        if (prevIdStr === newIdStr) return prev;

        const merged = prev.filter((s) => newIds.has(s.id));
        const existingIds = new Set(merged.map((s) => s.id));
        for (const s of newResults) {
          if (!existingIds.has(s.id)) merged.push(s);
        }
        return merged;
      });
    }

    pendingServicesRef.current = [];
    scanInFlightRef.current = false;
    silentScanRef.current = false;
    setLoading(false);
    if (completed) {
      setLastRefresh(new Date());
    }
  };

  // 流式扫描：逐个接收端口结果
  // silent=true 时静默刷新：不闪屏、不显示 loading，扫描完成后替换列表
  const refresh = useCallback(async (silent = false) => {
    if (!(window as any).__TAURI_INTERNALS__) return;
    if (scanInFlightRef.current) {
      if (!silent && streamFlushRef.current) flushPendingRef.current();
      return;
    }

    scanInFlightRef.current = true;
    silentScanRef.current = silent;
    // 清理上一轮扫描的 flush 定时器
    if (flushTimerRef.current) {
      clearInterval(flushTimerRef.current);
      flushTimerRef.current = null;
    }
    pendingServicesRef.current = [];
    seenServiceIdsRef.current = new Set();
    scanTotalRef.current = 0;
    if (!silent) {
      const isStream = streamFlushRef.current;
      if (isStream) {
        setServices([]);
      }
      setSelected(null);
      setKillTarget(null);
      setSelectedIds(new Set());
      setLoading(true);
      setScanTotal(0);
      setScannedCount(0);
      // 首次加载完成后，后续刷新走 diff 模式
      if (isStream) streamFlushRef.current = false;
    }
    try {
      await invoke("scan_ports_stream");
    } catch (err) {
      console.error("扫描启动失败:", err);
      finishScanRef.current(false);
      return;
    }

    // 如果 complete 事件丢失，也不要让界面停在扫描中。
    if (scanInFlightRef.current) {
      finishScanRef.current();
    }
  }, []);

  // 监听扫描事件
  useEffect(() => {
    if (!(window as any).__TAURI_INTERNALS__) return;

    const unlistenPromise = Promise.all([
      listen<number>("scan-start", (event) => {
        if (!scanInFlightRef.current) return;
        scanTotalRef.current = event.payload;
        setScanTotal(event.payload);
        setScannedCount(0);
        // 仅首次加载启动 flush 定时器（流式填充）
        // 后续刷新全程缓存到 ref，扫描结束后一次性 diff 替换
        if (streamFlushRef.current) {
          if (flushTimerRef.current) clearInterval(flushTimerRef.current);
          flushTimerRef.current = setInterval(() => flushPendingRef.current(), 150);
        }
      }),
      listen<PortService>("port-found", (event) => {
        if (!scanInFlightRef.current) return;
        if (seenServiceIdsRef.current.has(event.payload.id)) return;
        seenServiceIdsRef.current.add(event.payload.id);
        pendingServicesRef.current.push(event.payload);
        // 非流式模式（后续刷新），flush 定时器不跑，手动更新进度
        if (!streamFlushRef.current) {
          setScannedCount((prev) => {
            const next = prev + 1;
            return scanTotalRef.current > 0 ? Math.min(next, scanTotalRef.current) : next;
          });
        }
      }),
      listen("scan-complete", () => {
        if (!scanInFlightRef.current) return;
        finishScanRef.current();
      }),
    ]);

    return () => {
      unlistenPromise.then(([u1, u2, u3]) => { u1(); u2(); u3(); });
      if (flushTimerRef.current) {
        clearInterval(flushTimerRef.current);
        flushTimerRef.current = null;
      }
    };
  }, []);

  // 启动时自动扫描
  useEffect(() => {
    const timer = window.setTimeout(() => {
      refresh();
    }, 0);
    return () => window.clearTimeout(timer);
  }, [refresh]);

  // 智能轮询：窗口可见时每 10 秒静默刷新，隐藏时暂停
  useEffect(() => {
    let pollTimer: ReturnType<typeof setInterval> | null = null;

    const startPolling = () => {
      if (pollTimer) return;
      // 获得焦点时立即静默刷新一次
      refresh(true);
      pollTimer = setInterval(() => refresh(true), 10000);
    };

    const stopPolling = () => {
      if (pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
      }
    };

    const handleVisibility = () => {
      if (document.visibilityState === "visible") {
        startPolling();
      } else {
        stopPolling();
      }
    };

    // 页面可见性变化
    document.addEventListener("visibilitychange", handleVisibility);
    // 窗口焦点变化（兼容）
    window.addEventListener("focus", startPolling);
    window.addEventListener("blur", stopPolling);

    // 初始状态：如果可见就开始轮询
    if (document.visibilityState === "visible") {
      startPolling();
    }

    return () => {
      stopPolling();
      document.removeEventListener("visibilitychange", handleVisibility);
      window.removeEventListener("focus", startPolling);
      window.removeEventListener("blur", stopPolling);
    };
  }, [refresh]);

  // 键盘快捷键
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement).tagName;
      const isInput = tag === "INPUT" || tag === "TEXTAREA" || (e.target as HTMLElement).isContentEditable;

      // Escape: 逐层关闭面板
      if (e.key === "Escape") {
        if (killTarget) setKillTarget(null);
        else if (showSettings) setShowSettings(false);
        else if (selected) setSelected(null);
        else if (isInput) (e.target as HTMLElement).blur();
        return;
      }

      // 以下快捷键仅在非输入状态生效
      if (isInput) return;

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
        const visible = services.filter((s) => matchesSearch(s, search) && matchesFilter(s, filter));
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
      if ((e.key === "k" || e.key === "Delete") && selected) {
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
  }, [selected, killTarget, showSettings, search, filter, refresh]);

  // 切换收藏
  const toggleBookmark = (port: number) => {
    setBookmarkedPorts((prev) => {
      const next = new Set(prev);
      if (next.has(port)) next.delete(port);
      else next.add(port);
      return next;
    });
  };

  const filtered = useMemo(() => {
    const list = services.filter((service) => matchesSearch(service, search) && matchesFilter(service, filter));
    // 收藏端口置顶
    return [...list].sort((a, b) => {
      const aBk = bookmarkedPorts.has(a.port) ? 0 : 1;
      const bBk = bookmarkedPorts.has(b.port) ? 0 : 1;
      return aBk - bBk;
    });
  }, [services, search, filter, bookmarkedPorts]);

  useEffect(() => {
    if (selected && !filtered.some((service) => service.id === selected.id)) {
      setSelected(null);
    }
  }, [filtered, selected]);

  const handleSearchChange = (value: string) => {
    if (scanInFlightRef.current) flushPendingRef.current();
    setSearch(value);
  };

  const handleFilterChange = (nextFilter: FilterKey) => {
    if (scanInFlightRef.current) flushPendingRef.current();
    setFilter(nextFilter);
  };

  const handleKill = async (service: PortService, force: boolean) => {
    setKillTarget(null);

    try {
      const result = await invoke<{
        success: boolean;
        message: string;
        port_released: boolean;
      }>("terminate_process", { pid: service.pid, force });

      if (!result.success) {
        alert(result.message);
        return;
      }

      if (result.port_released) {
        // 端口已释放，从列表移除
        setServices((prev) => prev.filter((s) => s.id !== service.id));
        setSelected((prev) => (prev?.id === service.id ? null : prev));
        return;
      }

      // 进程已退出但端口可能仍处于 TIME_WAIT，轮询等待端口释放
      const maxRetries = 30; // 最多等 3 秒
      for (let i = 0; i < maxRetries; i++) {
        await new Promise((r) => setTimeout(r, 100));
        const listening = await invoke<boolean>("check_port_listening", {
          port: service.port,
        });
        if (!listening) {
          // 端口已释放，从列表移除
          setServices((prev) => prev.filter((s) => s.id !== service.id));
          setSelected((prev) => (prev?.id === service.id ? null : prev));
          return;
        }
      }

      // 超时仍未释放，刷新列表获取最新状态
      refresh();
    } catch (err) {
      alert(`${t("app.terminateFailed")} ${err}`);
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

  // 批量终止
  const handleBatchKill = async () => {
    const targets = services.filter((s) => selectedIds.has(s.id) && s.can_terminate);
    if (targets.length === 0) return;

    setBatchKilling(true);
    let successCount = 0;

    for (const svc of targets) {
      try {
        const result = await invoke<{
          success: boolean;
          message: string;
          port_released: boolean;
        }>("terminate_process", { pid: svc.pid, force: true });

        if (result.success) {
          successCount++;
          setServices((prev) => prev.filter((s) => s.id !== svc.id));
        }
      } catch {
        // 单个失败不影响其他
      }
    }

    setSelectedIds(new Set());
    setSelected(null);
    setBatchKilling(false);

    // 如果有失败的，刷新列表
    if (successCount < targets.length) {
      refresh();
    }
  };

  // 导出端口列表
  const handleExport = (format: "csv" | "json") => {
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

  const isTauri = !!(window as any).__TAURI_INTERNALS__;

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
    <div className="app">
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
      <header className="header">
        <div className="header-left">
          <h1 className="title">Port Guardian</h1>
          <span className="subtitle">v{appVersion}</span>
        </div>
        <div className="header-stats">
          <span className="stat">
            {t("app.stats.listeningPorts")} <strong>{services.length}</strong>
          </span>
          <span className="stat stat-safe">
            {t("app.stats.safe")} <strong>{safeCount}</strong>
          </span>
          <span className="stat stat-caution">
            {t("app.stats.caution")} <strong>{cautionCount}</strong>
          </span>
          {dangerCount > 0 && (
            <span className="stat stat-danger">
              {t("app.stats.danger")} <strong>{dangerCount}</strong>
            </span>
          )}
        </div>
        <div className="header-right">
          {lastRefresh && (
            <span className="refresh-time">
              {lastRefresh.toLocaleTimeString()}
            </span>
          )}
          <button className="btn btn-refresh" onClick={() => refresh()} disabled={loading}>
            {loading ? t("common.scanning") : t("app.refresh")}
          </button>
          {services.length > 0 && (
            <div className="export-dropdown">
              <button className="btn btn-export">{t("app.export")} ▾</button>
              <div className="export-menu">
                <button onClick={() => handleExport("csv")}>CSV</button>
                <button onClick={() => handleExport("json")}>JSON</button>
              </div>
            </div>
          )}
          <button
            className="btn btn-icon"
            onClick={() => setShowSettings(true)}
            title={t("common.settings")}
          >
            &#9881;
          </button>
        </div>
      </header>

      <div className="toolbar">
        <SearchBar ref={searchInputRef} value={search} onChange={handleSearchChange} />
        <div className="filters">
          {FILTER_OPTIONS.map((f) => (
            <button
              key={f.key}
              className={`filter-btn ${filter === f.key ? "active" : ""}`}
              onClick={() => handleFilterChange(f.key)}
            >
              {f.label}
            </button>
          ))}
        </div>
        {selectedIds.size > 0 && (
          <button
            className="btn btn-batch-kill"
            onClick={handleBatchKill}
            disabled={batchKilling}
          >
            {batchKilling
              ? t("app.batchKilling")
              : t("app.batchKill", { count: selectedIds.size })}
          </button>
        )}
      </div>

      <div className="main">
        <div
          className="table-area"
          onClick={() => {
            if (!rowClickedRef.current) {
              setSelected(null);
            }
            rowClickedRef.current = false;
          }}
        >
          <PortTable
            services={filtered}
            selected={selected}
            loading={loading}
            scanTotal={scanTotal}
            scannedCount={scannedCount}
            hasFilter={search !== "" || filter !== "all"}
            selectedIds={selectedIds}
            onSelect={(s) => {
              rowClickedRef.current = true;
              setSelected(s);
            }}
            onKill={(s) => setKillTarget(s)}
            onToggleSelect={toggleSelect}
            onToggleSelectAll={toggleSelectAll}
            bookmarkedPorts={bookmarkedPorts}
            onToggleBookmark={toggleBookmark}
          />
        </div>
        {selected && (
          <div className="detail-area">
            <ServiceDetail
              service={selected}
              onKill={() => setKillTarget(selected)}
              onClose={() => setSelected(null)}
            />
          </div>
        )}
      </div>

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
