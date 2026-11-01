//! Desktop GUI (egui/eframe): a pure-Rust, fully offline graphine-style
//! diagram editor. Drag shapes onto the canvas, connect them, style them,
//! and save/load locally as JSON or export SVG / PNG / Mermaid.
//!
//! Module layout: [`model`] is the egui-free document + editor core (unit
//! tested headlessly), [`shapes`] is the palette catalog + SVG geometry,
//! [`canvas`] is the egui canvas widget, and [`app`] is the eframe shell.

pub mod app;
pub mod canvas;
pub mod model;
pub mod shapes;

pub use app::run;
pub use model::{Editor, GuiDocument, GuiEdge, GuiNode};
