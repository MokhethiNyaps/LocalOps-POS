# Build Plan

## Delivery rules

Each vertical slice must include domain rules, persistence, audit/history, reporting impact, and automated tests. A tested slice receives its own commit. GitHub pushes will follow each tested commit after a remote is connected.

## Phases

1. **Architecture foundation** — decisions record, ERD, V1 schema, risks/gaps.
2. **Executable foundation** — Tauri/React shell, Rust workspace, SQLite connection, migrations, health check, data-directory handling.
3. **Business setup** — business, departments, locations, terminal, local user/PIN, roles and permissions.
4. **Catalogue** — categories, units, products, services, pricing, packaging, recipes and consumables.
5. **Inventory** — movement ledger, balances, receiving, transfers, wastage and stock counts.
6. **POS and payments** — atomic sale completion, split payments, cash/change, stock and recipe consumption, receipts.
7. **Shifts and expenses** — opening/closing, reconciliation and basic expense records.
8. **Reports and dashboard** — business and department totals, payments, stock and variances.
9. **Safety** — backups/restores, migration verification, audit coverage and offline tests.
10. **Packaging** — Windows installer, printing adapter and release checklist.

## First build milestone

A Windows desktop shell that creates or opens a versioned SQLite database in the user application-data directory, runs migrations safely, and passes database integrity tests without network access.
