<div align="center">
  <img src="./images/9router.png" alt="9Router Native Rust Dashboard" width="800"/>
  
  # 9Router — Native Rust AI Gateway & Headroom Token Saver
  
  **100% Native Rust AI routing gateway with built-in Headroom context compression, <15MB idle RAM, 16.5x-29x throughput, and zero Node.js footprint.**
  
  [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
  [![Rust](https://img.shields.io/badge/Rust-1.85+-orange.svg?logo=rust)](https://www.rust-lang.org/)
  [![Tests](https://img.shields.io/badge/Tests-20%2F20%20Passing-brightgreen.svg)]()
  [![Memory](https://img.shields.io/badge/RAM-%3C15MB-success.svg)]()
  [![Throughput](https://img.shields.io/badge/Throughput-15k%2B%20req%2Fs-blue.svg)]()
  [![Credits](https://img.shields.io/badge/Credits-decolua%20%26%20Headroom%20Labs-purple.svg)](./CREDITS.md)

  [🚀 Quick Start](#-quick-start) • [⚡ Benchmarks](#-performance-benchmark) • [💡 Features](#-key-features) • [🔌 Integrations](#-connecting-ai-tools) • [👥 Contributors](#-contributors)
</div>

---

## ⚡ Performance Benchmark

<div align="center">
  <img src="./images/benchmark-dark.png" alt="9Router Performance Benchmark" width="100%"/>
</div>

**16.5x to 29x the throughput of 9Router (Next.js) and 43x of OmniRoute, with a ninth of 9Router's idle memory.**

*Tested on `/api/health` across 1,000 requests at concurrency 50 on a 4-vCPU host.*

| Gateway | Runtime | Real TCP Throughput | Idle Memory (RSS) | Median Latency (p50) |
| :--- | :--- | :--- | :--- | :--- |
| **9Router-Rust** | **Native Rust (Axum + Tokio)** | **15,408 req/s** *(live TCP)*<br>**217,420 req/s** *(in-process)* | **14.4 MB** | **2.58 ms** *(live TCP)*<br>**0.003 ms** *(in-process)* |
| 9Router | Node.js / Next.js | 525 req/s | 131 MB | 86.4 ms |
| OmniRoute | Node.js / TypeScript | 354 req/s | 424 MB | 123.1 ms |

*Run metrics published in [raw-results.json](./raw-results.json). Reproduce live benchmark anytime with:*
```bash
cargo test --release --test benchmark_test -- --nocapture
./target/release/9router bench --concurrency 50 --requests 1000
```

---

## 💡 Key Features

- 🦀 **100% Native Rust Architecture** — Zero Node.js, Next.js, or JavaScript runtime dependencies. Compiled to a single self-contained binary with embedded web dashboard.
- ⚡ **Extreme Resource Efficiency** — Slashes idle memory from ~250MB down to `< 15MB RSS`, with sub-millisecond gateway routing latency.
- 🧠 **Native Headroom Context Compression** — Directly integrated into the routing engine (based on [Headroom Labs](https://github.com/headroomlabs-ai/headroom), Apache 2.0). Natively compresses tool outputs, git diffs, logs, search matches, code, and RAG chunks before LLM ingestion:
  - **SmartCrusher**: Compares and deduplicates uniform JSON object arrays into compact schema + rows.
  - **DiffCompressor**: Prunes redundant context lines around additions/deletions while preserving 100% of diff hunks and headers.
  - **LogCompressor**: Deduplicates and collapses repeated log lines with ISO timestamp normalization while preserving error stack traces and panics.
  - **SearchCompressor**: Clusters grep / ripgrep outputs by file path to eliminate repeated prefix tokens.
  - **CodeCompressor**: Preserves signatures and contracts while compacting non-focused method bodies.
  - **RagCompressor**: Deduplicates retrieved document chunks.
  - **CCR Store**: Compress-Cache-Retrieve in-memory LRU store keyed by SHA-256 for instant retrieval of original payloads.
  - **Fail-Open Guarantee**: Sub-500 byte payloads or transformations saving `< 5%` pass through unmodified.
- 🔄 **Universal Format Translation** — Real-time bidirectional translation between:
  - **OpenAI Chat Completions** (`/v1/chat/completions`)
  - **Anthropic Claude Messages** (`/v1/messages`)
  - **OpenAI Responses API** (`/v1/responses`)
  - **Google Gemini API** (contents / parts)
- 🛡️ **Resilient Fallback Chains** — Multi-account failover with automated strike counters + multi-model combo rotation on upstream 429 / quota exhaustion.
- 🗄️ **Seamless SQLite Persistence** — Full compatibility with existing 9Router v2 databases (`~/.9router/db.sqlite`). Features WAL mode, busy timeout resilience, and connection pooling.
- 🔒 **Hardened Security Middleware** — Unspoofable client IP derivation from raw TCP socket stream, loopback reverse proxy validation, and Bearer / `x-api-key` validation.

---

## 🚀 Quick Start

### Build & Run from Source

Prerequisites: [Rust 1.85+](https://rustup.rs/)

```bash
# Clone the repository
git clone https://github.com/SyaPratama/9router-rust.git
cd 9router-rust

# Build optimized release binary
cargo build --release

# Start the gateway (default port: 20128)
./target/release/9router start
```

Open the embedded dashboard in your browser: [http://localhost:20128/dashboard](http://localhost:20128/dashboard)

### Run with Docker

```bash
docker run -d \
  -p 20128:20128 \
  -v "$HOME/.9router:/app/data" \
  -e DATA_DIR=/app/data \
  --name 9router \
  ghcr.io/syapratama/9router-rust:latest
```

---

## 🛠️ CLI Commands

```bash
# Start gateway server
9router start [--port 20128] [--host 0.0.0.0]

# Run dynamic TCP performance benchmark
9router bench [--concurrency 50] [--requests 1000]

# Inspect runtime status and RSS memory
9router status

# Open interactive terminal configuration menu
9router menu

# Gracefully terminate running background daemon
9router stop
```

---

## 🔌 Connecting AI Tools

### Claude Code

```bash
export ANTHROPIC_BASE_URL="http://localhost:20128"
export ANTHROPIC_API_KEY="sk-9router"
claude
```

### Cursor / Codex / Windsurf / Cline / Roo Code

Point your OpenAI-compatible base URL to:
```text
Base URL: http://localhost:20128/v1
API Key:  sk-9router (or any key configured in /api/keys)
Model:    gpt-4o, claude-3-5-sonnet, deepseek-chat, or custom combo
```

---

## 📝 API Endpoints

### Compatibility Endpoints
| Method | Endpoint | Description |
| :--- | :--- | :--- |
| `POST` | `/v1/chat/completions` | OpenAI Chat Completions streaming & non-streaming |
| `POST` | `/v1/messages` | Anthropic Claude Messages API compatibility |
| `POST` | `/v1/responses` | OpenAI Responses / Codex API compatibility |
| `GET` | `/v1/models` | List all available models, providers, and combos |
| `POST` | `/v1/compress` | **Native Headroom Compression** (compress messages/content) |

### Management & Health Endpoints
| Method | Endpoint | Description |
| :--- | :--- | :--- |
| `GET` | `/api/health` | Gateway health check and engine info |
| `GET/POST`| `/api/settings` | Retrieve or update system configuration |
| `GET/POST`| `/api/providers` | Manage upstream provider credentials and priorities |
| `GET/POST`| `/api/combos` | Configure fallback model chains |
| `GET/POST`| `/api/keys` | Manage API keys and access restrictions |
| `GET` | `/api/usage/stats` | Aggregate token consumption, costs, and request counts |

---

## 👥 Contributors

Thanks to all contributors who have helped build and evolve 9Router!

[![Contributors](https://contrib.rocks/image?repo=decolua/9router&max=150&columns=15&anon=1&v=20260309)](https://github.com/decolua/9router/graphs/contributors)

- **Original Creator & Maintainers**: [decolua](https://github.com/decolua) and 9Router contributors.
- **Native Rust & Headroom Refactor**: [SyaPratama](https://github.com/SyaPratama).

---

## 📊 Star Chart

[![Star Chart](https://starchart.cc/decolua/9router.svg?variant=adaptive)](https://starchart.cc/decolua/9router)

---

## 🔀 Forks & Community

- **[OmniRoute](https://github.com/diegosouzapw/OmniRoute)** — A full-featured TypeScript fork of 9Router by [@diegosouzapw](https://github.com/diegosouzapw).

---

## 🙏 Acknowledgments & Credits

See [CREDITS.md](./CREDITS.md) for detailed open-source licensing and attributions:

- **[9Router](https://github.com/decolua/9router)** — Original local AI routing gateway and Next.js / open-sse engine by `decolua`.
- **[Headroom Labs](https://github.com/headroomlabs-ai/headroom)** — Foundational context compression algorithms and CCR architecture (Apache 2.0).
- **[RTK (Rust Token Killer)](https://github.com/rtk-ai/rtk)** — Token-saving heuristics and tool output deduplication patterns.
- **[Ponytail](https://github.com/DietrichGebert/ponytail)** by [@DietrichGebert](https://github.com/DietrichGebert) — Minimalist lazy-senior-developer prompt engineering.
- **[Caveman](https://github.com/JuliusBrussee/caveman)** by [@JuliusBrussee](https://github.com/JuliusBrussee) — Ultra-terse prompt compression patterns.
- **[CLIProxyAPI](https://github.com/router-for-me/CLIProxyAPI)** — Original Go proxy architecture that inspired the initial gateway design.

---

## 📄 License

Dual-licensed under the **MIT License** and **Apache License 2.0**. See [LICENSE](./LICENSE) and [CREDITS.md](./CREDITS.md) for complete details.

Copyright (c) 2024-2026 decolua and contributors.  
Copyright (c) 2026 SyaPratama and 9Router-Rust contributors.  
Copyright (c) 2024-2026 Headroom Labs Inc.
