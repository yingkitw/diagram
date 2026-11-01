//! eframe application shell: menu bar, shape palette, properties panel,
//! status bar, file dialogs, and global keyboard shortcuts.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use egui::{Color32, CornerRadius, Pos2, Rect, Sense, Stroke, Vec2};

use crate::diagram::EdgeStyle;

use super::canvas::{self, CanvasState};
use super::model::{Editor, GuiDocument, ZOrder};
use super::shapes::{self, Category};

pub struct GuiApp {
    editor: Editor,
    canvas: CanvasState,
    path: Option<PathBuf>,
    palette_filter: String,
    placing: Option<&'static str>,
    status: Option<(Instant, String)>,
}

impl Default for GuiApp {
    fn default() -> Self {
        Self::new()
    }
}

impl GuiApp {
    pub fn new() -> Self {
        Self {
            editor: Editor::new(),
            canvas: CanvasState::default(),
            path: None,
            palette_filter: String::new(),
            placing: None,
            status: None,
        }
    }

    fn notify(&mut self, msg: impl Into<String>) {
        self.status = Some((Instant::now(), msg.into()));
    }

    // -- file operations ---------------------------------------------------

    fn new_document(&mut self) {
        self.editor = Editor::new();
        self.path = None;
        self.canvas = CanvasState::default();
        self.notify("New document");
    }

    fn open(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Diagram JSON", &["json"])
            .pick_file()
        else {
            return;
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => match GuiDocument::from_json(&text) {
                Ok(doc) => {
                    self.editor = Editor::from_document(doc);
                    self.path = Some(path);
                    self.notify("Opened");
                }
                Err(e) => self.notify(format!("Open failed: {e}")),
            },
            Err(e) => self.notify(format!("Open failed: {e}")),
        }
    }

    fn save(&mut self, save_as: bool) {
        let path = if save_as || self.path.is_none() {
            let Some(p) = rfd::FileDialog::new()
                .add_filter("Diagram JSON", &["json"])
                .set_file_name(format!("{}.json", self.editor.doc.name))
                .save_file()
            else {
                return;
            };
            p
        } else {
            self.path.clone().unwrap()
        };
        match self.editor.doc.to_json() {
            Ok(json) => match std::fs::write(&path, json) {
                Ok(()) => {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        self.editor.doc.name = stem.to_string();
                    }
                    self.editor.dirty = false;
                    self.path = Some(path);
                    self.notify("Saved");
                }
                Err(e) => self.notify(format!("Save failed: {e}")),
            },
            Err(e) => self.notify(format!("Save failed: {e}")),
        }
    }

    fn export(&mut self, kind: ExportKind) {
        let (label, ext) = match kind {
            ExportKind::Svg => ("SVG", "svg"),
            ExportKind::Png => ("PNG", "png"),
            ExportKind::Mermaid => ("Mermaid", "mmd"),
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter(label, &[ext])
            .set_file_name(format!("{}.{ext}", self.editor.doc.name))
            .save_file()
        else {
            return;
        };
        let result = match kind {
            ExportKind::Svg | ExportKind::Mermaid => {
                let text = match kind {
                    ExportKind::Svg => self.editor.to_svg(),
                    _ => self.editor.to_mermaid(),
                };
                std::fs::write(&path, text).map_err(|e| e.to_string())
            }
            ExportKind::Png => crate::png::svg_to_png(&self.editor.to_svg())
                .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string())),
        };
        match result {
            Ok(()) => self.notify(format!("Exported {label}")),
            Err(e) => self.notify(format!("Export failed: {e}")),
        }
    }

    // -- keyboard shortcuts --------------------------------------------------

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return; // a text field has focus
        }
        let (mods, del, z, y, d, a, s, o, n) = ctx.input(|i| {
            (
                i.modifiers,
                i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace),
                i.key_pressed(egui::Key::Z),
                i.key_pressed(egui::Key::Y),
                i.key_pressed(egui::Key::D),
                i.key_pressed(egui::Key::A),
                i.key_pressed(egui::Key::S),
                i.key_pressed(egui::Key::O),
                i.key_pressed(egui::Key::N),
            )
        });
        let arrows = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowLeft),
                i.key_pressed(egui::Key::ArrowRight),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
            )
        });
        let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));

        if del {
            self.editor.delete_selected();
        } else if mods.command && z && !mods.shift {
            self.editor.undo();
        } else if (mods.command && y) || (mods.command && z && mods.shift) {
            self.editor.redo();
        } else if mods.command && d {
            self.editor.duplicate_selected();
        } else if mods.command && a {
            self.editor.select_all();
        } else if mods.command && s && mods.shift {
            self.save(true);
        } else if mods.command && s {
            self.save(false);
        } else if mods.command && o {
            self.open();
        } else if mods.command && n {
            self.new_document();
        } else if esc {
            self.placing = None;
            self.canvas.label_edit = None;
            self.editor.clear_selection();
        } else if arrows != (false, false, false, false) {
            let step = if mods.shift { 16.0 } else { 1.0 };
            let (dx, dy) = match arrows {
                (true, _, _, _) => (-step, 0.0),
                (_, true, _, _) => (step, 0.0),
                (_, _, true, _) => (0.0, -step),
                _ => (0.0, step),
            };
            self.editor.move_selected_by(dx, dy, false);
            self.editor.end_gesture();
        }
    }

    // -- panels ----------------------------------------------------------------

    fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("menu-bar").show(ui, |ui| {
            let ctx = ui.ctx().clone();
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New\tCtrl+N").clicked() {
                        self.new_document();
                        ui.close();
                    }
                    if ui.button("Open…\tCtrl+O").clicked() {
                        self.open();
                        ui.close();
                    }
                    if ui.button("Save\tCtrl+S").clicked() {
                        self.save(false);
                        ui.close();
                    }
                    if ui.button("Save As…\tCtrl+Shift+S").clicked() {
                        self.save(true);
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Export SVG…").clicked() {
                        self.export(ExportKind::Svg);
                        ui.close();
                    }
                    if ui.button("Export PNG…").clicked() {
                        self.export(ExportKind::Png);
                        ui.close();
                    }
                    if ui.button("Export Mermaid…").clicked() {
                        self.export(ExportKind::Mermaid);
                        ui.close();
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui
                        .add_enabled(self.editor.can_undo(), egui::Button::new("Undo\tCtrl+Z"))
                        .clicked()
                    {
                        self.editor.undo();
                        ui.close();
                    }
                    if ui
                        .add_enabled(self.editor.can_redo(), egui::Button::new("Redo\tCtrl+Shift+Z"))
                        .clicked()
                    {
                        self.editor.redo();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Duplicate\tCtrl+D").clicked() {
                        self.editor.duplicate_selected();
                        ui.close();
                    }
                    if ui.button("Delete\tDel").clicked() {
                        self.editor.delete_selected();
                        ui.close();
                    }
                    if ui.button("Select All\tCtrl+A").clicked() {
                        self.editor.select_all();
                        ui.close();
                    }
                });
                ui.menu_button("Arrange", |ui| {
                    ui.add_enabled_ui(self.editor.selection.len() == 1, |ui| {
                        let id = self.editor.selection.first().cloned().unwrap_or_default();
                        for (label, cmd) in [
                            ("Bring to Front", ZOrder::ToFront),
                            ("Bring Forward", ZOrder::Forward),
                            ("Send Backward", ZOrder::Backward),
                            ("Send to Back", ZOrder::ToBack),
                        ] {
                            if ui.button(label).clicked() {
                                self.editor.reorder(&id, cmd);
                                self.editor.end_gesture();
                                ui.close();
                            }
                        }
                    });
                });
                ui.menu_button("View", |ui| {
                    if ui.button("Zoom In\tCtrl+Scroll").clicked() {
                        let vp = ctx.viewport_rect();
                        self.canvas.zoom_by(vp, 1.25, vp.center());
                        ui.close();
                    }
                    if ui.button("Zoom Out").clicked() {
                        let vp = ctx.viewport_rect();
                        self.canvas.zoom_by(vp, 0.8, vp.center());
                        ui.close();
                    }
                    if ui.button("Fit to Content").clicked() {
                        self.canvas.fit(self.editor.doc.bounds(), ctx.viewport_rect());
                        ui.close();
                    }
                    ui.separator();
                    ui.checkbox(&mut self.canvas.show_grid, "Show Grid");
                    ui.checkbox(&mut self.canvas.snap, "Snap to Grid");
                });
            });
        });
    }

    fn palette(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("palette")
            .default_size(180.0)
            .size_range(140.0..=280.0)
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.heading("Shapes");
                ui.add(
                    egui::TextEdit::singleline(&mut self.palette_filter)
                        .hint_text("Filter…")
                        .desired_width(f32::INFINITY),
                );
                ui.separator();
                let filter = self.palette_filter.to_lowercase();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for cat in Category::all() {
                        let defs: Vec<&shapes::ShapeDef> = shapes::CATALOG
                            .iter()
                            .filter(|d| {
                                d.category == cat
                                    && (filter.is_empty() || d.name.to_lowercase().contains(&filter))
                            })
                            .collect();
                        if defs.is_empty() {
                            continue;
                        }
                        egui::CollapsingHeader::new(cat.label())
                            .default_open(matches!(cat, Category::Basic | Category::Flowchart))
                            .show(ui, |ui| {
                                for def in defs {
                                    self.palette_item(ui, def);
                                }
                            });
                    }
                });
            });
    }

    fn palette_item(&mut self, ui: &mut egui::Ui, def: &'static shapes::ShapeDef) {
        let id = egui::Id::unique(("palette", def.id));
        let selected = self.placing == Some(def.id);
        let resp = ui
            .dnd_drag_source(id, def.id.to_string(), |ui| {
                let frame = egui::Frame::group(ui.style());
                let inner = frame
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(36.0, 26.0), Sense::hover());
                            let painter = ui.painter_at(rect);
                            let preview = fit_rect(def, rect);
                            canvas::paint_shape(
                                &painter,
                                def.kind,
                                preview,
                                Color32::from_rgb(def.fill[0], def.fill[1], def.fill[2]),
                                Stroke::new(1.0, Color32::from_rgb(def.stroke[0], def.stroke[1], def.stroke[2])),
                            );
                            ui.label(def.name);
                        });
                    })
                    .response;
                if selected {
                    ui.painter().rect_stroke(
                        inner.rect,
                        CornerRadius::same(4),
                        Stroke::new(1.5, Color32::from_rgb(9, 105, 218)),
                        egui::StrokeKind::Outside,
                    );
                }
            })
            .response;
        if resp.clicked() {
            self.placing = if selected { None } else { Some(def.id) };
        }
        resp.on_hover_text("Drag onto the canvas, or click then click the canvas");
    }

    fn properties(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("properties")
            .default_size(220.0)
            .size_range(180.0..=320.0)
            .show(ui, |ui| {
                ui.add_space(6.0);
                ui.heading("Properties");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.editor.selection.len() == 1 {
                        let id = self.editor.selection[0].clone();
                        self.node_properties(ui, &id);
                    } else if self.editor.selection.len() > 1 {
                        ui.label(format!("{} nodes selected", self.editor.selection.len()));
                        if ui.button("Delete Selected").clicked() {
                            self.editor.delete_selected();
                        }
                    } else if self.editor.edge_selection.len() == 1 {
                        let id = self.editor.edge_selection[0].clone();
                        self.edge_properties(ui, &id);
                    } else {
                        ui.label("Nothing selected.");
                        ui.add_space(8.0);
                        ui.label("Drag shapes from the palette onto the canvas. Drag from a port dot to connect nodes.");
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            let mut name = self.editor.doc.name.clone();
                            if ui.text_edit_singleline(&mut name).changed() {
                                self.editor.doc.name = name;
                                self.editor.dirty = true;
                            }
                        });
                        ui.label(format!("Nodes: {}", self.editor.doc.nodes.len()));
                        ui.label(format!("Edges: {}", self.editor.doc.edges.len()));
                    }
                });
            });
    }

    fn node_properties(&mut self, ui: &mut egui::Ui, id: &str) {
        let Some(node) = self.editor.doc.node(id).cloned() else {
            return;
        };
        ui.label(format!("Node {}", node.id));
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Label:");
            let mut label = node.label.clone();
            if ui.text_edit_singleline(&mut label).changed() {
                self.editor.set_label(id, &label);
            }
        });

        ui.horizontal(|ui| {
            ui.label("Shape:");
            let current = shapes::shape_def(&node.shape)
                .map(|d| d.name)
                .unwrap_or("Rectangle");
            egui::ComboBox::from_id_salt("shape-swap")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    for cat in Category::all() {
                        ui.label(cat.label());
                        for def in shapes::CATALOG.iter().filter(|d| d.category == cat) {
                            if ui
                                .selectable_label(node.shape == def.id, def.name)
                                .clicked()
                            {
                                self.editor.set_shape(id, def.id);
                            }
                        }
                    }
                });
        });

        ui.horizontal(|ui| {
            ui.label("Size:");
            let (mut w, mut h) = (node.w, node.h);
            let changed = ui
                .add(egui::DragValue::new(&mut w).range(24.0..=2000.0).speed(1.0))
                .changed()
                | ui
                    .add(egui::DragValue::new(&mut h).range(24.0..=2000.0).speed(1.0))
                    .changed();
            if changed {
                self.editor.resize_node(id, w, h);
            }
        });

        ui.horizontal(|ui| {
            ui.label("Position:");
            let (mut x, mut y) = (node.x, node.y);
            let changed = ui
                .add(egui::DragValue::new(&mut x).speed(1.0))
                .changed()
                | ui.add(egui::DragValue::new(&mut y).speed(1.0)).changed();
            if changed {
                self.editor.set_pos(id, x, y);
            }
        });

        ui.add_space(4.0);
        color_row(ui, "Fill:", node.fill, |c| {
            self.editor.set_colors(id, Some(c), None, None);
        });
        color_row(ui, "Stroke:", node.stroke, |c| {
            self.editor.set_colors(id, None, Some(c), None);
        });
        color_row(ui, "Text:", node.text, |c| {
            self.editor.set_colors(id, None, None, Some(c));
        });
        ui.add_space(4.0);
        if ui.button("Delete Node").clicked() {
            self.editor.select_only(id);
            self.editor.delete_selected();
        }
    }

    fn edge_properties(&mut self, ui: &mut egui::Ui, id: &str) {
        let Some(edge) = self.editor.doc.edge(id).cloned() else {
            return;
        };
        ui.label(format!("Edge {} → {}", edge.from, edge.to));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Label:");
            let mut label = edge.label.clone();
            if ui.text_edit_singleline(&mut label).changed() {
                self.editor.set_edge_label(id, &label);
            }
        });
        ui.horizontal(|ui| {
            ui.label("Style:");
            egui::ComboBox::from_id_salt("edge-style")
                .selected_text(match edge.style {
                    EdgeStyle::Arrow => "Arrow",
                    EdgeStyle::Dashed => "Dashed",
                    EdgeStyle::Thick => "Thick",
                })
                .show_ui(ui, |ui| {
                    for (label, style) in [
                        ("Arrow", EdgeStyle::Arrow),
                        ("Dashed", EdgeStyle::Dashed),
                        ("Thick", EdgeStyle::Thick),
                    ] {
                        if ui.selectable_label(edge.style == style, label).clicked() {
                            self.editor.set_edge_style(id, style);
                        }
                    }
                });
        });
        color_row(ui, "Color:", edge.color, |c| {
            self.editor.set_edge_color(id, c);
        });
        ui.add_space(4.0);
        if ui.button("Delete Edge").clicked() {
            self.editor.select_only_edge(id);
            self.editor.delete_selected();
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("status-bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                let dirty = if self.editor.dirty { " ●" } else { "" };
                ui.label(format!("{}{}", self.editor.doc.name, dirty));
                ui.separator();
                ui.label(format!("zoom {:.0}%", self.canvas.zoom * 100.0));
                ui.separator();
                ui.label(format!(
                    "{} nodes · {} edges",
                    self.editor.doc.nodes.len(),
                    self.editor.doc.edges.len()
                ));
                if let Some((t, msg)) = &self.status
                    && t.elapsed() < Duration::from_secs(4)
                {
                    ui.separator();
                    ui.label(msg);
                    ui.ctx().request_repaint_after(Duration::from_millis(500));
                }
                if self.placing.is_some() {
                    ui.separator();
                    ui.label("Click the canvas to place the shape (Esc to cancel)");
                }
            });
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportKind {
    Svg,
    Png,
    Mermaid,
}

/// A color picker row; commits [u8;3] on change.
fn color_row(ui: &mut egui::Ui, label: &str, color: [u8; 3], mut on_change: impl FnMut([u8; 3])) {
    ui.horizontal(|ui| {
        ui.label(label);
        let mut c = [
            color[0] as f32 / 255.0,
            color[1] as f32 / 255.0,
            color[2] as f32 / 255.0,
        ];
        if ui.color_edit_button_rgb(&mut c).changed() {
            on_change([
                (c[0] * 255.0).round() as u8,
                (c[1] * 255.0).round() as u8,
                (c[2] * 255.0).round() as u8,
            ]);
        }
    });
}

/// Fit a shape's aspect ratio into a palette preview rect with padding.
fn fit_rect(def: &shapes::ShapeDef, rect: Rect) -> Rect {
    let avail = rect.shrink(2.0);
    let scale = (avail.width() / def.w).min(avail.height() / def.h);
    let size = Vec2::new(def.w * scale, def.h * scale);
    Rect::from_center_size(Pos2::new(avail.center().x, avail.center().y), size)
}

impl eframe::App for GuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.shortcuts(&ctx);
        self.menu_bar(ui);
        self.palette(ui);
        self.properties(ui);
        self.status_bar(ui);
        egui::CentralPanel::default().show(ui, |ui| {
            canvas::show(ui, &mut self.editor, &mut self.canvas, &mut self.placing);
        });
        // Window title reflects document + dirty state.
        let dirty = if self.editor.dirty { " *" } else { "" };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "diagram-gui — {}{}",
            self.editor.doc.name, dirty
        )));
    }
}

/// Launch the desktop application.
pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("diagram-gui")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "diagram-gui",
        options,
        Box::new(|_cc| Ok(Box::new(GuiApp::new()))),
    )
}
