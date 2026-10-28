# 9Router Test Bench

Isolated read-only website and unit tests. It does not change production UI or API state.

```sh
bun install
bun run test
bun run build
bun run dev
```

Open `http://127.0.0.1:4173` while the Rust dashboard is running on `http://127.0.0.1:20129`. Set `ROUTER_URL` to another loopback Rust URL if needed. The page reads health, providers, quota, and persisted usage through a same-origin Vite proxy.

`bun run test:oauth` opens visible Chrome for an interactive provider login. Run it only with an empty Rust provider-connection store and configured OAuth callback/client settings. Review provider scopes yourself. It never reads or logs credentials, and leaves the newly connected account saved so its live quota can be checked.
