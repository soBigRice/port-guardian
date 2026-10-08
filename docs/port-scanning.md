# 端口扫描、刷新与终止

核对日期：2026-10-08。基于 `4d2a2db` 上的本次未提交修改；自动验证通过，桌面主观体验及 Windows 实机仍待验收。

## 入口和职责

| 模块 | 入口 | 职责 |
| --- | --- | --- |
| `src/hooks/usePortScan.ts` | `usePortScan` / `refresh` / `removeProcess` | 监听就绪后首扫、流式首屏、完整快照收尾、轮询、错误状态与清理 |
| `src/utils/scanState.ts` | `mergeScannedServices` / `matchesSearch` / `sortServices` / `uniqueProcesses` / `groupServices` | 更新与去重、查询、排序、批量按 PID 去重、按完整路径分组 |
| `src/App.tsx` | `handleKill` / `handleBatchKill` | 筛选与详情组合、确认操作、执行反馈 |
| `src/components/PortTable.tsx` / `ServiceDetail.tsx` / `src/styles/workspace.css` | `PortTable` / `ServiceDetail` | 项目折叠、行内详情、命令复制、完整技术列与主题/窄窗样式 |
| `src-tauri/src/commands.rs` | `scan_services` / `scan_ports_stream` / `terminate_process` | 后端扫描互斥、按 PID 复用解析、事件与最终结果、执行前复核 |
| `src-tauri/src/port_scanner.rs` | `scan_listening_ports` / `add_port` | TCP LISTEN、UDP 绑定，以及地址合并 |
| `src-tauri/src/process_resolver.rs` | `resolve_process` / `get_cwd_macos` | 进程和工作目录；macOS 使用已有 `libc` 的系统 ABI 类型 |
| `src-tauri/src/service_classifier.rs` | `classify` | 进程名、命令和来源分类；端口号本身不构成数据库或基础设施身份 |

## 扫描链路和不变项

`监听全部就绪 → refresh(scanId) → scan_ports_stream → spawn_blocking → ScanGuard → prefetch → 扫描套接字 → 本轮 PID 解析缓存 → 分类/风险 → ScanResult`

- `scan-start` 和 `scan-progress` 携带 `{scan_id,total,processed,skipped}`；`port-found` 携带 `{scan_id,service}`。只接收当前扫描 ID。
- 首屏每 150 ms 批量写入流式条目。后续扫描保留已有列表，直到命令返回完整 `{scan_id,total,skipped,services}` 快照。
- 最终快照不依赖事件送达顺序或完整性。不要恢复“空的完成事件到达就用 pending 替换列表”的路径。
- 事件监听注册失败时清理已注册监听，仍通过命令完整返回值完成扫描和后续刷新。
- 正常完整扫描可以删除不存在的条目，并更新同 ID 的所有字段；无变化时保留对象和数组引用。
- 解析被跳过的部分扫描仅更新已收到条目，保留未核实的旧条目并明确警告。启动失败和 30 秒超时不替换后续刷新中的旧列表，也不更新“上次完整刷新”时间。
- 每条记录的 ID 仍为 `protocol-port-pid`。多个 IPv4/IPv6 地址合并到 `local_address`，不增加重复服务行。
- 排除系统返回的未绑定端口（local_port 为 0）；这类套接字没有可释放的端口，不应展示为服务。
- 手动刷新接管正在执行的后台扫描并显示反馈，保留详情、多选和筛选。可见窗口在首扫结束后每 10 秒轮询；隐藏或失焦暂停。
- watchdog 不会取消系统查询。后端 `ScanGuard` 禁止重叠扫描；旧请求返回和晚到事件不会写入下一轮状态。
- 组件卸载时注销监听并关闭 watchdog、流式 flush 和轮询。未完成的命令返回后不写入已卸载组件。

## 查询和界面

纯数字是精确端口查询；`port:3000` 和 `pid:123` 也是精确查询。其他文字继续搜索命令、目录、名称、来源，并支持地址、协议和用户。默认按端口排序，收藏优先；也可按项目目录、进程名或 PID 排序。TCP/UDP 默认都保留，可单独筛选。

默认项目工作区由 `App` 组合搜索、协议切换、视图选择和筛选菜单；`PortTable` 调用 `groupServices` 按完整 cwd 分组，同名目录不合并。去除尾部分隔符并兼容 Windows 路径，根目录、空目录及系统/应用服务归入“系统与其他”，不丢弃条目。组内沿用排序结果，“系统与其他”最后展示；平铺视图保留完整排序顺序。

`PortTable` 点击端口 → 同一行下方 `ServiceDetail` 展开目录、PID、协议、全部绑定地址、来源和完整命令。长命令可滚动与复制，“更多信息”提供可执行路径、用户、风险依据及进程链。刷新同 ID 的 cwd 改变时跟随新的项目组。箭头键按分组后顺序移动并打开目标组；新搜索自动展开组，仍可手动折叠当前结果。行选择与批量勾选独立，查看详情不会自动选中终止目标。

“详细列”显示独立 PID、命令与目录列，使用至少 1400 px 的可横向滚动表格，避免固定列宽之和挤掉进程列。风险、类型、排序和详细列保留在筛选菜单；菜单外点击、Escape 和选择导出后关闭浮层。首帧等待监听注册时已经显示扫描状态，不误报“扫描完成”。顶部保留完整刷新时间，底部显示扫描进度、部分/失败警告；刷新期间不清空行内详情。

浅色为新用户默认；已有主题、收藏与语言偏好继续保留。深色、跟随系统、中英文均可切换。macOS `Overlay` 标题栏保留原生交通灯；只有标题区域标记 `data-tauri-drag-region`，主窗口仅增加 `core:window:allow-start-dragging` 以支持拖动。Windows 标题栏效果尚未实机验证。

图标选用 `@phosphor-icons/react 2.1.10`（MIT），按图标直接导入；真实来源仍优先使用系统应用图标，库图标只作为不可读取时的回退。选择理由是现有 React 组件直接可用、单一图标风格及小范围依赖；未为常规图标自绘素材或引入额外 UI 框架。核查日期 2026-10-08，来源：[官方仓库](https://github.com/phosphor-icons/react)、[Tauri WindowConfig](https://v2.tauri.app/reference/config/#windowconfig)。视觉目标见 `docs/design/project-workspace.png`，对照与交互记录见根目录 `design-qa.md`。

## 终止链路

`单条/批量确认 → terminate_process(pid,force,port,protocol,expectedExecutablePath,expectedCommandLine) → 复核端口归属 → 重新解析身份和风险 → 发信号 → 等待退出 → 核对所选协议端口 → 前端按 PID 移除全部行 → 再扫描`

- 批量先展示去重后的进程目标；未知服务需要输入端口确认。默认普通终止，只有用户勾选才强制终止。
- 普通终止不自动升级为强杀。超时未退出返回失败，保留条目，并允许重新确认强制终止。
- 终止命令在独立异步任务执行，进程退出等待不占用桌面 UI 线程。
- 后端拒绝系统根 PID、本工具、被识别为危险的服务、已不拥有所选端口的进程、其他用户进程，以及与预期可执行路径不符的进程（无法取得预期路径时比较命令）。身份检查不是操作系统提供的原子进程句柄，不能承诺完全消除 PID 复用竞态。
- “进程退出”和“端口可用”分别核实，避免自动重启或其他进程接管端口时错误报告释放。
- 同 PID 的多个端口只发一次终止请求。进行中的旧扫描不得重新插入已经退出的 PID；其后排队复扫确认当前状态。
- 单条失败和批量部分失败都显示原因，批量不会静默吞掉失败。
- Windows 当前采用 `taskkill` / `/F`。普通模式失败后应由用户明确选择强制模式；本次没有 Windows 实机验证。

## 已验证问题和防复发

| 关键词 | 已证实原因 | 修复及优先检查 |
| --- | --- | --- |
| macOS 工作目录全空 | 手写缓冲区 2336 字节，系统 ABI 要求 2352 字节，调用返回 0 | 使用 `libc::proc_vnodepathinfo`，要求完整返回尺寸；回归断言当前进程目录等于真实 cwd。以后优先查 ABI、返回长度和实际原始数据，不先调 UI |
| 刷新后信息旧/条目丢失 | 只比较 ID 集合并复用旧字段；失败也当完整快照合并 | 明确 complete/partial，更新字段并保留未核实条目；见前端回归 |
| 数据库误分类 | 普通开发进程被常见端口规则提前归类 | 删除仅凭端口的身份推断；覆盖 Vite 在 5432/6379/9000/11434、真实 PostgreSQL 自定义端口和未知进程 |
| 双栈地址丢失 | `protocol-port-pid` 去重时丢弃后续地址 | 保留合并记录，同时聚合地址；保留历史防重复行约束 |
| 列出“端口 0” | 系统枚举包含未绑定 UDP 套接字，`lsof` 对应 `*:*` | 在记录合并入口排除 local_port 为 0，测试覆盖；先核对系统原始输入，勿仅在 UI 隐藏 |
| 批量/退出状态误导 | 按端口重复终止 PID，默认强杀，仅删除一行，把信号发送成功当退出 | 按 PID 去重、明确确认、等待实际退出、全部 PID 行移除与复扫 |

## 验证

- `npm test`：`tests/scan-state.test.mjs`，12 项状态、查询、排序、PID 去重和完整路径分组回归。
- `cargo test --locked --lib`：13 项 Rust 回归，包括真实 macOS 当前目录读取、未绑定套接字过滤、双栈地址聚合、分类和终止保护。
- `npm run build`：TypeScript 和 Vite 构建。
- 本机真实扫描成功读取工作目录，同进程双栈地址保留。服务数量随本机运行状态变化，本次未进行相同负载的性能基准对比。
- 两个临时监听子进程：陈旧身份请求被拒绝；正常 SIGTERM 退出；忽略 SIGTERM 时返回失败且进程继续存活；明确 force 后退出。子进程与临时文件均在检查后清理。
- 浏览器使用隔离 IPC fixture 验证首屏、刷新保留、失败/部分扫描、30 秒超时、事件丢失/监听注册失败、旧事件隔离、筛选及批量确认；不将 fixture 视为桌面 IPC 或 Windows 实机验收。
- 本地 macOS `.app` 测试包通过构建，使用一次性配置覆盖关闭 updater 产物生成（本地无更新签名私钥），项目发布配置保持原样。
- 真实 macOS 桌面窗口确认首扫、精确查询 5173、开发服务目录/命令详情和手动刷新保留详情；最终测试包排除未绑定的“端口 0”，保留 TCP/UDP 服务。测试应用、浏览器回归页和临时服务均已关闭，隔离 fixture 缓存及临时文件已清理。主观体验仍待用户验收。

参考：已有 `libc 0.2.186` 的 macOS 类型、[Apple proc_info.h](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/proc_info.h)、[Tauri 2 command 返回值与异步调用](https://v2.tauri.app/develop/calling-rust/)。扫描后端未新增依赖；UI 图标依赖见上文。

## UI 预览包与现有安装的边界

本地调试包和已安装应用共用 `com.port-guardian.app` 时，窗口工具可能读到其他实例；本次曾观察到测试目录 `.app` 版本变为 0.2.10，且二进制 SHA 与刚构建的调试二进制不同。不能将该窗口当作当前代码证据。预览构建使用临时 CLI 配置：`productName=Port Guardian UI Preview`、`identifier=com.port-guardian.ui-preview`、updater endpoint 指向本机未监听的 HTTPS 端口并关闭 updater 产物生成。这些覆盖仅用于本地验证，仓库发布标识、版本和更新地址保持原样。核对 `.app` Info.plist 版本及与编译输出相同的二进制 SHA 后，才检查首扫、查询、详情和标题栏拖动。

最终 UI 验证：12 项前端回归通过，浏览器精确搜索、折叠/平铺、技术列、复制、批量确认、JSON 导出、失败保留及 800×600 中英文/深色布局通过；控制台无 error/warn。独立 macOS 预览包版本 0.2.8、与编译输出 SHA 一致，真实扫描、1439 查询、长路径/命令、标题拖动操作及刷新后保留详情通过。临时服务、下载、页面、测试进程和 fixture 缓存已清理；主观效果待用户验收。
