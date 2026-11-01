//! egui canvas: world/screen transform, grid, shape & edge painting, and all
//! pointer interactions (pan/zoom, node drag, marquee select, edge creation,
//! resize, palette drop, inline label editing).

use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Vec2,
    epaint::{PathShape, PathStroke},
};

use crate::diagram::EdgeStyle;

use super::model::{self, Editor, GuiEdge, GuiNode, snap};
use super::shapes::{self, ShapeKind};

const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 4.0;
const GRID_STEP: f32 = 16.0;
const PORT_HIT: f32 = 8.0;
const HANDLE: f32 = 9.0;

/// Pointer interaction in progress.
#[derive(Debug, Clone)]
enum Drag {
    Pan { last: Pos2 },
    Nodes { last: Pos2 },
    Marquee { start: Pos2 },
    Link { from: String },
    Resize { id: String, start_w: f32, start_h: f32, start: Pos2 },
}

/// Inline label editing state.
#[derive(Debug, Clone)]
pub struct LabelEdit {
    pub node_id: String,
    pub buffer: String,
}

/// Canvas view + interaction state (owned by the app).
#[derive(Debug, Clone)]
pub struct CanvasState {
    /// World coordinate shown at the viewport's top-left.
    pub pan: Vec2,
    pub zoom: f32,
    pub show_grid: bool,
    pub snap: bool,
    drag: Option<Drag>,
    pub label_edit: Option<LabelEdit>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
            show_grid: true,
            snap: true,
            drag: None,
            label_edit: None,
        }
    }
}

impl CanvasState {
    pub fn world_to_screen(&self, rect: Rect, wx: f32, wy: f32) -> Pos2 {
        rect.min + (Vec2::new(wx, wy) - self.pan) * self.zoom
    }

    pub fn screen_to_world(&self, rect: Rect, p: Pos2) -> Pos2 {
        let v = (p - rect.min) / self.zoom + self.pan;
        Pos2::new(v.x, v.y)
    }

    pub fn zoom_by(&mut self, rect: Rect, factor: f32, around: Pos2) {
        let before = self.screen_to_world(rect, around);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let after = self.screen_to_world(rect, around);
        self.pan += before - after;
    }

    /// Fit the document bounds into the viewport with a margin.
    pub fn fit(&mut self, bounds: Option<(f32, f32, f32, f32)>, viewport: Rect) {
        let Some((minx, miny, maxx, maxy)) = bounds else {
            self.pan = Vec2::ZERO;
            self.zoom = 1.0;
            return;
        };
        let w = (maxx - minx).max(1.0);
        let h = (maxy - miny).max(1.0);
        let margin = 48.0;
        let zx = (viewport.width() - margin * 2.0) / w;
        let zy = (viewport.height() - margin * 2.0) / h;
        self.zoom = zx.min(zy).clamp(MIN_ZOOM, MAX_ZOOM);
        let cx = (minx + maxx) / 2.0;
        let cy = (miny + maxy) / 2.0;
        self.pan = Vec2::new(
            cx - viewport.width() / (2.0 * self.zoom),
            cy - viewport.height() / (2.0 * self.zoom),
        );
    }
}

fn color32(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

// ---------------------------------------------------------------------------
// Shape painting (screen space)
// ---------------------------------------------------------------------------

fn poly(painter: &egui::Painter, pts: Vec<Pos2>, fill: Color32, stroke: Stroke) {
    painter.add(Shape::Path(PathShape {
        points: pts,
        closed: true,
        fill,
        stroke: stroke.into(),
    }));
}

fn polyline(painter: &egui::Painter, pts: Vec<Pos2>, stroke: Stroke) {
    painter.add(Shape::Path(PathShape {
        points: pts,
        closed: false,
        fill: Color32::TRANSPARENT,
        stroke: PathStroke::from(stroke),
    }));
}

fn ellipse_pts(center: Pos2, rx: f32, ry: f32) -> Vec<Pos2> {
    (0..32)
        .map(|i| {
            let a = std::f32::consts::TAU * i as f32 / 32.0;
            Pos2::new(center.x + rx * a.cos(), center.y + ry * a.sin())
        })
        .collect()
}

fn arc_pts(center: Pos2, rx: f32, ry: f32, a0: f32, a1: f32, n: usize) -> Vec<Pos2> {
    (0..=n)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / n as f32;
            Pos2::new(center.x + rx * a.cos(), center.y + ry * a.sin())
        })
        .collect()
}

fn cubic(p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2, n: usize) -> Vec<Pos2> {
    (1..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let mt = 1.0 - t;
            let x = mt * mt * mt * p0.x + 3.0 * mt * mt * t * p1.x + 3.0 * mt * t * t * p2.x + t * t * t * p3.x;
            let y = mt * mt * mt * p0.y + 3.0 * mt * mt * t * p1.y + 3.0 * mt * t * t * p2.y + t * t * t * p3.y;
            Pos2::new(x, y)
        })
        .collect()
}

/// Paint a shape body into a screen-space rect.
pub fn paint_shape(painter: &egui::Painter, kind: ShapeKind, r: Rect, fill: Color32, stroke: Stroke) {
    let c = r.center();
    let (w, h) = (r.width(), r.height());
    match kind {
        ShapeKind::Rect => {
            painter.rect(r, CornerRadius::ZERO, fill, stroke, StrokeKind::Inside);
        }
        ShapeKind::Rounded => {
            painter.rect(r, CornerRadius::same((h * 0.2).min(255.0) as u8), fill, stroke, StrokeKind::Inside);
        }
        ShapeKind::Stadium => {
            painter.rect(r, CornerRadius::same((h / 2.0).min(255.0) as u8), fill, stroke, StrokeKind::Inside);
        }
        ShapeKind::Circle => poly(painter, ellipse_pts(c, w / 2.0, h / 2.0), fill, stroke),
        ShapeKind::Diamond => poly(
            painter,
            vec![
                Pos2::new(c.x, r.top()),
                Pos2::new(r.right(), c.y),
                Pos2::new(c.x, r.bottom()),
                Pos2::new(r.left(), c.y),
            ],
            fill,
            stroke,
        ),
        ShapeKind::Hexagon => {
            let dx = w * 0.22;
            poly(
                painter,
                vec![
                    Pos2::new(r.left() + dx, r.top()),
                    Pos2::new(r.right() - dx, r.top()),
                    Pos2::new(r.right(), c.y),
                    Pos2::new(r.right() - dx, r.bottom()),
                    Pos2::new(r.left() + dx, r.bottom()),
                    Pos2::new(r.left(), c.y),
                ],
                fill,
                stroke,
            )
        }
        ShapeKind::Triangle => poly(
            painter,
            vec![
                Pos2::new(c.x, r.top()),
                Pos2::new(r.right(), r.bottom()),
                Pos2::new(r.left(), r.bottom()),
            ],
            fill,
            stroke,
        ),
        ShapeKind::Parallelogram => {
            let s = w * 0.18;
            poly(
                painter,
                vec![
                    Pos2::new(r.left() + s, r.top()),
                    Pos2::new(r.right(), r.top()),
                    Pos2::new(r.right() - s, r.bottom()),
                    Pos2::new(r.left(), r.bottom()),
                ],
                fill,
                stroke,
            )
        }
        ShapeKind::Trapezoid => {
            let s = w * 0.18;
            poly(
                painter,
                vec![
                    Pos2::new(r.left() + s, r.top()),
                    Pos2::new(r.right() - s, r.top()),
                    Pos2::new(r.right(), r.bottom()),
                    Pos2::new(r.left(), r.bottom()),
                ],
                fill,
                stroke,
            )
        }
        ShapeKind::Cylinder => {
            let ry = (h * 0.14).max(3.0);
            // Body fill: rect + top ellipse + front bottom arc cap.
            let body = Rect::from_min_max(
                Pos2::new(r.left(), r.top() + ry),
                Pos2::new(r.right(), r.bottom() - ry),
            );
            painter.rect_filled(body, CornerRadius::ZERO, fill);
            poly(painter, ellipse_pts(Pos2::new(c.x, r.top() + ry), w / 2.0, ry), fill, stroke);
            // Bottom front arc (fill cap + stroke).
            let mut cap = arc_pts(Pos2::new(c.x, r.bottom() - ry), w / 2.0, ry, 0.0, std::f32::consts::PI, 16);
            let mut cap_fill = cap.clone();
            cap_fill.reverse();
            cap_fill.extend(arc_pts(Pos2::new(c.x, r.bottom() - ry), w / 2.0, ry, std::f32::consts::PI, std::f32::consts::TAU, 16));
            poly(painter, cap_fill, fill, Stroke::NONE);
            polyline(painter, std::mem::take(&mut cap), stroke);
            polyline(painter, vec![Pos2::new(r.left(), r.top() + ry), Pos2::new(r.left(), r.bottom() - ry)], stroke);
            polyline(painter, vec![Pos2::new(r.right(), r.top() + ry), Pos2::new(r.right(), r.bottom() - ry)], stroke);
        }
        ShapeKind::Cloud => {
            // Canonical 100×60 cloud outline scaled into the rect.
            let p = |x: f32, y: f32| Pos2::new(r.left() + x * w / 100.0, r.top() + y * h / 60.0);
            let mut pts = vec![p(20.0, 50.0)];
            pts.extend(cubic(p(20.0, 50.0), p(8.0, 50.0), p(4.0, 40.0), p(10.0, 33.0), 6));
            pts.extend(cubic(p(10.0, 33.0), p(6.0, 22.0), p(16.0, 14.0), p(26.0, 18.0), 6));
            pts.extend(cubic(p(26.0, 18.0), p(30.0, 6.0), p(46.0, 4.0), p(54.0, 12.0), 6));
            pts.extend(cubic(p(54.0, 12.0), p(64.0, 6.0), p(78.0, 10.0), p(80.0, 20.0), 6));
            pts.extend(cubic(p(80.0, 20.0), p(92.0, 20.0), p(97.0, 30.0), p(91.0, 38.0), 6));
            pts.extend(cubic(p(91.0, 38.0), p(96.0, 46.0), p(88.0, 50.0), p(80.0, 50.0), 6));
            poly(painter, pts, fill, stroke);
        }
        ShapeKind::Document => {
            let wave = (h * 0.12).max(2.0);
            let y0 = r.bottom() - wave;
            let mut pts = vec![
                Pos2::new(r.left(), r.top()),
                Pos2::new(r.right(), r.top()),
                Pos2::new(r.right(), y0),
            ];
            pts.extend(sample_quad(
                Pos2::new(r.right(), y0),
                Pos2::new(c.x + w * 0.25, r.bottom() + wave),
                Pos2::new(c.x, y0),
                8,
            ));
            pts.extend(sample_quad(
                Pos2::new(c.x, y0),
                Pos2::new(c.x - w * 0.25, r.bottom() + wave),
                Pos2::new(r.left(), y0),
                8,
            ));
            poly(painter, pts, fill, stroke);
        }
        ShapeKind::Actor => {
            let head_r = h * 0.13;
            let head_c = Pos2::new(c.x, r.top() + head_r);
            poly(painter, ellipse_pts(head_c, head_r, head_r), fill, stroke);
            let neck = r.top() + head_r * 2.0;
            let hip = c.y + h * 0.22;
            let arm_y = neck + h * 0.10;
            let arm_dx = h * 0.22;
            let leg_dx = h * 0.16;
            polyline(painter, vec![Pos2::new(c.x, neck), Pos2::new(c.x, hip)], stroke);
            polyline(painter, vec![Pos2::new(c.x - arm_dx, arm_y), Pos2::new(c.x + arm_dx, arm_y)], stroke);
            polyline(painter, vec![Pos2::new(c.x, hip), Pos2::new(c.x - leg_dx, r.bottom())], stroke);
            polyline(painter, vec![Pos2::new(c.x, hip), Pos2::new(c.x + leg_dx, r.bottom())], stroke);
        }
        ShapeKind::Component => {
            painter.rect(r, CornerRadius::ZERO, fill, stroke, StrokeKind::Inside);
            let tab_w = (w * 0.10).max(4.0);
            let tab_h = h * 0.22;
            for i in [0.18_f32, 0.60] {
                let tab = Rect::from_min_size(
                    Pos2::new(r.left() - tab_w / 2.0, r.top() + h * i),
                    Vec2::new(tab_w, tab_h),
                );
                painter.rect(tab, CornerRadius::ZERO, fill, stroke, StrokeKind::Inside);
            }
        }
        ShapeKind::Package => {
            let tab_w = w * 0.42;
            let tab_h = h * 0.22;
            poly(
                painter,
                vec![
                    Pos2::new(r.left(), r.top() + tab_h),
                    Pos2::new(r.left(), r.bottom()),
                    Pos2::new(r.right(), r.bottom()),
                    Pos2::new(r.right(), r.top() + tab_h),
                    Pos2::new(r.left() + tab_w, r.top() + tab_h),
                    Pos2::new(r.left() + tab_w, r.top()),
                ],
                fill,
                stroke,
            );
            polyline(
                painter,
                vec![
                    Pos2::new(r.left(), r.top() + tab_h),
                    Pos2::new(r.left() + tab_w, r.top() + tab_h),
                ],
                stroke,
            );
        }
        ShapeKind::Shield => {
            let p = |x: f32, y: f32| Pos2::new(r.left() + x * w, r.top() + y * h);
            let mut pts = vec![p(0.5, 0.0), p(1.0, 0.15), p(1.0, 0.45)];
            pts.extend(cubic(p(1.0, 0.45), p(1.0, 0.8), p(0.5, 0.9), p(0.5, 1.0), 8));
            pts.extend(cubic(p(0.5, 1.0), p(0.5, 0.9), p(0.0, 0.8), p(0.0, 0.45), 8));
            pts.push(p(0.0, 0.15));
            poly(painter, pts, fill, stroke);
        }
        ShapeKind::DataStore => {
            painter.rect_filled(r, CornerRadius::ZERO, fill);
            polyline(
                painter,
                vec![
                    Pos2::new(r.right(), r.top()),
                    Pos2::new(r.left(), r.top()),
                    Pos2::new(r.left(), r.bottom()),
                    Pos2::new(r.right(), r.bottom()),
                ],
                stroke,
            );
        }
    }
}

fn sample_quad(p0: Pos2, p1: Pos2, p2: Pos2, n: usize) -> Vec<Pos2> {
    (1..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let mt = 1.0 - t;
            Pos2::new(
                mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x,
                mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y,
            )
        })
        .collect()
}

/// Paint a node (body + provider badge + centered multi-line label).
pub fn paint_node(painter: &egui::Painter, node: &GuiNode, r: Rect) {
    let stroke = Stroke::new(1.5, color32(node.stroke));
    paint_shape(painter, node.kind(), r, color32(node.fill), stroke);
    if let Some(badge) = shapes::shape_def(&node.shape).and_then(|d| d.badge) {
        painter.text(
            Pos2::new(r.left() + 6.0, r.top() + 6.0),
            Align2::LEFT_TOP,
            badge,
            FontId::proportional(8.0),
            color32(node.stroke).gamma_multiply(0.8),
        );
    }
    let lines = node.label.split('\n').count() as f32;
    let line_h = 15.0;
    let first_y = r.center().y - (lines - 1.0) * line_h / 2.0;
    for (i, line) in node.label.split('\n').enumerate() {
        painter.text(
            Pos2::new(r.center().x, first_y + i as f32 * line_h),
            Align2::CENTER_CENTER,
            line,
            FontId::proportional(13.0),
            color32(node.text),
        );
    }
}

// ---------------------------------------------------------------------------
// Edge painting
// ---------------------------------------------------------------------------

fn dashed(painter: &egui::Painter, a: Pos2, b: Pos2, stroke: Stroke) {
    let dir = b - a;
    let len = dir.length();
    if len < 1.0 {
        return;
    }
    let unit = dir / len;
    let (dash, gap) = (6.0_f32, 4.0_f32);
    let mut d = 0.0;
    while d < len {
        let e = (d + dash).min(len);
        painter.line_segment([a + unit * d, a + unit * e], stroke);
        d = e + gap;
    }
}

fn arrowhead(painter: &egui::Painter, from: Pos2, to: Pos2, color: Color32) {
    let dir = to - from;
    let len = dir.length();
    if len < 1.0 {
        return;
    }
    let unit = dir / len;
    let perp = Vec2::new(-unit.y, unit.x);
    let (l, hw) = (11.0_f32, 4.5_f32);
    poly(
        painter,
        vec![to, to - unit * l + perp * hw, to - unit * l - perp * hw],
        color,
        Stroke::NONE,
    );
}

fn paint_edge(
    painter: &egui::Painter,
    state: &CanvasState,
    vp: Rect,
    g: &model::EdgeGeom,
    edge: &GuiEdge,
    selected: bool,
) {
    let a = state.world_to_screen(vp, g.x1, g.y1);
    let b = state.world_to_screen(vp, g.x2, g.y2);
    let c = color32(edge.color);
    let width = match edge.style {
        EdgeStyle::Thick => 3.0,
        _ => 1.5,
    };
    let stroke = Stroke::new(if selected { width + 1.0 } else { width }, c);
    match edge.style {
        EdgeStyle::Dashed => dashed(painter, a, b, stroke),
        _ => {
            painter.line_segment([a, b], stroke);
        }
    }
    arrowhead(painter, a, b, c);
    if selected {
        painter.circle_stroke(a, 4.0, Stroke::new(1.0, c));
        painter.circle_stroke(b, 4.0, Stroke::new(1.0, c));
    }
    if !edge.label.is_empty() {
        let mid = Pos2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0 - 8.0);
        let font = FontId::proportional(11.0);
        let galley = painter.layout_no_wrap(edge.label.clone(), font, c);
        let bg = Rect::from_center_size(mid, galley.size() + Vec2::new(6.0, 4.0));
        painter.rect_filled(bg, CornerRadius::same(3), Color32::WHITE.gamma_multiply(0.9));
        painter.galley(bg.min + Vec2::new(3.0, 2.0), galley, c);
    }
}

// ---------------------------------------------------------------------------
// Canvas widget
// ---------------------------------------------------------------------------

/// Four edge-midpoint port positions (screen space) for a node rect.
fn port_points(r: Rect) -> [Pos2; 4] {
    let c = r.center();
    [
        Pos2::new(c.x, r.top()),
        Pos2::new(r.right(), c.y),
        Pos2::new(c.x, r.bottom()),
        Pos2::new(r.left(), c.y),
    ]
}

/// Node bounding rect in screen space.
fn node_rect(state: &CanvasState, vp: Rect, n: &GuiNode) -> Rect {
    Rect::from_min_max(
        state.world_to_screen(vp, n.left(), n.top()),
        state.world_to_screen(vp, n.right(), n.bottom()),
    )
}

/// Render the canvas and apply interactions. `placing` is the click-to-place
/// shape id (set by clicking a palette item); cleared once the node is placed.
pub fn show(
    ui: &mut egui::Ui,
    editor: &mut Editor,
    state: &mut CanvasState,
    placing: &mut Option<&'static str>,
) {
    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let vp = response.rect;
    let ctx = ui.ctx().clone();

    // ---- zoom / pan via scroll ------------------------------------------
    let scroll = ctx.input(|i| i.smooth_scroll_delta);
    let mods = ctx.input(|i| i.modifiers);
    let hover = ctx.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::ZERO);
    if scroll.y != 0.0 && vp.contains(hover) {
        if mods.command {
            state.zoom_by(vp, (1.0 + scroll.y * 0.0015).clamp(0.5, 2.0), hover);
        } else {
            state.pan += scroll / state.zoom;
        }
    }

    // ---- background ------------------------------------------------------
    let bg = if ui.visuals().dark_mode {
        Color32::from_gray(24)
    } else {
        Color32::from_gray(250)
    };
    painter.rect_filled(vp, CornerRadius::ZERO, bg);
    if state.show_grid && state.zoom > 0.35 {
        let grid_color = if ui.visuals().dark_mode {
            Color32::from_gray(48)
        } else {
            Color32::from_gray(225)
        };
        let step = GRID_STEP * state.zoom;
        let start_x = vp.left() - (state.pan.x * state.zoom).rem_euclid(step);
        let start_y = vp.top() - (state.pan.y * state.zoom).rem_euclid(step);
        let mut x = start_x;
        while x < vp.right() {
            let mut y = start_y;
            while y < vp.bottom() {
                painter.circle_filled(Pos2::new(x, y), 1.0, grid_color);
                y += step;
            }
            x += step;
        }
    }

    // ---- palette drop ----------------------------------------------------
    if let Some(payload) = response.dnd_release_payload::<String>()
        && let Some(screen) = ctx.input(|i| i.pointer.interact_pos())
        && vp.contains(screen)
    {
        let w = state.screen_to_world(vp, screen);
        editor.add_node(payload.as_str(), snap(w.x), snap(w.y));
        editor.end_gesture();
    }
    // Hover ghost while dragging a palette shape over the canvas.
    if let Some(payload) = response.dnd_hover_payload::<String>()
        && let Some(screen) = ctx.input(|i| i.pointer.hover_pos())
        && vp.contains(screen)
    {
        let w = state.screen_to_world(vp, screen);
        let def = shapes::shape_def(payload.as_str());
        let (nw, nh) = def.map(|d| (d.w, d.h)).unwrap_or((120.0, 60.0));
        let ghost = Rect::from_center_size(w, Vec2::new(nw, nh));
        let ghost_screen = Rect::from_min_max(
            state.world_to_screen(vp, ghost.min.x, ghost.min.y),
            state.world_to_screen(vp, ghost.max.x, ghost.max.y),
        );
        painter.rect_stroke(
            ghost_screen,
            CornerRadius::same(4),
            Stroke::new(1.0, Color32::from_rgb(9, 105, 218)),
            StrokeKind::Inside,
        );
    }

    // ---- pointer world position ------------------------------------------
    let pointer_screen = ctx.input(|i| i.pointer.interact_pos()).unwrap_or(Pos2::ZERO);
    let pointer_world = state.screen_to_world(vp, pointer_screen);
    let primary_down = ctx.input(|i| i.pointer.primary_down());
    let space_down = ctx.input(|i| i.key_down(egui::Key::Space));
    let middle_down = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Middle));

    // Middle-mouse drag pans (tracked per-frame; primary drags use Drag state).
    if middle_down && vp.contains(pointer_screen) && state.drag.is_none() {
        state.drag = Some(Drag::Pan { last: pointer_screen });
    }
    // Clear a finished pan once all buttons are released.
    if !middle_down && !primary_down && matches!(state.drag, Some(Drag::Pan { .. })) {
        state.drag = None;
    }

    // ---- click-to-place (simple click; drags place at drag start) --------
    if response.clicked()
        && let Some(shape_id) = placing.take()
    {
        editor.add_node(shape_id, snap(pointer_world.x), snap(pointer_world.y));
        editor.end_gesture();
    }

    // ---- drag start ------------------------------------------------------
    if response.drag_started_by(egui::PointerButton::Primary) {
        // Click-to-place mode from the palette.
        if let Some(shape_id) = placing.take() {
            editor.add_node(shape_id, snap(pointer_world.x), snap(pointer_world.y));
            editor.end_gesture();
        } else if space_down {
            state.drag = Some(Drag::Pan { last: pointer_screen });
        } else {
            // Port handle of the single selected node → start an edge.
            let port_hit = if editor.selection.len() == 1 {
                editor
                    .doc
                    .node(&editor.selection[0])
                    .map(|n| {
                        let r = node_rect(state, vp, n);
                        port_points(r)
                            .iter()
                            .any(|p| p.distance(pointer_screen) < PORT_HIT)
                    })
                    .unwrap_or(false)
            } else {
                false
            };
            // Resize handle of the single selected node.
            let resize_hit = if editor.selection.len() == 1 {
                editor
                    .doc
                    .node(&editor.selection[0])
                    .map(|n| {
                        let r = node_rect(state, vp, n);
                        let handle = Rect::from_center_size(
                            Pos2::new(r.right(), r.bottom()),
                            Vec2::splat(HANDLE + 4.0),
                        );
                        handle.contains(pointer_screen)
                    })
                    .unwrap_or(false)
            } else {
                false
            };

            if port_hit {
                state.drag = Some(Drag::Link {
                    from: editor.selection[0].clone(),
                });
            } else if resize_hit {
                let id = editor.selection[0].clone();
                let n = editor.doc.node(&id).unwrap();
                state.drag = Some(Drag::Resize {
                    id,
                    start_w: n.w,
                    start_h: n.h,
                    start: pointer_world,
                });
            } else if let Some(hit) = editor.hit_node(pointer_world.x, pointer_world.y).map(str::to_string) {
                if mods.shift {
                    editor.toggle_select(&hit);
                } else if !editor.selection.contains(&hit) {
                    editor.select_only(&hit);
                }
                state.drag = Some(Drag::Nodes { last: pointer_world });
            } else if let Some(edge_id) = editor
                .hit_edge(pointer_world.x, pointer_world.y, 6.0 / state.zoom)
                .map(str::to_string)
            {
                editor.select_only_edge(&edge_id);
            } else {
                if !mods.shift {
                    editor.clear_selection();
                }
                state.drag = Some(Drag::Marquee { start: pointer_world });
            }
        }
    }

    // ---- drag update -----------------------------------------------------
    if primary_down || middle_down {
        match state.drag.clone() {
            Some(Drag::Pan { last }) => {
                state.pan -= (pointer_screen - last) / state.zoom;
                state.drag = Some(Drag::Pan { last: pointer_screen });
            }
            Some(Drag::Nodes { last }) => {
                let d = pointer_world - last;
                editor.move_selected_by(d.x, d.y, false);
                state.drag = Some(Drag::Nodes { last: pointer_world });
            }
            Some(Drag::Resize { id, start_w, start_h, start }) => {
                let dw = pointer_world.x - start.x;
                let dh = pointer_world.y - start.y;
                editor.resize_node(&id, start_w + dw, start_h + dh);
                state.drag = Some(Drag::Resize { id, start_w, start_h, start });
            }
            _ => {}
        }
    }

    // ---- drag stop -------------------------------------------------------
    if response.drag_stopped() {
        match state.drag.take() {
            Some(Drag::Link { from }) => {
                if let Some(target) = editor.hit_node(pointer_world.x, pointer_world.y).map(str::to_string) {
                    editor.connect(&from, &target);
                }
            }
            Some(Drag::Marquee { start }) => {
                let minx = start.x.min(pointer_world.x);
                let miny = start.y.min(pointer_world.y);
                let maxx = start.x.max(pointer_world.x);
                let maxy = start.y.max(pointer_world.y);
                if (maxx - minx) * state.zoom > 4.0 && (maxy - miny) * state.zoom > 4.0 {
                    let mut hits = editor.nodes_in_rect(minx, miny, maxx, maxy);
                    if mods.shift {
                        editor.selection.append(&mut hits);
                        editor.selection.dedup();
                    } else {
                        editor.selection = hits;
                    }
                }
            }
            Some(Drag::Nodes { .. }) if state.snap && !mods.alt => {
                editor.snap_selected();
            }
            _ => {}
        }
        editor.end_gesture();
    }

    // ---- double-click: inline label edit ----------------------------------
    if response.double_clicked()
        && let Some(hit) = editor.hit_node(pointer_world.x, pointer_world.y).map(str::to_string)
        && let Some(n) = editor.doc.node(&hit)
    {
        state.label_edit = Some(LabelEdit {
            node_id: hit,
            buffer: n.label.clone(),
        });
    }

    // ---- paint edges ------------------------------------------------------
    let edge_selected: std::collections::HashSet<&str> =
        editor.edge_selection.iter().map(String::as_str).collect();
    for e in &editor.doc.edges {
        if let Some(g) = model::edge_geometry(&editor.doc, e) {
            paint_edge(
                &painter,
                state,
                vp,
                &g,
                e,
                edge_selected.contains(e.id.as_str()),
            );
        }
    }

    // ---- link-in-progress --------------------------------------------------
    if let Some(Drag::Link { from }) = &state.drag
        && let Some(n) = editor.doc.node(from)
    {
        let a = state.world_to_screen(vp, n.x, n.y);
        dashed(&painter, a, pointer_screen, Stroke::new(1.5, Color32::from_rgb(9, 105, 218)));
        arrowhead(&painter, a, pointer_screen, Color32::from_rgb(9, 105, 218));
    }

    // ---- paint nodes --------------------------------------------------------
    for n in &editor.doc.nodes {
        paint_node(&painter, n, node_rect(state, vp, n));
    }

    // ---- selection adornments -------------------------------------------------
    let sel_color = Color32::from_rgb(9, 105, 218);
    for id in &editor.selection {
        if let Some(n) = editor.doc.node(id) {
            let r = node_rect(state, vp, n).expand(2.0);
            painter.rect_stroke(r, CornerRadius::same(2), Stroke::new(1.5, sel_color), StrokeKind::Inside);
            if editor.selection.len() == 1 {
                // Ports + resize handle for the single selected node.
                for p in port_points(r) {
                    painter.circle_filled(p, 4.0, Color32::WHITE);
                    painter.circle_stroke(p, 4.0, Stroke::new(1.5, sel_color));
                }
                let handle = Rect::from_center_size(Pos2::new(r.right(), r.bottom()), Vec2::splat(HANDLE));
                painter.rect_filled(handle, CornerRadius::same(2), sel_color);
            }
        }
    }

    // ---- marquee ----------------------------------------------------------------
    if let Some(Drag::Marquee { start }) = &state.drag {
        let a = state.world_to_screen(vp, start.x, start.y);
        let r = Rect::from_two_pos(a, pointer_screen);
        painter.rect_filled(r, CornerRadius::ZERO, sel_color.gamma_multiply(0.08));
        painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, sel_color), StrokeKind::Inside);
    }

    // ---- inline label edit overlay ------------------------------------------------
    if let Some(edit) = state.label_edit.clone() {
        if let Some(n) = editor.doc.node(&edit.node_id) {
            let r = node_rect(state, vp, n);
            egui::Area::new(egui::Id::unique("node-label-edit"))
                .order(egui::Order::Foreground)
                .fixed_pos(r.min)
                .show(&ctx, |ui| {
                    let mut buffer = edit.buffer.clone();
                    let resp = ui.add(
                        egui::TextEdit::multiline(&mut buffer)
                            .desired_width(r.width())
                            .font(FontId::proportional(13.0))
                            .align(egui::Align2::CENTER_CENTER),
                    );
                    resp.request_focus();
                    let mut e = edit.clone();
                    e.buffer = buffer;
                    state.label_edit = Some(e);
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter)) && !ui.input(|i| i.modifiers.shift);
                    let esc = ui.input(|i| i.key_pressed(egui::Key::Escape));
                    if enter {
                        let e = state.label_edit.take().unwrap();
                        editor.set_label(&e.node_id, e.buffer.trim());
                        editor.end_gesture();
                    } else if esc {
                        state.label_edit = None;
                    }
                });
        } else {
            state.label_edit = None;
        }
    }
}
