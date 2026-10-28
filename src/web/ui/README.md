# Rust Dashboard UI

React 19 and Vite source for the dashboard. The Rust service serves the production bundle from `../assets`.

```sh
bun install
bun run dev
```

Build the embedded production assets with `bun run build`. The bundle is written to `src/web/assets` and must be rebuilt after frontend changes.

