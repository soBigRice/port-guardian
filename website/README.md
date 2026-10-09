# Port Guardian 官网

官网：[中文](https://sobigrice.github.io/port-guardian/) / [English](https://sobigrice.github.io/port-guardian/en/)。本模块只提供产品介绍、演示和已发布安装包下载，不调用桌面 IPC，不更改应用构建、签名或 Release 流程。

## 实现入口与部署

`website/content.mjs → scripts/build-website.mjs → website/dist/{index.html,en/index.html} → .github/workflows/pages.yml → GitHub Pages`

- `index.html` 是共享语义化模板；`content.mjs` 集中维护两种语言。中文位于根路径，英文位于 `/en/`。语言链接和静态资源均兼容项目站点的 `/port-guardian/` 前缀，分别设置 title、description、canonical 和 hreflang。
- `site.css` 管理浅灰底、蓝色强调、字体、布局和响应式规则；手机自然改为纵向布局，尊重减少动效设置。
- `site.js` 的 `port-demo` 只在网页内切换“占用 / 可用”演示状态，明确标注不会访问本机进程。
- 应用图标复用 `src-tauri/icons/128x128.png`；产品图复用 `docs/design/project-workspace-implemented.png`，明确属于演示数据。英文页面注明截图是中文界面，应用本身支持英文。不使用生成概念图冒充应用截图。
- 构建脚本通过 GitHub 官方 API 验证最新稳定 Release 的 universal DMG、Windows x64 EXE 与非空大小，再一起输出版本及链接；缺少安装包或查询失败时终止构建。脚本仅重建 `website/dist/`，不安装依赖。
- 浏览器用有界 5 秒请求更新版本和下载链接，两个平台验证通过才一起更新。限流、超时或禁用 JS 时沿用已验证的构建快照，仍可正常下载；始终保留 GitHub Releases 入口。
- 网站相关文件推送至 `main` 时自动部署，也可以手动运行 Pages workflow。新桌面版本无需等待网站重新发布，网页会查询最新 Release。首次启用 Pages 的发布源应设为 GitHub Actions。

构建命令：`node scripts/build-website.mjs`，Node.js ≥ 20，无额外 npm 依赖。CI 使用 Node.js 24。GitHub Actions 的 token 只在构建进程读取，绝不写入输出。

部署采用 [GitHub 官方 Pages 工作流](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages)，核查日期 2026-10-08。此站为纯静态展示，不需要服务端或新增托管平台；不将原桌面 Vite 入口作为网页发布。

## 视觉与验证记录（2026-10-08）

视觉方向由内置 Image Gen 生成，参考图：`docs/design/website-concept.png`。提示词核心约束：完整紧凑单页、浅灰白底、黑色大字与钴蓝强调、`:3000` 细网格、现有产品截图、开放三列功能、双平台下载、安装提示和中英文；禁止虚构指标、渐变、3D 素材和卡片矩阵。

使用 Codex 内置浏览器直接渲染并操作，未使用 Playwright 独立浏览器替代。用 `view_image` 对照概念与最终渲染；概念原尺寸 904 × 1740，额外核查 904px 宽布局；实际桌面视口 1280 × 720，并检查 390 × 844、320px 手机布局。两种语言均无横向溢出，应用图片成功加载。视觉审阅仍由用户决定。

| 对照项 | 概念 / 最终实现核对 |
| --- | --- |
| 配色 | 保留 `#f7f8fa`、深色文字、钴蓝强调，无额外背景滤镜 |
| 首屏排版 | 大字左侧、细网格端口右侧；手机自然堆叠 |
| 首屏文案 | 中文两行主标题、说明、导航与 CTA 保持；英文完整翻译 |
| 产品预览 | 保持独立宽图与居中标题，使用真实应用演示截图替换生成 UI |
| 功能段落 | 三列开放布局与竖向分隔，手机变为编号纵向列表 |
| 下载区 | 两个平台独立入口，实际 Release 链接、架构与版本可见 |
| 安装提示 | 原生 details 可展开和收起；说明 macOS 未签名 / 公证和 Windows SmartScreen |
| 图标与演示 | 复用原应用图标；原稿中错误的 EADDRINUSE + 可用状态改为占用初态，点击后才变可用，新增纯演示说明 |

上述最后一行、真实截图及下载架构/版本为有意调整，以保持产品事实准确；删除概念中无意义的小英文标签。未发现需要修复的布局偏差。

已执行：脚本语法检查、双语生产输出、两种语言双向切换、下载锚点、演示释放与重置、安装提示展开收起、桌面与手机截图检查、GitHub Release 安装包来源核对、浏览器 error/warn 检查（无输出）。静态 HTML 已包含下载链接及完整文案，核心内容不依赖 JS。未重新测试不受本次修改影响的桌面扫描与终止功能。

## 应用与官网图标统一（2026-10-09）

用户确认 `docs/design/icon-modern-proposal.png` 后，以该文件原样替换 `icon/icon.png`，使用现有 Tauri icon 命令导出应用资源。官网无需另存独立图标：构建脚本仍从 `src-tauri/icons/128x128.png` 复制，中文、英文的导航、页脚及 favicon 自动统一；该资源修改会触发 Pages workflow。

已重新构建双语输出，实际核对桌面 1280 × 720 与手机 390 × 844 的图标、语言切换和水平溢出。新图标加载成功，`docs/design/website-implemented.png` 更新为当前桌面渲染。安装包链接仍指向经过验证的最新稳定 Release；更换官网图标不会修改历史发行安装包。应用导出和本机安装证据见根目录 `design-qa.md`。
