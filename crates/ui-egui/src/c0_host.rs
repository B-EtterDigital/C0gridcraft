//! C0sheet's guest contract over GridCraft's existing control and command paths.

mod c0_files;
mod c0_panels;
pub use c0_files::{begin_picker, picker_pending, poll};

use gridcraft_engine::{core::RangeRef, model::Visibility};
use serde_json::{Value, json};

use crate::{SheetApp, control::Outcome};

/// Transport limits, shared by the page picker and the native contract.
pub const MAX_FILE_BYTES: usize = 72 * 1024 * 1024;
pub const MAX_REQUEST_BYTES: usize = 100 * 1024 * 1024;
pub const METHODS: &[&str] = &["c0.inspect", "c0.goTo", "c0.documentBytes", "c0.openFile", "c0.newDocument", "c0.cancelFileDialog", "c0.set"];

pub fn handle(app: &mut SheetApp, ctx: &egui::Context, method: &str, params: &Value) -> Outcome {
    let result = dispatch(app, ctx, method, params);
    if result.is_ok() && !matches!(method, "c0.inspect" | "c0.documentBytes") {
        ctx.request_repaint();
    }
    Outcome::Done(match result {
        Ok(result) => json!({"ok": true, "result": result}),
        Err(error) => json!({"ok": false, "error": error}),
    })
}

fn dispatch(app: &mut SheetApp, ctx: &egui::Context, method: &str, params: &Value) -> Result<Value, String> {
    if !params.is_object() && !params.is_null() {
        return Err("params must be an object".into());
    }
    match method {
        "c0.inspect" => inspect(app),
        "c0.goTo" => go_to(app, params),
        "c0.documentBytes" => c0_files::document_bytes(app, params),
        "c0.openFile" => c0_files::open(app, ctx, params),
        "c0.newDocument" => c0_files::new_document(app, ctx, params),
        "c0.cancelFileDialog" => Ok(json!({"cancelled": c0_files::cancel_picker(ctx)})),
        "c0.set" => c0_panels::set(app, ctx, params),
        _ => Err(format!("unknown host method {method:?}")),
    }
}

fn inspect(app: &SheetApp) -> Result<Value, String> {
    let d = app.session.active().ok_or("no workbook open")?;
    let wb = &d.wb;
    let sh = wb.active().ok_or("no active sheet")?;
    Ok(json!({
        "title": d.display_title(), "dirty": d.is_dirty(), "revision": d.revision,
        "sheets": wb.sheets.iter().enumerate().map(|(i, sh)| json!({
            "name": sh.name, "active": i == wb.active_sheet, "hidden": sh.visibility != Visibility::Visible,
        })).collect::<Vec<_>>(),
        "activeSheet": wb.active_sheet, "activeCell": d.selection.active.a1(), "selection": d.selection.a1(),
        "names": wb.names.iter().map(|n| json!({"name": n.name, "ref": format!("={}", n.formula)})).collect::<Vec<_>>(),
        "tables": wb.sheets.iter().flat_map(|sh| sh.tables.iter().map(|t| json!({
            "name": t.name, "sheet": sh.name, "ref": t.range.a1(),
        }))).collect::<Vec<_>>(),
        "canUndo": !d.undo.is_empty(), "canRedo": !d.redo.is_empty(),
        "panes": {"formulaBar": app.session.prefs.formula_bar, "headings": sh.show_headings,
                  "gridlines": sh.show_gridlines, "statusBar": app.ui.status_bar},
        "ribbonCollapsed": app.ui.ribbon_collapsed, "theme": app.ui.theme_mode(),
    }))
}

fn go_to(app: &mut SheetApp, params: &Value) -> Result<Value, String> {
    let d = app.session.active().ok_or("no workbook open")?;
    let sheet = match params.get("sheet") {
        None => d.wb.active_sheet,
        Some(Value::String(name)) => d.wb.sheet_index(name).ok_or("no sheet with that name")?,
        Some(value) => value.as_u64().and_then(|i| usize::try_from(i).ok()).ok_or("sheet must be a name or zero-based index")?,
    };
    let sh = d.wb.sheet(sheet).ok_or("no such sheet")?;
    if sh.visibility != Visibility::Visible {
        return Err("sheet is hidden; unhide it before navigating".into());
    }
    let range = match params.get("ref") {
        None => None,
        Some(Value::String(text)) => Some(RangeRef::parse(text).ok_or("ref must be an A1-style range")?),
        Some(_) => return Err("ref must be an A1-style range".into()),
    };
    commit_edit(app)?;
    let result = match range {
        Some(range) => app.run("selection.set", json!({"sheet": sheet, "range": range.a1()}))?,
        None => app.run("sheet.activate", json!({"sheet": sheet}))?,
    };
    app.grid.ensure_visible = true;
    app.grid.reveal_moving_end = false;
    app.backstage = None;
    Ok(result)
}

/// A host gets validation errors in its reply; a failed draft must not open a message box.
fn commit_edit(app: &mut SheetApp) -> Result<(), String> {
    app.commit_text_box_edit()?;
    let previous = app.message.take();
    let committed = app.commit_edit(0, 0, false, false);
    let feedback = app.message.take();
    app.message = previous;
    if committed { Ok(()) } else { Err(feedback.map(|(_, text)| text).unwrap_or_else(|| "the current edit could not be committed".into())) }
}

/// Refuse programmatic Agent Control entry, including engine.execute and menu invocation.
pub(crate) fn check_command(app: &SheetApp, id: &str, params: &Value) -> Result<(), String> {
    if !app.chrome.agent_control && id == "ui.dialog" && params.get("name").and_then(Value::as_str) == Some("agents") {
        Err("Agent Control is hidden by the host".into())
    } else {
        Ok(())
    }
}

/// Keep the default About text intact; a hosted About still carries authorship and licence.
pub(crate) fn about(ui: &mut egui::Ui, community_links: bool) {
    if community_links {
        ui.label("A clean-room, open-source, Rust-native spreadsheet. Part of ArtCraft.");
        ui.hyperlink_to("getartcraft.com/apps/gridcraft", "https://getartcraft.com/apps/gridcraft");
        ui.hyperlink_to("Join the ArtCraft Discord", "https://discord.gg/artcraft");
    } else {
        ui.label("A clean-room, open-source, Rust-native spreadsheet.");
        ui.label("By the GridCraft contributors (see Contributors).");
        ui.label("Licensed under MIT OR Apache-2.0.");
    }
}
