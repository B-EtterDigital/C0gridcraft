# Patch debt: lines of ours inside upstream files

Substantive host implementation and tests live in new files. This table includes **every**
modified upstream file: replacement lines, indentation, tests and dependency bookkeeping count.
`Lines` is the added column of:

`git diff --numstat --diff-filter=M main c0/main -- . ':!c0'`

| File | Lines | What | Why | Offered upstream |
|---|---:|---|---|---|
| `Cargo.lock` | 1 | UI crate's js-sys dependency edge | Browser wall clock for the wasm bug fix | Yes, with clock fix |
| `apps/gridcraft-web/Cargo.toml` | 4 | Opt-in feature with rfd/file-handle-inner (3 lines) and browser Event/EventTarget/File features (1 replacement) | Compile native feature tests and the browser pass/picker transport | Not yet (embedding seam) |
| `apps/gridcraft-web/src/main.rs` | 2 | Feature-gated page-control module | Ordinary browser builds expose no page function | Not yet (embedding seam) |
| `apps/gridcraft-web/src/web.rs` | 6 | Retain/lend WebRunner (4), attach app transport (2) | Run the same app and real control queue without presentation | Not yet (embedding seam) |
| `crates/ui-egui/Cargo.toml` | 1 | wasm-only js-sys | Browser wall clock for the wasm bug fix | Yes, with clock fix |
| `crates/ui-egui/src/control.rs` | 2 | Single c0-family arm (1), route ui.dialog through run (1 replacement) | Host contract plus uniform programmatic Agent Control refusal | Not yet (embedding seam) |
| `crates/ui-egui/src/dialogs.rs` | 4 | Guard native Agent Control (1), pass community switch (2 replacements), About helper (1 replacement) | Hide promotional content and agent entry while retaining the existing dialog structure | Not yet (host chrome) |
| `crates/ui-egui/src/lib.rs` | 6 | Register module (1), default-true Chrome fields/default (3), command guard (1), replacement poll (1) | Describe the host without persisting chrome; retain the upstream unsaved-changes path | Not yet (embedding seam) |
| `crates/ui-egui/src/lib.rs` | 1 | Replace wasm now_ms zero with js_sys::Date::now | AutoSave, toast expiry and screenshot deadlines need a working browser clock | yes, as is |
| `crates/ui-egui/src/ribbon.rs` | 6 | Agent Control guard and indented original render site | Hidden switch suppresses button and its KeyTip | Not yet (host chrome) |
| `crates/ui-egui/src/theme.rs` | 3 | Register host palette, resolve at Tokens::get, resolve widget palette during apply | Overrides reach all token reads and survive upstream appearance reapplication | Not yet (generic palette seam) |
| `crates/ui-egui/tests/host_chrome.rs` | 2 | Default-fill two existing Chrome literals | Retain original assertions after extending the host description | With Chrome extension |

Total: 38 added lines in 11 modified upstream files, including replacements.
Ratchet: 38

## New files: no upstream patch debt

- `c0/gridcraft-host/*`: page, build config, contract, debt and lane evidence.
- `apps/gridcraft-web/src/page_control.rs`, `page_control/{bridge,files}.rs`: transport and bounded chooser.
- `crates/ui-egui/src/c0_host.rs`, `c0_host/{c0_files,c0_panels}.rs`: host contract and lifecycle.
- `crates/ui-egui/src/theme/host.rs`: context-local runtime palette.
- `crates/ui-egui/tests/c0_*.rs`: native host, rendering, palette and unpainted-pass coverage.

## Couplings to re-check on every upstream pull

| Our code | Upstream coupling | Evidence |
|---|---|---|
| `page_control/bridge.rs` | WebRunner event listeners lend AppRunner; logic runs an unpainted pass; runner must be retained before startup | wasm feature check, page build and native queue tests |
| `page_control.rs` | CommandSpec query semantics are `!journal && !undoable`; synthetic input acknowledges queueing | Native feature tests and input/queue tests |
| `c0_host` | DocState public revision, workbook sheets/names/tables and selection state | Inspect contract and navigation tests |
| `c0_host/c0_files.rs` | GridCraft Open/New add documents; file.close alone owns Save Changes; removal of the old document uid signals accepted replacement | Rendered Cancel/Don't Save/Save lifecycle tests |
| `c0_host` and `c0_host/c0_files.rs` | Draft validation and import warnings can set app.message; host calls return validation errors and retain import warnings in results/notices | Invalid-draft retention and generated legacy XLS warning tests |
| `c0_host/c0_files.rs` | io::save_bytes and file.saveBytes use the same encoder; snapshots intentionally leave drafts and UI scroll state alone | XLSX/CSV round trips and immutable-state assertions |
| `page_control/files.rs` | rfd FileHandle.inner exposes Blob size and array_buffer; Services.inbox owns importer, warning and password handling | wasm check/build and atomic cancellation test |
| Chrome switches | About is the sole ArtCraft/getartcraft/Discord promotional render site; credits are separate; native menus call open_dialog | Source grep plus rendered About/Automate tests and command refusal tests |
| `theme/host.rs` | Every scalar Color32 token must be registered; theme::apply derives both widget palettes | Complete field registry test, clear/default/system tests and light/dark persistence |

The existing `?host=parent` bridge and upstream `dist/web` configuration are unchanged. Browser
Save's asynchronous lifecycle is documented in README; the host never force-discards dirty work
because an async download was merely started. No spreadsheet parity score or roadmap claim changes.
