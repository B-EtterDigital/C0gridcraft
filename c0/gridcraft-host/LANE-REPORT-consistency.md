# C0sheet consistency lane: tool modes, right dock, PDF bytes, frameless render

Date: 2026-10-22
Branch: c0/main (upstream `main` unchanged). No pushes performed.

## Tool-mode decision

GridCraft has a **ribbon**, not a tool strip, so PhotoCraft's three keys map onto the ribbon:

| key | ribbon |
|---|---|
| `toolbar` (default true) | the command card under the tabs; false collapses to the tab row (the "collapsed to tabs" mode) |
| `toolbar_double` | compact card: every big button becomes an icon button (name and tooltip kept), so the ribbon is narrower |
| `toolbar_floating` | the whole ribbon is a movable palette (an egui area with a "Move Toolbar" grip) inside the canvas area; the canvas takes the docked height back |

What fits: "single column / double column" has no meaning for a ribbon whose groups already stack up to
three rows, so `toolbar_double` is *density*, not column count; the card height cannot shrink without
clipping the stacked groups, so the gain is width (verified: the height is identical on all eight tabs).
GridCraft has no floating palette of its own, so the floating mode is a movable strip within the canvas
area, as the packet allows. `ribbon_collapsed` stays the user's own switch, independent of `toolbar`.
All keys are host description in `Chrome::layout`: not persisted, defaults draw upstream unchanged.

`right_dock_collapsed` folds the task panes (Comments, Watch Window, Selection, Format Chart Area, and
the PivotTable fields pane) to a 34 pt icon strip. Each icon opens its pane and unfolds the dock, the
chevron unfolds it as it was, and an open pane's header has a fold button. Immediate; only the canvas
width changes.

## Commits

- `d6326c5` feat(c0sheet): ribbon tool modes, foldable right dock, pdf bytes, frameless ui.render
- the follow-up docs commit carrying this report (SHA in the hand-back)

## Tests

All in `crates/ui-egui/tests/`, debug and release profile both green:

| suite | passed | failed |
|---|---:|---:|
| c0_layout (new: modes, combinations, defaults = upstream, unknown key, dock fold/unfold, inspect) | 12 | 0 |
| c0_export (new: documentBytes pdf, refusals, document left untouched) | 5 | 0 |
| c0_render (new: ui.render pixels, sheet/scale params, refusals, no state touched) | 6 | 0 |
| c0_host_contract | 15 | 0 |
| c0_chrome | 6 | 0 |
| c0_control_passes | 5 | 0 |
| c0_theme | 5 | 0 |
| **c0 total (debug, then release)** | **54** | **0** |

- Full `cargo test -p gridcraft-ui-egui`: 752 passed, 0 failed (729 before this lane plus 23 new).
- `cargo test -p gridcraft-web --features page-control`: 5 passed, 0 failed.
- `cargo clippy -p gridcraft-ui-egui -p gridcraft-web --all-targets --features page-control -- -D warnings`: clean.
- `cargo check --target wasm32-unknown-unknown -p gridcraft-web --features page-control`: passed.
- `cargo fmt --all -- --check`, `cargo xtask layers` (16 crates, no violations), `cargo xtask assets` (34 files attributed): passed.
- `cargo xtask ci` as a whole was not run this session; its fmt, clippy, tests, assets, layers and wasm steps were run individually as above (parity and keytips steps not re-run).
- One test of mine failed first time (`double_is_a_compact_ribbon...` looked up "Paste" on the Automate tab); fixed in the test, not in the code.

## Page build and headless smoke

`trunk build --release` from `c0/gridcraft-host`: passed (3m 49s).
Wasm: `dist/c0host/gridcraft-web-d9e2cbf029e6a30b_bg.wasm` = **19,178,649 bytes** (previous lane: 18,999,522, +179,127, +0.94%).
`dist/c0host` total 19,342,666 bytes in 3 files. The one build warning is the untouched `backstage.rs` unused variants.
Pin cap: I could not find a pin-cap value in the repo or the packet, so I cannot state the margin (see below).

Headless Chromium smoke (puppeteer-core over the built page, 16 checks, all passed, no page errors): defaults;
each of `toolbar_double`, `toolbar_floating`, `toolbar`, `right_dock_collapsed` set true/false and read back through
`c0.inspect`; unknown key reported; `documentBytes {format:"pdf"}` returns `%PDF-` bytes (654 bytes, 1 page) and
`docx` is refused; `ui.render` returns a valid PNG at scale 1 (1266x674) and 2 (2532x1348), scale 99 refused;
the floating compact ribbon screenshot and the frameless render were inspected by eye.

## Patch debt

Ratchet **48** (was 38; this lane added 10 lines in four more upstream files: `grid.rs` 1, `panes.rs` 2,
`pivot_pane.rs` 1, `widgets.rs` 3, plus `lib.rs` +2 (now 9 across its two rows) and `ribbon.rs` +1, with `control.rs` unchanged at 2). Measured with
`git diff --numstat --diff-filter=M main c0/main -- . ':!c0'`. Older debt untouched. New code is in
`c0_host/{c0_layout,c0_raster,c0_render}.rs` and `tests/c0_{layout,export,render}.rs`.

## Host load

`/proc/loadavg` (1/5/15 min, running/total):

| point | load |
|---|---|
| before the test/clippy/wasm/release gate | 20.75 21.04 16.49 |
| after it | 7.24 10.53 13.41 |
| before the page build | 8.48 9.19 11.84 |
| after the page build | 6.52 10.21 13.26 |

All cargo runs used `CARGO_BUILD_JOBS=2` under `smoac-slot`. The first test run waited about 55 minutes behind a
landing-mode hold on the gate pool; nothing was run outside the wrapper. Disk stayed 78 to 88% (peak read 87.5% exact).

## Assumptions I could not confirm

- The pin cap for the wasm size: not stated anywhere I could read, so only the absolute size and delta are recorded.
- `ui.render` uses the grid area as last laid out; a guest never shown uses 960 x 600 points. Right-to-left sheets
  are not mirrored and Page Layout view draws as the normal grid. Text is rasterised with nearest-neighbour texture
  reads, so it is close to, not identical with, the GPU output.
- `documentBytes` pdf passes `sheets`, `range` and `fitToPage` to the engine; only the default (active sheet) was smoke-tested in the browser.
- The outer C0VIBE mount was not available; the guest page was tested standalone in Chromium.
- No Excel observation was made for ribbon compaction; the compact mode is our own density choice.
