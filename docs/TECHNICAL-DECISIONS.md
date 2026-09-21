# Technical Decisions Record

Status: Approved foundation, 2026-09-21

## Platform and stack

| Decision | V1 choice | Rationale |
|---|---|---|
| Platform | Installed Windows 10/11 desktop application | Matches local database, filesystem, scanner and receipt-printing assumptions. |
| Hardware/input | Keyboard/mouse and touch-friendly UI; USB barcode scanner acts as keyboard | Requires no cloud or proprietary scanner SDK. |
| Shell/UI | Tauri 2 + React + TypeScript | Small Windows package and native filesystem/process access. |
| Core | Rust application/domain services | Strong types and explicit transaction/error handling. UI never writes SQL. |
| Database | SQLite, foreign keys enabled; WAL on the single local machine | Reliable embedded source of truth. A shared network SQLite file is prohibited. |
| Access layer | `rusqlite` with explicit repositories and SQL migrations | Keeps SQL/transactions visible and avoids ORM ambiguity in integrity rules. |

## Data and integrity

- **Identifiers:** UUIDv7 lowercase text, generated in the application. Stable and merge-safe for future import/synchronisation. Every principal table uses the same strategy.
- **Time:** UTC RFC 3339 text for events, with the business IANA timezone retained for display/report boundaries. Business dates use ISO `YYYY-MM-DD`.
- **Money:** signed 64-bit integer minor units. Currency is ISO 4217 on the business. V1 is single-currency per business. Binary floating point is forbidden.
- **Quantities:** signed 64-bit scaled integers with six decimal places (`quantity_micros`). Conversion factors are positive rational numerator/denominator integers; calculations use checked wide intermediates.
- **Tax:** configurable per business and item. Rate is integer parts per million. Prices may be tax-inclusive or exclusive according to explicit business configuration.
- **Rounding:** centralized round-half-away-from-zero to the nearest minor unit. Compute tax per final sale line after quantity and discount; reports sum persisted line values rather than recalculating.
- **Negative stock:** blocked by default. A business may enable manager-authorized override; actor, reason and resulting movement are audited.
- **Sales:** completion is one immediate SQLite transaction containing sale, immutable line snapshots, payments, inventory movements and audit events. An idempotency key prevents duplicate submission.
- **Payment:** completed sales require allocated payment equal to amount due. Cash tender may exceed the allocation and records change; non-cash overpayment is rejected.
- **Deletion:** historical/financial records are never deleted. Configurable records become inactive. Restrictive foreign keys protect history.

## Units and catalogue

Universal units define a dimension and conversion to a canonical unit. Product packaging defines product-specific conversion to that product's base stock unit. They are intentionally related through a common exact factor representation but remain different records. Products and services are distinct; `sellable_items` is the common POS identity. A product/service may be available in many departments. Categories may be assigned to many departments. Locations may serve many departments.

## Authentication and authorization

Local username plus 4–12 digit PIN. Store an Argon2id hash and never the PIN. Sessions are process-local, tied to a user and terminal, lock after 5 minutes of inactivity by default, and end at logout/application exit. Permission checks occur in Rust services. Manager override creates a short-lived authorization event and audit entry. No remote authentication exists.

## Migrations, backup and recovery

Embedded, numbered, forward-only SQL migrations are recorded in `schema_migrations`. Before an upgrade: close writes, run SQLite integrity check, create and verify a timestamped backup, migrate transactionally where SQLite permits, then run foreign-key and integrity checks. Failed upgrades restore no data automatically over the original; the untouched backup and error are surfaced. Retention default: 7 daily, 4 weekly and 12 monthly verified backups.

## Concurrency

V1 has one application process and local database. SQLite busy timeout and short write transactions apply. Future terminals must connect to a local server/single-writer API; they must not open a database over a network share.

## Printing and barcode

The domain produces structured receipt data. V1 uses the Windows OS print dialog via a printer adapter; ESC/POS can be added as another adapter. Barcode, SKU and product code resolve solely from local indexed data. Scanner input is buffered keyboard input ending with Enter.

## Testing

Rust unit tests for calculations and permissions; repository/integration tests against temporary SQLite databases; migration tests from every supported version; React component tests; and end-to-end business scenarios. Integrity/failure cases include rollback, duplicate submissions, insufficient stock, payment mismatch/overage, over-refund, duplicate refund, count variance, backup/restore and operation with networking disabled.

## Tax and refund V1 boundaries

V1 supports one configured tax regime per business plus item taxable status; it is not a tax engine. Refunds are append-only events linked to original lines and payment allocations. Split-payment refund allocation is selected explicitly by an authorized user and may not exceed refundable amounts. Stock restoration is an explicit per-line choice, never inferred.
