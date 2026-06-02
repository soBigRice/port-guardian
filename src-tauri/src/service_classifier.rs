use serde::Serialize;

use crate::process_resolver::ProcessInfo;
use crate::process_tree::ProcessNode;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceType {
    DevService,
    AiDevService,
    DockerService,
    DatabaseService,
    WebServer,
    SystemService,
    InfraService,
    AppService,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceClassification {
    pub service_type: ServiceType,
    pub service_name: String,
}

/// 根据进程信息和服务来源，对服务进行分类
pub fn classify(
    process: &ProcessInfo,
    parent_chain: &[ProcessNode],
    port: u16,
    source: &str,
) -> ServiceClassification {
    let name_lower = process.name.to_lowercase();
    let cmd_lower = process.command_line.to_lowercase();

    // Windows 进程名带 .exe 后缀（如 node.exe），统一剥离以便匹配
    let name_stripped = name_lower.strip_suffix(".exe").unwrap_or(&name_lower);

    // 1. 系统服务检测
    if is_system_service(&name_lower) {
        return ServiceClassification {
            service_type: ServiceType::SystemService,
            service_name: friendly_system_name(&name_stripped),
        };
    }

    // 2. Docker 服务检测
    if is_docker_service(&name_stripped, &cmd_lower, parent_chain) {
        return ServiceClassification {
            service_type: ServiceType::DockerService,
            service_name: "Docker".to_string(),
        };
    }

    // 3. 数据库服务检测
    if let Some(db_name) = detect_database(&name_stripped, port) {
        return ServiceClassification {
            service_type: ServiceType::DatabaseService,
            service_name: db_name,
        };
    }

    // 4. 基础设施服务检测
    if let Some(infra_name) = detect_infra(&name_stripped, port) {
        return ServiceClassification {
            service_type: ServiceType::InfraService,
            service_name: infra_name,
        };
    }

    // 5. Web 服务器检测
    if let Some(ws_name) = detect_web_server(&name_stripped, &cmd_lower) {
        return ServiceClassification {
            service_type: ServiceType::WebServer,
            service_name: ws_name,
        };
    }

    // 6. 用户应用程序检测（浏览器、通讯工具等）
    if let Some(app_name) = detect_app(&name_stripped, &cmd_lower) {
        return ServiceClassification {
            service_type: ServiceType::AppService,
            service_name: app_name,
        };
    }

    // 7. 开发服务检测
    if let Some((is_ai, dev_name)) =
        detect_dev_service(&name_stripped, &cmd_lower, source, parent_chain)
    {
        return ServiceClassification {
            service_type: if is_ai {
                ServiceType::AiDevService
            } else {
                ServiceType::DevService
            },
            service_name: dev_name,
        };
    }

    ServiceClassification {
        service_type: ServiceType::Unknown,
        service_name: String::new(),
    }
}

fn friendly_system_name(name: &str) -> String {
    match name {
        // macOS
        "launchd" => "Launch Daemon".into(),
        "kernel_task" => "Kernel".into(),
        "windowserver" => "Window Server".into(),
        "loginwindow" => "Login Window".into(),
        "sshd" => "SSH Server".into(),
        "rapportd" => "AirPlay".into(),
        "controlcenter" => "Control Center".into(),
        "sharingd" => "Sharing Service".into(),
        "syslogd" => "System Log".into(),
        "coreaudiod" => "Audio Service".into(),
        "bluetoothd" => "Bluetooth".into(),
        "airportd" => "WiFi".into(),
        "mds" | "mds_stores" => "Spotlight".into(),
        // Windows
        "system" => "Windows System".into(),
        "registry" => "Windows Registry".into(),
        "smss" | "smss.exe" => "Session Manager".into(),
        "csrss" | "csrss.exe" => "Client Server Runtime".into(),
        "wininit" | "wininit.exe" => "Windows Init".into(),
        "lsass" | "lsass.exe" => "Local Security Authority".into(),
        "services" | "services.exe" => "Service Control Manager".into(),
        "svchost" | "svchost.exe" => "Service Host".into(),
        "dwm" | "dwm.exe" => "Desktop Window Manager".into(),
        "winlogon" | "winlogon.exe" => "Windows Logon".into(),
        "spoolsv" | "spoolsv.exe" => "Print Spooler".into(),
        "msdtc" | "msdtc.exe" => "Distributed Transaction".into(),
        other => other.to_string(),
    }
}

fn is_system_service(name: &str) -> bool {
    // 先剥离 .exe 后缀再匹配（Windows 进程名带 .exe）
    let stripped = name.strip_suffix(".exe").unwrap_or(name);

    const SYSTEM_PROCESSES: &[&str] = &[
        // macOS 系统进程
        "launchd",
        "kernel_task",
        "windowserver",
        "loginwindow",
        "syslogd",
        "distnoted",
        "cfprefsd",
        "usernoted",
        "coreservicesd",
        "lsd",
        "suggestd",
        "iconservicesd",
        "rapportd",
        "sharingd",
        "nsurlsessiond",
        "coreaudiod",
        "bluetoothd",
        "airportd",
        "controlcenter",
        "spotlight",
        "mds",
        "mds_stores",
        "diskarbitrationd",
        "apsd",
        "securityd",
        "secd",
        "keybagd",
        "fseventsd",
        "sandboxd",
        "taskgated",
        "hidd",
        "powerd",
        "thermald",
        "clpd",
        "colorsyncuseragent",
        "fontd",
        // SSH（macOS/Linux）
        "sshd",
        // Linux 系统进程
        "systemd",
        "init",
        "kthreadd",
        // Windows 系统进程（不带 .exe，匹配时已剥离）
        "system",
        "registry",
        "smss",
        "csrss",
        "wininit",
        "lsass",
        "services",
        "svchost",
        "dwm",
        "winlogon",
        "spoolsv",
        "msdtc",
        "sihost",
        "securityhealthservice",
        "searchhost",
        "searchapp",
        "startmenuexperiencehost",
        "textinputhost",
        "runtimebroker",
        "wmiprvse",
        "dllhost",
        "conhost",
    ];
    SYSTEM_PROCESSES.iter().any(|&s| stripped == s)
}

fn is_docker_service(name: &str, cmd: &str, chain: &[ProcessNode]) -> bool {
    if name.contains("docker") || cmd.contains("docker") {
        return true;
    }
    chain.iter().any(|n| {
        n.name.to_lowercase().contains("docker") || n.command_line.to_lowercase().contains("docker")
    })
}

fn detect_database(name: &str, port: u16) -> Option<String> {
    // 按进程名匹配
    if name.contains("postgres") {
        return Some("PostgreSQL".into());
    }
    if name.contains("mysql") || name.contains("mysqld") {
        return Some("MySQL".into());
    }
    if name.contains("redis") {
        return Some("Redis".into());
    }
    if name.contains("mongod") || name.contains("mongos") {
        return Some("MongoDB".into());
    }
    if name.contains("memcached") {
        return Some("Memcached".into());
    }
    if name.contains("sqlite") {
        return Some("SQLite".into());
    }
    if name.contains("influxd") {
        return Some("InfluxDB".into());
    }
    if name.contains("clickhouse") {
        return Some("ClickHouse".into());
    }
    if name.contains("neo4j") {
        return Some("Neo4j".into());
    }
    if name.contains("cockroach") {
        return Some("CockroachDB".into());
    }
    if name.contains("mariadbd") {
        return Some("MariaDB".into());
    }

    // 按常见端口匹配（仅在进程名无法判断时）
    match port {
        5432 => Some("PostgreSQL".into()),
        3306 => Some("MySQL".into()),
        6379 => Some("Redis".into()),
        27017 => Some("MongoDB".into()),
        11211 => Some("Memcached".into()),
        8086 => Some("InfluxDB".into()),
        9000 => None, // 9000 太常见，留给 MinIO
        _ => None,
    }
}

fn detect_infra(name: &str, port: u16) -> Option<String> {
    if name.contains("minio") {
        return Some("MinIO".into());
    }
    if name.contains("ollama") {
        return Some("Ollama".into());
    }
    if name.contains("rabbitmq") || (name.contains("beam") && port == 5672) {
        return Some("RabbitMQ".into());
    }
    if name.contains("elastic") {
        return Some("Elasticsearch".into());
    }
    if name.contains("consul") {
        return Some("Consul".into());
    }
    if name.contains("etcd") {
        return Some("etcd".into());
    }
    if name.contains("kafka") {
        return Some("Kafka".into());
    }
    if name.contains("zookeeper") {
        return Some("ZooKeeper".into());
    }
    if name.contains("nats-server") {
        return Some("NATS".into());
    }
    if name.contains("redis-sentinel") {
        return Some("Redis Sentinel".into());
    }
    if name.contains("grafana") {
        return Some("Grafana".into());
    }
    if name.contains("prometheus") {
        return Some("Prometheus".into());
    }
    if name.contains("jaeger") {
        return Some("Jaeger".into());
    }
    if name.contains("registry") && port == 5000 {
        return Some("Docker Registry".into());
    }

    // 按端口匹配
    match port {
        9000 => Some("MinIO".into()),
        11434 => Some("Ollama".into()),
        5672 => Some("RabbitMQ".into()),
        9092 => Some("Kafka".into()),
        2181 => Some("ZooKeeper".into()),
        3000 if name.contains("grafana") => Some("Grafana".into()),
        9090 if name.contains("prometheus") => Some("Prometheus".into()),
        _ => None,
    }
}

fn detect_web_server(name: &str, cmd: &str) -> Option<String> {
    match name {
        "nginx" => Some("Nginx".into()),
        "httpd" | "apache2" => Some("Apache".into()),
        "caddy" => Some("Caddy".into()),
        "lighttpd" => Some("Lighttpd".into()),
        "traefik" => Some("Traefik".into()),
        "haproxy" => Some("HAProxy".into()),
        _ => {
            // 命令行中包含 web server 关键字
            if cmd.contains("nginx") {
                return Some("Nginx".into());
            }
            if cmd.contains("caddy") {
                return Some("Caddy".into());
            }
            None
        }
    }
}

/// 检测用户应用程序（浏览器、通讯工具等）
fn detect_app(name: &str, cmd: &str) -> Option<String> {
    // 浏览器（通常监听远程调试端口 9222 等）
    if name.contains("google chrome") || cmd.contains("google chrome") {
        return Some("Chrome".into());
    }
    if name.contains("firefox") || cmd.contains("firefox") {
        return Some("Firefox".into());
    }
    if name.contains("safari") {
        return Some("Safari".into());
    }
    if name.contains("microsoft edge") || cmd.contains("microsoft edge") {
        return Some("Edge".into());
    }
    if name.contains("brave") || cmd.contains("brave") {
        return Some("Brave".into());
    }
    if name.contains("arc") || cmd.contains("arc.app") {
        return Some("Arc".into());
    }
    if name.contains("opera") {
        return Some("Opera".into());
    }
    if name.contains("vivaldi") {
        return Some("Vivaldi".into());
    }

    // 通讯 / 生产力工具
    if name.contains("slack") {
        return Some("Slack".into());
    }
    if name.contains("discord") {
        return Some("Discord".into());
    }
    if name.contains("telegram") {
        return Some("Telegram".into());
    }
    if name.contains("whatsapp") {
        return Some("WhatsApp".into());
    }
    if name.contains("zoom") {
        return Some("Zoom".into());
    }
    if name.contains("teams") {
        return Some("Teams".into());
    }
    if name.contains("notion") {
        return Some("Notion".into());
    }
    if name.contains("obsidian") {
        return Some("Obsidian".into());
    }
    if name.contains("raycast") {
        return Some("Raycast".into());
    }
    if name.contains("alfred") {
        return Some("Alfred".into());
    }
    if name.contains("1password") {
        return Some("1Password".into());
    }
    if name.contains("little_snitch") || name.contains("little snitch") {
        return Some("Little Snitch".into());
    }
    if name.contains("surge") {
        return Some("Surge".into());
    }
    if name.contains("clash") {
        return Some("Clash".into());
    }
    if name.contains("v2ray") {
        return Some("V2Ray".into());
    }
    if name.contains("shadowsocks") {
        return Some("Shadowsocks".into());
    }
    if name.contains("trojan") {
        return Some("Trojan".into());
    }
    if name.contains("postman") {
        return Some("Postman".into());
    }
    if name.contains("figma") {
        return Some("Figma".into());
    }
    if name.contains("spotify") {
        return Some("Spotify".into());
    }
    if name.contains("music") && cmd.contains("music.app") {
        return Some("Apple Music".into());
    }

    // 中国常用应用
    if name.contains("feishu") || name.contains("lark") {
        return Some("飞书".into());
    }
    if name.contains("dingtalk") {
        return Some("钉钉".into());
    }
    if name.contains("wechat") {
        return Some("微信".into());
    }
    if name.contains("qq") {
        return Some("QQ".into());
    }
    if name.contains("bytedance") || name.contains("tiktok") {
        return Some("ByteDance".into());
    }

    // JetBrains IDE
    if cmd.contains("jetbrains")
        || name.contains("idea")
        || name.contains("webstorm")
        || name.contains("pycharm")
        || name.contains("goland")
        || name.contains("clion")
        || name.contains("rider")
        || name.contains("datagrip")
        || name.contains("phpstorm")
    {
        return Some("JetBrains".into());
    }

    // Xcode
    if name.contains("xcode") {
        return Some("Xcode".into());
    }

    None
}

fn detect_dev_service(
    name: &str,
    cmd: &str,
    source: &str,
    chain: &[ProcessNode],
) -> Option<(bool, String)> {
    let is_ai_source = matches!(source, "Cursor" | "Codex" | "Windsurf" | "Claude");

    let is_dev_process = matches!(
        name,
        "node"
            | "npm"
            | "pnpm"
            | "yarn"
            | "bun"
            | "python"
            | "python3"
            | "uvicorn"
            | "gunicorn"
            | "ts-node"
            | "tsx"
            | "esbuild"
            | "deno"
            | "ruby"
            | "rails"
            | "cargo"
            | "rustc"
    );

    if !is_dev_process {
        // 检查父进程链中是否有开发进程
        let has_dev_in_chain = chain.iter().any(|n| {
            let nl = n.name.to_lowercase();
            matches!(
                nl.as_str(),
                "node"
                    | "npm"
                    | "pnpm"
                    | "yarn"
                    | "bun"
                    | "python"
                    | "python3"
                    | "cargo"
                    | "rustc"
                    | "deno"
                    | "ruby"
            )
        });
        if !has_dev_in_chain {
            return None;
        }
    }

    // 检测具体框架
    let service_name = detect_framework(cmd);
    Some((is_ai_source, service_name))
}

fn detect_framework(cmd: &str) -> String {
    // 前端框架 / 工具链
    if cmd.contains("vite") {
        return "Vite".into();
    }
    if cmd.contains("next") {
        return "Next.js".into();
    }
    if cmd.contains("astro") {
        return "Astro".into();
    }
    if cmd.contains("nuxt") {
        return "Nuxt".into();
    }
    if cmd.contains("svelte") || cmd.contains("sveltekit") {
        return "SvelteKit".into();
    }
    if cmd.contains("remix") {
        return "Remix".into();
    }
    if cmd.contains("gatsby") {
        return "Gatsby".into();
    }
    if cmd.contains("webpack-dev-server") || cmd.contains("webpack serve") {
        return "Webpack Dev Server".into();
    }
    if cmd.contains("parcel") {
        return "Parcel".into();
    }
    if cmd.contains("turbo") {
        return "Turborepo".into();
    }
    if cmd.contains("esbuild") {
        return "esbuild".into();
    }

    // 后端框架
    if cmd.contains("uvicorn") || cmd.contains("gunicorn") {
        return "Python Server".into();
    }
    if cmd.contains("flask") {
        return "Flask".into();
    }
    if cmd.contains("django") {
        return "Django".into();
    }
    if cmd.contains("fastapi") {
        return "FastAPI".into();
    }
    if cmd.contains("tornado") {
        return "Tornado".into();
    }
    if cmd.contains("aiohttp") {
        return "aiohttp".into();
    }
    if cmd.contains("rails") {
        return "Rails".into();
    }
    if cmd.contains("sinatra") {
        return "Sinatra".into();
    }
    if cmd.contains("spring") || cmd.contains("tomcat") {
        return "Spring/Tomcat".into();
    }
    if cmd.contains("express") {
        return "Express".into();
    }
    if cmd.contains("koa") {
        return "Koa".into();
    }
    if cmd.contains("hono") {
        return "Hono".into();
    }
    if cmd.contains("actix") {
        return "Actix".into();
    }
    if cmd.contains("axum") {
        return "Axum".into();
    }
    if cmd.contains("rocket") {
        return "Rocket".into();
    }
    if cmd.contains("gin") {
        return "Gin".into();
    }

    // 测试 / 工具
    if cmd.contains("storybook") {
        return "Storybook".into();
    }
    if cmd.contains("jest") {
        return "Jest".into();
    }
    if cmd.contains("vitest") {
        return "Vitest".into();
    }
    if cmd.contains("cypress") {
        return "Cypress".into();
    }
    if cmd.contains("playwright") {
        return "Playwright".into();
    }
    if cmd.contains("http-server") {
        return "HTTP Server".into();
    }
    if cmd.contains("serve") {
        return "Serve".into();
    }
    if cmd.contains("http.server") {
        return "Python HTTP Server".into();
    }
    if cmd.contains("live-server") {
        return "Live Server".into();
    }
    if cmd.contains("json-server") {
        return "JSON Server".into();
    }

    // 通用
    if cmd.contains("dev") {
        return "Dev Server".into();
    }
    if cmd.contains("start") {
        return "Start Script".into();
    }

    "Dev Service".into()
}
