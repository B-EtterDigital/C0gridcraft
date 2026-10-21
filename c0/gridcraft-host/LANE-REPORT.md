C0sheet GridCraft guest-page lane report
Date: 2026-10-21
Branch: c0/main
Upstream main: 68e58a2eae462ba997a8ba5a1269e03a79b61136 (unchanged)
No pushes performed.

Commits:
dba44fa916be2198b87a0e13d986610b7276bc91 C0sheet: add GridCraft guest page and host control
The report-only follow-up commit is identified by SHA in the stdout hand-back.

Delivered:
Opt-in page-control bridge and C0sheet Trunk page; all seven c0 methods;
unpainted WebRunner passes; bounded cancellable picker; default-true Chrome
switches; runtime colour overrides; read-only XLSX/CSV snapshots; normal
Save Changes replacement lifecycle; programmatic validation/import feedback
without new message boxes; working wasm wall clock.

Upstream files edited, added lines (including replacement/indentation lines):
Cargo.lock: 1
apps/gridcraft-web/Cargo.toml: 4
apps/gridcraft-web/src/main.rs: 2
apps/gridcraft-web/src/web.rs: 6
crates/ui-egui/Cargo.toml: 1
crates/ui-egui/src/control.rs: 2
crates/ui-egui/src/dialogs.rs: 4
crates/ui-egui/src/lib.rs: 7
crates/ui-egui/src/ribbon.rs: 6
crates/ui-egui/src/theme.rs: 3
crates/ui-egui/tests/host_chrome.rs: 2
Total: 38 added lines in 11 modified upstream files.
Ratchet: 38
Measured after the implementation commit using:
git diff --numstat --diff-filter=M main c0/main -- . ':!c0'
PATCH-DEBT.md matches every file/count. The wasm clock fix is marked
Offered upstream: yes, as is.

Test results:
c0_host_contract: 15 passed, 0 failed (including generated XLS warnings and invalid drafts).
c0_chrome: 6 passed, 0 failed.
c0_control_passes: 5 passed, 0 failed.
c0_theme: 5 passed, 0 failed.
New native host tests total: 31 passed, 0 failed.
Full UI suite: 729 passed, 0 failed, 0 ignored (698 existing plus 31 new).
Full workspace: 1689 passed, 0 failed, 0 ignored, across 103 harness results;
this total includes the UI and host tests above.
cargo test -p gridcraft-web --features page-control: 5 passed, 0 failed (K-41).
cargo check --target wasm32-unknown-unknown -p gridcraft-web --features page-control: passed.
cargo clippy -p gridcraft-ui-egui -p gridcraft-web --all-targets --features page-control -- -D warnings: passed, no warnings.
cargo xtask ci: all 8 steps passed (fmt, clippy, tests, assets, layers, wasm, parity, keytips).
Wasm check/build retains one warning from untouched backstage.rs: unused Modified variants.

Chromium smoke: passed on the final built page. All seven host methods were
exercised, including CSV Open, navigation, XLSX/CSV snapshots, dirty replacement
cancellation, settings reports, hidden Agent Control, and malformed requests.
Animation frames and timers were paused for control calls. Invalid drafts were
retained with errors returned in replies and no message box. No JavaScript
exceptions were reported. The rendered About/chrome screenshot was inspected:
/tmp/c0sheet-browser.png

Final page build: passed from c0/gridcraft-host with
smoac-slot run heavy 2 3600 -- env CARGO_BUILD_JOBS=2 BINARYEN_CORES=2 NO_COLOR=true trunk build --release
Wasm: dist/c0host/gridcraft-web-a602e12f8dd40d43_bg.wasm
Wasm size: 18999522 bytes.
Total dist/c0host size: 19163539 bytes in 3 files (also du -sb).
Generated JS: 162637 bytes; index.html: 1380 bytes.

Host load from cat /proc/loadavg before the final slot-wrapped build:
16.96 18.05 18.55 10/1765 333235
Host load after the final build:
16.69 17.53 18.22 26/1725 443385
All Cargo runs used CARGO_BUILD_JOBS=2. One debug/release overlap briefly
exceeded two compiler jobs; I held CI until release Cargo finished and then
serialized subsequent Cargo runs. No fleet redispatch, claims, leases or SSB logging.

Assumptions I could not confirm:
The outer C0VIBE C0sheet mount was not supplied; the actual guest page and alias
were tested in Chromium, rather than inside that outer shell.
Shared craftrules and ignored plan files were absent locally; the referenced
craftrules AGENTS.md URL returned 404. Repository AGENTS.md and committed
architecture/control documentation governed the implementation.
No external template identifiers were supplied: newDocument accepts blank,
empty or omitted template. No vendor-authored test corpus was supplied.
The scout's 39-colour count did not match source: Tokens has 36 Color32 fields;
all 36 are supported and tested. The non-colour dark selector is not a token.

LANE-DONE
