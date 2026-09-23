# LocalOps POS Progress

## Status: Phase 5 - In Progress

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

## Current Phase Tasks

- [x] Authoritative inventory movement ledger and cached balances
- [ ] Receiving and purchase documents
- [ ] Transfers, wastage, and stock counts
- [ ] Negative-stock enforcement and inventory audit
- [ ] Inventory command boundary, UI, and failure scenarios

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

## Next Step

Build receiving and purchase documents on top of the movement ledger.
