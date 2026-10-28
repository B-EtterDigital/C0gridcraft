//! Real queues and unpainted egui passes; no window or frame presentation.

use std::sync::mpsc::{Sender, channel};

use egui_kittest::kittest::Queryable as _;
use gridcraft_engine::{Session, io};
use gridcraft_ui_egui::{ControlRequest, Services, SheetApp};
use serde_json::{Value, json};

type Harness = egui_kittest::Harness<'static, SheetApp>;

fn harness() -> (Harness, Sender<ControlRequest>) {
    let mut session = Session::new();
    session.new_workbook();
    let (tx, rx) = channel();
    let mut app = SheetApp::new(session, Services::default());
    app.control_rx = Some(rx);
    app.ui.animations = false;
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        app,
    );
    h.run_steps(8);
    (h, tx)
}

fn request(h: &mut Harness, tx: &Sender<ControlRequest>, method: &str, params: Value) -> Value {
    let (request, rx) = ControlRequest::new(method, params);
    tx.send(request).unwrap();
    assert!(rx.try_recv().is_err());
    h.step();
    let reply = rx.try_recv().expect("one unpainted pass must answer the queue");
    assert_eq!(reply["ok"], true, "{method}: {reply}");
    reply["result"].clone()
}

#[test]
fn every_host_action_is_answered_by_unpainted_passes() {
    let (mut h, tx) = harness();
    for method in ["c0.inspect", "c0.documentBytes", "ui.inspect", "document.inspect", "engine.commands"] {
        request(&mut h, &tx, method, json!({}));
    }
    request(&mut h, &tx, "c0.set", json!({"panels":{"title_bar":false,"community_links":false}}));
    request(&mut h, &tx, "c0.goTo", json!({"ref":"D800:F802"}));
    assert!(h.state().view().scroll.y > 0.0, "navigation actually scrolls during layout");
    request(&mut h, &tx, "c0.cancelFileDialog", json!({}));
    request(&mut h, &tx, "c0.newDocument", json!({}));
    request(&mut h, &tx, "c0.openFile", json!({"name":"host.csv","bytes":io::base64_encode(b"Host,42\n")}));
    assert_eq!(request(&mut h, &tx, "cell.get", json!({"cell":"A1"}))["value"], "Host");
}

#[test]
fn ordered_requests_observe_changes_in_one_pass() {
    let (mut h, tx) = harness();
    let (set, set_reply) = ControlRequest::new("c0.set", json!({"panels":{"status_bar":false,"ribbon_collapsed":true}}));
    let (inspect, inspect_reply) = ControlRequest::new("c0.inspect", json!({}));
    tx.send(set).unwrap();
    tx.send(inspect).unwrap();
    h.step();
    assert_eq!(set_reply.try_recv().unwrap()["ok"], true);
    let reply = inspect_reply.try_recv().unwrap();
    assert_eq!(reply["result"]["panes"]["statusBar"], false);
    assert_eq!(reply["result"]["ribbonCollapsed"], true);
}

#[test]
fn dirty_open_cancel_keeps_the_original_and_dont_save_finishes_replacement() {
    let (mut h, tx) = harness();
    request(&mut h, &tx, "cell.set", json!({"cell":"A1","input":"Keep me"}));
    let uid = h.state().session.active().unwrap().uid;
    let params = json!({"name":"host.csv","bytes":io::base64_encode(b"Replacement,42\n")});
    assert_eq!(request(&mut h, &tx, "c0.openFile", params.clone()), json!({"pending":true}));
    assert_eq!(h.state().session.active().unwrap().uid, uid);
    h.run_steps(3);
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Cancel").click();
    h.run_steps(3);
    assert_eq!(h.state().session.active().unwrap().uid, uid);
    assert_eq!(request(&mut h, &tx, "cell.get", json!({"cell":"A1"}))["value"], "Keep me");
    assert_eq!(request(&mut h, &tx, "c0.openFile", params), json!({"pending":true}));
    h.run_steps(3);
    h.get_by_label("Don't Save").click();
    h.run_steps(4);
    assert_ne!(h.state().session.active().unwrap().uid, uid);
    assert_eq!(h.state().session.documents().len(), 1);
    assert_eq!(request(&mut h, &tx, "cell.get", json!({"cell":"A1"}))["value"], "Replacement");
}

#[test]
fn dirty_new_document_uses_the_same_cancel_and_discard_prompt() {
    let (mut h, tx) = harness();
    request(&mut h, &tx, "cell.set", json!({"cell":"A1","input":"42"}));
    let uid = h.state().session.active().unwrap().uid;
    assert_eq!(request(&mut h, &tx, "c0.newDocument", json!({})), json!({"pending":true}));
    request(&mut h, &tx, "c0.cancelFileDialog", json!({}));
    assert_eq!(h.state().dialog.as_ref().unwrap().name, "saveChanges");
    request(&mut h, &tx, "ui.dialog.cancel", json!({}));
    assert_eq!(h.state().session.active().unwrap().uid, uid);
    request(&mut h, &tx, "c0.newDocument", json!({}));
    h.run_steps(3);
    h.get_by_label("Don't Save").click();
    h.run_steps(4);
    assert_ne!(h.state().session.active().unwrap().uid, uid);
    assert!(!h.state().session.active().unwrap().is_dirty());
    assert!(h.state().session.active().unwrap().wb.sheets.iter().all(|s| s.cells.is_empty()));
}

#[test]
fn existing_file_can_be_saved_before_the_replacement_continues() {
    let (mut h, tx) = harness();
    let path = std::env::temp_dir().join(format!("c0sheet-save-{}.xlsx", std::process::id()));
    request(&mut h, &tx, "cell.set", json!({"cell":"A1","input":"42"}));
    h.state_mut().session.active_mut().unwrap().path = Some(path.to_string_lossy().into_owned());
    request(&mut h, &tx, "c0.newDocument", json!({}));
    h.run_steps(3);
    h.get_by_label("Save").click();
    h.run_steps(4);
    let (saved, _) = io::open_bytes("saved.xlsx", &std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved.active().unwrap().value(gridcraft_engine::core::CellRef::default()), gridcraft_engine::core::Value::Number(42.0));
    assert!(h.state().session.active().unwrap().wb.active().unwrap().cells.is_empty());
    std::fs::remove_file(path).unwrap();
}
