//! Native coverage of the guest's actual control and engine paths.

use std::sync::Arc;

use gridcraft_engine::{Session, io, model::Visibility};
use gridcraft_ui_egui::{
    Services, SheetApp, c0_host,
    control::{self, Outcome},
};
use serde_json::{Value, json};

fn app() -> SheetApp {
    let mut session = Session::new();
    session.new_workbook();
    SheetApp::new(session, Services::default())
}

fn reply(app: &mut SheetApp, ctx: &egui::Context, method: &str, params: Value) -> Value {
    let Outcome::Done(reply) = control::handle(app, ctx, method, &params) else { panic!("unexpected deferred response") };
    reply
}

fn ok(app: &mut SheetApp, ctx: &egui::Context, method: &str, params: Value) -> Value {
    let reply = reply(app, ctx, method, params);
    assert_eq!(reply["ok"], true, "{method}: {reply}");
    assert_eq!(reply.as_object().unwrap().len(), 2);
    reply["result"].clone()
}

#[test]
fn inspect_has_the_complete_stable_shape_and_real_revision() {
    let mut app = app();
    let ctx = egui::Context::default();
    let result = ok(&mut app, &ctx, "c0.inspect", json!({}));
    let keys = [
        "title",
        "dirty",
        "revision",
        "sheets",
        "activeSheet",
        "activeCell",
        "selection",
        "names",
        "tables",
        "canUndo",
        "canRedo",
        "panes",
        "ribbonCollapsed",
        "theme",
    ];
    assert_eq!(result.as_object().unwrap().len(), keys.len());
    for key in keys {
        assert!(!result[key].is_null(), "missing {key}");
    }
    assert_eq!(result["dirty"], false);
    assert_eq!(result["activeCell"], "A1");
    assert_eq!(result["theme"], "light");
    assert_eq!(result["revision"], app.session.active().unwrap().revision);
    app.run("cell.set", json!({"cell":"B3", "input":"42"})).unwrap();
    let changed = ok(&mut app, &ctx, "c0.inspect", json!({}));
    assert!(changed["revision"].as_u64().unwrap() > result["revision"].as_u64().unwrap());
    assert_eq!(changed["dirty"], true);
    assert_eq!(changed["canUndo"], true);
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(ok(&mut app, &ctx, "c0.inspect", json!({}))["canRedo"], true);
}

#[test]
fn inspect_lists_sheets_names_and_tables_across_the_workbook() {
    let mut app = app();
    let ctx = egui::Context::default();
    app.run("cell.set", json!({"cell":"A1","input":"Item"})).unwrap();
    app.run("insert.table", json!({"range":"A1:B3", "name":"Sales", "header":true})).unwrap();
    app.run("formulas.defineName", json!({"name":"Rate", "refersTo":"=Sheet1!$A$2"})).unwrap();
    app.run("home.insertSheet", json!({"name":"Other"})).unwrap();
    let wb = Arc::make_mut(&mut app.session.active_mut().unwrap().wb);
    Arc::make_mut(wb.sheets.get_mut(0).unwrap()).visibility = Visibility::VeryHidden;
    let result = ok(&mut app, &ctx, "c0.inspect", json!({}));
    assert_eq!(result["sheets"].as_array().unwrap().len(), 2);
    assert!(result["sheets"].as_array().unwrap().iter().any(|s| s["hidden"] == true && s["active"] == false));
    assert!(result["sheets"].as_array().unwrap().iter().any(|s| s["name"] == "Other" && s["active"] == true));
    assert_eq!(result["names"][0], json!({"name":"Rate", "ref":"=Sheet1!$A$2"}));
    assert_eq!(result["tables"][0], json!({"name":"Sales", "sheet":"Sheet1", "ref":"A1:B3"}));
}

#[test]
fn go_to_activates_selects_and_requests_reveal_without_dirtying() {
    let mut app = app();
    let ctx = egui::Context::default();
    app.run("home.insertSheet", json!({"name":"Data"})).unwrap();
    let d = app.session.active_mut().unwrap();
    d.saved = d.wb.clone();
    ok(&mut app, &ctx, "c0.goTo", json!({"sheet":"Sheet1", "ref":"$D$800:$F$802"}));
    let result = ok(&mut app, &ctx, "c0.inspect", json!({}));
    assert_eq!(result["activeSheet"], 0);
    assert_eq!(result["selection"], "D800:F802");
    assert_eq!(result["activeCell"], "D800");
    assert_eq!(result["dirty"], false);
    assert!(app.grid.ensure_visible);
    ok(&mut app, &ctx, "c0.goTo", json!({"sheet":1}));
    assert_eq!(app.session.active().unwrap().wb.active_sheet, 1);
    ok(&mut app, &ctx, "c0.goTo", json!({}));
}

#[test]
fn bad_navigation_is_atomic_and_bounded() {
    let mut app = app();
    let ctx = egui::Context::default();
    app.run("home.insertSheet", json!({"name":"Data"})).unwrap();
    for params in [
        json!({"sheet":0,"ref":"bad"}),
        json!({"sheet":"missing"}),
        json!({"sheet":u64::MAX}),
        json!({"sheet":true}),
        json!({"ref":3}),
        json!({"ref":"XFE1048577"}),
    ] {
        let result = reply(&mut app, &ctx, "c0.goTo", params);
        assert_eq!(result["ok"], false, "{result}");
        assert!(result["error"].is_string());
        assert_eq!(app.session.active().unwrap().wb.active_sheet, 1);
    }
}

#[test]
fn snapshots_are_read_only_and_reopen_with_both_formats() {
    for format in ["xlsx", "csv"] {
        let mut app = app();
        let ctx = egui::Context::default();
        app.run("cell.set", json!({"cell":"A1","input":"42"})).unwrap();
        app.run("cell.set", json!({"cell":"B1","input":"hello"})).unwrap();
        let d = app.session.active().unwrap();
        let (wb, saved, rev, undo, journal) = (d.wb.clone(), d.saved.clone(), d.revision, d.undo.len(), app.session.journal.len());
        app.begin_edit(Some("uncommitted".into()), false);
        let snapshot = ok(&mut app, &ctx, "c0.documentBytes", json!({"format":format}));
        assert_eq!(snapshot.as_object().unwrap().len(), 4);
        assert_eq!(snapshot["format"], format);
        assert_eq!(snapshot["revision"], rev);
        let d = app.session.active().unwrap();
        assert!(Arc::ptr_eq(&wb, &d.wb));
        assert!(Arc::ptr_eq(&saved, &d.saved));
        assert_eq!(d.undo.len(), undo);
        assert_eq!(app.session.journal.len(), journal);
        assert!(app.editor.is_some());
        let mut target = self::app();
        ok(&mut target, &ctx, "c0.openFile", json!({"name":snapshot["name"], "bytes":snapshot["bytes"]}));
        assert_eq!(target.session.documents().len(), 1);
        assert_eq!(ok(&mut target, &ctx, "cell.get", json!({"cell":"A1"}))["value"].as_f64(), Some(42.0));
        assert_eq!(ok(&mut target, &ctx, "cell.get", json!({"cell":"B1"}))["value"], "hello");
    }
}

#[test]
fn default_snapshot_matches_file_save_bytes_exactly() {
    let mut app = app();
    let ctx = egui::Context::default();
    let snapshot = ok(&mut app, &ctx, "c0.documentBytes", json!({}));
    let engine = app.session.run("file.saveBytes", json!({})).unwrap();
    assert_eq!(snapshot["format"], "xlsx");
    assert_eq!(snapshot["name"], engine["name"]);
    let (wb, _) = io::open_bytes("snapshot.xlsx", &io::base64_decode(snapshot["bytes"].as_str().unwrap()).unwrap()).unwrap();
    let (reference, _) = io::open_bytes("reference.xlsx", &io::base64_decode(engine["base64"].as_str().unwrap()).unwrap()).unwrap();
    assert_eq!(wb.sheets.len(), reference.sheets.len());
    assert_eq!(wb.active_sheet, reference.active_sheet);
}

#[test]
fn open_uses_the_engine_importers_for_text_and_json() {
    let source = app();
    let ctx = egui::Context::default();
    for (name, bytes) in [
        ("book.tsv", b"A\tB\n1\t2\n".to_vec()),
        ("book.txt", b"A\tB\n1\t2\n".to_vec()),
        ("book.csv", b"A,B\n1,2\n".to_vec()),
        ("book.json", io::save_bytes(&source.session.active().unwrap().wb, "book.json").unwrap()),
    ] {
        let mut target = app();
        ok(&mut target, &ctx, "c0.openFile", json!({"name":name,"bytes":io::base64_encode(&bytes)}));
        assert_eq!(target.session.documents().len(), 1);
        assert!(target.dialog.is_none());
    }
}

#[test]
fn invalid_imports_and_formats_leave_workbook_and_picker_intact() {
    let mut app = app();
    let ctx = egui::Context::default();
    let original = app.session.active().unwrap().uid;
    let pending = c0_host::begin_picker(&ctx);
    for params in [
        json!({}),
        json!({"name":"bad.xlsx","bytes":"not base64!"}),
        json!({"name":"bad.xlsx","bytes":"AQIDBA=="}),
        json!({"name":"../book.csv","bytes":""}),
        json!({"name":"book.csv","bytes":2}),
    ] {
        assert_eq!(reply(&mut app, &ctx, "c0.openFile", params)["ok"], false);
        assert_eq!(app.session.active().unwrap().uid, original);
        assert!(app.dialog.is_none());
        assert!(pending.load(std::sync::atomic::Ordering::Relaxed));
    }
    for format in [json!("pdf"), json!(3), Value::Null] {
        assert_eq!(reply(&mut app, &ctx, "c0.documentBytes", json!({"format":format}))["ok"], false);
    }
}

#[test]
fn new_document_is_blank_and_retires_the_picker() {
    let mut app = app();
    let ctx = egui::Context::default();
    let original = app.session.active().unwrap().uid;
    let pending = c0_host::begin_picker(&ctx);
    ok(&mut app, &ctx, "c0.newDocument", json!({"template":"blank"}));
    assert_ne!(app.session.active().unwrap().uid, original);
    assert_eq!(app.session.documents().len(), 1);
    assert!(!app.session.active().unwrap().is_dirty());
    assert!(app.session.active().unwrap().wb.sheets.iter().all(|s| s.cells.is_empty()));
    assert!(!pending.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(reply(&mut app, &ctx, "c0.newDocument", json!({"template":"missing"}))["ok"], false);
}

#[test]
fn cancellation_is_idempotent_and_does_not_answer_save_changes() {
    let mut app = app();
    let ctx = egui::Context::default();
    let first = c0_host::begin_picker(&ctx);
    let second = c0_host::begin_picker(&ctx);
    assert!(!first.load(std::sync::atomic::Ordering::Relaxed));
    assert!(c0_host::picker_pending(&ctx));
    app.open_dialog("saveChanges", json!({}));
    assert_eq!(ok(&mut app, &ctx, "c0.cancelFileDialog", json!({})), json!({"cancelled":true}));
    assert!(!second.load(std::sync::atomic::Ordering::Relaxed));
    assert_eq!(app.dialog.as_ref().unwrap().name, "saveChanges");
    assert_eq!(ok(&mut app, &ctx, "c0.cancelFileDialog", json!({})), json!({"cancelled":false}));
}

#[test]
fn errors_have_the_same_envelope_for_every_host_method() {
    let mut app = app();
    let ctx = egui::Context::default();
    for method in c0_host::METHODS.iter().copied().chain(["c0.unknown"]) {
        let result = reply(&mut app, &ctx, method, json!(3));
        assert_eq!(result["ok"], false);
        assert!(result["error"].is_string());
        assert_eq!(result.as_object().unwrap().len(), 2);
    }
    assert_eq!(reply(&mut app, &ctx, "c0.unknown", json!({}))["ok"], false);
}

#[test]
fn encrypted_import_is_rejected_before_the_unsaved_changes_prompt() {
    let mut app = app();
    let ctx = egui::Context::default();
    app.run("cell.set", json!({"cell":"A1","input":"Keep me"})).unwrap();
    let uid = app.session.active().unwrap().uid;
    let plain = io::save_bytes(&app.session.active().unwrap().wb, "book.xlsx").unwrap();
    let encrypted = gridcraft_xlsx::testing::encrypt_workbook(&plain, "secret");
    let result = reply(&mut app, &ctx, "c0.openFile", json!({"name":"secret.xlsx","bytes":io::base64_encode(&encrypted)}));
    assert_eq!(result["ok"], false);
    assert_eq!(app.session.active().unwrap().uid, uid);
    assert!(app.session.active().unwrap().is_dirty());
    assert!(app.dialog.is_none());
}

#[test]
fn a_replacement_does_not_touch_an_unrelated_open_dialog() {
    let mut app = app();
    let ctx = egui::Context::default();
    app.open_dialog("about", json!({}));
    let uid = app.session.active().unwrap().uid;
    let result = reply(&mut app, &ctx, "c0.newDocument", json!({}));
    assert_eq!(result["ok"], false);
    assert_eq!(app.dialog.as_ref().unwrap().name, "about");
    assert_eq!(app.session.active().unwrap().uid, uid);
}

#[test]
fn invalid_drafts_return_errors_without_opening_a_message_box_or_losing_the_draft() {
    for (method, params) in [
        ("c0.goTo", json!({"ref":"B3"})),
        ("c0.newDocument", json!({})),
        ("c0.openFile", json!({"name":"host.csv","bytes":io::base64_encode(b"42\n")})),
    ] {
        let mut app = app();
        let ctx = egui::Context::default();
        let uid = app.session.active().unwrap().uid;
        app.begin_edit(Some("=(".into()), false);
        assert_eq!(reply(&mut app, &ctx, method, params)["ok"], false, "{method}");
        assert!(app.message.is_none(), "{method} must return the error, not open a message box");
        assert!(app.dialog.is_none());
        assert_eq!(app.editor.as_ref().unwrap().text, "=(");
        assert_eq!(app.session.active().unwrap().uid, uid);
        assert_eq!(app.session.active().unwrap().selection.active.a1(), "A1");
    }
}

#[test]
fn legacy_import_warnings_are_returned_without_a_message_box() {
    // A synthetic BIFF8 Workbook stream inside the existing test-support CFB writer.
    // All records are generated here; no vendor-authored workbook is used.
    fn record(id: u16, body: &[u8]) -> Vec<u8> {
        let mut bytes = id.to_le_bytes().to_vec();
        bytes.extend_from_slice(&(body.len() as u16).to_le_bytes());
        bytes.extend_from_slice(body);
        bytes
    }
    let bof = |kind: u16| {
        let mut body = [0u8; 16];
        body.get_mut(..2).unwrap().copy_from_slice(&0x0600u16.to_le_bytes());
        body.get_mut(2..4).unwrap().copy_from_slice(&kind.to_le_bytes());
        record(0x0809, &body)
    };
    let globals = bof(0x0005);
    let eof = record(0x000A, &[]);
    let mut bound = vec![0, 0, 0, 0, 0, 0, 6, 0];
    bound.extend_from_slice(b"Sheet1");
    let sheet_offset = (globals.len() + 4 + bound.len() + eof.len()) as u32;
    bound.get_mut(..4).unwrap().copy_from_slice(&sheet_offset.to_le_bytes());
    let mut stream = globals;
    stream.extend(record(0x0085, &bound));
    stream.extend_from_slice(&eof);
    stream.extend(bof(0x0010));
    let mut number = vec![0u8; 6]; // row 0, column 0, default format
    number.extend_from_slice(&42.0f64.to_le_bytes());
    stream.extend(record(0x0203, &number));
    stream.extend(eof);
    let bytes = gridcraft_xlsx::testing::compound_file(&[("Workbook", &stream)]);
    let mut app = app();
    let ctx = egui::Context::default();
    let result = ok(&mut app, &ctx, "c0.openFile", json!({"name":"legacy.xls","bytes":io::base64_encode(&bytes)}));
    assert!(!result["warnings"].as_array().unwrap().is_empty());
    assert!(app.message.is_none(), "warnings belong in the host reply and notice strip");
    assert!(app.dialog.is_none());
    assert!(app.notice.is_some());
    assert_eq!(ok(&mut app, &ctx, "cell.get", json!({"cell":"A1"}))["value"].as_f64(), Some(42.0));
}
