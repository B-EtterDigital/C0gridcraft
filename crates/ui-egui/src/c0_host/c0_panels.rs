//! Idempotent host switches and per-key settings reports.

use serde_json::{Value, json};

use crate::{SheetApp, theme::host};

pub(super) fn set(app: &mut SheetApp, ctx: &egui::Context, params: &Value) -> Result<Value, String> {
    if params.get("panels").is_some_and(|v| !v.is_object()) {
        return Err("panels must be an object".into());
    }
    if params.get("themeTokens").is_some_and(|v| !v.is_object() && !v.is_null()) {
        return Err("themeTokens must be an object or null".into());
    }
    let mut report = host::Report::default();
    for key in params.as_object().into_iter().flat_map(|p| p.keys()) {
        if !matches!(key.as_str(), "themeTokens" | "panels") {
            report.unknown.push(key.clone());
        }
    }
    if let Some(tokens) = params.get("themeTokens") {
        let tokens = host::set(ctx, tokens)?;
        report.applied.extend(tokens.applied.into_iter().map(|k| format!("themeTokens.{k}")));
        report.unknown.extend(tokens.unknown.into_iter().map(|k| format!("themeTokens.{k}")));
        report.invalid.extend(tokens.invalid.into_iter().map(|k| format!("themeTokens.{k}")));
    }
    if let Some(panels) = params.get("panels").and_then(Value::as_object) {
        for (key, value) in panels {
            let key_path = format!("panels.{key}");
            if !matches!(
                key.as_str(),
                "title_bar"
                    | "file_tab"
                    | "community_links"
                    | "agent_control"
                    | "ribbon_collapsed"
                    | "formula_bar"
                    | "headings"
                    | "gridlines"
                    | "status_bar"
            ) {
                report.unknown.push(key_path);
                continue;
            }
            let Some(on) = value.as_bool() else {
                report.invalid.push(key_path);
                continue;
            };
            let result = match key.as_str() {
                "title_bar" => {
                    app.chrome.title_bar = on;
                    Ok(json!({}))
                }
                "file_tab" => {
                    app.chrome.file_tab = on;
                    if !on {
                        app.backstage = None;
                    }
                    Ok(json!({}))
                }
                "community_links" => {
                    app.chrome.community_links = on;
                    Ok(json!({}))
                }
                "agent_control" => {
                    app.chrome.agent_control = on;
                    if !on && app.dialog.as_ref().is_some_and(|d| d.name == "agents") {
                        app.dialog = None;
                    }
                    Ok(json!({}))
                }
                "ribbon_collapsed" => {
                    if app.ui.ribbon_collapsed == on {
                        Ok(json!({}))
                    } else {
                        app.run("view.collapseRibbon", json!({}))
                    }
                }
                "formula_bar" => app.run("view.formulaBar", json!({"on": on})),
                "headings" => app.run("view.headings", json!({"on": on})),
                "gridlines" => app.run("view.gridlines", json!({"on": on})),
                "status_bar" => {
                    app.ui.status_bar = on;
                    Ok(json!({}))
                }
                _ => Err("unregistered panel".into()),
            };
            if result.is_ok() {
                report.applied.push(key_path);
            } else {
                report.invalid.push(key_path);
            }
        }
    }
    Ok(report.to_json())
}
