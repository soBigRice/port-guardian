import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { PortService, ScanProgress, ScanResult } from "../types";
import { mergeScanResult, mergeScannedServices } from "../utils/scanState";

export type ScanIssue = { kind: "failed" | "timeout" | "partial"; message?: string; skipped?: number };
export const isTauriRuntime = () => "__TAURI_INTERNALS__" in window;

export function usePortScan() {
  const [services, setServices] = useState<PortService[]>([]);
  // 原生首帧已经等待扫描，不能在监听注册期间误报“完成”或“没有端口”。
  const [loading, setLoading] = useState(isTauriRuntime);
  const [scanning, setScanning] = useState(isTauriRuntime);
  const [scanTotal, setScanTotal] = useState(0);
  const [scannedCount, setScannedCount] = useState(0);
  const [lastRefresh, setLastRefresh] = useState<Date | null>(null);
  const [durationMs, setDurationMs] = useState<number | null>(null);
  const [issue, setIssue] = useState<ScanIssue | null>(null);
  const [settled, setSettled] = useState(false);
  const activeIdRef = useRef<string | null>(null);
  const completedRef = useRef(false);
  const mountedRef = useRef(false);
  const readyRef = useRef(false);
  const pendingRef = useRef<PortService[]>([]);
  const streamingRef = useRef(false);
  const terminatedPidsRef = useRef(new Set<number>());
  const queuedRef = useRef(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const flushRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const refreshRef = useRef<(silent?: boolean, queue?: boolean) => Promise<void>>(async () => {});

  const stopTimers = useCallback(() => {
    if (timerRef.current) clearTimeout(timerRef.current);
    if (flushRef.current) clearInterval(flushRef.current);
    timerRef.current = null;
    flushRef.current = null;
  }, []);

  const flushPending = useCallback(() => {
    const pending = pendingRef.current.filter((service) => !terminatedPidsRef.current.has(service.pid));
    pendingRef.current = [];
    if (pending.length) setServices((prev) => mergeScannedServices(prev, pending, false));
  }, []);

  const refresh = useCallback(async (silent = false, queue = false) => {
    if (!isTauriRuntime() || !mountedRef.current || !readyRef.current) return;
    if (activeIdRef.current) {
      if (!silent) setLoading(true); // 手动刷新接管当前后台扫描，立即给出反馈。
      if (queue) queuedRef.current = true;
      return;
    }
    const scanId = crypto.randomUUID();
    const started = performance.now();
    activeIdRef.current = scanId;
    terminatedPidsRef.current.clear();
    pendingRef.current = [];
    streamingRef.current = !completedRef.current;
    setLoading(!silent);
    setScanning(true);
    setIssue(null);
    setScanTotal(0);
    setScannedCount(0);

    const finish = (result: ScanResult | null, failure: ScanIssue | null) => {
      if (!mountedRef.current || activeIdRef.current !== scanId) return;
      stopTimers();
      if (result) {
        const snapshot = result.services.filter((service) => !terminatedPidsRef.current.has(service.pid));
        setServices((prev) => mergeScanResult(prev, {...result, services: snapshot}));
        setScannedCount(result.total);
        setScanTotal(result.total);
        setDurationMs(Math.round(performance.now() - started));
        completedRef.current = true;
        setLastRefresh(new Date());
        if (result.skipped) {
          failure = {kind: "partial", skipped: result.skipped};
        }
      } else if (streamingRef.current) {
        flushPending();
      }
      pendingRef.current = [];
      activeIdRef.current = null;
      setLoading(false);
      setScanning(false);
      setIssue(failure);
      setSettled(true);
      if (queuedRef.current) {
        queuedRef.current = false;
        queueMicrotask(() => { if (mountedRef.current) void refreshRef.current(true); });
      }
    };

    timerRef.current = setTimeout(() => finish(null, {kind: "timeout"}), 30000);
    if (streamingRef.current) flushRef.current = setInterval(flushPending, 150);
    try {
      // 完整命令返回值独立于事件派发：晚到、丢失或旧轮事件不能污染快照。
      const result = await invoke<ScanResult>("scan_ports_stream", {scanId, stream: streamingRef.current});
      if (result.scan_id !== scanId) throw new Error("Scan response ID does not match request");
      finish(result, null);
    } catch (error) {
      finish(null, {kind: "failed", message: String(error)});
    }
  }, [flushPending, stopTimers]);
  refreshRef.current = refresh;

  const removeProcess = useCallback((pid: number) => {
    terminatedPidsRef.current.add(pid);
    pendingRef.current = pendingRef.current.filter((service) => service.pid !== pid);
    setServices((prev) => prev.filter((service) => service.pid !== pid));
    // 终止期间正在执行的扫描可能持有旧数据，随后再核实一次实时状态。
    void refreshRef.current(true, true);
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    mountedRef.current = true;
    let disposed = false;
    const updateProgress = ({payload}: {payload: ScanProgress}) => {
      if (disposed || payload.scan_id !== activeIdRef.current) return;
      setScanTotal(payload.total);
      setScannedCount(payload.processed);
    };
    const registration = Promise.allSettled([
      listen<ScanProgress>("scan-start", updateProgress),
      listen<ScanProgress>("scan-progress", updateProgress),
      listen<{scan_id: string; service: PortService}>("port-found", ({payload}) => {
        if (disposed || payload.scan_id !== activeIdRef.current || !streamingRef.current) return;
        if (!terminatedPidsRef.current.has(payload.service.pid)) pendingRef.current.push(payload.service);
      }),
    ]);
    let cleaned = false;
    const cleanupListeners = (results: Awaited<typeof registration>) => {
      if (cleaned) return;
      cleaned = true;
      for (const result of results) if (result.status === "fulfilled") result.value();
    };
    void registration.then((results) => {
      if (disposed) {
        cleanupListeners(results);
        return;
      }
      // 事件只增强进度反馈；监听注册失败时仍可用命令的完整返回值扫描和刷新。
      if (results.some((result) => result.status === "rejected")) cleanupListeners(results);
      readyRef.current = true;
      void refreshRef.current();
    });
    return () => {
      disposed = true;
      mountedRef.current = false;
      readyRef.current = false;
      activeIdRef.current = null;
      queuedRef.current = false;
      stopTimers();
      void registration.then(cleanupListeners);
    };
  }, [stopTimers]);

  useEffect(() => {
    if (!settled) return;
    let poll: ReturnType<typeof setInterval> | null = null;
    const stop = () => { if (poll) clearInterval(poll); poll = null; };
    const start = (immediate = false) => {
      if (poll || document.visibilityState !== "visible") return;
      if (immediate) void refresh(true);
      poll = setInterval(() => { void refresh(true); }, 10000);
    };
    const focus = () => start(true);
    const visibility = () => document.visibilityState === "visible" ? start(true) : stop();
    document.addEventListener("visibilitychange", visibility);
    window.addEventListener("focus", focus);
    window.addEventListener("blur", stop);
    start();
    return () => {
      stop();
      document.removeEventListener("visibilitychange", visibility);
      window.removeEventListener("focus", focus);
      window.removeEventListener("blur", stop);
    };
  }, [settled, refresh]);

  return {services, loading, scanning, scanTotal, scannedCount, lastRefresh, durationMs, issue, refresh, removeProcess};
}
