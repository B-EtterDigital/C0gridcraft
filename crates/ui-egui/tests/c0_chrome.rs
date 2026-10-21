//! Rendered promotional and Agent Control surfaces, plus host panel behaviour.

use egui_kittest::kittest::Queryable as _;
use gridcraft_engine::Session;
use gridcraft_ui_egui::{
    Chrome, Services, SheetApp,
    control::{self, Outcome},
};
use serde_json::{Value, json};

fn app() -> SheetApp {
    let mut session = Session::new();
    session.new_workbook();
    SheetApp::new(session, Services::default())
}

fn harness(app: SheetApp) -> egui_kittest::Harness<'static, SheetApp> {
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        app,
    );
    h.run_steps(4);
    h
}

fn set(app: &mut SheetApp, ctx: &egui::Context, params: Value) -> Value {
    let Outcome::Done(reply) = control::handle(app, ctx, "c0.set", &params) else { panic!("deferred settings") };
    assert_eq!(reply["ok"], true, "{reply}");
    reply["result"].clone()
}

fn visible_text(h: &egui_kittest::Harness<'_, SheetApp>) -> String {
    fn collect(shape: &egui::Shape, out: &mut String) {
        match shape {
            egui::Shape::Text(text) => {
                out.push_str(text.galley.text());
                out.push('\n');
            }
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, out);
                }
            }
            _ => {}
        }
    }
    let mut text = String::new();
    for shape in &h.output().shapes {
        collect(&shape.shape, &mut text);
    }
    text
}

#[test]
fn default_chrome_keeps_the_upstream_about_and_agent_button() {
    let mut app = app();
    assert_eq!(app.chrome, Chrome { title_bar: true, file_tab: true, community_links: true, agent_control: true });
    app.ui.ribbon_tab = "Automate".into();
    let mut h = harness(app);
    assert!(visible_text(&h).lines().any(|line| line == "Agent"));
    h.state_mut().open_dialog("about", json!({}));
    h.run_steps(3);
    assert!(h.query_by_label("A clean-room, open-source, Rust-native spreadsheet. Part of ArtCraft.").is_some());
    assert!(h.query_by_label("getartcraft.com/apps/gridcraft").is_some());
    assert!(h.query_by_label("Join the ArtCraft Discord").is_some());
}

#[test]
fn hidden_community_has_no_promotional_text_and_keeps_credit_and_licence() {
    let mut app = app();
    app.chrome.community_links = false;
    app.open_dialog("about", json!({}));
    let mut h = harness(app);
    for text in
        ["A clean-room, open-source, Rust-native spreadsheet. Part of ArtCraft.", "getartcraft.com/apps/gridcraft", "Join the ArtCraft Discord"]
    {
        assert!(h.query_by_label(text).is_none(), "{text}");
    }
    let rendered = visible_text(&h).to_ascii_lowercase();
    for promotion in ["artcraft", "discord", "getartcraft"] {
        assert!(!rendered.contains(promotion), "{rendered}");
    }
    assert!(h.query_by_label("GridCraft").is_some());
    assert!(h.query_by_label("By the GridCraft contributors (see Contributors).").is_some());
    assert!(h.query_by_label("Licensed under MIT OR Apache-2.0.").is_some());
    if let Ok(path) = std::env::var("C0SHEET_SNAPSHOT") {
        h.render().unwrap().save(path).unwrap();
    }
    h.get_by_label("Contributors").click();
    h.run_steps(3);
    assert!(h.state().dialog.as_ref().is_some_and(|d| d.tab == "Contributors"));
}

#[test]
fn agent_switch_hides_rendered_button_and_refuses_every_programmatic_entry() {
    let mut app = app();
    app.chrome.agent_control = false;
    app.ui.ribbon_tab = "Automate".into();
    let h = harness(app);
    assert!(!visible_text(&h).lines().any(|line| line == "Agent"));
    let mut app = self::app();
    app.chrome.agent_control = false;
    let ctx = egui::Context::default();
    for (method, params) in [
        ("ui.dialog", json!({"name":"agents"})),
        ("engine.execute", json!({"command":"ui.dialog", "params":{"name":"agents"}})),
        ("command", json!({"id":"ui.dialog", "params":{"name":"agents"}})),
        ("ui.menu.invoke", json!({"id":"ui.dialog", "params":{"name":"agents"}})),
    ] {
        let Outcome::Done(reply) = control::handle(&mut app, &ctx, method, &params) else { panic!("deferred dialog") };
        assert_eq!(reply["ok"], false, "{reply}");
        assert!(app.dialog.is_none());
    }
    app.open_dialog("agents", json!({}));
    assert!(app.dialog.is_none(), "native menu entry is also guarded");
    assert!(gridcraft_ui_egui::dialogs::Dialog::open(&mut app, "agents", json!({})).is_none());
}

#[test]
fn panel_reports_are_per_key_and_repeated_values_do_not_toggle() {
    let mut app = app();
    let ctx = egui::Context::default();
    app.open_dialog("agents", json!({}));
    let params = json!({"panels":{
        "title_bar":false,"file_tab":false,"community_links":false,"agent_control":false,
        "ribbon_collapsed":true,"formula_bar":false,"headings":false,"gridlines":false,"status_bar":false,
        "not_a_panel":false,
    }});
    for _ in 0..2 {
        let result = set(&mut app, &ctx, params.clone());
        assert_eq!(result["applied"].as_array().unwrap().len(), 9);
        assert_eq!(result["unknown"], json!(["panels.not_a_panel"]));
        assert_eq!(result["invalid"], json!([]));
    }
    assert!(app.dialog.is_none());
    assert!(app.ui.ribbon_collapsed);
    assert!(!app.ui.status_bar);
    assert!(!app.chrome.title_bar && !app.chrome.file_tab && !app.chrome.community_links && !app.chrome.agent_control);
    assert!(!app.session.prefs.formula_bar);
    let sh = app.session.active().unwrap().wb.active().unwrap();
    assert!(!sh.show_headings && !sh.show_gridlines);
    let result = set(&mut app, &ctx, json!({"panels":{"ribbon_collapsed":"no", "unknown":3}, "other":true}));
    assert_eq!(result["invalid"], json!(["panels.ribbon_collapsed"]));
    assert_eq!(result["unknown"], json!(["other", "panels.unknown"]));
    assert!(app.ui.ribbon_collapsed);
}

#[test]
fn containers_are_validated_before_settings_are_applied() {
    let mut app = app();
    let ctx = egui::Context::default();
    for params in [json!({"panels":false,"themeTokens":{"accent":"#123456"}}), json!({"panels":{"title_bar":false},"themeTokens":3})] {
        let Outcome::Done(reply) = control::handle(&mut app, &ctx, "c0.set", &params) else { panic!("deferred settings") };
        assert_eq!(reply["ok"], false);
        assert!(app.chrome.title_bar);
    }
}

#[test]
fn host_fields_are_not_persisted_as_user_preferences() {
    let app = app();
    let prefs = serde_json::to_string(&app.ui).unwrap();
    for field in ["community_links", "communityLinks", "agent_control", "agentControl", "title_bar", "file_tab"] {
        assert!(!prefs.contains(field));
    }
}
