<div align="center">
  <img src="./images/9router.png" alt="9Router Dashboard" width="800"/>

  # 9Router - Native Rust AI Gateway & Token Saver

  **Never stop coding. Save 20-60% tokens with built-in Headroom + auto-fallback to FREE & cheap AI models.**

  **Connect All AI Code Tools (Claude Code, Cursor, Windsurf, Codex, Gemini, OpenCode, Cline, Roo Code...) to 40+ AI Providers & 100+ Models.**

  [![License: MIT](https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-blue.svg)](./LICENSE)
  [![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg?logo=rust)](https://www.rust-lang.org/)
  [![GitHub Release](https://img.shields.io/github/v/release/SyaPratama/9router-rust?logo=github&color=blue)](https://github.com/SyaPratama/9router-rust/releases)
  [![GHCR](https://img.shields.io/badge/GHCR-Package-blue?logo=github)](https://github.com/SyaPratama/9router-rust/pkgs/container/9router-rust)
  [![Memory](https://img.shields.io/badge/RAM-%3C15MB-success.svg)]()
  [![Throughput](https://img.shields.io/badge/Throughput-15k%2B%20req%2Fs-blue.svg)]()

  [🚀 Quick Start](#-quick-start) • [⚡ Benchmarks](#-performance-benchmark) • [💡 Features](#-key-features) • [🔌 Connect Tools](#-connect-ai-tools) • [📖 Setup Guide](#-setup-guide)
</div>

---

## 🤔 Why 9Router?

**Stop wasting money, tokens, and hitting limits:**

- ❌ Subscription quota expires unused every month
- ❌ Rate limits stop you mid-coding
- ❌ Tool outputs (git diff, grep, logs, test outputs) burn tokens fast
- ❌ Expensive APIs ($20–50/month per provider)
- ❌ Manual switching between providers when one goes down

**9Router solves this:**

- ✅ **Native Headroom Token Saver** — Auto-compress tool outputs, diffs, logs, and JSON arrays to cut 20–60% tokens per request
- ✅ **100% Native Rust** — Ultra-lightweight `< 15MB RAM`, sub-millisecond gateway latency, zero Node.js footprint
- ✅ **Auto-Fallback Chains** — Subscription → Cheap → Free models with zero downtime
- ✅ **Multi-Account Rotation** — Round-robin between multiple accounts per provider with automated strike counters
- ✅ **Universal Compatibility** — Bidirectional translation between OpenAI, Claude Messages, Responses (Codex), and Gemini APIs

---

## 🔄 How It Works

```
┌──────────────────────────────────────────────────────────┐
│                      Your AI Tools                       │
│     (Claude Code, Cursor, Windsurf, Cline, Roo, Codex)   │
└────────────────────────────┬─────────────────────────────┘
                             │ http://localhost:20128/v1
                             ↓
┌──────────────────────────────────────────────────────────┐
│                  9Router-Rust Gateway                    │
│  • Headroom Engine (compress diffs, logs, tool_results)  │
│  • Format Translation (OpenAI ↔ Claude ↔ Gemini ↔ Codex) │
│  • Quota Tracking & Strike Counters                      │
│  • SQLite Persistence with WAL mode                      │
└────────┬────────────────────────────────────────┬────────┘
         │                                        │
         ├─→ [Tier 1: SUBSCRIPTION]               ├─→ [Direct Providers]
         │   Claude Code, Codex, Copilot          │   OpenAI, Anthropic,
         │   ↓ quota exhausted                    │   DeepSeek, Google,
         ├─→ [Tier 2: CHEAP]                      │   Groq, OpenRouter,
         │   DeepSeek ($0.14/1M), MiniMax, GLM    │   Ollama, Mistral...
         │   ↓ rate limit                         │
         └─→ [Tier 3: FREE]                       │
             Kiro AI, OpenCode Free, Gemini       │

Result: Never stop coding · 20-60% token savings · <15MB RAM
```

---

## ⚡ Performance Benchmark

<div align="center">
  <img src="./images/benchmark-dark.png" alt="9Router Performance Benchmark" width="100%"/>
</div>

| Gateway | Runtime | Real TCP Throughput | Idle Memory (RSS) | Median Latency (p50) |
| :--- | :--- | :--- | :--- | :--- |
| **9Router-Rust** | **Native Rust (Axum + Tokio)** | **15,408 req/s** *(live TCP)*<br>**217,420 req/s** *(in-process)* | **14.4 MB** | **2.58 ms** *(live TCP)*<br>**0.003 ms** *(in-process)* |
| 9Router | Node.js / Next.js | 525 req/s | 131 MB | 86.4 ms |
| OmniRoute | Node.js / TypeScript | 354 req/s | 424 MB | 123.1 ms |

---

## ⚡ Quick Start

### 1. Run with Docker (Recommended)

Pull and run directly from GitHub Packages:

```bash
docker run -d \
  -p 20128:20128 \
  -v "$HOME/.9router:/app/data" \
  -e DATA_DIR=/app/data \
  --name 9router \
  ghcr.io/syapratama/9router-rust:latest
```

Open dashboard: [http://localhost:20128/dashboard](http://localhost:20128/dashboard)

### 2. Download Prebuilt Binary

Download the compiled binary for your operating system from [GitHub Releases](https://github.com/SyaPratama/9router-rust/releases/latest):

- **Linux x86_64 / ARM64** (`.tar.gz`)
- **Windows x86_64** (`.zip`)
- **macOS Apple Silicon / Intel** (`.tar.gz`)

Extract and launch:
```bash
# Linux / macOS
tar -xzf 9router-*.tar.gz
./9router start

# Windows
.\9router.exe start
```

### 3. Build from Source

Prerequisites: [Rust 1.85+](https://rustup.rs/)

```bash
git clone https://github.com/SyaPratama/9router-rust.git
cd 9router-rust
cargo build --release
./target/release/9router start
```

---

## 🛠️ Connect AI Tools

### Claude Code

```bash
export ANTHROPIC_BASE_URL="http://localhost:20128"
export ANTHROPIC_API_KEY="sk-9router"
claude
```

### Cursor / Windsurf / Codex / Cline / Roo Code / OpenCode

Configure your tool's OpenAI-compatible settings:

| Setting | Value |
| :--- | :--- |
| **Base URL** | `http://localhost:20128/v1` |
| **API Key** | `sk-9router` *(or key configured in `/api/keys`)* |
| **Model** | `gpt-4o`, `claude-3-5-sonnet`, `deepseek-chat`, or your custom combo |

---

## 📖 Setup Guide

### 1. Access Dashboard
Navigate to [http://localhost:20128/dashboard](http://localhost:20128/dashboard) to manage your gateway visually.

### 2. Configure Providers
- Go to **Providers** tab.
- Add your upstream API keys (OpenAI, Anthropic, DeepSeek, Google Gemini, Groq, OpenRouter, etc.).
- Set priorities and active states for automatic load-balancing and account failover.

### 3. Create Model Combos (Fallback Chains)
- Go to **Combos** tab.
- Define fallback chains (e.g. `claude-3-5-sonnet -> deepseek-chat -> gpt-4o-mini`).
- If the primary model encounters a rate limit or 429 quota exhaustion, 9Router automatically routes to the next model seamlessly.

---

## 🛠️ CLI Commands

```bash
# Start the gateway daemon
9router start [--port 20128] [--host 0.0.0.0]

# Run live TCP performance benchmark
9router bench [--concurrency 50] [--requests 1000]

# Check gateway status and RSS memory usage
9router status

# Open interactive terminal configuration menu
9router menu

# Stop the running gateway daemon
9router stop
```

---

## 📝 API Endpoints

### LLM Compatibility Endpoints
| Method | Endpoint | Description |
| :--- | :--- | :--- |
| `POST` | `/v1/chat/completions` | OpenAI Chat Completions (streaming & non-streaming) |
| `POST` | `/v1/messages` | Anthropic Claude Messages API compatibility |
| `POST` | `/v1/responses` | OpenAI Responses / Codex API compatibility |
| `GET` | `/v1/models` | List all available models, providers, and combos |
| `POST` | `/v1/compress` | Native Headroom context compression endpoint |

### Gateway Management Endpoints
| Method | Endpoint | Description |
| :--- | :--- | :--- |
| `GET` | `/api/health` | Gateway health status, uptime, and engine info |
| `GET/POST`| `/api/settings` | Read or update gateway settings |
| `GET/POST`| `/api/providers` | Manage provider accounts and credentials |
| `GET/POST`| `/api/combos` | Configure fallback model chains |
| `GET/POST`| `/api/keys` | Manage API keys and access restrictions |
| `GET` | `/api/usage/stats` | View token usage, costs, and request counts |

---

## 👥 Contributors

Thanks to everyone who helped build and evolve 9Router!

[![Contributors](https://contrib.rocks/image?repo=decolua/9router&max=150&columns=15&anon=1&v=20260309)](https://github.com/decolua/9router/graphs/contributors)

- **Original Creator & Maintainers**: [decolua](https://github.com/decolua) and 9Router contributors.
- **Native Rust & Headroom Refactor**: [SyaPratama](https://github.com/SyaPratama).

---

## 🔀 Forks & Acknowledgments

Full attributions are documented in [CREDITS.md](./CREDITS.md):

- **[9Router](https://github.com/decolua/9router)** — Original local AI routing gateway and Next.js / open-sse engine by `decolua`.
- **[OmniRoute](https://github.com/diegosouzapw/OmniRoute)** — Full-featured TypeScript fork of 9Router by [@diegosouzapw](https://github.com/diegosouzapw).
- **[Headroom Labs](https://github.com/headroomlabs-ai/headroom)** — Foundational context compression algorithms and CCR architecture (Apache 2.0).
- **[RTK](https://github.com/rtk-ai/rtk)** — Token-saving heuristics and tool output deduplication patterns.
- **[Ponytail](https://github.com/DietrichGebert/ponytail)** by [@DietrichGebert](https://github.com/DietrichGebert) — Minimalist lazy-senior-developer prompt engineering.
- **[Caveman](https://github.com/JuliusBrussee/caveman)** by [@JuliusBrussee](https://github.com/JuliusBrussee) — Ultra-terse prompt compression patterns.
- **[CLIProxyAPI](https://github.com/router-for-me/CLIProxyAPI)** — Original Go proxy architecture inspiration.

---

## 📄 License

Dual-licensed under the **MIT License** and **Apache License 2.0**. See [LICENSE](./LICENSE) and [CREDITS.md](./CREDITS.md) for details.

Copyright (c) 2024-2026 decolua and contributors.  
Copyright (c) 2026 SyaPratama and 9Router-Rust contributors.  
Copyright (c) 2024-2026 Headroom Labs Inc.
