//! Kind-aware diagram scaffolds for CLI and MCP generation.

use crate::formats::{self, Format};
use crate::ir::{self, Document, Kind, IrError};

/// Parse a kind name from CLI/MCP input.
pub fn parse_kind(s: &str) -> Result<Kind, String> {
    match s.to_lowercase().as_str() {
        "flowchart" | "graph" => Ok(Kind::Flowchart),
        "sequence" | "sequencediagram" => Ok(Kind::Sequence),
        "class" | "classdiagram" => Ok(Kind::Class),
        "gantt" => Ok(Kind::Gantt),
        _ => Err(format!(
            "unknown kind '{s}'; expected flowchart, sequence, class, or gantt"
        )),
    }
}

/// Minimal Mermaid scaffold for each diagram kind.
pub fn mermaid_scaffold(kind: Kind) -> &'static str {
    match kind {
        Kind::Flowchart => "graph TD\n    A[Start] --> B[End]\n",
        Kind::Sequence => {
            "sequenceDiagram\n    participant A\n    participant B\n    A->>B: Message\n"
        }
        Kind::Class => {
            "classDiagram\n    class Example {\n        +field\n        +method()\n    }\n"
        }
        Kind::Gantt => {
            "gantt\n    title Project Plan\n    dateFormat YYYY-MM-DD\n    section Phase 1\n    Task :a1, 2024-01-01, 7d\n"
        }
    }
}

/// Minimal DOT digraph scaffold for flowcharts.
pub fn dot_scaffold() -> &'static str {
    "digraph G {\n    A [label=\"Start\"];\n    B [label=\"End\"];\n    A -> B;\n}\n"
}

/// Build a canonical Document from a kind scaffold.
pub fn scaffold_document(kind: Kind) -> Result<Document, IrError> {
    ir::from_mermaid(mermaid_scaffold(kind))
}

/// Write a new scaffold file (refuses to overwrite). Format follows output extension.
pub fn write_scaffold(kind_str: &str, path: &str) -> Result<Kind, String> {
    if std::path::Path::new(path).exists() {
        return Err(format!("refusing to overwrite existing file '{path}'"));
    }
    let kind = parse_kind(kind_str)?;
    let format = formats::detect("", Some(path));
    match format {
        Format::JsonIr => {
            let doc = scaffold_document(kind).map_err(|e| e.to_string())?;
            formats::export_path(&doc, path, Some(Format::JsonIr)).map_err(|e| e.to_string())?;
        }
        Format::Mermaid => {
            std::fs::write(path, mermaid_scaffold(kind))
                .map_err(|e| format!("Failed to write '{path}': {e}"))?;
        }
        Format::Dot => {
            if kind != Kind::Flowchart {
                return Err(format!(
                    "DOT scaffolds only support flowchart (got {kind})"
                ));
            }
            std::fs::write(path, dot_scaffold())
                .map_err(|e| format!("Failed to write '{path}': {e}"))?;
        }
    }
    Ok(kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffolds_parse_for_all_kinds() {
        for kind in [
            Kind::Flowchart,
            Kind::Sequence,
            Kind::Class,
            Kind::Gantt,
        ] {
            let doc = scaffold_document(kind).unwrap();
            assert_eq!(doc.primary().unwrap().kind(), kind);
        }
    }

    #[test]
    fn write_mermaid_scaffold() {
        let path = std::env::temp_dir().join(format!("diagram_create_{}.mmd", std::process::id()));
        let p = path.to_str().unwrap();
        let kind = write_scaffold("flowchart", p).unwrap();
        assert_eq!(kind, Kind::Flowchart);
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("graph TD"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn write_json_scaffold() {
        let path = std::env::temp_dir().join(format!("diagram_create_{}.json", std::process::id()));
        let p = path.to_str().unwrap();
        let kind = write_scaffold("sequence", p).unwrap();
        assert_eq!(kind, Kind::Sequence);
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("\"kind\": \"sequence\"") || body.contains("\"kind\":\"sequence\""));
        let _ = std::fs::remove_file(&path);
    }
}
