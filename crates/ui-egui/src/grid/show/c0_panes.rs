//! `paint_panes` with its old argument list, for `c0_host::c0_render`. Upstream bundled the arguments into
//! `PanesInput`, which is private to `show`; a child module can still build it, so the host keeps one call.
use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_panes(
    app: &SheetApp,
    painter: &Painter,
    geo: &Geo,
    wb: &Workbook,
    si: usize,
    sh: &Sheet,
    sel: &gridcraft_engine::Selection,
    outline: Rect,
    cf: &mut CfCache,
    t: &Tokens,
) -> Vec<PaneQuad> {
    let input = PanesInput { app, painter, geo, workbook: wb, sheet_index: si, sheet: sh, selection: sel, outline, tokens: t };
    super::paint_panes(input, cf)
}
