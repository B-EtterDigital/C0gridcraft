//! `ui.render {sheet?, scale?}`: a frameless PNG of the visible grid, drawn without a window.

use gridcraft_engine::Session;
use gridcraft_ui_egui::{
    Services, SheetApp,
    control::{self, Outcome},
    grid::Geo,
};
use serde_json::{Value, json};

fn app() -> SheetApp {
    let mut session = Session::new();
    session.new_workbook();
    SheetApp::new(session, Services::default())
}

fn call(app: &mut SheetApp, params: Value) -> Value {
    let ctx = egui::Context::default();
    let Outcome::Done(reply) = control::handle(app, &ctx, "ui.render", &params) else { panic!("deferred") };
    reply
}

fn render(app: &mut SheetApp, params: Value) -> (Value, image::RgbaImage) {
    let reply = call(app, params);
    assert_eq!(reply["ok"], true, "{reply}");
    let result = reply["result"].clone();
    let png = gridcraft_engine::io::base64_decode(result["base64"].as_str().unwrap()).expect("standard base64");
    assert!(png.starts_with(b"\x89PNG"));
    (result, image::load_from_memory(&png).expect("a decodable PNG").to_rgba8())
}

/// Pixels in `area` (points, at `scale`) that are noticeably darker than white.
fn ink(img: &image::RgbaImage, area: egui::Rect, scale: f32) -> usize {
    let (x0, y0) = ((area.left() * scale) as u32, (area.top() * scale) as u32);
    let (x1, y1) = (((area.right() * scale) as u32).min(img.width()), ((area.bottom() * scale) as u32).min(img.height()));
    (y0..y1).flat_map(|y| (x0..x1).map(move |x| (x, y))).filter(|&(x, y)| img.get_pixel(x, y).0[..3].iter().all(|&c| c < 120)).count()
}

fn cell_rect(app: &SheetApp, cell: &str) -> egui::Rect {
    let d = app.session.active().unwrap();
    let sh = d.wb.active().unwrap();
    let size = app.grid.rect.map(|r| r.size()).unwrap_or(egui::vec2(960.0, 600.0));
    Geo::new(sh, egui::Rect::from_min_size(egui::Pos2::ZERO, size), egui::Vec2::ZERO)
        .cell_rect(sh, gridcraft_engine::core::CellRef::parse(cell).unwrap())
}

#[test]
fn it_draws_the_grid_without_any_frame_ever_having_been_presented() {
    let mut app = app();
    assert!(app.grid.rect.is_none(), "no frame was presented");
    let (result, img) = render(&mut app, json!({}));
    // The grid area only: no ribbon, tabs or window around it.
    assert_eq!((img.width(), img.height()), (960, 600));
    assert_eq!((result["width"].as_u64(), result["height"].as_u64()), (Some(960), Some(600)));
    assert_eq!(result["sheet"], "Sheet1");
    // Gridlines and headings are drawn: the picture is not blank.
    let distinct: std::collections::HashSet<_> = img.pixels().map(|p| p.0).collect();
    assert!(distinct.len() > 3, "{}", distinct.len());
}

#[test]
fn cell_text_and_fill_appear_where_the_cell_is() {
    let mut app = app();
    let (_, empty) = render(&mut app, json!({}));
    app.run("cell.set", json!({"cell": "B2", "input": "WWWWWW"})).unwrap();
    let b2 = cell_rect(&app, "B2");
    let (_, texted) = render(&mut app, json!({}));
    assert_eq!(ink(&empty, b2, 1.0), 0);
    assert!(ink(&texted, b2, 1.0) > 20, "{}", ink(&texted, b2, 1.0));
    // A neighbour that was left alone stays blank.
    assert_eq!(ink(&texted, cell_rect(&app, "D6"), 1.0), 0);
    app.run("home.fillColor", json!({"range": "C3", "color": "#000000"})).unwrap();
    let (_, filled) = render(&mut app, json!({}));
    let c3 = cell_rect(&app, "C3");
    assert!(ink(&filled, c3.shrink(3.0), 1.0) as f32 > c3.shrink(3.0).area() * 0.9);
}

#[test]
fn scale_sets_the_pixel_size_and_the_sheet_param_picks_another_sheet() {
    let mut app = app();
    let (_, one) = render(&mut app, json!({"scale": 1}));
    let (result, two) = render(&mut app, json!({"scale": 2}));
    assert_eq!((two.width(), two.height()), (one.width() * 2, one.height() * 2));
    assert_eq!(result["scale"], 2.0);
    app.run("home.insertSheet", json!({"name": "Second"})).unwrap();
    app.run("sheet.activate", json!({"sheet": 0})).unwrap();
    app.run("cell.set", json!({"cell": "A1", "input": "only on the first"})).unwrap();
    let (named, second) = render(&mut app, json!({"sheet": "Second"}));
    assert_eq!(named["sheet"], "Second");
    let a1 = cell_rect(&app, "A1");
    assert_eq!(ink(&second, a1, 1.0), 0);
    let (_, first) = render(&mut app, json!({"sheet": 0}));
    assert!(ink(&first, a1, 1.0) > 0);
}

#[test]
fn bad_parameters_and_hidden_sheets_are_errors_and_change_nothing() {
    let mut app = app();
    app.run("home.insertSheet", json!({"name": "Back"})).unwrap();
    app.run("sheet.activate", json!({"sheet": 0})).unwrap();
    app.run("sheet.hide", json!({"sheet": "Back"})).unwrap();
    for params in [
        json!({"scale": 0}),
        json!({"scale": 9}),
        json!({"scale": "big"}),
        json!({"scale": -1}),
        json!({"sheet": "Nope"}),
        json!({"sheet": 99}),
        json!({"sheet": "Back"}),
        json!({"sheet": true}),
    ] {
        let reply = call(&mut app, params.clone());
        assert_eq!(reply["ok"], false, "{params}: {reply}");
    }
    let mut none = SheetApp::new(Session::new(), Services::default());
    assert_eq!(call(&mut none, json!({}))["ok"], false);
}

#[test]
fn rendering_leaves_the_open_app_exactly_as_it_was() {
    let mut app = app();
    app.run("cell.set", json!({"cell": "A1", "input": "x"})).unwrap();
    let state = |app: &SheetApp| {
        let d = app.session.active().unwrap();
        (d.revision, d.is_dirty(), d.undo.len(), d.selection.a1(), app.grid.rect, app.grid.ensure_visible, app.views.len(), app.message.clone())
    };
    let before = state(&app);
    render(&mut app, json!({"scale": 2}));
    render(&mut app, json!({"sheet": 0}));
    assert_eq!(state(&app), before);
}

#[test]
fn it_follows_the_grid_area_the_app_laid_out() {
    let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1000.0, 700.0)).build_ui_state(
        |ui, app: &mut SheetApp| {
            let ctx = ui.ctx().clone();
            app.logic(&ctx);
            app.ui(ui);
        },
        app(),
    );
    h.run_steps(4);
    let grid = h.state().grid.rect.expect("laid out");
    let ctx = h.ctx.clone();
    let Outcome::Done(reply) = control::handle(h.state_mut(), &ctx, "ui.render", &json!({})) else { panic!("deferred") };
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(reply["result"]["width"].as_f64().unwrap().round(), f64::from(grid.width().round()));
    assert_eq!(reply["result"]["height"].as_f64().unwrap().round(), f64::from(grid.height().round()));
    assert!(grid.width() < 1000.0 && grid.height() < 700.0, "frameless: smaller than the window");
}
