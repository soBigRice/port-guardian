import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import vm from "node:vm";
import ts from "typescript";

const source = readFileSync(new URL("../src/utils/scanState.ts", import.meta.url), "utf8");
const exports = {};
vm.runInNewContext(ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 } }).outputText, { exports });
const { mergeScanResult, mergeScannedServices, matchesSearch, sortServices, uniqueProcesses, groupServices } = exports;
const service = (overrides = {}) => ({
  id: "TCP-3000-10", port: 3000, protocol: "TCP", pid: 10,
  local_address: "127.0.0.1", state: "LISTEN", process_name: "node", executable_path: "/usr/bin/node",
  command_line: "node app.js", cwd: "/demo", user: "developer", parent_chain: [], source: "Terminal",
  service_type: "dev-service", service_name: "Vite", safety_level: "safe", safety_reason: "Development service", can_terminate: true,
  ...overrides,
});

test("same ID refresh updates directory, address and risk", () => {
  const old = service();
  const next = service({ cwd: "/new-project", local_address: "0.0.0.0, ::", safety_level: "caution" });
  const result = mergeScannedServices([old], [next]);
  assert.equal(result[0], next);
});

test("new IDs do not cause existing entries to retain stale metadata", () => {
  const old = service();
  const next = service({ service_name: "Next.js" });
  const added = service({ id: "TCP-4000-11", port: 4000, pid: 11 });
  assert.equal(mergeScannedServices([old], [next, added])[0], next);
});

test("partial scan updates received entries and retains unverified entries", () => {
  const old = service();
  const other = service({ id: "TCP-4000-11", port: 4000 });
  const next = service({ cwd: "/updated" });
  const result = mergeScannedServices([old, other], [next], false);
  assert.equal(result.length, 2);
  assert.equal(result[0], next);
  assert.equal(result[1], other);
});

test("completed scan with an unidentified process removes expired ports and retains its protected row", () => {
  const old = service();
  const expired = service({ id: "TCP-4000-11", port: 4000, pid: 11 });
  const next = service({ cwd: "/updated" });
  const unresolved = service({
    id: "UDP-54450-11288", port: 54450, protocol: "UDP", pid: 11288,
    process_name: "PID 11288", service_type: "unknown", service_name: "Unresolved",
    safety_level: "danger", can_terminate: false,
  });
  const result = mergeScanResult([old, expired], {
    scan_id: "completed-with-unidentified-process", total: 2, skipped: 1,
    services: [next, unresolved],
  });

  assert.equal(result.length, 2);
  assert.equal(result[0], next);
  assert.equal(result[1], unresolved);
  assert.equal(result.some((entry) => entry.id === expired.id), false);
  assert.equal(result[1].can_terminate, false);
});

test("only complete scans remove absent entries, including a successful empty scan", () => {
  const old = service();
  const other = service({ id: "TCP-4000-11", port: 4000 });
  assert.equal(mergeScannedServices([old, other], [old]).length, 1);
  assert.equal(mergeScannedServices([old], []).length, 0);
  assert.equal(mergeScannedServices([old], [], false)[0], old);
});

test("unchanged snapshots preserve state identity and duplicate IDs are cleaned", () => {
  const old = service();
  const prev = [old];
  assert.equal(mergeScannedServices(prev, [service()]), prev);
  assert.equal(mergeScannedServices([old, old], [old, old]).length, 1);
});

test("numeric search matches exact ports, excluding commands and PID matches", () => {
  assert.equal(matchesSearch(service(), "3000"), true);
  assert.equal(matchesSearch(service({ port: 13000 }), "3000"), false);
  assert.equal(matchesSearch(service({ port: 4000, pid: 3000, command_line: "node --port 3000" }), "3000"), false);
  assert.equal(matchesSearch(service(), " 3000 "), true);
  assert.equal(matchesSearch(service(), "pid:10"), true);
  assert.equal(matchesSearch(service(), "port:3000"), true);
});

test("text search retains project and command search and supports addresses and protocol", () => {
  assert.equal(matchesSearch(service(), "DEMO"), true);
  assert.equal(matchesSearch(service(), "app.js"), true);
  assert.equal(matchesSearch(service(), "127.0.0.1"), true);
  assert.equal(matchesSearch(service(), "UDP"), false);
});

test("port sorting is stable across protocols and keeps bookmarks first", () => {
  const low = service({ port: 80, pid: 30 });
  const high = service({ port: 9000, pid: 20 });
  const tcp = service();
  const udp = service({ protocol: "UDP" });
  const sorted = sortServices([udp, high, tcp, low], new Set([9000]), "port");
  assert.equal(sorted[0], high);
  assert.equal(sorted[1], low);
  assert.equal(sorted[2], tcp);
});

test("batch operations deduplicate processes across their ports", () => {
  const samePid = service({ id: "TCP-4000-10", port: 4000 });
  const differentPid = service({ id: "TCP-5000-20", port: 5000, pid: 20 });
  assert.equal(uniqueProcesses([service(), samePid, differentPid]).length, 2);
});

test("project groups distinguish identical folder names by their full paths", () => {
  const first = service({ cwd: "/work/client/app" });
  const second = service({ id: "TCP-4000-11", cwd: "/work/server/app" });
  const third = service({ id: "TCP-5000-12", cwd: "/work/client/app/" });
  const groups = groupServices([first, second, third], true);
  assert.equal(groups.length, 2);
  assert.equal(groups[0].name, "app");
  assert.notEqual(groups[0].key, groups[1].key);
  assert.equal(groups[0].services[1], third);
});

test("grouping keeps roots and system services visible in the last other group", () => {
  const rows = [service({ cwd: "/" }), service({ cwd: "", id: "unknown" }),
    service({ cwd: "C:\\", id: "root" }), service({ cwd: "/System/Library", service_type: "system-service", id: "system" }),
    service({ cwd: "C:\\work\\client\\", id: "project" })];
  const groups = groupServices(rows, true);
  assert.equal(groups.length, 2);
  assert.equal(groups[0].path, "C:/work/client");
  assert.equal(groups[1].key, "other");
  assert.equal(groups[1].services.length, 4);
  assert.equal(new Set(groups.flatMap((group) => group.services)).size, rows.length);
});

test("flat view retains sort order and project moves use the fresh snapshot", () => {
  const old = service({ cwd: "/old-project" });
  const next = service({ cwd: "/new-project" });
  const current = mergeScannedServices([old], [next]);
  assert.equal(groupServices(current, true)[0].path, "/new-project");
  assert.equal(groupServices(current, false)[0].services, current);
});
