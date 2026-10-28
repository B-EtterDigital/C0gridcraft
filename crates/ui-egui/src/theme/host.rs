//! Context-local colour overrides. The upstream palettes remain the source of defaults.

use std::sync::Arc;

use egui::Color32;
use serde_json::{Map, Value, json};

use super::Tokens;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub applied: Vec<String>,
    pub unknown: Vec<String>,
    pub invalid: Vec<String>,
}

impl Report {
    pub fn to_json(&self) -> Value {
        json!({"applied": self.applied, "unknown": self.unknown, "invalid": self.invalid})
    }
}

fn store() -> egui::Id {
    egui::Id::unique("gridcraft-host-palette")
}

/// Replace the override set. Null or an empty object clears it; valid siblings still apply.
pub fn set(ctx: &egui::Context, value: &Value) -> Result<Report, String> {
    let entries = match value {
        Value::Null => None,
        Value::Object(entries) => Some(entries),
        _ => return Err("themeTokens must be an object or null".into()),
    };
    let mut report = Report::default();
    let mut clean = Map::new();
    let mut scratch = Tokens::light();
    for (key, value) in entries.into_iter().flatten() {
        if !assign(&mut scratch, key, Color32::TRANSPARENT) {
            report.unknown.push(key.clone());
        } else if colour(value).is_some() {
            clean.insert(key.clone(), value.clone());
            report.applied.push(key.clone());
        } else {
            report.invalid.push(key.clone());
        }
    }
    ctx.data_mut(|d| {
        d.remove::<Arc<Value>>(store());
        if !clean.is_empty() {
            d.insert_temp(store(), Arc::new(Value::Object(clean)));
        }
    });
    // Refresh egui's widget visuals with the same palette. Preserve System and fallback mode.
    let options = ctx.options(|o| (o.theme_preference, o.fallback_theme));
    super::apply(ctx, ctx.global_style().visuals.dark_mode);
    ctx.options_mut(|o| {
        o.theme_preference = options.0;
        o.fallback_theme = options.1;
    });
    Ok(report)
}

pub(super) fn resolve(ctx: &egui::Context, mut tokens: Tokens) -> Tokens {
    if let Some(value) = ctx.data(|d| d.get_temp::<Arc<Value>>(store()))
        && let Some(entries) = value.as_object()
    {
        for (key, value) in entries {
            if let Some(color) = colour(value) {
                assign(&mut tokens, key, color);
            }
        }
    }
    tokens
}

fn colour(value: &Value) -> Option<Color32> {
    let hex = value.as_str()?.strip_prefix('#')?;
    if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |at| hex.get(at..at + 2).and_then(|s| u8::from_str_radix(s, 16).ok());
    Some(Color32::from_rgba_unmultiplied(byte(0)?, byte(2)?, byte(4)?, if hex.len() == 8 { byte(6)? } else { 255 }))
}

// `dark` is the non-colour theme selector; all scalar Color32 fields are registered here.
fn assign(t: &mut Tokens, key: &str, color: Color32) -> bool {
    macro_rules! colors {
        ($($field:ident),* $(,)?) => {
            match key { $(stringify!($field) => { t.$field = color; true },)* _ => false }
        };
    }
    colors!(
        window,
        ribbon,
        ribbon_border,
        text,
        text_dim,
        text_disabled,
        accent,
        accent_dark,
        accent_soft,
        hover,
        pressed,
        separator,
        grid_bg,
        desk,
        gridline,
        header_bg,
        header_text,
        header_line,
        header_sel_bg,
        header_sel_text,
        header_all_bg,
        sel_fill,
        sel_border,
        input_bg,
        input_border,
        tab_bar,
        tab_active,
        status_bar,
        cell_text,
        menu_bg,
        shadow,
        danger,
        notice_warn_bg,
        notice_warn_border,
        notice_info_bg,
        notice_info_border,
        error_check_green,
        warning_fill,
        warning_ink,
        warning,
        on_accent,
        on_warning,
        paper,
        sheet_control_bg,
        sheet_control_border,
        sheet_control_text,
        sheet_track,
    )
}
