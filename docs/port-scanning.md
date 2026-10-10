# 端口扫描、刷新与终止

核对日期：2026-10-10。v0.2.14 已公开，扫描与终止实现继承 v0.2.12，已合入 `b19ef86`（v0.2.10）的兼容修复。Windows 查询超时修复通过本机和 Windows CI 自动验证，发行包及更新签名核验通过；报告问题的 Windows 机器仍待更新后复测，发布核验见文末。

## 入口和职责

| 模块 | 入口 | 职责 |
| --- | --- | --- |
| `src/hooks/usePortScan.ts` | `usePortScan` / `refresh` / `removeProcess` | 监听就绪后首扫、流式首屏、完整快照收尾、轮询、错误状态与清理 |
| `src/utils/scanState.ts` | `mergeScannedServices` / `matchesSearch` / `sortServices` / `uniqueProcesses` / `groupServices` | 更新与去重、查询、排序、批量按 PID 去重、按完整路径分组 |
| `src/App.tsx` | `handleKill` / `handleBatchKill` | 筛选与详情组合、确认操作、执行反馈 |
| `src/components/PortTable.tsx` / `ServiceDetail.tsx` / `src/styles/workspace.css` | `PortTable` / `ServiceDetail` | 项目折叠、行内详情、命令复制、完整技术列与主题/窄窗样式 |
| `src-tauri/src/commands.rs` | `scan_services` / `scan_ports_stream` / `terminate_process` | 后端扫描互斥、按 PID 复用解析、事件与最终结果、执行前复核 |
| `src-tauri/src/port_scanner.rs` | `scan_listening_ports` / `add_port` | TCP LISTEN、UDP 绑定，以及基于索引的地址合并 |
| `src-tauri/src/process_resolver.rs` | `resolve_process` / `get_cwd_macos` | 进程和工作目录；macOS 使用已有 `libc` 的系统 ABI 类型 |
| `src-tauri/src/windows_command.rs` | `QueryBudget` / `powershell_output` / `output_with_timeout` | Windows 查询的本轮预算、执行时限、错误上下文与子进程/输出线程回收 |
| `src-tauri/src/service_classifier.rs` | `classify` | 进程名、命令和来源分类；端口号本身不构成数据库或基础设施身份 |

## 扫描链路和不变项

`监听全部就绪 → refresh(scanId) → scan_ports_stream → spawn_blocking → ScanGuard → prefetch → 扫描套接字 → 本轮 PID 解析缓存 → 分类/风险 → ScanResult`

- `scan-start` 和 `scan-progress` 携带 `{scan_id,total,processed,skipped}`；`port-found` 携带 `{scan_id,service}`。只接收当前扫描 ID。
- 首屏每 150 ms 批量写入流式条目。后续扫描传 `stream: false`，仅起止进度及完整命令快照跨 IPC；保留已有列表，直到命令返回完整 `{scan_id,total,skipped,services}` 快照。
- 最终快照不依赖事件送达顺序或完整性。不要恢复“空的完成事件到达就用 pending 替换列表”的路径。
- 事件监听注册失败时清理已注册监听，仍通过命令完整返回值完成扫描和后续刷新。
- 正常完整扫描可以删除不存在的条目，并更新同 ID 的所有字段；无变化时保留对象和数组引用。
- 无法解析进程时显示不可终止的 `Unresolved` 行。解析不完整的部分扫描仅更新已收到条目，保留未核实的旧条目并明确警告。启动失败和 30 秒超时不替换后续刷新中的旧列表，也不更新“上次完整刷新”时间。
- Windows 扫描沿用 v0.2.10 的轻量父进程缓存及原生用户/图标查询；终止前通过 `resolve_process_for_termination` 绕过 30 秒缓存读取实时身份。
- 每条记录的 ID 仍为 `protocol-port-pid`。多个 IPv4/IPv6 地址合并到 `local_address`，不增加重复服务行。
- 排除系统返回的未绑定端口（local_port 为 0）；这类套接字没有可释放的端口，不应展示为服务。
- 手动刷新接管正在执行的后台扫描并显示反馈，保留详情、多选和筛选。可见窗口在首扫结束后每 10 秒轮询；隐藏或失焦暂停。
- 前端 watchdog 不会取消系统查询。后端 `ScanGuard` 禁止重叠扫描；旧请求返回和晚到事件不会写入下一轮状态。Windows 的 PowerShell 查询由后端管理超时和回收，规则见下节；原生系统 API 仍不支持由该 watchdog 强制取消。
- 组件卸载时注销监听并关闭 watchdog、流式 flush 和轮询。未完成的命令返回后不写入已卸载组件。

## Windows 查询超时与恢复

`scan_services → ScanGuard → QueryBudget(25 秒) → prefetch_all_processes → powershell_output → output_with_timeout → 完整缓存 → 端口/PID/父进程解析`

- 同一个扫描工作线程内的 PowerShell 查询共享 25 秒预算；单次查询也最多 25 秒，实际使用本轮剩余预算，不对每个 PID 重新计算完整预算。保留接近既有 30 秒前端等待上限的处理窗口，并为回收和 IPC 收尾预留时间。`Get-CimInstance` 同时指定 `-OperationTimeoutSec 20 -ErrorAction Stop`。预算耗尽后不再启动查询，每个端口处理前及最终返回前检查预算。预算只限定查询及后续处理入口，不承诺原生套接字、token/账户或 PEB API 的绝对执行时限。
- 不再只依靠前端的 30 秒计时器：阻塞查询超过执行时限后，停止本工具创建的 PowerShell 子进程并等待回收，同时回收 stdout/stderr 读取线程，再沿错误返回路径释放 `ScanGuard`。不终止被扫描的用户进程。
- 等待查询时并发读取两个输出管道，防止全量进程快照超过管道容量后出现“等待退出，但进程正在等待写入”的死锁。不产生临时输出文件。
- 批量查询启动失败、执行超时、退出码非零或没有可识别记录，均返回包含查询阶段的错误；不更新缓存时间，也不静默转成逐 PID/父进程的连续查询。前端现有失败分支保留旧列表和上次完整刷新时间。
- `QueryBudget` 退出时恢复线程原有预算，不影响其他线程的终止身份复核。保留 30 秒扫描缓存、轻量父进程读取、原生 owner/cwd、终止前绕过缓存的身份检查及普通/强制终止选择。

2026-10-10 用户反馈：最新版 Windows 显示“扫描超时，已保留现有结果”。代码已证实此前所有 PowerShell 查询使用无执行时限的 `.output()`，批量失败被吞掉，前端超时不能释放仍在运行的后端扫描锁。缺少该 Windows 机器的查询耗时和错误，**尚不能确认此次实际阻塞发生在 WMI、某个 PID/父进程还是原生系统 API**。不能把有缺陷的超时机制直接写成这台机器的已复现根因，也不能用延长前端等待时间代替查询生命周期修复。

本机先用原有 `.output()` 行为运行模拟阻塞回归，测试失败；修复后验证超时回收、查询进程已退出、下一次查询成功、双管道各 1 MiB 输出、非零退出码/stderr 保留、预算截止及恢复，均通过。Windows 专用的真实 PowerShell 超时恢复与错误上下文用例已在 v0.2.14 的 Windows CI 执行通过；本机没有 Windows 工具链或运行环境，不能将 CI 结果代替用户机器复测。

下次同类故障优先看错误中的查询阶段和 PID，再区分批量 WMI、单 PID、父进程及原生账户查询；检查失败后是否还有查询子进程与扫描锁，避免叠加刷新。实现依据：[Microsoft Windows PowerShell 5.1 Get-CimInstance](https://learn.microsoft.com/en-us/powershell/module/cimcmdlets/get-ciminstance?view=powershell-5.1)、[Rust Child 的 kill/wait/try_wait 生命周期](https://doc.rust-lang.org/std/process/struct.Child.html)。没有新增依赖或改变发布配置。

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
- `cargo test --locked --lib`：2026-10-10 本机及 macOS CI 20 项 Rust 回归、Windows CI 22 项回归通过，包括真实 macOS 当前目录读取、未绑定套接字过滤、双栈地址聚合、分类、Unresolved 行、终止保护、查询超时/回收及 Windows PowerShell 超时恢复和异常反馈。
- `npm run build`：TypeScript 和 Vite 构建。
- 本机真实扫描成功读取工作目录，同进程双栈地址保留。服务数量随本机运行状态变化，本次未进行相同负载的性能基准对比。
- 两个临时监听子进程：陈旧身份请求被拒绝；正常 SIGTERM 退出；忽略 SIGTERM 时返回失败且进程继续存活；明确 force 后退出。子进程与临时文件均在检查后清理。
- 浏览器使用隔离 IPC fixture 验证首屏、刷新保留、失败/部分扫描、30 秒超时、事件丢失/监听注册失败、旧事件隔离、筛选及批量确认；不将 fixture 视为桌面 IPC 或 Windows 实机验收。
- 本地 macOS `.app` 测试包通过构建，使用一次性配置覆盖关闭 updater 产物生成（本地无更新签名私钥），项目发布配置保持原样。
- 真实 macOS 桌面窗口确认首扫、精确查询 5173、开发服务目录/命令详情和手动刷新保留详情；最终测试包排除未绑定的“端口 0”，保留 TCP/UDP 服务。测试应用、浏览器回归页和临时服务均已关闭，隔离 fixture 缓存及临时文件已清理。主观体验仍待用户验收。

参考：已有 `libc 0.2.186` 的 macOS 类型、[Apple proc_info.h](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/proc_info.h)、[Tauri 2 command 返回值与异步调用](https://v2.tauri.app/develop/calling-rust/)。扫描后端未新增依赖，保留 v0.2.10 的 Windows `systemicons` 依赖；UI 图标依赖见上文。

## UI 预览包与现有安装的边界

本地调试包和已安装应用共用 `com.port-guardian.app` 时，窗口工具可能读到其他实例；本次曾观察到测试目录 `.app` 版本变为 0.2.10，且二进制 SHA 与刚构建的调试二进制不同。不能将该窗口当作当前代码证据。预览构建使用临时 CLI 配置：`productName=Port Guardian UI Preview`、`identifier=com.port-guardian.ui-preview`、updater endpoint 指向本机未监听的 HTTPS 端口并关闭 updater 产物生成。这些覆盖仅用于本地验证，仓库发布标识、版本和更新地址保持原样。核对 `.app` Info.plist 版本及与编译输出相同的二进制 SHA 后，才检查首扫、查询、详情和标题栏拖动。

最终 UI 验证：12 项前端回归通过，浏览器精确搜索、折叠/平铺、技术列、复制、批量确认、JSON 导出、失败保留及 800×600 中英文/深色布局通过；控制台无 error/warn。独立 macOS 预览包版本 0.2.8、与编译输出 SHA 一致，真实扫描、1439 查询、长路径/命令、标题拖动操作及刷新后保留详情通过。临时服务、下载、页面、测试进程和 fixture 缓存已清理；主观效果待用户验收。

发布整合验证（2026-10-08）：最终发布代码通过 `npm test` 12 项、`npm run build`、本机 `cargo test --locked --lib` 14 项。Windows CI 增加缓存污染下实时身份读取的回归；发布工作流在两个平台运行前端及 Rust 测试，再构建安装包并签署 updater 产物。

Windows 发布回归入口（Node 20，2026-10-08）：v0.2.11 流水线日志证明 npm 的 Windows shell 把 `tests/*.test.mjs` 原样传给 Node，导致找不到测试文件；macOS shell 可展开所以本机及 macOS CI 未暴露。入口改为 `node --test tests/scan-state.test.mjs`。以后跨平台脚本避免依赖 shell 通配符展开；保留回归门槛，v0.2.11 未公开，使用新标签 v0.2.12 重建。

## v0.2.12 发行核验

2026-10-08 已公开 [v0.2.12](https://github.com/soBigRice/port-guardian/releases/tag/v0.2.12)，发行代码为 `041f751e33b1fa3a9f712f98f640b4afb1f0d38d`；[Release 流水线 37754195435](https://github.com/soBigRice/port-guardian/actions/runs/37754195435) 的 macOS、Windows 和公开发布任务均成功。两个平台各通过 12 项前端及 14 项 Rust 回归，Windows 包含扫描缓存污染下实时身份读取用例。

- 经本机现有系统代理匿名下载公开安装包和 `latest.json`；元数据版本、标签指向及完整更新日志一致。比较日志内容时统一 LF/CRLF，Windows 换行本身不构成内容丢失。
- macOS 更新包中的 `CFBundleShortVersionString=0.2.12`、`CFBundleIdentifier=com.port-guardian.app`；`lipo` 确认 Intel `x86_64` 与 Apple Silicon `arm64`。
- 六个平台元数据项对应的三个唯一更新包均通过 `minisign` 对仓库现有公钥的验证；签名附件与元数据签名一致。安装包与更新包 SHA-256 均匹配 GitHub 公布的 digest。
- v0.2.11 未公开，失败草稿已清理，保留标签和失败流水线历史。v0.2.12 发布未改写既有标签。
- 未执行 v0.2.12 的实际覆盖安装及 Windows 实机交互，不能将产物、签名或 CI 成功当作这些场景已验收。此前原生 UI 证据对应上文明确标识的预览包。

| 更新包 | SHA-256 |
| --- | --- |
| `Port.Guardian_universal.app.tar.gz` | `502434315549c43ce2016902c4ec261496a73e387ef78aa58120a200c2772f47` |
| `Port.Guardian_0.2.12_x64-setup.nsis.zip` | `29050152218b49ea64436db17428a1144e069e30cfa0cfc2a711ce7718261fe5` |
| `Port.Guardian_0.2.12_x64-setup.exe` | `40f41f18f6a0abc6f492d1144fc0c1b4106b3067b06be6f415f525541d1133a5` |

## v0.2.13 发行核验

2026-10-09 已公开 [v0.2.13](https://github.com/soBigRice/port-guardian/releases/tag/v0.2.13)，发行提交 `0515dc6ab4f85a988c920a41c84b1ba42775eea3`；[Release 流水线 37893457939](https://github.com/soBigRice/port-guardian/actions/runs/37893457939) 的 macOS、Windows 与公开发布任务均成功。此次发行包含已确认的新图标及中英文官网；本机 12 项前端回归、14 项 Rust 回归、生产构建、各版本文件与完整 changelog 提取检查通过，两平台 CI 回归和正式打包均成功。

- 八个公开资产均通过现有系统代理匿名下载，大小和 SHA-256 匹配 GitHub 公布的 digest。`releases/latest/download/latest.json` 与本次资产一致，版本 `0.2.13`；Release body 与 updater notes 均包含完整本版日志（统一 LF/CRLF 后比较）。
- 六个平台元数据项对应三个唯一更新包，通过 `minisign` 对仓库原有公钥的验证；元数据签名与 `.sig` 附件一致。
- macOS DMG 只读挂载后核对实际 `.app`，同时核对更新归档：版本 `0.2.13`、标识 `com.port-guardian.app`、Intel `x86_64` / Apple Silicon `arm64` 均正确，两者应用二进制相同；包内 `icon.icns` 与已确认导出资源逐字节一致。挂载已卸载。
- 从 Windows NSIS 安装包只读提取 `port-guardian.exe`，未执行 Windows 代码；实际应用为 x64、版本 `0.2.13.0`，PE 图标组的六个图像 payload 均与新 `icon.ico` 完全相同。应用二进制 SHA-256 为 `210bba99bb9e42794da958329224f18aa7dbdf3339bd453e54c579049af11466`。安装器版本亦为 `0.2.13.0`，legacy updater ZIP 内的安装器与公开 EXE 相同。
- [Pages 流水线 37895378115](https://github.com/soBigRice/port-guardian/actions/runs/37895378115) 成功；中英文公开 HTML 均与本地 v0.2.13 构建逐字节一致，静态下载链接也已更新，官网图标与应用导出资源一致。
- 未执行本次安装包的实际覆盖安装或 Windows 实机交互，不把产物核验和 CI 成功当作这些场景已验收。临时安装包、解包目录、校验脚本与便携工具均在记录证据后删除，未安装全局工具或清理用户已有构建缓存。

| 发行产物 | SHA-256 |
| --- | --- |
| `Port.Guardian_0.2.13_universal.dmg` | `7df04393cd8d21401a77c863b762848fb1a695e8b5b5eafa63452916db7f5858` |
| `Port.Guardian_universal.app.tar.gz` | `7830eadb6550a66fafbe2639865361a61b93a3cc3c043a56d780e403b7edb091` |
| `Port.Guardian_0.2.13_x64-setup.nsis.zip` | `14a110738b0b1b1cdb22601727b445fbc991eb21825dcc3a2c77d2dc707d87a8` |
| `Port.Guardian_0.2.13_x64-setup.exe` | `9934e1442e904726aeb71b3da161d940df4526f271cdfcb14aa317a1a7e8d2cf` |

核验经验：Windows 的 NSIS 安装器外壳图标与应用 EXE 图标分别配置。首次核验误把外层安装器默认资源当成应用图标，随后解包实际应用，六尺寸严格比对通过；v0.2.13 的外层安装器仍采用 NSIS 默认图标。下次核对应用图标应先定位包内应用，不能只查看安装器文件。资源目录读取依据 [Microsoft PE 格式](https://learn.microsoft.com/en-us/windows/win32/debug/pe-format#the-rsrc-section)，独立配置见 [Tauri installerIcon](https://v2.tauri.app/reference/config/#installericon)；解包使用 [7-Zip 官方便携版 26.04](https://www.7-zip.org/download.html)，下载 digest 与官方 Release 一致，仅在临时目录使用并清理。

下载经验：本机直连 GitHub 超时；现有代理可返回公开资产 HTTP 200，但大文件传输曾触发限时。已用断点续传完成文件并通过 digest / 签名验证；首轮并发超时的具体原因没有充分证据，不归因为软件或资产损坏。HEAD 成功不代替完整文件核验，慢链路优先复用已验证文件并续传部分文件。


## v0.2.14 发行核验

2026-10-10 已公开 [v0.2.14](https://github.com/soBigRice/port-guardian/releases/tag/v0.2.14)，发行提交 `a7f939b86af4b8946a84b2b9965f862ba8a647b8`；[Release 流水线 38027960023](https://github.com/soBigRice/port-guardian/actions/runs/38027960023) 的 macOS、Windows 和公开发布任务全部成功。两平台各通过 12 项前端回归，macOS 20 项 Rust 回归、Windows 22 项 Rust 回归通过。Windows 包含真实 PowerShell 超时后恢复、失败上下文及绕过扫描缓存的终止身份读取用例。

- 八个公开资产经现有系统代理匿名下载，实际大小和 SHA-256 全部匹配 GitHub 的 digest；Release body 和 updater notes 与本版完整 changelog 一致，比较时统一 LF/CRLF。
- `releases/latest/download/latest.json` 与 v0.2.14 的公开元数据逐字节相同；六个平台项对应三个唯一更新包，全部通过 `minisign` 对原有公钥的验证，元数据签名与 `.sig` 附件一致。
- macOS 更新归档内的实际 `.app` 版本 `0.2.14`、标识 `com.port-guardian.app`，应用二进制同时包含 Intel `x86_64` 和 Apple Silicon `arm64`。只读提取必要文件核验，没有安装或启动本次发行包。
- Windows legacy updater ZIP 内的安装器与公开 x64 EXE 的 SHA-256 相同。本机未执行 Windows 代码，也未在报告问题的机器安装或复测扫描；不能将 CI、签名或发行成功等同于该机器的问题已验收。
- [Pages 流水线 38029314959](https://github.com/soBigRice/port-guardian/actions/runs/38029314959) 成功；中英文公开页面均显示 v0.2.14，macOS/Windows 下载链接指向本版公开安装包，不再指向 v0.2.13。

| 发行产物 | SHA-256 |
| --- | --- |
| `Port.Guardian_0.2.14_universal.dmg` | `89ea2a2d9cb42a3650725c1246a5c9ff184c7d0c52ea514f1b124ae44e400f87` |
| `Port.Guardian_universal.app.tar.gz` | `37ac1562b7d13bf27d73957c868fa1bd3847d2ab092015e3ae9c3475405d0c68` |
| `Port.Guardian_0.2.14_x64-setup.exe` | `c9af7e0fc50491f8bd113ef884d2a2c56d5c7efe7a5503bcb9ff90b5a954a544` |
| `Port.Guardian_0.2.14_x64-setup.nsis.zip` | `ea8bc1b98a0ac9dfa4d866389357a21e6db7dd50777c0a9b3d894b6a16903956` |
| `latest.json` | `d832e432a445c1d377887b3c1e9b65eda9cd6eb817072ce50af8d360a9767043` |

本次临时下载、只读提取文件、签名转换、核验脚本及监视日志均已清理；所有本机测试和发布监视进程已结束。只清理本次生成的测试可执行文件，保留已有共享依赖和常规构建缓存。
