//! Host layout switches: the ribbon's tool modes and the folding right dock.
//!
//! The ribbon is GridCraft's tool strip, so the PhotoCraft tool-mode keys map onto it:
//!
//! | keys | ribbon |
//! |---|---|
//! | `toolbar` (default true) | the command card under the tabs; false collapses to the tab row |
//! | `toolbar_double` | compact card: big buttons become icon buttons, so the ribbon is narrower |
//! | `toolbar_floating` | the whole ribbon is a movable palette over the canvas, not a docked panel |
//!
//! Everything here is host description, like [`crate::Chrome`]: never persisted, and the defaults
//! draw upstream's ribbon and dock unchanged.

use egui::{Color32, Id, Order, Pos2, Rect, Sense, Stroke, Ui, vec2};

use crate::SheetApp;
use crate::theme::Tokens;

/// What the host asked of the ribbon and the right dock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    /// The ribbon's command card is shown (false: tabs only).
    pub toolbar: bool,
    /// The compact card.
    pub toolbar_double: bool,
    /// The ribbon floats over the canvas.
    pub toolbar_floating: bool,
    /// The floating ribbon's corner, in whole points from the canvas corner; `None`: the default.
    pub float_at: Option<(i32, i32)>,
    /// The right dock is folded to its icon strip.
    pub right_dock_collapsed: bool,
}

impl Default for Layout {
    fn default() -> Self {
        Layout { toolbar: true, toolbar_double: false, toolbar_floating: false, float_at: None, right_dock_collapsed: false }
    }
}

/// The floating ribbon's offset from the canvas corner before it was dragged.
/// The ribbon card's height, as upstream draws it.
const FULL_CARD: f32 = 88.0;
const DEFAULT_AT: (i32, i32) = (12, 12);
/// It stays this far inside the canvas.
const EDGE: f32 = 6.0;
const GRIP: f32 = 12.0;
/// A compact button's hover chip.
const CHIP: f32 = 30.0;
const MAX_WIDTH: f32 = 980.0;
/// The tab row of upstream's ribbon panel.
const TABS: f32 = 34.0;

fn compact_id() -> Id {
    Id::unique("c0-ribbon-compact")
}

fn compact(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(compact_id())).unwrap_or(false)
}

/// In the compact ribbon a big button is an icon button: the same 72 pt slot, so the groups keep
/// their height, but only as wide as its icon, with the label as tooltip and accessible name.
pub(crate) fn compact_button(ui: &mut Ui, icon: crate::icons::Icon, label: &str, tip: &str, dropdown: bool) -> Option<egui::Response> {
    if !compact(ui.ctx()) {
        return None;
    }
    let t = Tokens::get(ui.ctx());
    let chevron = if dropdown { 12.0 } else { 0.0 };
    let (slot, resp) = ui.allocate_exact_size(vec2(CHIP + chevron, 72.0), Sense::click());
    let chip = Rect::from_center_size(slot.center() - vec2(chevron / 2.0, 0.0), vec2(CHIP, CHIP));
    if resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(chip, 5.0, t.pressed);
    } else if resp.hovered() {
        ui.painter().rect_filled(chip, 5.0, t.hover);
    }
    crate::icons::paint(ui.painter(), Rect::from_center_size(chip.center(), vec2(22.0, 22.0)), icon, t.text);
    if dropdown {
        let at = Pos2::new(chip.right() + 2.0, chip.center().y + 1.0);
        crate::icons::paint(ui.painter(), Rect::from_center_size(at, vec2(10.0, 10.0)), crate::icons::Icon::Chevron, t.text_dim);
    }
    let name = label.replace('\n', " ");
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &name));
    Some(resp.on_hover_text(crate::i18n::current(ui.ctx()).tr(tip)))
}

/// The ribbon, docked as upstream draws it or floating, in the host's density.
pub(crate) fn ribbon(app: &mut SheetApp, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    ctx.data_mut(|d| d.insert_temp(compact_id(), app.chrome.layout.toolbar_double));
    if app.chrome.layout.toolbar_floating {
        float(app, ui);
    } else {
        crate::ribbon::show(app, ui);
    }
    ctx.data_mut(|d| d.insert_temp(compact_id(), false));
}

/// The floating ribbon's top-left corner: its stored offset, kept inside the canvas.
fn place(layout: &Layout, canvas: Rect, size: egui::Vec2) -> Pos2 {
    let (dx, dy) = layout.float_at.unwrap_or(DEFAULT_AT);
    let max_x = (canvas.right() - size.x - EDGE).max(canvas.left() + EDGE);
    let max_y = (canvas.bottom() - size.y - EDGE).max(canvas.top() + EDGE);
    Pos2::new((canvas.left() + dx as f32).clamp(canvas.left() + EDGE, max_x), (canvas.top() + dy as f32).clamp(canvas.top() + EDGE, max_y))
}

fn float(app: &mut SheetApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    // The canvas is the grid area as last laid out, so the palette never covers the formula bar.
    let canvas = app.grid.rect.unwrap_or_else(|| ui.available_rect_before_wrap());
    let width = (canvas.width() - 2.0 * EDGE).clamp(320.0, MAX_WIDTH);
    let card = if app.ui.ribbon_collapsed || !app.chrome.layout.toolbar { 0.0 } else { FULL_CARD + 4.0 };
    let size = vec2(width, GRIP + TABS + card);
    let at = place(&app.chrome.layout, canvas, size);
    let frame = egui::Frame::NONE.fill(t.window).stroke(Stroke::new(1.0, t.ribbon_border)).corner_radius(10.0).shadow(egui::Shadow {
        offset: [0, 4],
        blur: 14,
        spread: 0,
        color: t.shadow,
    });
    let mut moved = vec2(0.0, 0.0);
    egui::Area::new(Id::unique("c0-floating-ribbon")).order(Order::Foreground).fixed_pos(at).show(ui.ctx(), |ui| {
        frame.show(ui, |ui| {
            ui.set_width(width);
            let (rect, grip) = ui.allocate_exact_size(vec2(width, GRIP), Sense::drag());
            grip.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Move Toolbar"));
            let tone = if grip.hovered() || grip.dragged() { t.text } else { t.text_dim };
            ui.painter().rect_filled(Rect::from_center_size(rect.center(), vec2(36.0, 4.0)), 2.0, tone);
            moved = grip.drag_delta();
            ui.set_height(TABS + card);
            crate::ribbon::show(app, ui);
        });
    });
    if moved != vec2(0.0, 0.0) {
        let (x, y) = app.chrome.layout.float_at.unwrap_or(DEFAULT_AT);
        let next = (x.saturating_add(moved.x.round() as i32), y.saturating_add(moved.y.round() as i32));
        let kept = place(&Layout { float_at: Some(next), ..app.chrome.layout }, canvas, size);
        app.chrome.layout.float_at = Some(((kept.x - canvas.left()).round() as i32, (kept.y - canvas.top()).round() as i32));
    }
}

/// The panes the right dock holds: their command name, icon and label.
const DOCK_PANES: [(&str, crate::icons::Icon, &str); 4] = [
    ("comments", crate::icons::Icon::Comment, "Comments"),
    ("watch", crate::icons::Icon::Search, "Watch Window"),
    ("selection", crate::icons::Icon::Shapes, "Selection"),
    ("formatChart", crate::icons::Icon::Chart, "Format Chart Area"),
];
const DOCK_STRIP: f32 = 34.0;

/// Which dock pane is open.
fn open_pane(app: &SheetApp) -> Option<String> {
    app.grid.pane.clone().or_else(|| app.session.comments_pane.then(|| "comments".to_string()))
}

/// Whether the dock itself is drawn. Folded: the narrow icon strip takes its place and this is false.
/// An icon opens its pane, unfolding the dock; the chevron unfolds it as it was.
pub(crate) fn dock_open(app: &mut SheetApp, ui: &mut Ui) -> bool {
    if !app.chrome.layout.right_dock_collapsed {
        return true;
    }
    let t = Tokens::get(ui.ctx());
    let open = open_pane(app);
    let pivot = crate::pivot_pane::active_pivot(app).is_some();
    let mut chosen: Option<Option<&str>> = None;
    egui::Panel::right("c0-dock-strip").exact_size(DOCK_STRIP).resizable(false).frame(egui::Frame::NONE.fill(t.ribbon).inner_margin(4)).show(
        ui,
        |ui| {
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                let expand = crate::widgets::icon_button(ui, crate::icons::Icon::Left, t.text_dim, "Expand panels", vec2(26.0, 26.0));
                expand.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Expand panels"));
                if expand.clicked() {
                    chosen = Some(None);
                }
                for (name, icon, label) in DOCK_PANES {
                    let on = open.as_deref() == Some(name);
                    let resp = crate::widgets::toggle_button(ui, icon, on, label);
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
                    if resp.clicked() {
                        chosen = Some(Some(name));
                    }
                }
                if pivot {
                    let resp = crate::widgets::toggle_button(ui, crate::icons::Icon::PivotTable, true, "PivotTable Fields");
                    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "PivotTable Fields"));
                    if resp.clicked() {
                        chosen = Some(None);
                    }
                }
            });
        },
    );
    if let Some(pane) = chosen {
        app.chrome.layout.right_dock_collapsed = false;
        // Comments included: the pane code turns `grid.pane == "comments"` into the engine's switch.
        if let Some(name) = pane {
            app.grid.pane = Some(name.to_string());
        }
    }
    false
}

/// The task pane's header button that folds the dock to its icon strip.
pub(crate) fn fold_button(app: &mut SheetApp, ui: &mut Ui) {
    let color: Color32 = Tokens::get(ui.ctx()).text_dim;
    let fold = crate::widgets::icon_button(ui, crate::icons::Icon::Right, color, "Fold panels", vec2(22.0, 22.0));
    fold.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Fold panels"));
    if fold.clicked() {
        app.chrome.layout.right_dock_collapsed = true;
    }
}
