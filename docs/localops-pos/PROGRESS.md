# LocalOps POS Progress

## Status: V1 Implementation Complete

## Quick Reference

- Research: `RESEARCH.md`
- Implementation: `IMPLEMENTATION.md`
- Architecture: `../MASTER-ARCHITECTURE.md`
- Build plan: `../BUILD-PLAN.md`

## Completed

- Architecture and executable foundations
- SQLite bootstrap, migration 0001, integrity checks, and pre-upgrade backup
- Business, department, location, terminal, category, unit, product, service, and user repositories
- Secure local Argon2id authentication with lockout and legacy-hash upgrade
- Data-driven roles and permissions
- Atomic product/service creation
- Department sellable availability and price overrides
- Working first-run business creation UI and frontend test
- Schema migration 0002 with terminal device identity and legacy role reconciliation
- Department/location defaults, local session lifecycle, login/logout commands, and atomic owner onboarding
- Exact integer unit conversions, product packaging, and versioned product/service recipes
- Session-scoped catalogue commands with permission checks and atomic audit events
- Login-aware desktop workspace for categories, exact units, products, services, packaging, recipes, and department availability
- Ledger-backed inventory workspace for suppliers, receiving, transfers, waste/damage, stock counts, balances, and recent movements
- Atomic POS workspace with split payments, structured receipts, idempotent completion, recipe/direct stock consumption, partial refunds, and full voids
- Audited shift opening/closing with preserved cash variance plus idempotent expense recording/voiding and cash-drawer effects
- Persisted-fact business dashboard with department, payment, expense, shift, stock, low-stock, and gross-profit summaries plus local CSV export
- Verified SQLite-native backups with 7 daily/4 weekly/12 monthly retention, path-safe restore, migration-matrix tests, and database/audit reconciliation UI
- Signed-ready NSIS packaging, OS print-dialog receipts, locally configured barcode/SKU/product-code scanner input, and verified install/launch/uninstall behavior
- Customer data isolated in `%LOCALAPPDATA%\LocalOps\POS` with WAL-aware migration from the legacy install-directory location

## Phase 10 Tasks

- [x] Windows installer and release configuration
- [x] Structured receipt print adapter
- [x] Barcode keyboard-input workflow
- [x] End-to-end release checklist and packaged validation

## Decisions

- Preserve migration 0001; all structural reconciliation uses numbered forward migrations.
- Continue using exact integer money and quantity representations.
- Treat low-level sellable/subtype inserts as crate-private and expose atomic aggregate creation.
- Use existing architecture documents instead of inventing parallel domain rules.

## Blockers

- None currently.

## Session Log

### 2026-09-23

- Repaired frontend test failure and Rust warnings.
- Replaced weak PIN hashing with Argon2id and added business-scoped lockout behavior.
- Added roles/permissions, atomic catalogue creation, department availability, and first-run UI.
- Verified 93 Rust tests, frontend tests, production build, formatting, and strict lint.
- Completed Phase 3 with schema-v2 migration coverage and 101 passing Rust tests.
- Added exact conversions, packaging, and recipe consumption with 107 passing Rust tests and strict lint.
- Completed Phase 4 with audited session-scoped commands, a signed-in catalogue workspace, and mixed-operation integration coverage.
- Started Phase 5 with an authoritative, audited movement ledger, atomic balance cache, negative-stock enforcement, duplicate-reference protection, and reconciliation checks.
- Added schema migration 0003 plus atomic supplier purchasing/receiving, exact packaging conversion, centralized monetary rounding, latest-cost updates, and stock movements.
- Added migration 0004 and atomic transfers, wastage/damage, and stock counts that preserve expected/count/variance history, including zero-variance counts.
- Completed Phase 5 with session-scoped inventory commands, live reconciliation status, operational forms, movement history, frontend exact-input tests, and all required negative-stock/rollback scenarios.
- Added atomic sale completion with immutable item snapshots, exact tax rounding, split payments, cash change, direct/recipe stock consumption, duplicate-request replay, and rollback failure coverage; verified 126 Rust tests and strict Clippy.
- Completed Phase 6 with migration 0005 consumption snapshots, append-only split-payment refunds and voids, explicit stock restoration, structured receipts, session-scoped POS commands, and a desktop till; verified 129 Rust tests, 2 frontend tests, production build, and strict Clippy.
- Completed Phase 7 with migration 0006, one-open-shift enforcement, immutable expected/actual/variance close records, idempotent expenses, auditable expense voids, cash-drawer reconciliation, permissions, and desktop workflows; verified 131 Rust tests, 2 frontend tests, production build, and strict Clippy.
- Completed Phase 8 with persisted-fact business/department totals, payment and refund summaries, expenses, shift variance, current stock valuation, low-stock indicators, estimated gross profit, dashboard UI, and local CSV export; verified 132 Rust tests, 2 frontend tests, production build, and strict Clippy.
- Completed Phase 9 with automatic verified backup retention, manual backup/restore plus pre-restore safety copy, strict backup path validation, schema v1-v6 migration-matrix tests, health/reconciliation reporting, and corrupt-restore failure coverage; verified 136 Rust tests, 2 frontend tests, production build, and strict Clippy.
- Completed Phase 10 with a signed-ready NSIS installer, Windows print-dialog receipt adapter, local barcode/SKU/product-code scanner workflow, and a real current-user install/launch/uninstall smoke test.
- Corrected installer/data-directory overlap discovered during packaged validation. Customer state now lives in `%LOCALAPPDATA%\LocalOps\POS`; the first launch performs a verified SQLite/WAL-aware copy from the legacy location and retains the source as a safety copy.
- Final gates: 138 core tests, 3 desktop-command tests, 3 frontend tests, production build, formatting, strict Clippy, NSIS build, packaged launch, and byte-for-byte customer-data retention after uninstall.

## Next Step

The V1 implementation is complete. Before distributing a customer release, complete the environment-specific operator and code-signing items in `docs/RELEASE-CHECKLIST.md`.
