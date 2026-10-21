//! Bounded chooser reads; cancellation prevents late browser answers from importing a file.

use std::sync::atomic::Ordering;

use gridcraft_ui_egui::{SheetApp, c0_host};
use wasm_bindgen_futures::{JsFuture, spawn_local};

pub fn attach(app: &mut SheetApp, ctx: &egui::Context) {
    let inbox = app.services.inbox.clone();
    let ctx = ctx.clone();
    app.services.open_async = Some(Box::new(move || {
        let Some(inbox) = inbox.clone() else { return };
        let pending = c0_host::begin_picker(&ctx);
        let ctx = ctx.clone();
        spawn_local(async move {
            let file = rfd::AsyncFileDialog::new()
                .add_filter("Spreadsheets", &["xlsx", "xlsm", "xls", "xlsb", "ods", "csv", "tsv", "txt", "json"])
                .pick_file()
                .await;
            if !pending.load(Ordering::Relaxed) {
                return;
            }
            let Some(file) = file else {
                pending.store(false, Ordering::Relaxed);
                return;
            };
            let size = file.inner().size();
            let held = inbox.lock().unwrap_or_else(std::sync::PoisonError::into_inner).iter().map(|(_, b)| b.len()).fold(0, usize::saturating_add);
            if !size.is_finite() || size < 0.0 || size > c0_host::MAX_FILE_BYTES.saturating_sub(held) as f64 {
                pending.store(false, Ordering::Relaxed);
                log::warn!("page-control: picked file exceeds the host file limit");
                return;
            }
            let bytes = match JsFuture::from(file.inner().array_buffer()).await {
                Ok(buffer) => js_sys::Uint8Array::new(&buffer).to_vec(),
                Err(error) => {
                    pending.store(false, Ordering::Relaxed);
                    log::warn!("page-control: could not read picked file: {error:?}");
                    return;
                }
            };
            let mut inbox = inbox.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let held = inbox.iter().map(|(_, b)| b.len()).fold(0, usize::saturating_add);
            if bytes.len() > c0_host::MAX_FILE_BYTES.saturating_sub(held) {
                pending.store(false, Ordering::Relaxed);
                return;
            }
            if !pending.swap(false, Ordering::Relaxed) {
                return;
            }
            inbox.push((file.file_name(), bytes));
            ctx.request_repaint();
        });
    }));
}
