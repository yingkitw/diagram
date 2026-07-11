//! Format detection and import/export adapters around the canonical IR.

pub mod dot;
pub mod mermaid;
pub mod plantuml;

use crate::ir::{Document, IrError};

/// Concrete serialization format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Mermaid,
    JsonIr,
    Dot,
    PlantUml,
}

impl Format {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mermaid" | "mmd" => Some(Self::Mermaid),
            "json" | "ir" | "json-ir" | "json_ir" => Some(Self::JsonIr),
            "dot" | "graphviz" | "gv" => Some(Self::Dot),
            "plantuml" | "puml" => Some(Self::PlantUml),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mermaid => "mermaid",
            Self::JsonIr => "json",
            Self::Dot => "dot",
            Self::PlantUml => "plantuml",
        }
    }
}

/// Detect format from content and optional path extension.
pub fn detect(source: &str, path: Option<&str>) -> Format {
    let trimmed = source.trim_start();
    if trimmed.starts_with('{') {
        return Format::JsonIr;
    }
    if plantuml::is_plantuml(source) {
        return Format::PlantUml;
    }
    if dot::is_dot(source) {
        return Format::Dot;
    }
    if let Some(p) = path {
        let lower = p.to_lowercase();
        if lower.ends_with(".json") {
            return Format::JsonIr;
        }
        if lower.ends_with(".dot") || lower.ends_with(".gv") {
            return Format::Dot;
        }
        if lower.ends_with(".puml") || lower.ends_with(".plantuml") {
            return Format::PlantUml;
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
        Format::PlantUml => plantuml::parse_to_document(source),
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
        Format::PlantUml => plantuml::export_document(doc),
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
    let format = to.unwrap_or_else(|| {
        let lower = path.to_lowercase();
        if lower.ends_with(".json") {
            Format::JsonIr
        } else if lower.ends_with(".dot") || lower.ends_with(".gv") {
            Format::Dot
        } else if lower.ends_with(".puml") || lower.ends_with(".plantuml") {
            Format::PlantUml
        } else {
            Format::Mermaid
        }
    });
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
    let format = to.unwrap_or_else(|| {
        let lower = path.to_lowercase();
        if lower.ends_with(".json") {
            Format::JsonIr
        } else if lower.ends_with(".dot") || lower.ends_with(".gv") {
            Format::Dot
        } else if lower.ends_with(".puml") || lower.ends_with(".plantuml") {
            Format::PlantUml
        } else {
            Format::Mermaid
        }
    });
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
