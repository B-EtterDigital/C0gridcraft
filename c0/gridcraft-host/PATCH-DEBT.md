# Patch debt: lines of ours inside upstream files

Substantive host implementation and tests live in new files. This table includes **every**
modified upstream file: replacement lines, indentation, tests and dependency bookkeeping count.
`Lines` is the added column of:

`git diff --numstat --diff-filter=M <upstream> c0/main -- . ':!c0'`

Recounted on the upstream pull of 2026-10-28 against upstream `11bb673` (merge `b11be05`, fixes up to `7716a83`).
Before that pull the table listed 15 files and 47 lines; the upstream directory split moved several hooks into
new files (`app.rs`, `app/commands.rs`, `services.rs`, `dialogs/about.rs`, `ribbon/automate.rs`, `grid/show.rs`),
and the wasm clock lines (`Cargo.lock` and `crates/ui-egui/Cargo.toml`, one `js-sys` edge each) were removed because
upstream's own `clock.rs` now provides the monotonic clock. Net: 47 lines again, no growth.

| File | Lines | What | Why | Offered upstream |
|---|---:|---|---|---|
| `apps/gridcraft-web/Cargo.toml` | 4 | Opt-in feature with rfd/file-handle-inner (3 lines) and browser Event/EventTarget/File features (1 replacement) | Compile native feature tests and the browser pass/picker transport | Not yet (embedding seam) |
| `apps/gridcraft-web/src/main.rs` | 2 | Feature-gated page-control module | Ordinary browser builds expose no page function | Not yet (embedding seam) |
| `apps/gridcraft-web/src/web.rs` | 7 | Retain/lend WebRunner (4), attach app transport (2), `..Default::default()` on the `Chrome` literal (1, forced by the new Chrome fields) | Run the same app and real control queue without presentation | Not yet (embedding seam) |
| `crates/ui-egui/src/app.rs` | 2 | c0 poll after `control::poll` (1), ribbon call site replaced by `c0_host::ribbon` (1 replacement) | Host poll and toolbar layout run in the same frame order as before | Not yet (embedding seam) |
| `crates/ui-egui/src/app/commands.rs` | 1 | `c0_host::check_command` guard at the top of `run_command` | Uniform programmatic Agent Control refusal | Not yet (embedding seam) |
| `crates/ui-egui/src/control.rs` | 2 | Single c0-family arm, which also routes ui.render (1), route ui.dialog through run (1 replacement) | Host contract, frameless render and uniform programmatic Agent Control refusal | Not yet (embedding seam) |
| `crates/ui-egui/src/dialogs.rs` | 1 | `"agents" if app.chrome.agent_control` guard | Hide the agent entry for native menus too | Not yet (host chrome) |
| `crates/ui-egui/src/dialogs/about.rs` | 2 | `community_links` parameter (1), About body replaced by `c0_host::about` (1 replacement; the three promotional lines are removed) | Hide promotional content while keeping the existing dialog structure | Not yet (host chrome) |
| `crates/ui-egui/src/dialogs/show.rs` | 1 | Passes `app.chrome.community_links` to `about` | Same switch as above | Not yet (host chrome) |
| `crates/ui-egui/src/grid.rs` | 1 | Re-exports `paint_headers` and `paint_panes` (`pub(crate) use show::{…}`) | `c0_render` draws with the app's own painter; `show` is a private module | Yes, as is |
| `crates/ui-egui/src/grid/show.rs` | 1 | Declares `c0_panes`, which keeps `paint_panes`' old argument list for the host (upstream bundled them into a private `PanesInput` on 20261030) | Frameless ui.render draws the visible grid with the app's own painter | Yes, as is |
| `crates/ui-egui/src/lib.rs` | 1 | Register module `pub mod c0_host` | Host contract module | Not yet (embedding seam) |
| `crates/ui-egui/src/panes.rs` | 2 | Folded-dock early return (1), fold button in the pane header (1) | right_dock_collapsed | Not yet (host chrome) |
| `crates/ui-egui/src/pivot_pane.rs` | 1 | Folded dock hides the PivotTable pane (1 replacement) | right_dock_collapsed | Not yet (host chrome) |
| `crates/ui-egui/src/ribbon.rs` | 1 | Toolbar-off collapses to the tab row (1 replacement) | Hidden switch suppresses the card | Not yet (host chrome) |
| `crates/ui-egui/src/ribbon/automate.rs` | 6 | Agent Control button wrapped in `if app.chrome.agent_control` (2 lines and 4 re-indented lines) | Hidden switch suppresses button and its KeyTip | Not yet (host chrome) |
| `crates/ui-egui/src/services.rs` | 4 | `Chrome` fields `community_links`, `agent_control`, `layout` (3) and the default (1 replacement) | Describe the host without persisting chrome; defaults draw upstream unchanged | Not yet (embedding seam) |
| `crates/ui-egui/src/theme.rs` | 3 | Register host palette, resolve at Tokens::get, resolve widget palette during apply | Overrides reach all token reads and survive upstream appearance reapplication | Not yet (generic palette seam) |
| `crates/ui-egui/src/widgets.rs` | 3 | Compact-ribbon hook at the top of `big_button` | toolbar_double turns big buttons into icon buttons | Not yet (host chrome) |
| `crates/ui-egui/tests/it/host_chrome.rs` | 2 | Default-fill two existing Chrome literals (`..Chrome::default()`) | Retain original assertions after extending the host description | With Chrome extension |

Removed in the 2026-10-28 pull: `Cargo.lock` (1) and `crates/ui-egui/Cargo.toml` (1), the UI crate's `js-sys`
edge and dependency. Upstream's `clock.rs` now provides the wasm monotonic clock through `web-sys` `Performance`.
Proof: no `js_sys` use remains in `crates/ui-egui`; `cargo check --target wasm32-unknown-unknown` passes for
`gridcraft-web` with and without `page-control`.

Total: 47 added lines in 20 modified upstream files, including replacements.
Ratchet: 47 (unchanged across the 2026-10-28 pull; the +1 in `web.rs` is forced by the new `Chrome` fields and is
offset by the −2 from the removed `js-sys` edge).

## New files: no upstream patch debt

- `c0/gridcraft-host/*`: page, build config, contract, debt and lane evidence.
- `apps/gridcraft-web/src/page_control.rs`, `page_control/{bridge,files}.rs`: transport and bounded chooser.
- `crates/ui-egui/src/c0_host.rs`, `c0_host/{c0_ai,c0_files,c0_panels,c0_layout,c0_raster,c0_render}.rs` (c0_ai: `c0.selection`, `c0.place`; no upstream lines added): host contract and lifecycle.
- `crates/ui-egui/src/theme/host.rs`: context-local runtime palette.
- `crates/ui-egui/tests/c0_*.rs`: native host, rendering, palette and unpainted-pass coverage.

## Couplings to re-check on every upstream pull

| Our code | Upstream coupling | Evidence |
|---|---|---|
| `page_control/bridge.rs` | WebRunner event listeners lend AppRunner; logic runs an unpainted pass; runner must be retained before startup | wasm feature check, page build and native queue tests |
| `page_control.rs` | CommandSpec query semantics are `!journal && !undoable`; synthetic input acknowledges queueing | Native feature tests and input/queue tests |
| `c0_host` | DocState public revision, workbook sheets/names/tables and selection state; `Workbook::active_sheet` is an `ActiveSheet` (read with `.get()`) | Inspect contract and navigation tests |
| `c0_host/c0_files.rs` | GridCraft Open/New add documents; file.close alone owns Save Changes; removal of the old document uid signals accepted replacement | Rendered Cancel/Don't Save/Save lifecycle tests |
| `c0_host` and `c0_host/c0_files.rs` | Draft validation and import warnings can set app.message; host calls return validation errors and retain import warnings in results/notices | Invalid-draft retention and generated legacy XLS warning tests |
| `c0_host/c0_files.rs` | io::save_bytes and file.saveBytes use the same encoder; snapshots intentionally leave drafts and UI scroll state alone | XLSX/CSV round trips and immutable-state assertions |
| `page_control/files.rs` | rfd FileHandle.inner exposes Blob size and array_buffer; Services.inbox owns importer, warning and password handling | wasm check/build and atomic cancellation test |
| `c0_host/c0_layout.rs` | Upstream's ribbon draws big buttons through `widgets::big_button` and its panel as `Panel::top("ribbon")` at 34 + 92 pt; panes live in `panes::show`; the grid area is `app.grid.rect` | Layout tests: card height, compact width, floating bounds, fold/unfold |
| `c0_host/c0_render.rs`, `c0_raster.rs` | `grid::paint_panes`, `paint_headers`, `chartview::paint_objects` and `outline::paint` signatures; egui tessellation and texture deltas | Render tests: pixel assertions on cells, headings and gridlines at two scales |
| Chrome switches | About is the sole in-app ArtCraft/getartcraft/Discord promotional render site. Upstream also added two promotional items to the desktop Help menu (`apps/gridcraft/src/native_menu.rs`); the web host does not build that menu, so they are not hidden by the switch | Source grep plus rendered About/Automate tests and command refusal tests |
| `theme/host.rs` | Every scalar Color32 token of `Tokens` must be in the `colors!` registry (upstream added eight on 2026-10-28); theme::apply derives both widget palettes | Complete field registry test (`c0_theme`), clear/default/system tests and light/dark persistence |

The existing `?host=parent` bridge and upstream `dist/web` configuration are unchanged. Browser
Save's asynchronous lifecycle is documented in README; the host never force-discards dirty work
because an async download was merely started. No spreadsheet parity score or roadmap claim changes.
