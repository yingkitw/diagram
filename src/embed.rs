//! String-in / SVG-or-JSON-out helpers for Wasm and other embeds (no filesystem).

use crate::formats;
use crate::renderer::Theme;

fn parse_theme(theme: &str) -> Result<Theme, String> {
    match theme.trim().to_lowercase().as_str() {
        "" | "dark" => Ok(Theme::Dark),
        "light" => Ok(Theme::Light),
        other => Err(format!("unknown theme '{other}' (expected dark|light)")),
    }
}

/// Auto-detect Format, import to IR, and render SVG.
pub fn render_to_svg(source: &str, theme: &str) -> Result<String, String> {
    let theme = parse_theme(theme)?;
    let format = formats::detect(source, None);
    let doc = formats::import_str(source, format).map_err(|e| e.to_string())?;
    doc.render_svg(theme)
}

/// Auto-detect Format and serialize canonical Document IR as JSON.
pub fn parse_to_ir_json(source: &str) -> Result<String, String> {
    let format = formats::detect(source, None);
    let doc = formats::import_str(source, format).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_flowchart_svg() {
        let svg = render_to_svg("graph TD\n  A-->B\n", "dark").unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("A") || svg.contains(">A<"));
    }

    #[test]
    fn render_accepts_light_and_empty_theme() {
        let a = render_to_svg("graph TD\n  A-->B\n", "light").unwrap();
        let b = render_to_svg("graph TD\n  A-->B\n", "").unwrap();
        let c = render_to_svg("graph TD\n  A-->B\n", "DARK").unwrap();
        assert!(a.contains("<svg") && b.contains("<svg") && c.contains("<svg"));
    }

    #[test]
    fn parse_to_json_roundtrip_shape() {
        let json = parse_to_ir_json("graph LR\n  X[Hi] --> Y\n").unwrap();
        assert!(json.contains("\"X\"") || json.contains("Hi"));
        let svg = render_to_svg(&json, "dark").unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Hi"));
    }

    #[test]
    fn rejects_bad_theme() {
        assert!(render_to_svg("graph TD\n  A-->B\n", "neon").is_err());
    }

    #[test]
    fn render_sequence_class_state() {
        assert!(render_to_svg("sequenceDiagram\n  A->>B: x\n", "dark")
            .unwrap()
            .contains("<svg"));
        assert!(render_to_svg("classDiagram\n  class A\n", "light")
            .unwrap()
            .contains("<svg"));
        assert!(render_to_svg("stateDiagram-v2\n  [*] --> A\n", "dark")
            .unwrap()
            .contains("<svg"));
    }

    #[test]
    fn render_dot_d2_plantuml() {
        assert!(render_to_svg("digraph { A -> B }", "dark")
            .unwrap()
            .contains("<svg"));
        assert!(render_to_svg("a -> b: go\n", "dark").unwrap().contains("<svg"));
        assert!(render_to_svg("@startuml\nA -> B\n@enduml", "dark")
            .unwrap()
            .contains("<svg"));
    }
}
