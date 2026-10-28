//! `ui.render {sheet?, scale?}`: a PNG of the visible grid without a window frame.
//!
//! `ui.screenshot` needs presented pixels, so a hidden guest cannot answer it. This draws the
//! grid with the app's own painters (cells, borders, gridlines, headings, objects) on a private
//! egui context, tessellates that pass, and rasterises it on the CPU: no window, no GPU, no
//! presented frame, no ribbon, panes or tabs, and none of the open app's state is touched.

use gridcraft_engine::{Selection, model::Visibility};
use serde_json::{Value, json};

use super::c0_raster::{Canvas, Textures, draw};
use crate::SheetApp;
use crate::grid::Geo;
use crate::theme::Tokens;

/// The grid area when the app has not laid one out yet (a guest that was never shown).
const DEFAULT_SIZE: egui::Vec2 = egui::vec2(960.0, 600.0);
const MAX_PIXELS: f32 = 12_000_000.0;

fn sheet_index(app: &SheetApp, params: &Value) -> Result<usize, String> {
    let wb = &app.session.active().ok_or("no workbook open")?.wb;
    let index = match params.get("sheet") {
        None | Some(Value::Null) => wb.active_sheet.get(),
        Some(Value::String(name)) => wb.sheet_index(name).ok_or("no sheet with that name")?,
        Some(value) => value.as_u64().and_then(|i| usize::try_from(i).ok()).ok_or("sheet must be a name or zero-based index")?,
    };
    let sheet = wb.sheet(index).ok_or("no such sheet")?;
    if sheet.visibility != Visibility::Visible {
        return Err("sheet is hidden".into());
    }
    Ok(index)
}

fn scale(params: &Value) -> Result<f32, String> {
    match params.get("scale") {
        None | Some(Value::Null) => Ok(1.0),
        Some(value) => {
            value.as_f64().filter(|s| (0.25..=4.0).contains(s)).map(|s| s as f32).ok_or_else(|| "scale must be a number from 0.25 to 4".to_string())
        }
    }
}

pub(super) fn render(app: &SheetApp, ctx: &egui::Context, params: &Value) -> Result<Value, String> {
    let si = sheet_index(app, params)?;
    let scale = scale(params)?;
    let d = app.session.active().ok_or("no workbook open")?;
    let wb = d.wb.clone();
    let sh = wb.sheet(si).ok_or("no such sheet")?;
    // The area the grid occupies on screen, so the picture is what a user sees there.
    let size = app.grid.rect.map(|r| r.size()).filter(|s| s.x >= 1.0 && s.y >= 1.0).unwrap_or(DEFAULT_SIZE);
    let (width, height) = ((size.x * scale).round().max(1.0), (size.y * scale).round().max(1.0));
    if width * height > MAX_PIXELS {
        return Err("render exceeds 12 million pixels; lower the scale".into());
    }
    let scroll = app.views.get(&(d.uid, si)).map(|v| v.scroll).unwrap_or_default().max(egui::Vec2::ZERO);
    let sel = if si == wb.active_sheet.get() {
        d.selection.clone()
    } else {
        d.sheet_selections.get(si).cloned().unwrap_or_else(|| Selection::at(sh.view_active))
    };

    // A private context: the open app's fonts and appearance, none of its frame state.
    let private = egui::Context::default();
    // The app's own fonts once it has bound them; before its first frame, the built-in roles.
    let fonts = if app.fonts_ready { ctx.fonts(|f| f.definitions().clone()) } else { crate::theme::font_definitions_for_language(app.ui.language) };
    private.set_fonts(fonts);
    crate::theme::apply(&private, ctx.global_style().visuals.dark_mode);
    private.set_pixels_per_point(scale);
    let t = Tokens::get(ctx);

    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
    let input = egui::RawInput { screen_rect: Some(rect), ..Default::default() };
    let mut output = private.run_ui(input, |ui| {
        let geo = Geo::new(sh, rect, scroll);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, t.grid_bg);
        let outline = geo.range_rect(sh, sel.current());
        let mut cf = gridcraft_engine::cf::CfCache::new();
        let quads = crate::grid::paint_panes(app, &painter, &geo, &wb, si, sh, &sel, outline, &mut cf, &t);
        crate::chartview::paint_objects(app, &painter.with_clip_rect(geo.cells), &geo, &wb, si, sh);
        if sh.show_headings {
            let headings = crate::grid::table_headings(&geo, sh, sel.active);
            crate::grid::paint_headers(&painter, &geo, sh, &sel, &t, &quads, &headings);
        }
        crate::outline::paint(&painter, &geo, sh, &t);
    });

    let mut textures = Textures::default();
    textures.apply(output.textures_delta.set.iter().map(|(id, deltas)| (id, deltas.as_slice())));
    output.textures_delta.clear();
    let primitives = private.tessellate(output.shapes, output.pixels_per_point);
    let mut canvas = Canvas::new(width as usize, height as usize, [255, 255, 255, 255]);
    draw(&mut canvas, &primitives, &textures, output.pixels_per_point);

    let image = image::RgbaImage::from_raw(width as u32, height as u32, canvas.rgba()).ok_or("the render buffer was malformed")?;
    let mut png = Vec::new();
    image::DynamicImage::ImageRgba8(image).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).map_err(|e| e.to_string())?;
    Ok(json!({"base64": gridcraft_engine::io::base64_encode(&png), "width": width as u32, "height": height as u32, "sheet": sh.name, "scale": scale}))
}
