# GridCraft upstream pull 2026-10-28: upstream `11bb673` into `c0/main`

Date: 2026-10-28. Branch `fix` (from `c0/main` `9b870f5`). Upstream: `11bb67386c9bb19b0717e21e98adc608162ae2e5`
(230 commits since the last sync `3099df9`). Merge commit: `b11be05` (merge, not rebase; `9b870f5` is an ancestor).
No pushes performed. Git identity: C0 upstream sync.

## Commits on `fix`

- `b11be05` merge: upstream 11bb67386c9b into c0/main
- `eef273a` fix(c0): follow upstream's ActiveSheet, egui Id and Role APIs
- `7716a83` fix(c0): register upstream's new colour tokens and scope test queries to buttons
- style commit: rustfmt on our five files and PATCH-DEBT recount (SHA in the hand-back)
- this report (SHA in the hand-back)

No `chore(c0): keep upstream trademarks and CI out of c0/main` commit was needed: the only tracked file under
`docs/brand/` is `LICENSE-brand.txt` (excepted by the rule), and no `.github/workflows/` files remain after the merge.

## Conflicts and how each was resolved

Five modify/delete conflicts, one per upstream-modified workflow. Kept deleted (fork policy: no upstream CI):
`.github/workflows/{ci,freebsd,packaging-lint,release,windows-arm64}.yml`. No c0 hook lived in them.

Four content conflicts. Upstream split these files into directories, so each was taken as upstream's version and
our hooks re-applied in the new location (moved out of the upstream file where a new file could not take it):

| conflicted file | upstream now | our hook | resolution |
|---|---|---|---|
| `crates/ui-egui/src/lib.rs` | module list | `pub mod c0_host` | re-applied (1 line) |
| `crates/ui-egui/src/lib.rs` | `Chrome` struct | 3 fields and default | **moved** to `services.rs` (the struct moved there) |
| `crates/ui-egui/src/lib.rs` | `run_command` | host command guard | **moved** to `app/commands.rs` |
| `crates/ui-egui/src/lib.rs` | `control::poll` and ribbon call | c0 poll, host ribbon | **moved** to `app.rs` |
| `crates/ui-egui/src/ribbon.rs` | `show` | toolbar-off collapse | re-applied (1 line) |
| `crates/ui-egui/src/ribbon.rs` | `automate` (now `ribbon/automate.rs`) | Agent Control guard | **moved** to `ribbon/automate.rs` (6 lines, incl. re-indent) |
| `crates/ui-egui/src/dialogs.rs` | agents dialog | `"agents" if app.chrome.agent_control` | re-applied (1 line) |
| `crates/ui-egui/src/dialogs.rs` | `about` (now `dialogs/about.rs`) | community switch and host About text | **moved** to `dialogs/about.rs`, call site in `dialogs/show.rs` |
| `crates/ui-egui/src/grid.rs` | `paint_panes` (now `grid/show.rs`) | `pub(crate)` | **moved**: visibility in `grid/show.rs`, re-export in `grid.rs` (`paint_headers` too, for `c0_render`) |

Auto-merged, and each hunk was read in its final position: `control.rs` (`c0.`/`ui.render` arm at the top of
`handle`; `ui.dialog` routed through `app.run`), `panes.rs` (`dock_open` at the top of `show`, fold button in the
header row), `pivot_pane.rs`, `widgets.rs` (`big_button` hook), `theme.rs` (`pub mod host`, `Tokens::get`, `apply`),
`apps/gridcraft-web/{Cargo.toml,src/main.rs,src/web.rs}`, `crates/ui-egui/tests/it/host_chrome.rs`. `Cargo.lock`
auto-merged without a conflict; cargo then removed our one `js-sys` edge, so the lock now equals upstream's.

Re-applied hooks needed follow-up edits for upstream API changes (in our code only):
- `Workbook::active_sheet` is now an `ActiveSheet` (read with `.get()`).
- `egui::Id::new` deprecated in the pinned egui fork: our c0 code uses `Id::unique`. The fork defines `Id::new` as
  `Self::unique(source)`, so the hash and the stored data keys are unchanged.
- `egui::WidgetType` renamed to `egui::Role`.
- The `Tokens` colour fields gained eight tokens (`warning`, `on_accent`, `on_warning`, `paper`,
  `sheet_control_bg`, `sheet_control_border`, `sheet_control_text`, `sheet_track`); registered in `theme/host.rs`.
  The `c0_theme` test enforces this.
- Two c0 tests queried a label that now also matches a container node; they now query `Role::Button`, as upstream's
  tests do.

## PATCH-DEBT before and after

- Before (c0/main vs upstream `3099df9`): **47** added lines in **15** files. The table listed `tests/host_chrome.rs`,
  but the real path is `tests/it/host_chrome.rs`.
- After (`fix` vs upstream `11bb673`): **47** added lines in **20** files. Ratchet not grown.
  - The +1 in `apps/gridcraft-web/src/web.rs` (`..Default::default()` on the `Chrome` literal) is forced by the new
    `Chrome` fields.
  - The −2 (`Cargo.lock` and `crates/ui-egui/Cargo.toml`, one `js-sys` edge each) is the removed obsolete clock patch.
    Upstream's `clock.rs` provides the monotonic clock through `web-sys` `Performance`. `js_sys` has no user left in
    `crates/ui-egui`, and both wasm checks pass without it. Patch-debt rows: removed, with the reason in
    `PATCH-DEBT.md`.
  - Grid visibility lines (`grid.rs` re-export +1, `grid/show.rs` +1) are forced by the upstream module split.
- No patch of ours was dropped; none was proven unnecessary except the `js-sys` edge above.

## Tests (host load before and after recorded in the proof logs)

Final tree (`eef273a`, `7716a83`, and the style commit), `cargo test -p gridcraft-ui-egui --no-fail-fast`, debug:

| target | passed | failed |
|---|---:|---:|
| lib (unit) | 232 | 0 |
| c0_chrome | 6 | 0 |
| c0_control_passes | 5 | 0 |
| c0_export | 5 | 0 |
| c0_host_contract | 20 | 0 |
| c0_layout | 12 | 0 |
| c0_render | 6 | 0 |
| c0_theme | 5 | 0 |
| doc_windows | 11 | 0 |
| header_autofit_fonts | 2 | 0 |
| it (main suite) | 850 | 2 (upstream-red, below) |
| keytips | 31 | 0 |

`cargo test -p gridcraft-web --features page-control`: 9 passed, 0 failed (final HEAD).

**Upstream-red** (identical failure on a worktree of pure `11bb673`, same command): `interaction::column_autocomplete_folds_case_beyond_ascii`
and `interaction::language_switch_renders_the_ribbon_and_persists`. Both panic in egui's font layer: no glyph for
`ẞ` (U+1E9E) or `ẻ` (U+1EBB) in the fonts installed on this host. Not ours; environment-dependent.

## Final verification (HEAD `e880bf1`, final log `sow-fix-gridcraft-logs/final.log`)

- `cargo test -p gridcraft-ui-egui --no-fail-fast`: every c0 target green; `it` 850 passed, 2 failed (the two
  upstream-red tests above, unchanged). Exit 101 because of those two only.
- `cargo test -p gridcraft-web --features page-control`: 9 passed, 0 failed.
- Earlier on this HEAD line: wasm checks (both feature sets) and the desktop `gridcraft` check passed on `e880bf1`'s
  parent code paths before the rustfmt-only commit; the formatting commit changes no code.

## Checks

- `cargo check --target wasm32-unknown-unknown -p gridcraft-web --features page-control`: pass.
- `cargo check --target wasm32-unknown-unknown -p gridcraft-web`: pass.
- `cargo check -p gridcraft` (desktop app, native): pass.
- `cargo clippy -p gridcraft-ui-egui -p gridcraft-web --all-targets --features page-control`: exit 0, **zero warnings**
  from our code (only the upstream `[patch]` notice, which is not ours).
- `cargo fmt --all -- --check`: failed on five of our files (the wrap of our own lines); rustfmt applied to exactly
  those five; formatting-only diff. Upstream files were already clean.
- Wasm release size: **not measured** (`trunk build` not run; the cap check is therefore not applicable here).

## New upstream commands and menus (for the Core Agent verbs)

From the `specs()` functions of `crates/engine/src/cmd/*` (dotted ids, base vs upstream):
- new: `data.outlineSettings`, `formulas.pasteName`, `pivot.options`, `view.statusBar`, `window.close`, `window.list`
- renamed or removed (to verify): `edit.fill` no longer appears; `edit.fillSeries` appears in upstream's diff.
  Please confirm against the `command_specs()` dump; the grep method may miss ids built from other literals.

New menu items and menu behaviour (not command ids): Status bar right-click statistics menu (Numerical Count, Minimum,
Maximum); Error Checking's Circular References submenu; F3 opens Paste Name; Switch Windows lists open workbooks;
Help menu entries "Check for Updates…" (`app.checkForUpdates`, `help.checkForUpdates`).

## What upstream changed, in plain words

- Structure (54 refactor commits, the bulk of the range): the large UI files (`dialogs`, `grid`, `ribbon`, `theme`,
  `control`, `lib`, the engine's `cmd` and the calc, xlsx and chart crates) were split into modules. This is why most
  hooks moved.
- Feature: open a saved workbook without recalculating it (`feat(engine)`).
- Fixes (12 `fix:` commits plus several plain-subject fixes): Subtotal and undo/redo/rollback batches (undo grouping;
  listing circular references of restored cells); editing a note or comment keeps its thread; sheetId kept across an
  XLSX rewrite; Shift+Arrow on a whole row or column keeps the view; the fill and iteration options; the cell draft
  rule ("a refused cell draft stops the command; an untouched F2 draft changes nothing"); keep the cell draft in its
  own workbook.
- Performance: "Speed up XLSX worksheet writing"; Go To lays out only the rows in view.
- Formulas: several function fixes (YEARFRAC, IMSQRT, ISOWEEKNUM/WEEKNUM, PERCENTRANK); a recursive LAMBDA stops at 140
  calls.
- Installer and desktop only: per-user install folder, install-scope policy, startup update prompt (native only; the
  web check is a no-op).
- Theme: new warning and on-accent colour tokens (registered in our palette, see above).

## Clashes with our UI

- **Promotional content, desktop only.** Upstream added two promotional items to the desktop Help menu
  (`apps/gridcraft/src/native_menu.rs:187-188`: getartcraft.com and Discord). Our switch hides the About text only;
  the web host does not build that menu, so there is no clash on the page. The desktop app is not hidden by the switch
  and was not changed in this pull. Decide whether the desktop Help menu needs the same switch.
- **About text.** Unchanged upstream. Our `community_links` switch still covers it (`dialogs/about.rs`).
- **Agent Control.** Hidden by `agent_control` at the dialog, button and command levels. Upstream's `run_command` is now
  in `app/commands.rs`; our guard sits at its top.
- **Theme.** Eight new colour tokens; all registered (`c0_theme` enforces it).
- **Home, title bar, panels.** Not checked visually. The layout tests (`c0_layout`) pass; no new screenshots were taken.

## Assumptions I could not confirm

- The command-id list is from the `specs()` text diff, not a dump of `command_specs()`. Confirm before creating verbs.
- `edit.fill` → `edit.fillSeries` is a rename I could not confirm.
- The two upstream-red tests are environment-dependent (font coverage). The same failure appears on pure upstream here,
  so it is not a regression of ours; it may pass on a host with those glyphs.
- Removing `js-sys` is proven at the compile level only (both wasm checks pass, no `js_sys` user remains). The
  wall-clock behaviour in the browser was not exercised.
- The page (`trunk build`) was not built, so the release wasm size and the browser smoke test were not run.
- The desktop `gridcraft` binary was only `cargo check`ed, not tested.
- `cargo xtask ci`, `cargo xtask layers` and `cargo xtask assets` were not run.
