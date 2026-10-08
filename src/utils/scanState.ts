import type { PortService } from "../types";

export type SortKey = "port" | "process" | "pid" | "project";

// 不完整扫描只能更新已收到的条目，不能据此删除未收到的条目。
export function mergeScannedServices(prev: PortService[], scanned: PortService[], complete = true) {
  const incoming = new Map(scanned.map((service) => [service.id, service]));
  const merged: PortService[] = [];
  const seen = new Set<string>();
  for (const old of prev) {
    if (seen.has(old.id)) continue;
    seen.add(old.id);
    const next = incoming.get(old.id);
    if (next) {
      merged.push(JSON.stringify(old) === JSON.stringify(next) ? old : next);
      incoming.delete(old.id);
    } else if (!complete) {
      merged.push(old);
    }
  }
  merged.push(...incoming.values());
  return merged.length === prev.length && merged.every((service, index) => service === prev[index]) ? prev : merged;
}

export function matchesSearch(service: PortService, query: string) {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  if (/^\d+$/.test(q)) return service.port === Number(q);
  const exact = /^(port|pid):(\d+)$/.exec(q);
  if (exact) return (exact[1] === "pid" ? service.pid : service.port) === Number(exact[2]);
  return [service.process_name, service.command_line, service.cwd, service.service_name, service.source,
    service.local_address, service.protocol, service.executable_path, service.user].some((value) => value.toLowerCase().includes(q));
}

export function sortServices(services: PortService[], bookmarks: Set<number>, sort: SortKey) {
  return [...services].sort((a, b) => {
    const bookmarked = Number(bookmarks.has(b.port)) - Number(bookmarks.has(a.port));
    if (bookmarked) return bookmarked;
    const primary = sort === "pid" ? a.pid - b.pid
      : sort === "process" ? a.process_name.localeCompare(b.process_name)
      : sort === "project" ? a.cwd.localeCompare(b.cwd) : a.port - b.port;
    return primary || a.port - b.port || a.protocol.localeCompare(b.protocol) || a.pid - b.pid;
  });
}

export function uniqueProcesses(services: PortService[]) {
  return [...new Map(services.map((service) => [service.pid, service])).values()];
}

export type ServiceGroup = {key: string; name: string; path: string; services: PortService[]};

export function projectPath(service: PortService) {
  const path = service.cwd.replace(/\\/g, "/").replace(/\/+$/, "");
  // 根目录和系统/应用的运行目录不能作为开发项目；所有条目仍可在其他组中查看。
  return !path || /^[A-Za-z]:$/.test(path) || ["system-service", "app-service"].includes(service.service_type) ? "" : path;
}

export function groupServices(services: PortService[], byProject: boolean): ServiceGroup[] {
  if (!byProject) return [{key: "all", name: "", path: "", services}];
  const groups = new Map<string, ServiceGroup>();
  for (const service of services) {
    const path = projectPath(service);
    const key = path ? `project:${path}` : "other";
    if (!groups.has(key)) groups.set(key, {key, path, name: path.split("/").pop() || "", services: []});
    groups.get(key)!.services.push(service);
  }
  return [...groups.values()].filter((group) => group.key !== "other").concat(groups.has("other") ? [groups.get("other")!] : []);
}
