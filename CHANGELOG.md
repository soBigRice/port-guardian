# Changelog

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
