use egui::{Color32, Theme, ThemePreference};
use gridcraft_ui_egui::{
    SheetApp,
    theme::{self, Tokens, host},
};
use serde_json::json;

#[test]
fn no_override_leaves_both_upstream_palettes_intact() {
    let ctx = egui::Context::default();
    SheetApp::setup_context(&ctx, false);
    assert_eq!(Tokens::get(&ctx), Tokens::light());
    ctx.set_theme(ThemePreference::Dark);
    assert_eq!(Tokens::get(&ctx), Tokens::dark());
}

#[test]
fn reports_and_colours_survive_theme_changes_and_widget_reapplication() {
    let ctx = egui::Context::default();
    SheetApp::setup_context(&ctx, false);
    let report = host::set(&ctx, &json!({"accent":"#123456", "sel_fill":"#aabbcc80", "text":42, "unknown":"#ffffff", "dark":"#000000"})).unwrap();
    assert_eq!(report.applied, ["accent", "sel_fill"]);
    assert_eq!(report.unknown, ["dark", "unknown"]);
    assert_eq!(report.invalid, ["text"]);
    for dark in [false, true, false] {
        theme::apply(&ctx, dark);
        assert_eq!(Tokens::get(&ctx).accent, Color32::from_rgb(0x12, 0x34, 0x56));
        assert_eq!(Tokens::get(&ctx).sel_fill, Color32::from_rgba_unmultiplied(0xaa, 0xbb, 0xcc, 0x80));
        assert_eq!(Tokens::get(&ctx).dark, dark);
        assert_eq!(ctx.global_style().visuals.selection.stroke.color, Tokens::get(&ctx).accent);
    }
}

#[test]
fn set_preserves_system_preference_and_clear_restores_widget_defaults() {
    let ctx = egui::Context::default();
    SheetApp::setup_context(&ctx, false);
    ctx.set_theme(ThemePreference::System);
    host::set(&ctx, &json!({"window":"#123456", "accent":"#123456"})).unwrap();
    assert_eq!(ctx.options(|o| o.theme_preference), ThemePreference::System);
    host::set(&ctx, &json!({})).unwrap();
    assert_eq!(Tokens::get(&ctx), Tokens::light());
    assert_eq!(ctx.style_of(Theme::Dark).visuals.selection.stroke.color, Tokens::dark().accent);
    host::set(&ctx, &json!({"accent":"#654321"})).unwrap();
    host::set(&ctx, &serde_json::Value::Null).unwrap();
    assert_eq!(Tokens::get(&ctx), Tokens::light());
}

#[test]
fn hostile_colour_values_are_invalid_without_panics() {
    let ctx = egui::Context::default();
    for value in [
        json!(null),
        json!(true),
        json!(8),
        json!("red"),
        json!("123456"),
        json!("#12345"),
        json!("#1234567"),
        json!("#zzzzzz"),
        json!("#ééé"),
        json!([1, 2, 3]),
    ] {
        let report = host::set(&ctx, &json!({"accent":value})).unwrap();
        assert_eq!(report.invalid, ["accent"]);
        assert!(report.applied.is_empty());
    }
}

#[test]
fn every_scalar_colour_token_is_accepted_and_other_contexts_stay_independent() {
    let ctx = egui::Context::default();
    let other = egui::Context::default();
    let initial = Tokens::get(&other);
    // An upstream field addition fails this contract until the registry includes it.
    let source = include_str!("../src/theme.rs");
    let fields = source.split("pub struct Tokens {").nth(1).unwrap().split('}').next().unwrap();
    let mut tokens = serde_json::Map::new();
    for line in fields.lines().filter(|l| l.contains(": Color32")) {
        let key = line.trim().strip_prefix("pub ").unwrap().split(':').next().unwrap();
        tokens.insert(key.into(), json!("#123456"));
    }
    let report = host::set(&ctx, &tokens.clone().into()).unwrap();
    assert_eq!(report.applied.len(), tokens.len());
    assert!(report.unknown.is_empty() && report.invalid.is_empty());
    assert_eq!(Tokens::get(&other), initial);
}
