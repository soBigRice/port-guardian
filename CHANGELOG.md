# Changelog

## [0.2.14] - 2026-10-10

### 🐛 Bug Fixes

- 修复 Windows 进程查询没有执行时限的问题；为查询增加本轮扫描预算，超时后停止并回收查询子进程，避免旧任务长期占用扫描锁。
- 批量进程查询失败时显示具体错误并保留现有结果，避免静默转入大量逐 PID 和父进程查询。
- 完善大量进程信息的输出读取，防止查询管道阻塞；增加超时恢复、进程回收和 Windows PowerShell 错误反馈回归。

## [0.2.13] - 2026-10-09

### ✨ New Features

- 更新应用图标：采用钴蓝底与白色端口守护符号，统一 macOS、Windows 安装包及官网的视觉标识。
- 新增极简中英文官网，提供产品介绍、网页交互演示和最新稳定版本下载：[中文](https://sobigrice.github.io/port-guardian/) / [English](https://sobigrice.github.io/port-guardian/en/)。

## [0.2.12] - 2026-10-08

### ✨ New Features

- 新增浅色项目工作区：按真实工作目录分组，支持折叠和平铺；点击端口直接展开行内详情，查看并复制完整命令。
- 优化搜索、TCP/UDP 筛选、排序和详细列；数字搜索精确匹配端口，保留深色主题、英文、收藏和导出。

### 🐛 Bug Fixes

- 修复 macOS 工作目录读取错误；合并同一服务的 IPv4/IPv6 地址，排除未绑定的端口 0。
- 修复扫描收尾、旧事件和刷新数据竞态；失败、超时及部分扫描保留未核实结果，刷新后保留当前详情与选择。
- 修复仅凭常见端口把开发服务误判为数据库或基础设施的问题。
- 加强单条和批量终止确认，按 PID 去重并复核实时身份；普通终止不自动强杀，成功退出后清理该 PID 的全部端口行。

### 🔧 Other

- 保留 v0.2.10 的 Windows 原生用户/图标查询、轻量进程链缓存、Unresolved 端口保护及后续刷新批量结果。
- 发布前运行前端和 Rust 回归；明确测试文件路径，兼容 Windows npm / Node 20；更新扫描逻辑说明和 UI 验证记录。


## [0.2.11] - 2026-10-08（未发布）

- Windows 回归入口的通配符未被 npm 展开，发布被自动拦截。本次改动在 0.2.12 重新验证后发布。

## [0.2.10] - 2026-07-03

### 🚀 Performance

- 优化 Windows 扫描链路：父进程链只读取缓存中的轻量字段，避免递归触发 owner/cwd 补齐。
- Windows owner 查询改为原生 Token API，减少 PowerShell / WMI 调用导致的扫描卡顿。
- 首屏之后的手动刷新和静默刷新改为批量 `scan-results`，降低端口多时的 IPC 和前端事件处理开销。

### 🐛 Bug Fixes

- 修复 Windows 上扫描可能停在少量端口、后续不继续展示的问题。
- 修复进程解析失败时端口被直接隐藏的问题；现在会显示为 `Unresolved`，并禁止终止。
- 修复 Windows 批量进程缓存命中后 `user` / `cwd` 为空，导致目录展示和安全判断不准确的问题。
- 修复同一端口/PID 在刷新后字段变化但前端仍保留旧详情的问题。

### 🔧 Other

- 补充 Windows 扫描卡顿、缓存字段和刷新 diff 的问题记录。
- 同步更新 README Mermaid 数据流，记录 `Unresolved` 安全分支。

## [0.2.9] - 2026-07-02

### 🐛 Bug Fixes

- 修复端口列表偶发重复显示同一 `端口 + PID + 协议` 服务的问题；扫描结果写入前端状态时统一按服务 id 去重。
- 修复首屏扫描和静默轮询的竞态问题；扫描现在只由 `scan-complete` 正常收尾，并通过 watchdog 兜底异常卡住场景。
- 修复 `tsx` / Node 启动命令中 `file://` 中文路径显示为 `%E5...` 百分号编码的问题。
- 修复 macOS `ps -o comm=` 返回路径片段时进程名显示成 `/Users/superrice` 或完整可执行路径的问题。

### 🔧 Other

- 补充扫描数据流 Mermaid 图和相关开发问题记录，避免后续重复踩坑。

## [0.2.8] - 2026-07-01

### 🐛 Bug Fixes

- 修复更新弹窗只显示版本日期的问题；Release workflow 现在会完整提取当前版本的 changelog，并写入 `latest.json.notes`。
- 延续 `0.2.7` 中 macOS 打包版“启动命令”中文路径乱码修复，确保从旧版本直接更新时也能拿到包含该修复的最新安装包。

## [0.2.7] - 2026-07-01

### 🐛 Bug Fixes

- 修复 macOS 打包版“启动命令”中文路径显示为 `M-xx` 乱码的问题；后端调用 `ps` 时显式设置 UTF-8 locale，避免 Finder 启动环境缺少 `LANG` / `LC_ALL`。

## [0.2.6] - 2026-07-01

### 🐛 Bug Fixes

- 修复 Tauri 2.11 updater `pubkey` 格式错误问题；`plugins.updater.pubkey` 改为 `.pub` 文件整体内容的 base64 字符串，并增强 Release workflow 对私钥和公钥格式的前置校验。

## [0.2.5] - 2026-07-01

### 🐛 Bug Fixes

- 修复 Release workflow 对 updater 签名私钥格式校验不足的问题；提前拦截整段 minisign 私钥文件、空格、换行或非 base64 字符，避免 macOS/Windows 打包到 updater 签名阶段才失败。

## [0.2.4] - 2026-06-30

### 🐛 Bug Fixes

- 修复 updater 签名私钥格式问题（base64 空格）

## [0.2.3] - 2026-06-30

### 🐛 Bug Fixes

- 修复 updater 签名密钥不匹配问题，重新生成密钥对

## [0.2.2] - 2026-06-30

### 🐛 Bug Fixes

- 修复 CI 构建中 CHANGELOG.md 路径找不到的问题

## [0.2.1] - 2026-06-30

### 🐛 Bug Fixes

- **macOS 打包后路径不显示**: 用 `proc_pidinfo` / `proc_pidpath` 系统调用替代 `lsof` 子进程，修复 Hardened Runtime 下无法读取其他进程信息的问题；中文路径正常显示
- **Windows 中文路径支持**: 通过 NT API (`NtQueryInformationProcess` + `ReadProcessMemory`) 直接读取进程 PEB 获取工作目录，支持中文路径
- **README 动态化**: 版本徽章改为 shields.io 自动读取，下载链接统一指向 `releases/latest`，不再需要手动更新

### 🔧 Other

- CI workflow 自动从 CHANGELOG.md 提取更新日志填充 Release body 和 `latest.json`

## [0.2.0] - 2026-06-30

### 🚀 Performance

- **macOS 端口扫描原生化**: 统一使用 `netstat2` API 替代 `lsof` 子进程，macOS 扫描速度提升 10 倍以上；三个平台（macOS/Windows/Linux）共用同一代码路径
- **Windows 中文路径修复**: 所有 PowerShell 调用增加 `chcp 65001` UTF-8 代码页，修复中文路径在生产构建中显示为乱码的问题
- **增量刷新优化**: 手动刷新不再清空列表后逐条重建，改为后台扫描完成后一次性 diff 替换；无变化时零重渲染

### ✨ New Features

- **UDP 端口扫描**: 扩展支持 UDP 绑定端口扫描，TCP + UDP 全覆盖；端口号旁显示紫色 UDP 标记
- **更新日志 Markdown 渲染**: 更新窗口支持渲染 GitHub Release 的 Markdown 格式更新日志（标题、列表、代码块、链接等）
- **快捷键体系**:
  - `R` / `F5` → 刷新
  - `/` / `Ctrl+K` → 聚焦搜索框
  - `↑` / `↓` → 切换选中行
  - `K` / `Delete` → 终止选中服务
  - `1-9` → 切换筛选器
  - `Escape` → 逐层关闭面板
- **一键批量终止**: 表格行首添加多选 checkbox，toolbar 显示「批量终止 (N)」按钮，支持批量强制终止
- **导出功能**: 右上角「导出」按钮，支持 CSV / JSON 两种格式下载
- **收藏/置顶端口**: 端口号旁添加星标收藏，收藏端口自动置顶；收藏状态持久化到 localStorage
- **进程树可视化**: 详情面板的进程链改为树形缩进展示，显示 PID，当前进程高亮标记

### 🐛 Bug Fixes

- 修复 `is_port_listening` 在 Unix 端仍使用 `lsof` 的问题，统一改为 `netstat2` API，支持 TCP + UDP
- 修复 `id` 字段未包含协议导致同一端口 TCP/UDP 冲突的问题
- 修复 ↑↓ 导航错误地调用 `setServices` 触发无意义 re-render 的问题
- 修复搜索/筛选切换时 `flushPendingRef` 在非流式模式下注入不完整数据的问题
- 修复重复点击刷新时 early return 路径残留旧 scan pending 数据的问题

### 🔧 Other

- 智能轮询间隔从 3 秒调整为 10 秒
- 移除不可靠的浏览器 Notification API 通知功能（后续将用 Tauri 原生通知插件替代）
