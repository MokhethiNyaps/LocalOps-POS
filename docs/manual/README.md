# Illustrated user manual

| File | What it is |
|---|---|
| `LocalOps-POS-User-Manual.pdf` | **The deliverable.** 45-page illustrated manual for tellers and trainers, A4, ready to download or print. |
| `user-manual.html` | Source of the PDF. Plain HTML + print CSS, no build step. |
| `images/*.png` | Real screenshots of every screen and state, captured from the running app. |

This manual is written in simple English and covers the interface as it exists today.
`../USER-MANUAL.md` remains the longer reference-style document.

## Screenshots

Every picture is a genuine screenshot of the React UI — no mock-ups. They were captured from the
dev-only fixture `tests/ui-preview.html`, which serves the real `App` component with sample data
(Moko's Lifestyle Centre, ZAR, Kitchen/Bar/Salon).

| Image | Screen / state |
|---|---|
| `01-sign-in` | Login screen |
| `02-first-run-setup` | First-run "Build workspace" (`?setup`) |
| `03-open-shift` | Order entry with no open shift (`?closed`) |
| `04-pos-sale-screen` | Order entry, empty order |
| `05-pos-cart` | Order entry with four items |
| `06-pos-search` | Search / barcode filter |
| `07-pos-payment`, `07b-payment-panel` | Payment allocation, balanced |
| `08-receipt`, `08b-receipt-panel` | Completed sale and latest receipt |
| `09-pos-history` | History tab |
| `10-refund` | Reversal allocation (refund / void) |
| `11-catalogue` | Products & services |
| `12-setup-tools` | Catalogue setup |
| `13-inventory` | Stock control |
| `14-shift-expenses` | Shifts & expenses |
| `15-reports` | Business dashboard |
| `16-backup-safety` | Backup & recovery |
| `17-pos-no-items` | Empty catalogue state (`?empty`) |
| `18-sale-error` | Failed sale message (`?fail`) |
| `19-sidebar`, `20-header` | Navigation rail and top bar close-ups |

## Regenerating

1. `npm run dev` (or `npx vite --host 0.0.0.0 --port 5173`).
2. Re-take screenshots by driving `http://localhost:5173/tests/ui-preview.html` with a headless
   Chromium (viewport 1440×900–1180, `deviceScaleFactor: 1.5`, hide the fixture's `<details>` panel).
3. Print the manual:
   `page.goto('http://localhost:5173/docs/manual/user-manual.html')` then `page.pdf({ format: 'A4',
   printBackground: true, margin: { top: '16mm', bottom: '18mm', left: '14mm', right: '14mm' } })`.

The HTML pulls its fonts from `node_modules/@fontsource-variable/*`, so it must be printed through
the dev server rather than opened from the file system.
