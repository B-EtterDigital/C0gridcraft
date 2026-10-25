//! Opt-in page transport for the same control queue as the desktop shell.
//! Each request lends an unpainted egui pass from the web runner before returning.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use serde_json::{Value, json};

#[cfg(target_arch = "wasm32")]
mod bridge;
#[cfg(target_arch = "wasm32")]
mod files;
#[cfg(target_arch = "wasm32")]
pub use bridge::{attach, runner};

pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
#[cfg(target_arch = "wasm32")]
pub const POLL_MS: i32 = 4;
pub const PUMP_MS: f64 = 16.0;
pub const MAX_INLINE_PASSES: usize = 40;
pub const MAX_CLICK_COUNT: u64 = 256;
pub const MAX_DRAG_STEPS: u64 = 32;

pub fn request_limit(method: &str) -> usize {
    if method.starts_with("c0.") { gridcraft_ui_egui::c0_host::MAX_REQUEST_BYTES } else { MAX_REQUEST_BYTES }
}

pub fn parse(text: &str) -> Result<(String, Value), String> {
    if text.len() > gridcraft_ui_egui::c0_host::MAX_REQUEST_BYTES {
        return Err("request exceeds the host request limit".into());
    }
    let value: Value = serde_json::from_str(text).map_err(|e| format!("request is not JSON: {e}"))?;
    let method = value.get("method").and_then(Value::as_str).ok_or("request has no method")?;
    if text.len() > request_limit(method) {
        return Err("request is larger than 1 MiB".into());
    }
    let params = value.get("params").filter(|v| !v.is_null()).cloned().unwrap_or(json!({}));
    if !params.is_object() {
        return Err("params must be an object".into());
    }
    let bound = match method {
        "ui.click" | "ui.cell.click" => Some(("count", MAX_CLICK_COUNT)),
        "ui.drag" => Some(("steps", MAX_DRAG_STEPS)),
        _ => None,
    };
    if let Some((key, limit)) = bound
        && params.get(key).and_then(Value::as_u64).is_some_and(|value| value > limit)
    {
        return Err(format!("{key} exceeds the page input limit of {limit}"));
    }
    Ok((method.to_owned(), params))
}

pub fn reads_only(method: &str, params: &Value) -> bool {
    match method {
        "ui.inspect" | "document.inspect" | "sheet.read" | "cell.get" | "selection.stats" | "engine.commands" | "engine.journal" | "c0.inspect"
        | "c0.documentBytes" | "c0.selection" | "ui.render" => true,
        "engine.execute" | "command" | "ui.menu.invoke" => params
            .get("command")
            .or_else(|| params.get("id"))
            .and_then(Value::as_str)
            .is_some_and(|id| gridcraft_engine::find_command(id).is_some_and(|c| !c.journal && !c.undoable)),
        _ => gridcraft_engine::find_command(method).is_some_and(|c| !c.journal && !c.undoable),
    }
}

pub fn inline_passes(method: &str) -> usize {
    if matches!(method, "ui.click" | "ui.move" | "ui.drag" | "ui.key" | "ui.text" | "ui.cell.click" | "ui.cell.drag") { MAX_INLINE_PASSES } else { 0 }
}

pub fn pump_due(wants_repaint: bool, now: f64, last: f64) -> bool {
    wants_repaint && now - last >= PUMP_MS
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn parses_requests_and_rejects_bad_envelopes() {
        assert_eq!(parse(r#"{"method":"c0.inspect"}"#).unwrap(), ("c0.inspect".into(), json!({})));
        assert_eq!(parse(r#"{"method":"c0.inspect","params":null}"#).unwrap().1, json!({}));
        for text in ["no", "[]", "{}", r#"{"method":3}"#, r#"{"method":"c0.inspect","params":4}"#] {
            assert!(parse(text).is_err());
        }
    }

    #[test]
    fn request_limits_allow_workbook_bytes() {
        let large = json!({"method":"ui.text", "params":{"text":"x".repeat(MAX_REQUEST_BYTES)}}).to_string();
        assert!(parse(&large).is_err());
        assert!(parse(&large.replace("ui.text", "c0.openFile")).is_ok());
    }

    #[test]
    fn read_methods_and_command_aliases_do_not_request_presentation() {
        for method in ["c0.inspect", "c0.documentBytes", "c0.selection", "ui.render", "ui.inspect", "document.inspect", "engine.commands", "cell.get"] {
            assert!(reads_only(method, &json!({})), "{method}");
        }
        for method in ["c0.goTo", "c0.openFile", "c0.newDocument", "c0.cancelFileDialog", "c0.set", "c0.place", "ui.screenshot", "unknown"] {
            assert!(!reads_only(method, &json!({})), "{method}");
        }
        assert!(reads_only("engine.execute", &json!({"command":"document.inspect"})));
        assert!(reads_only("command", &json!({"id":"file.saveBytes"})));
        assert!(!reads_only("engine.execute", &json!({"command":"cell.set"})));
    }

    #[test]
    fn input_and_delayed_work_have_bounded_passes() {
        assert!(inline_passes("ui.cell.drag") > 0);
        assert_eq!(inline_passes("c0.inspect"), 0);
        assert!(pump_due(true, 1016.0, 1000.0));
        assert!(!pump_due(true, 1015.0, 1000.0));
        assert!(!pump_due(false, 5000.0, 1000.0));
    }

    #[test]
    fn hostile_input_counts_cannot_allocate_unbounded_event_queues() {
        for (method, key, limit) in
            [("ui.click", "count", MAX_CLICK_COUNT), ("ui.cell.click", "count", MAX_CLICK_COUNT), ("ui.drag", "steps", MAX_DRAG_STEPS)]
        {
            assert!(parse(&json!({"method":method,"params":{key:limit}}).to_string()).is_ok());
            assert!(parse(&json!({"method":method,"params":{key:limit+1}}).to_string()).is_err());
            assert!(parse(&json!({"method":method,"params":{key:u64::MAX}}).to_string()).is_err());
        }
    }
}
