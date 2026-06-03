# 🛡️ Port Guardian

> **English Version** | **[中文版](README.md)**

<div align="center">

<img src="src-tauri/icons/icon.png" alt="Port Guardian Icon" width="96" height="96" />

**Development Port Service Identifier & Safe Cleanup Tool**

![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows-lightgrey)
![Tauri](https://img.shields.io/badge/Tauri-2-blue)
![React](https://img.shields.io/badge/React-19-61DAFB)
![Rust](https://img.shields.io/badge/Rust-2024-orange)
![License](https://img.shields.io/badge/license-MIT-green)
![Version](https://img.shields.io/badge/version-0.1.12-brightgreen)

[![Download macOS DMG](https://img.shields.io/badge/Download-macOS%20DMG-111111?logo=apple&logoColor=white)](https://github.com/soBigRice/port-guardian/releases/download/v0.1.12/Port.Guardian_0.1.12_universal.dmg)
[![Download Windows EXE](https://img.shields.io/badge/Download-Windows%20EXE-0078D4?logo=windows&logoColor=white)](https://github.com/soBigRice/port-guardian/releases/download/v0.1.12/Port.Guardian_0.1.12_x64-setup.exe)

</div>

---

## 📖 Introduction

Port Guardian is a **macOS / Windows desktop application** built with [Tauri 2](https://v2.tauri.app/) for one-click scanning, identifying, and safely cleaning up local processes occupying TCP ports.

In daily development, we often encounter the "port already in use" problem — `EADDRINUSE: address already in use :::3000`. The traditional approach is to manually run `lsof -i :3000` to find the PID and then `kill`, which is tedious and prone to accidentally terminating system processes.

Port Guardian automates all of this: **Scan → Identify → Classify → Assess Risk → Safely Terminate** — fully visualized, so you never have to worry about accidentally killing critical services.

---

## ✨ Core Features

### 🔍 Port Scanning & Process Identification
- Automatically scans all TCP listening ports on the local machine
- Resolves process information for each port (PID, process name, user, command line, working directory, executable path)

### 🌳 Process Tracing
- Automatically traverses the parent process chain (up to 20 levels) to trace the process origin
- Identifies launch sources: **Cursor / VSCode / JetBrains / Xcode / iTerm2 / Warp / Terminal / Claude / Docker** and dozens of other IDEs, terminals, and applications

### 🏷️ Smart Service Classification
Categorizes each port service into **9 major categories**:

| Category | Description | Examples |
|----------|-------------|----------|
| 🟢 DevService | Development services | Vite, Next.js, Webpack, Angular CLI |
| 🤖 AiDevService | AI development services | Ollama, Jupyter, TensorBoard |
| 🐳 DockerService | Docker services | Docker Desktop, container port mappings |
| 🗄️ DatabaseService | Database services | PostgreSQL, MySQL, Redis, MongoDB |
| 🌐 WebServer | Web servers | Nginx, Apache, Caddy |
| ⚙️ SystemService | System services | AirDrop, mDNSResponder |
| 🏗️ InfraService | Infrastructure services | RabbitMQ, Zookeeper |
| 📱 AppService | Application services | ClashX, Surge |
| ❓ Unknown | Unknown services | — |

### ⚠️ Safety Risk Assessment

| Risk Level | Color | Meaning | Action |
|------------|-------|---------|--------|
| 🟢 Safe | Green | Development service, safe to terminate | Confirm directly to terminate |
| 🟡 Caution | Yellow | Database/Docker etc., proceed with caution | Requires port number confirmation |
| 🔴 Danger | Red | Critical system service, termination forbidden | Terminate action disabled |
| ⚪ Unknown | Gray | Unidentified, user judgment required | Requires port number confirmation |

### 🎨 UI Features
- **Search & Filter**: Search by port number, process name, command, directory, service name, source, etc.
- **Quick Filters**: One-click filtering by risk level or service type
- **Detail Panel**: Click any port entry to expand full process information on the right
- **Theme Switching**: Supports 🌞 Light / 🌙 Dark / 💻 System-following themes
- **Kill Mode**: Supports SIGTERM (graceful termination) and SIGKILL (force termination)

---

## 🖼️ Interface Preview

> On launch, Port Guardian automatically scans and displays all listening ports:

```
┌─────────────────────────────────────────────────────────────────┐
│  🛡️ Port Guardian                              [⚙️] [🔄]       │
├─────────────────────────────────────────────────────────────────┤
│  🔍 Search ports, processes, services...                        │
│  [🟢 Dev] [🤖 AI] [🗄️ DB] [🐳 Docker] [⚙️ System]              │
├─────────────────────────────────────────────────────────────────┤
│  Port  │ Service Name   │ Process   │ Source     │ Risk │ Action │
│  3000  │ Vite           │ node      │ Cursor     │ 🟢   │ [Kill] │
│  5432  │ PostgreSQL     │ postgres  │ Terminal   │ 🟡   │ [Kill] │
│  8080  │ Next.js        │ node      │ VSCode     │ 🟢   │ [Kill] │
│  6379  │ Redis          │ redis-srv │ Docker     │ 🟡   │ [Kill] │
│  5000  │ AirDrop        │ launchd   │ System     │ 🔴   │ [N/A]  │
├─────────────────────────────────────────────────────────────────┤
│                                              Detail Panel →      │
└─────────────────────────────────────────────────────────────────┘
```

---

## 🛠️ Technical Architecture

```
port-guardian/
├── src/                          # Frontend (React + TypeScript)
│   ├── main.tsx                  # Application entry point
│   ├── App.tsx                   # Main component (state management, filtering, kill logic)
│   ├── App.css                   # Global styles (light/dark themes)
│   ├── types.ts                  # TypeScript interface definitions
│   └── components/
│       ├── PortTable.tsx         # Port list table
│       ├── SearchBar.tsx         # Search bar
│       ├── ServiceDetail.tsx     # Service detail side panel
│       ├── ConfirmKillDialog.tsx # Kill confirmation dialog
│       ├── RiskBadge.tsx         # Risk level badge
│       └── Settings.tsx          # Settings dialog (theme switching)
│
└── src-tauri/                    # Backend (Rust + Tauri 2)
    ├── src/
    │   ├── main.rs               # Rust entry point
    │   ├── lib.rs                # Tauri app build & command registration
    │   ├── commands.rs           # Tauri IPC commands
    │   ├── port_scanner.rs       # Port scanning (lsof)
    │   ├── process_resolver.rs   # Process info resolution (ps + lsof)
    │   ├── process_tree.rs       # Process tree tracing
    │   ├── service_classifier.rs # Service classification engine
    │   ├── safety_checker.rs     # Safety level assessment
    │   └── terminator.rs         # Process termination (SIGTERM/SIGKILL)
    ├── Cargo.toml                # Rust dependencies
    └── tauri.conf.json           # Tauri configuration
```

### Data Flow

```
  ┌──────────────┐     IPC invoke      ┌─────────────────┐
  │  React Front │ ──────────────────→ │  Tauri Backend   │
  │              │                     │                 │
  │  App.tsx     │ ←────────────────── │  commands.rs    │
  │  State Mgmt  │    JSON Response    │  scan_ports()   │
  │  UI Render   │                     │  terminate()    │
  └──────────────┘                     └─────────────────┘
                                              │
                                              ▼
                                    ┌─────────────────┐
                                    │  System Commands │
                                    │  lsof / ps / kill│
                                    └─────────────────┘
```

---

## 🚀 Quick Start

### Download & Install

Current version: `v0.1.12`

| Platform | Direct Download |
|----------|-----------------|
| macOS | [Download `Port.Guardian_0.1.12_universal.dmg`](https://github.com/soBigRice/port-guardian/releases/download/v0.1.12/Port.Guardian_0.1.12_universal.dmg) |
| Windows | [Download `Port.Guardian_0.1.12_x64-setup.exe`](https://github.com/soBigRice/port-guardian/releases/download/v0.1.12/Port.Guardian_0.1.12_x64-setup.exe) |

For historical versions or signature verification files, visit the [GitHub Releases](https://github.com/soBigRice/port-guardian/releases/latest) page.

After installation, launch the app. You can check for updates in the app's settings page.

#### macOS: First Launch Issues

Since the current installer is not yet signed/notarized with an Apple Developer ID, macOS may show a warning like "cannot be opened because the developer cannot be verified" on first launch. Follow these steps to allow it:

1. Double-click the `.dmg` and drag `Port Guardian.app` to the "Applications" folder.
2. If blocked on first launch, close the warning dialog.
3. Open "System Settings".
4. Go to "Privacy & Security".
5. In the "Security" section at the bottom, find the notice about `Port Guardian` being blocked.
6. Click "Open Anyway" or "仍要打开".
7. Confirm again; if prompted, enter your password or use Touch ID.

On Windows, if SmartScreen shows a warning, click "More info", verify the source is from this project's Release, then click "Run anyway".

### Prerequisites

- **Operating System**: macOS / Windows
- **Node.js**: ≥ 18
- **Rust**: ≥ 1.85 (supports edition 2024)
- **macOS local build**: Requires Xcode Command Line Tools

### Installation & Running

```bash
# 1. Clone the repository
git clone https://github.com/soBigRice/port-guardian.git
cd port-guardian

# 2. Install frontend dependencies
npm install

# 3. Start development mode
npm run tauri dev
```

### Build for Production

```bash
npm run tauri build
```

After the build completes, the `.dmg` installer can be found in the `src-tauri/target/release/bundle/dmg/` directory.

---

## 📋 Available Scripts

| Command | Description |
|---------|-------------|
| `npm run dev` | Start Vite dev server only (no Tauri) |
| `npm run build` | TypeScript compilation + Vite production build |
| `npm run preview` | Preview production build |
| `npm run tauri dev` | Start Tauri development mode (frontend + backend) |
| `npm run tauri build` | Build for production |

---

## 🔧 Configuration

### Tauri Configuration ([tauri.conf.json](src-tauri/tauri.conf.json))

| Config Key | Value | Description |
|------------|-------|-------------|
| Window Size | 1100 × 700 | Default window dimensions |
| App Identifier | `com.port-guardian.app` | Application unique identifier |
| Dev Server | `http://localhost:1420` | Vite development server address |

### Cargo Configuration ([.cargo/config.toml](src-tauri/.cargo/config.toml))

Uses USTC (University of Science and Technology of China) crates.io mirror by default to accelerate Rust dependency downloads.

### Development Notes (Troubleshooting Log)

#### 2026-06-01: App still shows old icon after changing icon files

- **Issue**: `icon/icon.png` was generated to `src-tauri/icons`, but the app still shows the old icon.
- **Root Cause**: Icon resources were generated, but the project didn't explicitly declare `bundle.icon`; also, a regular `tauri build` without specifying bundles only outputs the executable, which can mislead into thinking the `.app` icon has been updated.
- **Impact**: Desktop icon verification and delivery workflow (especially macOS `.app`) is prone to false "updated but visually unchanged" conclusions.
- **Solution**: Add `bundle.icon` entries (`32x32`, `128x128`, `128x128@2x`, `icon.icns`, `icon.ico`) to `src-tauri/tauri.conf.json`, and use `npm run tauri build -- --bundles app` to generate a new `.app` for icon verification.
- **Going Forward**: After each icon replacement, follow the "generate icon resources → verify `bundle.icon` → rebuild `app` bundle" process; if the system still shows the old icon, close old processes and confirm you're opening the latest generated `.app`.

#### 2026-06-01: Icon edges have excessive visual padding

- **Issue**: New icon appears with wide dark border padding, making the subject appear too small.
- **Root Cause**: The source image contains a wide dark background ring, which after scaling to small sizes creates an "edges too thick" visual effect.
- **Impact**: Small size icons (`32x32` / `64x64`) lose recognition; Dock/Finder/taskbar appearance looks "shrunk to center".
- **Solution**: Apply centered square cropping (`1146x1146+54+50`) to tighten edges, then regenerate all platform icons with `npm run tauri icon icon/icon.png`.
- **Going Forward**: When updating icons, besides checking dimensions are square, also verify small-size preview subject proportions; apply light cropping before generating multi-size icons if necessary.

#### 2026-06-01: Black right angles still appear after cropping

- **Issue**: Icon has visual rounded corners, but four corners still show black right-angle background.
- **Root Cause**: The source PNG lacks an Alpha channel; rounded corners are just color transitions, not real transparent corners.
- **Impact**: Square outline visible on both dark/light desktop backgrounds, affecting icon consistency.
- **Solution**: Add real transparent rounded corner mask to source image (`roundrectangle 0,0 1145,1145 180,180`), then re-run `npm run tauri icon icon/icon.png` and `npm run tauri build -- --bundles app`.
- **Going Forward**: After each icon change, verify `hasAlpha` and corner pixel alpha values (should be 0) to avoid visual-only rounded corners causing black corner regression.

#### 2026-06-01: image2 icon shows checkerboard background and excessive padding

- **Issue**: Directly using image2-generated icon results in checkerboard texture background and subject taking too small a proportion of the canvas.
- **Root Cause**: Model returned "transparent preview style" checkerboard background pixels (not truly transparent), and default composition has large padding.
- **Impact**: Icon looks like "not cleanly cut out", and small-size recognition decreases.
- **Solution**: Use four-corner connected domain transparency processing to remove checkerboard background, then `trim` and center-expand to `1024x1024`, ensuring corner alpha is 0, then run `npm run tauri icon icon/icon.png`.
- **Going Forward**: Before integrating image2 output, perform two checks: 1) Are corner pixels transparent? 2) Is subject proportion after `trim` up to standard (avoid excessive padding)?

#### 2026-06-01: macOS system still shows old icon (cache not refreshed)

- **Issue**: Icon resources inside app bundle are updated, but Finder/Dock still shows old icon.
- **Root Cause**: macOS `Dock` / `IconServices` / `LaunchServices` has caching that may continue to use historical icon index.
- **Impact**: User-side visual verification shows "file content changed but system display unchanged" illusion, easily misjudged as changes not taking effect.
- **Solution**: Run `touch <App>.app`, clear user-space icon caches (`com.apple.dock.iconcache`, `com.apple.iconservices.store`), restart `Finder` / `Dock` / `iconservicesagent`, and run `lsregister -f` on the target `.app` to re-register; if necessary, generate a `.app` with a new filename (e.g., `Port Guardian Fresh.app`) to bypass old cache index.
- **Going Forward**: When verifying icon changes, prioritize "in-bundle `icon.icns` check + new filename `.app` launch verification" as the standard, then do system cache refresh to avoid re-checking the image itself.

#### 2026-06-01: GitHub Actions macOS release task fails at Rust installation stage

- **Issue**: `release (macos-latest, universal-apple-darwin)` fails directly at `Install Rust stable` step.
- **Root Cause**: Mistakenly treated `universal-apple-darwin` as a `rustup target` to install; this value is a Tauri build target, not a downloadable `rust-std` target.
- **Impact**: macOS release package cannot be built; Windows task can continue running, causing incomplete release artifacts.
- **Solution**: Add `rust_targets` to the workflow; macOS installs `aarch64-apple-darwin,x86_64-apple-darwin`, Windows installs `x86_64-pc-windows-msvc`; `tauri-action` still uses `--target universal-apple-darwin`.
- **Going Forward**: In CI, distinguish between "build targets (Tauri/Cargo args)" and "Rust standard library targets (rustup targets)" — do not reuse the same field.

#### 2026-06-01: GitHub Actions build succeeds but artifact upload fails (No artifacts were found)

- **Issue**: `Build with Tauri` logs show Rust compilation completed, but step ends with `No artifacts were found` error.
- **Root Cause**: Workflow only passed `--target`, not explicitly `--bundles`; with current configuration, no uploadable installer/update package files are produced.
- **Impact**: macOS/Windows release flow fails at artifact upload stage; Release cannot include installers.
- **Solution**: Add `bundles` matrix field per platform: macOS uses `app,dmg`, Windows uses `nsis`, and change action parameters to `--target ... --bundles ...`.
- **Going Forward**: Seeing "Built application at .../release/<bin>" does not mean release artifacts are generated; verify `bundle/*` directory and installer files exist.

#### 2026-06-01: Update check not working (always shows no updates or silently fails)

- **Issue**: Clicking "Check for Updates" in-app shows no result; update dialog doesn't appear.
- **Root Cause**: Multiple factors: ① Release workflow uses `releaseDraft: true`, making `releases/latest/download/latest.json` unavailable; ② Repository is private, so anonymous client requests to GitHub Release resources return 404; ③ Interface version number is hardcoded to `0.1.0`, easily misjudging current version state; ④ Update check exceptions are swallowed, UI has no error state feedback.
- **Impact**: Auto-update pipeline is non-functional; users cannot get new versions via built-in updater.
- **Solution**: Change workflow to publish non-Draft Releases; CI build synchronizes `package.json` and `tauri.conf.json` version numbers by tag; frontend version number changed to runtime reading; show error state on update check failure.
- **Going Forward**: If continuing with private repository releases, provide an authenticatable update source; if using GitHub `latest.json` direct links, recommend using a public repository or self-hosted update distribution service.

#### 2026-06-01: Updater artifacts not generated, causing update metadata missing

- **Issue**: Release flow has installers, but update metadata/signature artifacts are incomplete; client update check is non-functional.
- **Root Cause**: `tauri.conf.json` didn't explicitly enable `bundle.createUpdaterArtifacts`; default value is `false`.
- **Impact**: Cannot reliably produce `latest.json` and corresponding signature chain; Updater dependencies are missing.
- **Solution**: Enable `"createUpdaterArtifacts": "v1Compatible"` in `bundle` config, consistent with current `latest.json` endpoint format.
- **Going Forward**: After each release flow adjustment, verify Release includes `latest.json`, platform installers, and signature files as three critical artifacts.

#### 2026-06-02: Windows release task fails at version sync step

- **Issue**: `release (windows-latest, x86_64-pc-windows-msvc)` fails at version sync step, preventing Windows Release artifacts from continuing to build.
- **Root Cause**: GitHub Actions Windows runner defaults to PowerShell for `run` scripts, but the version sync script uses bash syntax.
- **Impact**: Only Windows release task affected; macOS task unaffected by this shell default.
- **Solution**: In `Sync app version from tag` step, explicitly set `shell: bash` so all platforms use consistent script interpreter.
- **Going Forward**: In cross-platform workflows, if script contains bash variable expansion, here-doc, or POSIX conditional logic, must explicitly declare `shell: bash`.

#### 2026-06-02: Update still fails after making repository public (Release signing private key missing)

- **Issue**: After making repository public, in-app "Check for Updates" still fails; `latest.json` still returns 404.
- **Root Cause**: Making repository public only solves anonymous download permissions; actual Release workflow still cannot complete updater signing due to missing `TAURI_SIGNING_PRIVATE_KEY`, so GitHub doesn't successfully publish `latest.json`.
- **Impact**: Both macOS/Windows release tasks need this private key; as long as signing fails, auto-update pipeline has no downloadable update metadata.
- **Solution**: Regenerate Tauri updater signing key; write public key to `src-tauri/tauri.conf.json`; write private key to GitHub Actions Secret `TAURI_SIGNING_PRIVATE_KEY`; add pre-check for missing private key in workflow.
- **Going Forward**: Private key must not be committed to repository; if old public key version has been published externally, replacing updater public key will prevent old versions from auto-updating to new versions — must keep old private key or design a transition version.

#### 2026-06-02: GitHub Actions build times out accessing USTC crates mirror

- **Issue**: macOS release task fails at `Build with Tauri` stage downloading `tauri-build`; logs show `Updating ustc index` followed by curl timeout.
- **Root Cause**: `src-tauri/.cargo/config.toml` in repository replaces crates.io with USTC mirror; this config works for local Chinese network but GitHub overseas runners access this mirror unstably.
- **Impact**: Rust dependency resolution on GitHub Actions may randomly fail; both macOS/Windows potentially affected; local builds may not reproduce.
- **Solution**: Keep local mirror configuration in repository; in Release workflow, only temporarily rewrite Cargo config for CI runner to restore official crates.io.
- **Going Forward**: Network mirror configurations should distinguish between local development and CI environments; CI should prioritize sources more stable on the runner's network.

#### 2026-06-02: Cannot continue uploading assets after creating public Release

- **Issue**: `tauri-action` creates `v0.1.6` Release then fails uploading `.dmg`; logs show `Cannot upload assets to an immutable release`.
- **Root Cause**: Release flow directly creates public Release; GitHub marks it as immutable, subsequent asset uploads are rejected; matrix tasks processing same tag also amplifies race conditions.
- **Impact**: Release becomes empty-asset public version; `latest.json` not generated; app update check continues to fail.
- **Solution**: Build tasks first upload to Draft Release, limiting matrix publishing to serial; after all platform assets uploaded, independent `publish-release` job validates critical assets and publishes publicly.
- **Going Forward**: When appending multi-platform assets to same Release, prefer Draft aggregation, then unified publish; don't publish first then upload.

#### 2026-06-02: Draft Release cannot be queried via tag detail API

- **Issue**: `publish-release` job fails after assets uploaded to Draft Release, preventing Release from being automatically published.
- **Root Cause**: GitHub's `releases/tags/<tag>` API doesn't reliably return Draft Releases; script looking up `v0.1.7` couldn't find the just-created Draft.
- **Impact**: Installers and `latest.json` uploaded but stuck in Draft; client anonymous update check continues to 404.
- **Solution**: Publishing script changed to call `releases?per_page=100` list API, then find corresponding Draft/Release by `tag_name` to validate assets and PATCH to public.
- **Going Forward**: Automation involving Draft Releases should not rely on tag detail API; prefer filtering from release list.

---

## 📦 Dependencies

### Frontend Dependencies

| Package | Version | Description |
|---------|---------|-------------|
| `react` | ^19.1.0 | UI framework |
| `react-dom` | ^19.1.0 | React DOM rendering |
| `@tauri-apps/api` | ^2 | Tauri frontend API |
| `@tauri-apps/plugin-shell` | ^2 | Tauri Shell plugin |

### Backend Dependencies

| Crate | Version | Description |
|-------|---------|-------------|
| `tauri` | 2 | Tauri framework |
| `tauri-plugin-shell` | 2 | Shell command execution plugin |
| `serde` | 1 | Serialization/Deserialization |
| `serde_json` | 1 | JSON processing |

---

## ⚡ How It Works

### 1. Port Scanning
Executes `lsof -nP -iTCP -sTCP:LISTEN` to obtain all TCP listening ports, parsing port numbers, PIDs, and basic information.

### 2. Process Resolution
For each PID, executes `ps -p <PID>` to get process details (parent PID, user, command line), then uses `lsof -p <PID>` to get working directory and executable path.

### 3. Process Tree Tracing
Traverses the parent process chain from the current process (up to 20 levels), identifying the launch source (IDE, terminal, browser, etc.) through process name and command line characteristics.

### 4. Service Classification
Based on process name, command line arguments, and other characteristics, classifies services into 9 major categories. Supports identifying dozens of common services, including but not limited to:

- **Development Services**: Vite, Next.js, Webpack, Angular CLI, Nuxt, Remix, SvelteKit, Astro, Gatsby...
- **AI Services**: Ollama, Jupyter Notebook/Lab, TensorBoard, MLflow, Ray...
- **Databases**: PostgreSQL, MySQL, MongoDB, Redis, Elasticsearch, ClickHouse, MinIO...
- **Web Servers**: Nginx, Apache, Caddy, Traefik...
- **Docker**: Docker Desktop, container port mappings
- **Infrastructure**: RabbitMQ, Kafka, Zookeeper, Consul, Vault...

### 5. Safety Assessment
- **SystemService** → 🔴 Danger (cannot terminate)
- **DatabaseService / DockerService / InfraService / AppService** → 🟡 Caution (confirmation required)
- **DevService / AiDevService / WebServer** → 🟢 Safe (safe to terminate)
- **Unknown** → Determined by running user (root → Danger, others → Unknown)

### 6. Process Termination
- Default sends `SIGTERM` (signal 15) for graceful termination
- Optional `SIGKILL` (signal 9) for force termination
- Checks if process has exited 500ms after termination

---

## 🤝 Contributing

Issues and Pull Requests are welcome!

1. Fork this repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Commit your changes: `git commit -m 'Add amazing feature'`
4. Push the branch: `git push origin feature/amazing-feature`
5. Submit a Pull Request

---

## 📄 License

This project is open-sourced under the MIT License. See [LICENSE](LICENSE) for details.

---

<div align="center">

**Use Port Guardian — never worry about port management again 🛡️**

</div>
