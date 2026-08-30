//! Format detection and import/export adapters around the canonical IR.

pub mod d2;
pub mod dot;
pub mod drawio;
pub mod mermaid;
pub mod plantuml;

use crate::ir::{Document, IrError};

/// Concrete serialization format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Mermaid,
    JsonIr,
    Dot,
    D2,
    PlantUml,
    DrawIo,
}

impl Format {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mermaid" | "mmd" => Some(Self::Mermaid),
            "json" | "ir" | "json-ir" | "json_ir" => Some(Self::JsonIr),
            "dot" | "graphviz" | "gv" => Some(Self::Dot),
            "d2" | "d2lang" => Some(Self::D2),
            "plantuml" | "puml" => Some(Self::PlantUml),
            "drawio" | "draw.io" | "draw-io" | "mxgraph" => Some(Self::DrawIo),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mermaid => "mermaid",
            Self::JsonIr => "json",
            Self::Dot => "dot",
            Self::D2 => "d2",
            Self::PlantUml => "plantuml",
            Self::DrawIo => "drawio",
        }
    }

    /// Infer format from an output path extension (defaults to Mermaid).
    pub fn from_output_path(path: &str) -> Self {
        let lower = path.to_lowercase();
        if lower.ends_with(".json") {
            Self::JsonIr
        } else if lower.ends_with(".dot") || lower.ends_with(".gv") {
            Self::Dot
        } else if lower.ends_with(".d2") {
            Self::D2
        } else if lower.ends_with(".puml") || lower.ends_with(".plantuml") {
            Self::PlantUml
        } else if lower.ends_with(".drawio") || lower.ends_with(".xml") {
            // `.xml` is generic; only treat as draw.io when it is the explicit
            // output extension (content detection is the primary path for import).
            Self::DrawIo
        } else {
            Self::Mermaid
        }
    }
}

/// Detect format from content and optional path extension.
pub fn detect(source: &str, path: Option<&str>) -> Format {
    let trimmed = source.trim_start();
    if trimmed.starts_with('{') {
        return Format::JsonIr;
    }
    if drawio::is_drawio(source) {
        return Format::DrawIo;
    }
    if plantuml::is_plantuml(source) {
        return Format::PlantUml;
    }
    if dot::is_dot(source) {
        return Format::Dot;
    }
    // Pathless Mermaid (%% comments + headers) must win over greedy D2 heuristics.
    if path.is_none() && mermaid::looks_like_mermaid(source) {
        return Format::Mermaid;
    }
    if d2::is_d2(source) {
        return Format::D2;
    }
    if let Some(p) = path {
        let lower = p.to_lowercase();
        if lower.ends_with(".json") {
            return Format::JsonIr;
        }
        if lower.ends_with(".dot") || lower.ends_with(".gv") {
            return Format::Dot;
        }
        if lower.ends_with(".d2") {
            return Format::D2;
        }
        if lower.ends_with(".puml") || lower.ends_with(".plantuml") {
            return Format::PlantUml;
        }
        if lower.ends_with(".drawio") {
            return Format::DrawIo;
        }
        if lower.ends_with(".mmd") || lower.ends_with(".mermaid") {
            return Format::Mermaid;
        }
    }
    Format::Mermaid
}

/// Import source text in the given format into a Document.
pub fn import_str(source: &str, format: Format) -> Result<Document, IrError> {
    match format {
        Format::Mermaid => mermaid::parse_to_document(source),
        Format::JsonIr => Document::from_json(source)
            .map_err(|e| IrError::from(format!("invalid JSON IR: {e}"))),
        Format::Dot => dot::parse_to_document(source),
        Format::D2 => d2::parse_to_document(source),
        Format::PlantUml => plantuml::parse_to_document(source),
        Format::DrawIo => drawio::parse_to_document(source),
    }
}

/// Export a Document to the given format.
pub fn export_str(doc: &Document, format: Format) -> Result<String, IrError> {
    match format {
        Format::Mermaid => doc.to_mermaid().map_err(IrError::from),
        Format::JsonIr => doc
            .to_json()
            .map_err(|e| IrError::from(format!("JSON serialize failed: {e}"))),
        Format::Dot => dot::export_document(doc),
        Format::D2 => d2::export_document(doc),
        Format::PlantUml => plantuml::export_document(doc),
        Format::DrawIo => drawio::export_document(doc),
    }
}

/// Import a file into a Document (`from` overrides detection when set).
pub fn import_path(path: &str, from: Option<Format>) -> Result<(Document, Format), IrError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read '{path}': {e}"))?;
    let format = from.unwrap_or_else(|| detect(&content, Some(path)));
    Ok((import_str(&content, format)?, format))
}

/// Export a Document to a path (format from `to` or destination extension).
pub fn export_path(doc: &Document, path: &str, to: Option<Format>) -> Result<Format, IrError> {
    let format = to.unwrap_or_else(|| Format::from_output_path(path));
    let loss = crate::lossiness::report(doc, format);
    if !loss.export_supported {
        return Err(IrError::from(
            loss.warnings
                .first()
                .map(|w| w.message.clone())
                .unwrap_or_else(|| "export not supported".into()),
        ));
    }
    let body = export_str(doc, format)?;
    std::fs::write(path, body).map_err(|e| format!("Failed to write '{path}': {e}"))?;
    Ok(format)
}

/// Export with lossiness report (for MCP/CLI status).
pub fn export_with_report(
    doc: &Document,
    path: &str,
    to: Option<Format>,
) -> Result<(Format, crate::lossiness::LossinessReport), IrError> {
    let format = to.unwrap_or_else(|| Format::from_output_path(path));
    let report = crate::lossiness::report(doc, format);
    if !report.export_supported {
        return Err(IrError::from(
            report
                .warnings
                .first()
                .map(|w| w.message.clone())
                .unwrap_or_else(|| "export not supported".into()),
        ));
    }
    let body = export_str(doc, format)?;
    std::fs::write(path, body).map_err(|e| format!("Failed to write '{path}': {e}"))?;
    Ok((format, report))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_json_object() {
        assert_eq!(detect("  {\"version\":1}", None), Format::JsonIr);
    }

    #[test]
    fn detect_extension() {
        assert_eq!(detect("graph TD\n", Some("x.json")), Format::JsonIr);
        assert_eq!(detect("graph TD\n", Some("x.mmd")), Format::Mermaid);
    }

    #[test]
    fn detect_dot_extension() {
        assert_eq!(detect("digraph G {}", Some("x.dot")), Format::Dot);
    }

    #[test]
    fn detect_d2_extension() {
        assert_eq!(detect("a -> b\n", Some("x.d2")), Format::D2);
    }

    #[test]
    fn d2_import_to_mermaid() {
        let src = "start: Start\nstart -> end: go\n";
        let doc = import_str(src, Format::D2).unwrap();
        let out = export_str(&doc, Format::Mermaid).unwrap();
        assert!(out.contains("Start") || out.contains("start"));
        assert!(out.contains("end"));
    }

    #[test]
    fn d2_export_roundtrip() {
        let src = "direction: right\na: Alpha\nb: Beta\na -> b: link\n";
        let doc = import_str(src, Format::D2).unwrap();
        let out = export_str(&doc, Format::D2).unwrap();
        assert!(out.contains("direction: right"));
        assert!(out.contains("Alpha"));
        let doc2 = import_str(&out, Format::D2).unwrap();
        let mmd = export_str(&doc2, Format::Mermaid).unwrap();
        assert!(mmd.contains("Alpha"));
    }

    #[test]
    fn dot_import_to_mermaid() {
        let src = r#"digraph G { A [label="Start"] -> B [label="End"]; }"#;
        let doc = import_str(src, Format::Dot).unwrap();
        let out = export_str(&doc, Format::Mermaid).unwrap();
        assert!(out.contains("Start"));
        assert!(out.contains("B"));
    }

    #[test]
    fn plantuml_sequence_import() {
        let src = "@startuml\nAlice -> Bob: hi\n@enduml";
        let doc = import_str(src, Format::PlantUml).unwrap();
        let out = export_str(&doc, Format::Mermaid).unwrap();
        assert!(out.contains("sequenceDiagram"));
        assert!(out.contains("hi"));
    }

    #[test]
    fn plantuml_activity_export_roundtrip() {
        let src = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/activity.puml"),
        )
        .unwrap();
        let doc = import_str(&src, Format::PlantUml).unwrap();
        let out = export_str(&doc, Format::PlantUml).unwrap();
        assert!(out.contains("@startuml"));
        assert!(out.contains("Receive request"));
        let doc2 = import_str(&out, Format::PlantUml).unwrap();
        assert_eq!(
            doc.primary().unwrap().kind(),
            doc2.primary().unwrap().kind()
        );
    }

    #[test]
    fn plantuml_activity_import() {
        let src = "@startuml\nstart\n:Go;\nstop\n@enduml";
        let doc = import_str(src, Format::PlantUml).unwrap();
        assert_eq!(doc.primary().unwrap().kind(), crate::ir::Kind::Flowchart);
        let out = export_str(&doc, Format::Mermaid).unwrap();
        assert!(out.contains("Go"));
    }

    #[test]
    fn plantuml_export_roundtrip() {
        let src = "@startuml\nAlice -> Bob: hi\n@enduml";
        let doc = import_str(src, Format::PlantUml).unwrap();
        let out = export_str(&doc, Format::PlantUml).unwrap();
        assert!(out.contains("@startuml"));
        let doc2 = import_str(&out, Format::PlantUml).unwrap();
        let mmd = export_str(&doc2, Format::Mermaid).unwrap();
        assert!(mmd.contains("sequenceDiagram"));
    }

    #[test]
    fn dot_export_roundtrip() {
        let src = r#"digraph G { A [label="Start"] -> B [label="End"]; }"#;
        let doc = import_str(src, Format::Dot).unwrap();
        let out = export_str(&doc, Format::Dot).unwrap();
        assert!(out.contains("digraph"));
        assert!(out.contains("Start"));
        let doc2 = import_str(&out, Format::Dot).unwrap();
        let mmd = export_str(&doc2, Format::Mermaid).unwrap();
        assert!(mmd.contains("Start"));
    }

    #[test]
    fn mermaid_to_json_to_mermaid() {
        let src = "graph TD\n  A[Start] --> B[End]\n";
        let doc = import_str(src, Format::Mermaid).unwrap();
        let json = export_str(&doc, Format::JsonIr).unwrap();
        let doc2 = import_str(&json, Format::JsonIr).unwrap();
        let out = export_str(&doc2, Format::Mermaid).unwrap();
        assert!(out.contains("A"));
        assert!(out.contains("B"));
    }
}
