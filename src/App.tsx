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

  const finishScanRef = useRef<(completed?: boolean) => void>(() => {});
  finishScanRef.current = (completed = true) => {
    if (flushTimerRef.current) {
      clearInterval(flushTimerRef.current);
      flushTimerRef.current = null;
    }
    flushPendingRef.current();
    scanInFlightRef.current = false;
    setLoading(false);
    if (completed) {
      setLastRefresh(new Date());
    }
  };

  // 流式扫描：逐个接收端口结果
  const refresh = useCallback(async () => {
    if (!(window as any).__TAURI_INTERNALS__) return;
    if (scanInFlightRef.current) {
      flushPendingRef.current();
      return;
    }

    scanInFlightRef.current = true;
    // 清理上一轮扫描的 flush 定时器
    if (flushTimerRef.current) {
      clearInterval(flushTimerRef.current);
      flushTimerRef.current = null;
    }
    pendingServicesRef.current = [];
    seenServiceIdsRef.current = new Set();
    scanTotalRef.current = 0;
    setServices([]);
    setSelected(null);
    setKillTarget(null);
    setLoading(true);
    setScanTotal(0);
    setScannedCount(0);
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
        // 启动定时 flush（每 150ms 批量更新一次，避免逐条渲染卡顿）
        if (flushTimerRef.current) clearInterval(flushTimerRef.current);
        flushTimerRef.current = setInterval(() => flushPendingRef.current(), 150);
      }),
      listen<PortService>("port-found", (event) => {
        if (!scanInFlightRef.current) return;
        if (seenServiceIdsRef.current.has(event.payload.id)) return;
        seenServiceIdsRef.current.add(event.payload.id);
        // 先缓存到 ref，不直接触发 setState
        pendingServicesRef.current.push(event.payload);
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

  // Escape 关闭详情面板
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (killTarget) setKillTarget(null);
        else if (showSettings) setShowSettings(false);
        else if (selected) setSelected(null);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [selected, killTarget, showSettings]);

  const filtered = useMemo(() => {
    return services.filter((service) => matchesSearch(service, search) && matchesFilter(service, filter));
  }, [services, search, filter]);

  useEffect(() => {
    if (selected && !filtered.some((service) => service.id === selected.id)) {
      setSelected(null);
    }
  }, [filtered, selected]);

  const handleSearchChange = (value: string) => {
    flushPendingRef.current();
    setSearch(value);
  };

  const handleFilterChange = (nextFilter: FilterKey) => {
    flushPendingRef.current();
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
          <button className="btn btn-refresh" onClick={refresh} disabled={loading}>
            {loading ? t("common.scanning") : t("app.refresh")}
          </button>
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
        <SearchBar value={search} onChange={handleSearchChange} />
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
            onSelect={(s) => {
              rowClickedRef.current = true;
              setSelected(s);
            }}
            onKill={(s) => setKillTarget(s)}
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
