# LocalOps POS V1 Release Checklist

## Automated release gate

- [x] `cargo fmt --all -- --check`
- [x] `cargo test --workspace`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `npm test -- --run`
- [x] `npm run build`
- [x] `npm run release:windows`
- [x] Confirm the NSIS installer exists in `target/release/bundle/nsis` and is non-empty.

## Packaged lifecycle validation

- [x] Perform a silent current-user NSIS install and verify its registration.
- [x] Launch the packaged executable and verify it remains running.
- [x] Migrate legacy SQLite/WAL state and verified backups into the separate customer-data directory.
- [x] Run the packaged uninstaller and verify binaries and registration are removed.
- [x] Verify `%LOCALAPPDATA%\LocalOps\POS` and its database remain byte-for-byte unchanged after uninstall.

Evidence from 2026-09-24:

- Installer: `LocalOps POS_0.1.0_x64-setup.exe`
- Size: `3,041,063` bytes
- SHA-256: `4B0909B0C94F379C0BF047C728FA752CCA8805135833324DC731E206AB97E4C4`
- Automated results: 138 core tests, 3 desktop-command tests, and 3 frontend tests passed.

The remaining items below are release-operator sign-off for a specific customer distribution. They do not represent incomplete application code.

## Offline operational loop

- [ ] Disconnect networking before launch.
- [ ] Complete first-run business setup and sign in again.
- [ ] Create a category, unit, stocked product, service, packaging, and recipe.
- [ ] Receive stock, transfer it, record waste, and complete a stock count.
- [ ] Open a shift, scan/search an item, complete cash and split-payment sales, and print a receipt through the Windows print dialog.
- [ ] Complete a partial refund and full void; verify payment and inventory reversals.
- [ ] Record and void an expense, then close the shift and verify variance.
- [ ] Verify dashboard totals and export the CSV report.

## Recovery and installation

- [ ] Create and verify a manual backup.
- [ ] Restore that backup and sign in again.
- [ ] Confirm database integrity, foreign keys, audit coverage, and inventory reconciliation are healthy.
- [ ] Install over the previous application version and verify the customer database is retained.
- [x] Uninstall the application and confirm the uninstaller does not silently delete customer data/backups.
- [ ] Install on a clean supported Windows 10/11 machine with WebView2 available.

## Distribution

- [ ] Apply an organization code-signing certificate to the installer.
- [ ] Record release version, Git commit, build machine, date, installer SHA-256, and test operator.
- [ ] Archive the installer, checksum, release notes, and a verified empty-business smoke-test backup.
