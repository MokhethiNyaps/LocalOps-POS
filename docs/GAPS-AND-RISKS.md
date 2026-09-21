# Gaps, Conflicts and Risks

## Findings

- This is a greenfield project; there was no existing repository or stack to preserve.
- No conflict exists between the approved Windows/Tauri direction and the master architecture.
- The current environment has Node.js but does not have the Rust toolchain. Tauri/Rust code can be authored, but it cannot honestly be marked tested here until Rust is available. The build must not claim completion based only on static files.
- No GitHub CLI or GitHub credentials are available. A local Git repository has been created with frequent commits. A remote can be attached and all commits pushed once the user supplies an empty repository URL and usable authentication.

## Explicit V1 boundaries

- One application process and one local SQLite database; no network-share database and no multi-terminal server.
- One currency per business and one configurable tax regime; no accounting ledger or country tax engine.
- Sub-recipes, cloud sync, payroll, loyalty, payment processing and advanced costing are deferred.
- Windows OS print dialog first; direct ESC/POS is a later adapter.

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| Data loss during upgrade | Verified pre-migration backup, migration ledger, integrity checks and recovery runbook. |
| UI bypasses rules | All mutations are Tauri commands into Rust application services; no SQL in React. |
| Cross-business references | Business-scoped repositories plus transaction validation and integration tests. |
| Quantity overflow/conversion drift | Checked integer arithmetic and rational conversion factors; reject non-exact unsupported conversions. |
| Duplicate POS submission | Required business-scoped idempotency key. |
| Cached stock divergence | Movement ledger remains authoritative; reconciliation test and repair command rebuild cache. |
| PIN compromise | Argon2id hashes, lockout, no plaintext logs, manager permissions and local threat-model documentation. |
