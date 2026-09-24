# LocalOps POS

A local-first, offline business operations and point-of-sale desktop application for configurable, multi-department businesses.

## Status

All ten V1 architecture phases are implemented. The source-of-truth specification is in [`docs/MASTER-ARCHITECTURE.md`](docs/MASTER-ARCHITECTURE.md), with implementation status in [`docs/localops-pos/PROGRESS.md`](docs/localops-pos/PROGRESS.md).

## Approved V1 direction

- Windows desktop application
- Tauri shell with React and TypeScript UI
- Rust domain/application layer
- Local SQLite database as the source of truth
- Fully functional without an internet connection

See [`docs/BUILD-PLAN.md`](docs/BUILD-PLAN.md) for the incremental plan.

## Development

Prerequisites: Node.js, Rust, and the Windows WebView2 runtime.

```powershell
npm install
npm test -- --run
npm run build
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm run tauri dev
```

## Windows release

Build the signed-ready NSIS installer with:

```powershell
npm run release:windows
```

Artifacts are written below `target/release/bundle/nsis`. Customer business data and verified backups live below `%LOCALAPPDATA%\LocalOps\POS`, separate from the installation directory. Existing data from the legacy `%LOCALAPPDATA%\LocalOps POS` location is migrated with SQLite's backup API on first launch and the source is retained as a safety copy.

Receipts use the Windows OS print dialog. USB barcode scanners work as keyboard input and submit barcode, SKU, or product-code text with Enter.

See [`docs/RELEASE-CHECKLIST.md`](docs/RELEASE-CHECKLIST.md) before distributing a build.
