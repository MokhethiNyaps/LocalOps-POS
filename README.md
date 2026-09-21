# LocalOps POS

A local-first, offline business operations and point-of-sale desktop application for configurable, multi-department businesses.

## Status

Foundation planning and architecture. The source-of-truth specification is in [`docs/MASTER-ARCHITECTURE.md`](docs/MASTER-ARCHITECTURE.md), with a Word copy in [`docs/MASTER-ARCHITECTURE.docx`](docs/MASTER-ARCHITECTURE.docx).

## Approved V1 direction

- Windows desktop application
- Tauri shell with React and TypeScript UI
- Rust domain/application layer
- Local SQLite database as the source of truth
- Fully functional without an internet connection

See [`docs/BUILD-PLAN.md`](docs/BUILD-PLAN.md) for the incremental plan.
