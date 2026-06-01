import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { PortService, Theme } from "./types";
import PortTable from "./components/PortTable";
import ServiceDetail from "./components/ServiceDetail";
import ConfirmKillDialog from "./components/ConfirmKillDialog";
import SearchBar from "./components/SearchBar";
import Settings from "./components/Settings";
import UpdateChecker from "./components/UpdateChecker";

const VERSION = "0.1.0";

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
  const [services, setServices] = useState<PortService[]>([]);
  const [filtered, setFiltered] = useState<PortService[]>([]);
  const [selected, setSelected] = useState<PortService | null>(null);
  const [loading, setLoading] = useState(false);
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<string>("all");
  const [killTarget, setKillTarget] = useState<PortService | null>(null);
  const [lastRefresh, setLastRefresh] = useState<Date | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [theme, setTheme] = useState<Theme>(getInitialTheme);
  const [updateInfo, setUpdateInfo] = useState<Update | null>(null);
  const [showUpdate, setShowUpdate] = useState(false);
  const rowClickedRef = useRef(false);

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

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      if (!(window as any).__TAURI_INTERNALS__) {
        setLoading(false);
        return;
      }
      const result = await invoke<PortService[]>("scan_ports");
      setServices(result);
      setLastRefresh(new Date());
    } catch (err) {
      console.error("扫描失败:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
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

  useEffect(() => {
    let list = services;
    if (search) {
      const q = search.toLowerCase();
      list = list.filter(
        (s) =>
          s.port.toString().includes(q) ||
          s.process_name.toLowerCase().includes(q) ||
          s.command_line.toLowerCase().includes(q) ||
          s.cwd.toLowerCase().includes(q) ||
          s.service_name.toLowerCase().includes(q) ||
          s.source.toLowerCase().includes(q)
      );
    }
    if (filter !== "all") {
      if (filter === "safe") {
        list = list.filter((s) => s.safety_level === "safe");
      } else if (filter === "caution") {
        list = list.filter((s) => s.safety_level === "caution" || s.safety_level === "unknown");
      } else if (filter === "danger") {
        list = list.filter((s) => s.safety_level === "danger");
      } else {
        list = list.filter((s) => s.service_type === filter);
      }
    }
    setFiltered(list);
  }, [services, search, filter]);

  const handleKill = async (service: PortService, force: boolean) => {
    try {
      const result = await invoke<{ success: boolean; message: string }>(
        "terminate_process",
        { pid: service.pid, force }
      );
      if (result.success) {
        setKillTarget(null);
        setSelected(null);
        setTimeout(refresh, 500);
      } else {
        alert(result.message);
      }
    } catch (err) {
      alert(`终止失败: ${err}`);
    }
  };

  const safeCount = services.filter((s) => s.safety_level === "safe").length;
  const cautionCount = services.filter(
    (s) => s.safety_level === "caution" || s.safety_level === "unknown"
  ).length;
  const dangerCount = services.filter((s) => s.safety_level === "danger").length;

  const isTauri = !!(window as any).__TAURI_INTERNALS__;

  const handleCheckUpdate = useCallback(async () => {
    try {
      const update = await check();
      if (update) {
        setUpdateInfo(update);
        setShowUpdate(true);
      }
      return !!update;
    } catch {
      return false;
    }
  }, []);

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
          当前在浏览器中运行，请使用 <code>npm run tauri dev</code> 启动以获得完整功能
        </div>
      )}
      <header className="header">
        <div className="header-left">
          <h1 className="title">Port Guardian</h1>
          <span className="subtitle">v{VERSION}</span>
        </div>
        <div className="header-stats">
          <span className="stat">
            监听端口: <strong>{services.length}</strong>
          </span>
          <span className="stat stat-safe">
            安全: <strong>{safeCount}</strong>
          </span>
          <span className="stat stat-caution">
            谨慎: <strong>{cautionCount}</strong>
          </span>
          {dangerCount > 0 && (
            <span className="stat stat-danger">
              危险: <strong>{dangerCount}</strong>
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
            {loading ? "扫描中..." : "刷新"}
          </button>
          <button
            className="btn btn-icon"
            onClick={() => setShowSettings(true)}
            title="设置"
          >
            &#9881;
          </button>
        </div>
      </header>

      <div className="toolbar">
        <SearchBar value={search} onChange={setSearch} />
        <div className="filters">
          {[
            { key: "all", label: "全部" },
            { key: "safe", label: "安全可杀" },
            { key: "caution", label: "谨慎操作" },
            { key: "danger", label: "危险服务" },
            { key: "dev-service", label: "开发服务" },
            { key: "database-service", label: "数据库" },
            { key: "docker-service", label: "Docker" },
            { key: "system-service", label: "系统服务" },
            { key: "app-service", label: "应用程序" },
          ].map((f) => (
            <button
              key={f.key}
              className={`filter-btn ${filter === f.key ? "active" : ""}`}
              onClick={() => setFilter(f.key)}
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
          theme={theme}
          onThemeChange={setTheme}
          onClose={() => setShowSettings(false)}
          onCheckUpdate={handleCheckUpdate}
        />
      )}

      {isTauri && (
        <UpdateChecker
          show={showUpdate}
          updateInfo={updateInfo}
          onUpdateFound={(update) => {
            setUpdateInfo(update);
            setShowUpdate(true);
          }}
          onClose={() => setShowUpdate(false)}
        />
      )}
    </div>
  );
}

export default App;
