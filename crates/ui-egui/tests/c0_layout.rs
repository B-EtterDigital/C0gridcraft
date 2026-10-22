//! The ribbon's tool modes (`toolbar`, `toolbar_double`, `toolbar_floating`) and the foldable
//! right dock (`right_dock_collapsed`): set, inspect, defaults, unknown keys, and what they draw.

use egui_kittest::kittest::Queryable as _;
use gridcraft_engine::Session;
use gridcraft_ui_egui::{
    Services, SheetApp,
    control::{self, Outcome},
};
use serde_json::{Value, json};

const KEYS: [&str; 4] = ["toolbar", "toolbar_double", "toolbar_floating", "right_dock_collapsed"];
/// The ribbon panel's height as upstream draws it: the tab row and the 88 pt card, with margin.
const FULL_RIBBON: f32 = 34.0 + 92.0;

fn app() -> SheetApp {
    let mut session = Session::new();
    session.new_workbook();
    SheetApp::new(session, Services::default())
}

fn harness() -> egui_kittest::Harness<'static, SheetApp> {
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        app(),
    );
    h.run_steps(4);
    h
}

fn handle(h: &mut egui_kittest::Harness<'_, SheetApp>, method: &str, params: Value) -> Value {
    let ctx = h.ctx.clone();
    let Outcome::Done(reply) = control::handle(h.state_mut(), &ctx, method, &params) else { panic!("deferred {method}") };
    h.run_steps(3);
    reply
}

fn set(h: &mut egui_kittest::Harness<'_, SheetApp>, panels: Value) -> Value {
    let reply = handle(h, "c0.set", json!({"panels": panels}));
    assert_eq!(reply["ok"], true, "{reply}");
    reply["result"].clone()
}

fn panes(h: &mut egui_kittest::Harness<'_, SheetApp>) -> Value {
    let reply = handle(h, "c0.inspect", json!({}));
    assert_eq!(reply["ok"], true, "{reply}");
    reply["result"]["panes"].clone()
}

/// Every string drawn this frame (ribbon tabs are painted, not labelled widgets).
fn drawn_text(h: &egui_kittest::Harness<'_, SheetApp>) -> String {
    fn collect(shape: &egui::Shape, out: &mut String) {
        match shape {
            egui::Shape::Text(text) => {
                out.push_str(text.galley.text());
                out.push('\n');
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, out)),
            _ => {}
        }
    }
    let mut text = String::new();
    h.output().shapes.iter().for_each(|s| collect(&s.shape, &mut text));
    text
}

/// The grid's rectangle, which is what the ribbon and the right dock take their space from.
fn grid(h: &egui_kittest::Harness<'_, SheetApp>) -> egui::Rect {
    h.state().grid.rect.expect("the grid was drawn")
}

#[test]
fn defaults_are_the_upstream_layout_and_inspect_says_so() {
    let mut h = harness();
    let p = panes(&mut h);
    assert_eq!(p["toolbar"], true);
    for key in ["toolbar_double", "toolbar_floating", "right_dock_collapsed"] {
        assert_eq!(p[key], false, "{key}");
    }
    // The existing pane reports are untouched.
    for key in ["formulaBar", "headings", "gridlines", "statusBar"] {
        assert_eq!(p[key], true, "{key}");
    }
    assert_eq!(h.state().chrome.layout, gridcraft_ui_egui::c0_host::Layout::default());
    // Upstream's ribbon: tab row plus the 88 pt card, docked, and no dock strip or float grip.
    let title = h.state().grid.rect.map(|r| r.top()).unwrap_or_default();
    assert!(title >= FULL_RIBBON, "{title}");
    assert!(h.query_by_label("Move Toolbar").is_none());
    assert!(h.query_by_label("Expand panels").is_none());
}

#[test]
fn every_key_sets_inspects_and_returns_to_its_default() {
    let mut h = harness();
    for key in KEYS {
        let default = key == "toolbar";
        let result = set(&mut h, json!({ key: !default }));
        assert_eq!(result["applied"], json!([format!("panels.{key}")]), "{key}");
        assert_eq!(result["unknown"], json!([]));
        assert_eq!(result["invalid"], json!([]));
        assert_eq!(panes(&mut h)[key], !default, "{key}");
        // Repeating a value is idempotent.
        set(&mut h, json!({ key: !default }));
        assert_eq!(panes(&mut h)[key], !default, "{key}");
        set(&mut h, json!({ key: default }));
        assert_eq!(panes(&mut h)[key], default, "{key}");
    }
    assert_eq!(h.state().chrome.layout, gridcraft_ui_egui::c0_host::Layout::default());
}

#[test]
fn unknown_and_non_boolean_values_are_reported_and_change_nothing() {
    let mut h = harness();
    let result = set(&mut h, json!({"toolbar_triple": true, "toolbar_double": "yes", "toolbar_floating": 1, "right_dock_collapsed": null}));
    assert_eq!(result["applied"], json!([]));
    assert_eq!(result["unknown"], json!(["panels.toolbar_triple"]));
    let mut invalid: Vec<String> = result["invalid"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
    invalid.sort();
    assert_eq!(invalid, ["panels.right_dock_collapsed", "panels.toolbar_double", "panels.toolbar_floating"]);
    assert_eq!(h.state().chrome.layout, gridcraft_ui_egui::c0_host::Layout::default());
}

#[test]
fn the_modes_are_host_description_and_never_saved_as_preferences() {
    let mut h = harness();
    set(&mut h, json!({"toolbar": false, "toolbar_double": true, "toolbar_floating": true, "right_dock_collapsed": true}));
    let prefs = serde_json::to_string(&h.state().ui).unwrap().to_lowercase();
    for word in ["toolbar", "floating", "double", "dock", "layout"] {
        assert!(!prefs.contains(word), "{word}: {prefs}");
    }
    // The upstream collapse switch is independent of the host's toolbar switch.
    assert!(!h.state().ui.ribbon_collapsed);
}

#[test]
fn toolbar_false_collapses_the_ribbon_to_its_tabs_and_true_restores_it() {
    let mut h = harness();
    let full = grid(&h).top();
    set(&mut h, json!({"toolbar": false}));
    assert_eq!(full - grid(&h).top(), 92.0);
    // The tab row is still there, and a tab's click (which un-collapses upstream's own switch)
    // does not bring the card back.
    assert!(drawn_text(&h).lines().any(|l| l == "Insert"));
    h.state_mut().ui.ribbon_tab = "Insert".into();
    h.state_mut().ui.ribbon_collapsed = false;
    h.run_steps(3);
    assert_eq!(full - grid(&h).top(), 92.0);
    set(&mut h, json!({"toolbar": true}));
    assert_eq!(grid(&h).top(), full);
}

#[test]
fn double_is_a_compact_ribbon_of_icon_buttons_that_keep_their_names_and_their_height() {
    let mut h = harness();
    let full = grid(&h).top();
    let wide = h.get_by_label("Paste").rect().width();
    assert!(wide >= 40.0, "{wide}");
    set(&mut h, json!({"toolbar_double": true}));
    // The groups of this ribbon stack up to three rows, so the card cannot get shorter: the
    // density is in the width, a big button shrinking to its icon.
    assert_eq!(grid(&h).top(), full);
    let narrow = h.get_by_label("Paste").rect().width();
    assert!(narrow < wide - 8.0, "{narrow} {wide}");
    h.get_by_label("Paste").hover();
    h.run_steps(2);
    for tab in ["Home", "Insert", "Page Layout", "Formulas", "Data", "Review", "View", "Automate"] {
        h.state_mut().ui.ribbon_tab = tab.into();
        h.run_steps(3);
        assert_eq!(grid(&h).top(), full, "{tab}");
    }
    set(&mut h, json!({"toolbar_double": false}));
    assert_eq!(grid(&h).top(), full);
    h.state_mut().ui.ribbon_tab = "Home".into();
    h.run_steps(3);
    assert_eq!(h.get_by_label("Paste").rect().width(), wide);
}

#[test]
fn floating_lifts_the_ribbon_off_the_dock_and_it_stays_inside_the_canvas() {
    let mut h = harness();
    let full = grid(&h).top();
    set(&mut h, json!({"toolbar_floating": true}));
    // Nothing is docked: the canvas took the ribbon's whole height back.
    assert_eq!(full - grid(&h).top(), FULL_RIBBON);
    assert!(h.query_by_label("Move Toolbar").is_some());
    assert!(drawn_text(&h).lines().any(|l| l == "Home"));
    let id = egui::Id::new("c0-floating-ribbon");
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0));
    for at in [None, Some((-500, -500)), Some((9000, 9000))] {
        h.state_mut().chrome.layout.float_at = at;
        h.run_steps(3);
        let area = h.ctx.memory(|m| m.area_rect(id)).expect("the floating ribbon is an area");
        assert!(screen.contains_rect(area), "{at:?}: {area:?}");
        assert!(area.top() >= grid(&h).top() - 1.0, "{at:?}: {area:?}");
    }
    // Floating compact: the same palette with icon buttons.
    let wide = h.get_by_label("Paste").rect().width();
    set(&mut h, json!({"toolbar_double": true}));
    assert!(h.get_by_label("Paste").rect().width() < wide - 8.0);
    set(&mut h, json!({"toolbar_floating": false, "toolbar_double": false}));
    assert_eq!(grid(&h).top(), full);
    assert!(h.query_by_label("Move Toolbar").is_none());
}

#[test]
fn dragging_the_grip_moves_the_floating_ribbon() {
    let mut h = harness();
    set(&mut h, json!({"toolbar_floating": true}));
    let before = h.state().chrome.layout.float_at;
    assert_eq!(before, None);
    let grip = h.get_by_label("Move Toolbar").rect();
    let from = grip.center();
    h.input_mut().events.extend([
        egui::Event::PointerMoved(from),
        egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() },
    ]);
    h.step();
    h.input_mut().events.push(egui::Event::PointerMoved(from + egui::vec2(60.0, 40.0)));
    h.step();
    h.input_mut().events.push(egui::Event::PointerButton {
        pos: from + egui::vec2(60.0, 40.0),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    h.run_steps(3);
    let after = h.state().chrome.layout.float_at.expect("the grip stored a position");
    assert!(after.0 > 12 && after.1 > 12, "{after:?}");
}

fn open_watch_pane(h: &mut egui_kittest::Harness<'_, SheetApp>) {
    h.state_mut().grid.pane = Some("watch".into());
    h.run_steps(3);
}

#[test]
fn the_dock_folds_to_an_icon_strip_without_moving_the_canvas_vertically() {
    let mut h = harness();
    open_watch_pane(&mut h);
    let open = grid(&h);
    assert!(h.query_by_label("Watch Window").is_some());
    set(&mut h, json!({"right_dock_collapsed": true}));
    let folded = grid(&h);
    assert!(h.query_by_label("Watch Window").is_some(), "the strip's icon keeps the pane's name");
    assert!(h.query_by_label("Expand panels").is_some());
    // Only the width changes; the canvas content does not move or resize vertically.
    assert_eq!((folded.top(), folded.bottom(), folded.left()), (open.top(), open.bottom(), open.left()));
    assert!(folded.width() > open.width() + 200.0, "{open:?} {folded:?}");
    set(&mut h, json!({"right_dock_collapsed": false}));
    assert_eq!(grid(&h), open, "unfolding restores the dock as it was");
    assert!(h.query_by_label("Expand panels").is_none());
}

#[test]
fn a_strip_icon_opens_its_pane_and_the_header_button_folds_it_again() {
    let mut h = harness();
    set(&mut h, json!({"right_dock_collapsed": true}));
    let folded = grid(&h);
    for (label, pane) in [("Watch Window", "watch"), ("Selection", "selection"), ("Format Chart Area", "formatChart")] {
        h.get_by_label(label).click();
        h.run_steps(3);
        assert_eq!(panes(&mut h)["right_dock_collapsed"], false, "{label}");
        assert_eq!(h.state().grid.pane.as_deref(), Some(pane));
        assert!(grid(&h).width() < folded.width(), "{label}");
        h.get_by_label("Fold panels").click();
        h.run_steps(3);
        assert_eq!(panes(&mut h)["right_dock_collapsed"], true, "{label}");
        assert_eq!(grid(&h), folded);
    }
    // Comments is engine state: its icon turns it on.
    h.get_by_label("Comments").click();
    h.run_steps(3);
    assert!(h.state().session.comments_pane);
    assert_eq!(panes(&mut h)["right_dock_collapsed"], false);
}

#[test]
fn the_chevron_unfolds_to_the_pane_that_was_open() {
    let mut h = harness();
    open_watch_pane(&mut h);
    set(&mut h, json!({"right_dock_collapsed": true}));
    h.get_by_label("Expand panels").click();
    h.run_steps(3);
    assert!(!h.state().chrome.layout.right_dock_collapsed);
    assert_eq!(h.state().grid.pane.as_deref(), Some("watch"));
}

/// With `C0_LAYOUT_SNAPSHOTS=<dir>` set, writes one picture per mode for a human to look at.
#[test]
fn pictures_of_each_mode_when_asked_for() {
    let Ok(dir) = std::env::var("C0_LAYOUT_SNAPSHOTS") else { return };
    let mut h = harness();
    h.state_mut().grid.pane = Some("watch".into());
    let modes: [(&str, Value); 7] = [
        ("full", json!({})),
        ("compact", json!({"toolbar_double": true})),
        ("tabs-only", json!({"toolbar": false})),
        ("floating", json!({"toolbar_floating": true})),
        ("floating-compact", json!({"toolbar_floating": true, "toolbar_double": true})),
        ("floating-tabs", json!({"toolbar_floating": true, "toolbar": false})),
        ("dock-folded", json!({"right_dock_collapsed": true})),
    ];
    for (name, panels) in modes {
        let mut all = json!({"toolbar": true, "toolbar_double": false, "toolbar_floating": false, "right_dock_collapsed": false});
        for (k, v) in panels.as_object().unwrap() {
            all[k] = v.clone();
        }
        set(&mut h, all);
        h.run_steps(3);
        h.render().expect("a wgpu adapter").save(format!("{dir}/{name}.png")).expect("png");
    }
}
