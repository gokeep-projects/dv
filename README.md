<div align="center">

# DevTool

**High-performance Rust development toolkit with hot-swappable plugin architecture**

[![CI](https://github.com/gokeep-projects/dv/actions/workflows/ci.yml/badge.svg)](https://github.com/gokeep-projects/dv/actions/workflows/ci.yml)
[![Release](https://github.com/gokeep-projects/dv/actions/workflows/release.yml/badge.svg)](https://github.com/gokeep-projects/dv/releases)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

[Features](#features) | [Install](#installation) | [Usage](#usage) | [Architecture](#architecture) | [Plugins](#plugins) | [Build](#building)

</div>

---

## Features

- **Three Interfaces** — CLI, TUI (terminal UI), Web UI, all sharing the same plugin set
- **Hot-Swappable Plugins** — Dynamic `.so`/`.dll` loading via `libloading`, reload without restart
- **12 Built-in Plugins** — JSON, Crypto, Terminal, Log Search, Service Status, Middleware, Script Runner, Git, HTTP, File, Elasticsearch, Sysinfo
- **Real-time Dashboard** — CPU, memory, disk, network, process monitoring with anomaly detection
- **Middleware Management** — Redis, Elasticsearch, Kafka, Nginx, Tomcat, Caddy, Docker
- **Docker Management** — Container lifecycle, stats, logs, inspection
- **Cross-Platform** — Linux (x86_64, aarch64), macOS (Intel, Apple Silicon), Windows
- **Static Binaries** — musl-linked Linux builds, zero external dependencies, offline deployment
- **Auto-Discovery** — Detects running middleware services automatically

## Installation

### Download Pre-built Binaries

Download from [GitHub Releases](https://github.com/gokeep-projects/dv/releases):

| Platform | Architecture | File |
|----------|-------------|------|
| Linux | x86_64 | `devtool-linux-x86_64.tar.gz` |
| Linux | aarch64 | `devtool-linux-aarch64.tar.gz` |
| macOS | x86_64 | `devtool-macos-x86_64.tar.gz` |
| macOS | aarch64 | `devtool-macos-aarch64.tar.gz` |
| Windows | x86_64 | `devtool-windows-x86_64.zip` |

```bash
# Linux x86_64
curl -L https://github.com/gokeep-projects/dv/releases/latest/download/devtool-linux-x86_64.tar.gz | tar xz
chmod +x devtool
sudo mv devtool /usr/local/bin/

# macOS (Apple Silicon)
curl -L https://github.com/gokeep-projects/dv/releases/latest/download/devtool-macos-aarch64.tar.gz | tar xz
chmod +x devtool
sudo mv devtool /usr/local/bin/
```

### Build from Source

```bash
git clone https://github.com/gokeep-projects/dv.git
cd dv
cargo build --release --bin devtool
```

## Usage

### TUI Mode (Interactive Terminal UI)

```bash
devtool tui
```

**Keybindings:**

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Switch views (Dashboard / Plugins / Docker / Middleware) |
| `↑` / `↓` | Navigate items |
| `Enter` | Select / Execute |
| `q` | Quit |
| `/` | Search / Filter |
| `F1` | Help |

### Web Mode

```bash
devtool web --port 8080 --host 0.0.0.0
```

Opens a glassmorphism-themed web dashboard at `http://localhost:8080`.

### CLI Mode

```bash
# List all plugins
devtool list

# Execute a plugin directly
devtool exec json-tool format --input '{"b":2,"a":1}'
devtool exec crypto hash --algo sha256 --input "hello"
devtool exec crypto base64-encode --input "hello"
devtool exec log-search grep --pattern "ERROR" --path /var/log/syslog

# Plugin management
devtool plugin reload json-tool
devtool plugin load ./path/to/custom.so

# Generate shell completions
devtool completions bash > ~/.bash_completion.d/devtool
devtool completions zsh > ~/.zfunc/_devtool
```

## Architecture

```
devtool/
├── crates/
│   ├── core/          # Plugin trait, PluginManager, shared types
│   ├── cli/           # CLI binary entry point, arg parsing
│   ├── tui/           # Terminal UI (ratatui + crossterm)
│   │   ├── app.rs     # Main app state, event loop, rendering
│   │   ├── dashboard.rs  # System monitoring, anomaly detection
│   │   ├── theme.rs   # Color palette, styles
│   │   └── middleware/ # Redis, ES, Kafka, Nginx, Docker, etc.
│   ├── web/           # Web server (axum + WebSocket)
│   └── plugins/       # 12 plugin crates (cdylib)
├── assets/web/        # Web UI (HTML/CSS/JS, embedded via rust-embed)
└── scripts/           # Build & release scripts
```

### Plugin System

Each plugin is a Rust crate compiled as `cdylib`. The `Plugin` trait:

```rust
pub trait Plugin: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    fn execute(&self, input: PluginInput) -> PluginResult<PluginOutput>;
    fn tui_view(&self) -> Option<TuiViewDef>;
    fn web_handlers(&self) -> Vec<WebHandlerDef>;
    fn init(&mut self) -> PluginResult<()>;
    fn shutdown(&mut self);
}
```

Plugins are discovered at runtime from the plugin directory. Hot-reload via `devtool plugin reload <name>` or pressing `r` in TUI.

## Plugins

| Plugin | Key | Description |
|--------|-----|-------------|
| **JSON Tool** | `j` | Format, validate, query (jq-like), diff JSON |
| **Crypto** | `c` | AES, RSA, Base64, SHA256, MD5, JWT, HMAC |
| **Terminal** | `t` | Shell command execution, PTY interaction |
| **Log Search** | `l` | Regex/grep with highlighting, log parsing |
| **Service Status** | `s` | HTTP/TCP/process health checks |
| **Middleware** | `m` | Redis/MySQL/Kafka connection testing |
| **Script Runner** | `r` | Embedded Rhai script execution |
| **Git Tools** | `g` | Git operations, diff, log, status |
| **HTTP Client** | `h` | HTTP requests with full method support |
| **File Tool** | `f` | File operations, search, watch |
| **Elasticsearch** | `e` | ES cluster management, index operations |
| **Sysinfo** | `i` | System information gathering |

## Building

### Prerequisites

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# For cross-compilation
cargo install cross --git https://github.com/cross-rs/cross

# Linux x86_64 static build
sudo apt install musl-tools
```

### Build Commands

```bash
# Development build
cargo build

# Release build (optimized, stripped)
cargo build --release --bin devtool

# Cross-compile for aarch64
cross build --release --target aarch64-unknown-linux-musl --bin devtool

# Cross-compile for Windows
cross build --release --target x86_64-pc-windows-gnu --bin devtool

# Run tests
cargo test --workspace --lib

# Run linter
cargo clippy --workspace -- -D warnings
```

### Release

```bash
# Bump version and create tag
./scripts/release.sh patch   # 0.1.0 -> 0.1.1
./scripts/release.sh minor   # 0.1.1 -> 0.2.0
./scripts/release.sh major   # 0.2.0 -> 1.0.0

# Push tag to trigger GitHub Actions release
git push origin v0.1.1
```

## CI/CD

GitHub Actions automatically:

- **On PR/push to master**: `cargo check`, `cargo test`, `cargo clippy`
- **On tag push (`v*`)**: Build for 5 platforms, create GitHub Release with binaries

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing`)
3. Commit changes (`git commit -m 'feat: add amazing feature'`)
4. Push to branch (`git push origin feature/amazing`)
5. Open a Pull Request

## License

MIT License - see [LICENSE](LICENSE) for details.

---

<div align="center">

**Built with Rust** | **Powered by ratatui + axum**

</div>
