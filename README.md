# LocalOps POS

A local-first, offline business operations and point-of-sale desktop application for configurable, multi-department businesses.

## Status

All ten original V1 architecture phases are implemented. The broader architectural baseline is in [`docs/MASTER-ARCHITECTURE.md`](docs/MASTER-ARCHITECTURE.md), with implementation status in [`docs/localops-pos/PROGRESS.md`](docs/localops-pos/PROGRESS.md).

## Current implementation mandate

The normative specification for the next implementation is:

**[`docs/LOCALOPS_OWNER_CASHIER_ACCESS_CONTROL_SPEC_V2.md`](docs/LOCALOPS_OWNER_CASHIER_ACCESS_CONTROL_SPEC_V2.md)**

Implementation agents must read that specification completely before editing code. It is the authoritative contract for Owner/Cashier access control and supersedes earlier Owner/Cashier access-control drafts. Agents must also inspect the working implementation and the current supporting documents referenced by the specification rather than relying on assumptions.

Where the Owner/Cashier specification deliberately changes existing access behavior, follow it while preserving the broader architectural and data-integrity principles in `docs/MASTER-ARCHITECTURE.md`.

## Approved application direction

- Windows desktop application
- Tauri shell with React and TypeScript UI
- Rust domain/application layer
- Local SQLite database as the source of truth
- Fully functional without an internet connection

## Documentation

- [Owner/Cashier Access Control Specification — Revision 2](docs/LOCALOPS_OWNER_CASHIER_ACCESS_CONTROL_SPEC_V2.md) — normative implementation contract for the next feature
- [Master Architecture](docs/MASTER-ARCHITECTURE.md) — broader architecture and integrity principles
- [Schema V1](docs/SCHEMA-V1.md) — existing schema reference; verify it against migrations and code
- [Implementation Progress](docs/localops-pos/PROGRESS.md) — current implementation history and status
- [Release Checklist](docs/RELEASE-CHECKLIST.md) — release validation requirements

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
