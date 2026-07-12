//! Wasm bindings for in-browser SVG preview (no local preview server).

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn wasm_start() {
    console_error_panic_hook::set_once();
}

/// Render diagram source (Mermaid / DOT / D2 / PlantUML / JSON IR) to SVG.
/// `theme` is `"dark"` or `"light"`.
#[wasm_bindgen]
pub fn render_to_svg(source: &str, theme: &str) -> Result<String, JsValue> {
    crate::embed::render_to_svg(source, theme).map_err(|e| JsValue::from_str(&e))
}

/// Parse diagram source to canonical Document IR JSON.
#[wasm_bindgen]
pub fn parse_to_ir_json(source: &str) -> Result<String, JsValue> {
    crate::embed::parse_to_ir_json(source).map_err(|e| JsValue::from_str(&e))
}
