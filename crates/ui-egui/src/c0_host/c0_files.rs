//! Byte snapshots and replacement through GridCraft's normal Close / Save Changes path.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use gridcraft_engine::io;
use serde_json::{Value, json};

use super::MAX_FILE_BYTES;
use crate::SheetApp;

const MAX_FILE_NAME_BYTES: usize = 255;

#[derive(Clone)]
struct Replacement {
    uid: u64,
    command: &'static str,
    params: Arc<Value>,
}

fn replacement_id() -> egui::Id {
    egui::Id::new("gridcraft-host-replacement")
}

pub(super) fn document_bytes(app: &SheetApp, params: &Value) -> Result<Value, String> {
    let format = match params.get("format") {
        None => "xlsx",
        Some(Value::String(format)) if matches!(format.as_str(), "xlsx" | "csv") => format,
        _ => return Err("format must be xlsx or csv".into()),
    };
    let d = app.session.active().ok_or("no workbook open")?;
    // Use the same encoder as file.saveBytes, without store_view_state, edit commits or a save.
    let bytes = io::save_bytes(&d.wb, &format!("x.{format}")).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err("document exceeds the host file limit".into());
    }
    let title = d.display_title();
    let stem = std::path::Path::new(&title).file_stem().and_then(|s| s.to_str()).unwrap_or("Book");
    Ok(json!({"name": format!("{stem}.{format}"), "format": format, "bytes": io::base64_encode(&bytes), "revision": d.revision}))
}

pub(super) fn open(app: &mut SheetApp, ctx: &egui::Context, params: &Value) -> Result<Value, String> {
    let name = params.get("name").and_then(Value::as_str).filter(|s| !s.is_empty()).ok_or("name is required")?;
    if name.len() > MAX_FILE_NAME_BYTES || name.contains(['/', '\\', '\0']) || matches!(name, "." | "..") {
        return Err(format!("name must be a file basename of at most {MAX_FILE_NAME_BYTES} bytes"));
    }
    let text = params.get("bytes").and_then(Value::as_str).ok_or("bytes must be base64 text")?;
    if text.len() > MAX_FILE_BYTES.div_ceil(3).saturating_mul(4) {
        return Err("file exceeds the host file limit".into());
    }
    let bytes = io::base64_decode(text).ok_or("bytes must be valid base64")?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err("file exceeds the host file limit".into());
    }
    if io::base64_encode(&bytes) != text {
        return Err("bytes must be valid standard base64".into());
    }
    // All importers, including content sniffing, are owned by the engine. Never ask for a
    // password or discard the current document before discovering an invalid import.
    io::open_bytes(name, &bytes).map_err(|e| e.to_string())?;
    replace(app, ctx, "file.open", json!({"name": name, "base64": text}))
}

pub(super) fn new_document(app: &mut SheetApp, ctx: &egui::Context, params: &Value) -> Result<Value, String> {
    if params.get("template").is_some_and(|v| !matches!(v.as_str(), Some("" | "blank"))) {
        return Err("template must be blank or omitted".into());
    }
    replace(app, ctx, "file.new", json!({}))
}

fn replace(app: &mut SheetApp, ctx: &egui::Context, command: &'static str, params: Value) -> Result<Value, String> {
    if ctx.data(|d| d.get_temp::<Replacement>(replacement_id())).is_some() {
        return Err("a document replacement is awaiting Save Changes".into());
    }
    if app.dialog.is_some() {
        return Err("dismiss the current dialog before replacing the workbook".into());
    }
    super::commit_edit(app)?;
    cancel_picker(ctx);
    if let Some(uid) = app.session.active().map(|d| d.uid) {
        let closed = app.run("file.close", json!({}))?;
        if closed.get("closed").and_then(Value::as_bool) != Some(true) {
            ctx.data_mut(|d| d.insert_temp(replacement_id(), Replacement { uid, command, params: Arc::new(params) }));
            return Ok(json!({"pending": true}));
        }
    }
    finish(app, command, params)
}

fn finish(app: &mut SheetApp, command: &str, params: Value) -> Result<Value, String> {
    let previous_message = app.message.take();
    let result = app.run(command, params);
    // Import warnings belong in the reply and the existing notice strip, never a host popup.
    app.message = previous_message;
    let result = result?;
    app.backstage = None;
    if command == "file.open" {
        app.after_open(None, &result);
    } else {
        app.notice = None;
    }
    Ok(result)
}

/// A Close accepted by Save Changes removes the old uid. Cancel leaves it in place.
/// Run after control polling too, so a host can answer that prompt without presentation.
pub fn poll(app: &mut SheetApp, ctx: &egui::Context) {
    let Some(pending) = ctx.data(|d| d.get_temp::<Replacement>(replacement_id())) else { return };
    let exists = app.session.documents().iter().any(|d| d.uid == pending.uid);
    if exists && app.dialog.as_ref().is_some_and(|d| d.name == "saveChanges") {
        return;
    }
    ctx.data_mut(|d| d.remove::<Replacement>(replacement_id()));
    if !exists {
        if let Err(error) = finish(app, pending.command, pending.params.as_ref().clone()) {
            app.notice = Some(crate::Notice { text: crate::clean_error(&error), kind: crate::NoticeKind::Warning, action: None });
        }
        ctx.request_repaint();
    }
}

fn picker_id() -> egui::Id {
    egui::Id::new("gridcraft-host-open-picker")
}

/// A cancellation flag shared with the browser chooser, so late answers become inert.
pub fn begin_picker(ctx: &egui::Context) -> Arc<AtomicBool> {
    cancel_picker(ctx);
    let pending = Arc::new(AtomicBool::new(true));
    ctx.data_mut(|d| d.insert_temp(picker_id(), pending.clone()));
    pending
}

pub fn picker_pending(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<Arc<AtomicBool>>(picker_id())).is_some_and(|p| p.load(Ordering::Relaxed))
}

pub(super) fn cancel_picker(ctx: &egui::Context) -> bool {
    ctx.data_mut(|d| d.remove_temp::<Arc<AtomicBool>>(picker_id())).is_some_and(|p| p.swap(false, Ordering::Relaxed))
}
