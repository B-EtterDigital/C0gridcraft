//! `c0.documentBytes {format:"pdf"}`: the engine's PDF writer, read-only, without a file system.

use gridcraft_engine::{Session, io};
use gridcraft_ui_egui::{
    Services, SheetApp,
    control::{self, Outcome},
};
use serde_json::{Value, json};

fn app() -> SheetApp {
    let mut session = Session::new();
    session.new_workbook();
    SheetApp::new(session, Services::default())
}

fn call(app: &mut SheetApp, params: Value) -> Value {
    let ctx = egui::Context::default();
    let Outcome::Done(reply) = control::handle(app, &ctx, "c0.documentBytes", &params) else { panic!("deferred") };
    reply
}

fn pdf(app: &mut SheetApp, params: Value) -> (Value, Vec<u8>) {
    let reply = call(app, params);
    assert_eq!(reply["ok"], true, "{reply}");
    let result = reply["result"].clone();
    let bytes = io::base64_decode(result["bytes"].as_str().unwrap()).expect("standard base64");
    (result, bytes)
}

#[test]
fn pdf_is_a_real_document_with_the_workbook_text() {
    let mut app = app();
    app.run("cell.set", json!({"cell": "A1", "input": "Quarterly totals"})).unwrap();
    app.run("cell.set", json!({"cell": "B2", "input": "=6*7"})).unwrap();
    let (result, bytes) = pdf(&mut app, json!({"format": "pdf"}));
    assert!(bytes.starts_with(b"%PDF-"), "{:?}", &bytes[..bytes.len().min(8)]);
    assert!(bytes.windows(5).any(|w| w == b"%%EOF"));
    assert_eq!(result["format"], "pdf");
    assert!(result["name"].as_str().unwrap().ends_with(".pdf"));
    assert!(result["pages"].as_u64().unwrap() >= 1);
    assert_eq!(result["revision"], app.session.active().unwrap().revision);
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("Quarterly totals") && text.contains("42"), "the sheet's text is in the page content");
}

#[test]
fn exporting_leaves_the_open_document_exactly_as_it_was() {
    let mut app = app();
    app.run("cell.set", json!({"cell": "A1", "input": "x"})).unwrap();
    let state = |app: &SheetApp| {
        let d = app.session.active().unwrap();
        (d.revision, d.is_dirty(), d.undo.len(), d.redo.len(), d.selection.a1(), app.session.documents().len(), d.uid)
    };
    let before = state(&app);
    let journal = app.session.run("engine.journal", json!({})).ok().map(|j| j.to_string());
    pdf(&mut app, json!({"format": "pdf"}));
    pdf(&mut app, json!({"format": "pdf", "sheets": "all"}));
    assert_eq!(state(&app), before);
    assert_eq!(app.session.run("engine.journal", json!({})).ok().map(|j| j.to_string()), journal);
}

#[test]
fn engine_print_options_pass_through_and_bad_ones_are_refused() {
    let mut app = app();
    for (cell, input) in [("A1", "alpha"), ("C9", "omega")] {
        app.run("cell.set", json!({"cell": cell, "input": input})).unwrap();
    }
    let (_, whole) = pdf(&mut app, json!({"format": "pdf"}));
    let (_, part) = pdf(&mut app, json!({"format": "pdf", "range": "A1:B2"}));
    assert!(String::from_utf8_lossy(&whole).contains("omega"));
    assert!(!String::from_utf8_lossy(&part).contains("omega"));
    assert!(String::from_utf8_lossy(&part).contains("alpha"));
    let bad = call(&mut app, json!({"format": "pdf", "sheets": "bogus"}));
    assert_eq!(bad["ok"], false, "{bad}");
    let bad = call(&mut app, json!({"format": "pdf", "range": "not a range"}));
    assert_eq!(bad["ok"], false, "{bad}");
}

#[test]
fn the_other_formats_and_their_errors_are_unchanged() {
    let mut app = app();
    for format in ["xlsx", "csv"] {
        let reply = call(&mut app, json!({"format": format}));
        assert_eq!(reply["result"]["format"], format, "{reply}");
    }
    assert_eq!(call(&mut app, json!({}))["result"]["format"], "xlsx");
    for format in [json!("docx"), json!("PDF"), json!(3), Value::Null] {
        let reply = call(&mut app, json!({ "format": format }));
        assert_eq!(reply["ok"], false);
        assert_eq!(reply["error"], "format must be xlsx, csv or pdf");
    }
}

#[test]
fn an_empty_workbook_answers_with_an_error_or_a_valid_pdf_never_a_crash() {
    let mut app = app();
    let reply = call(&mut app, json!({"format": "pdf"}));
    if reply["ok"] == true {
        let bytes = io::base64_decode(reply["result"]["bytes"].as_str().unwrap()).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
    } else {
        assert!(reply["error"].as_str().is_some_and(|e| !e.is_empty()));
    }
}
