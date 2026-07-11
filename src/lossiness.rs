//! Export lossiness analysis: what IR semantics a target Format cannot represent.

use crate::formats::Format;
use crate::ir::{Diagram, Document};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct LossinessReport {
    pub target_format: String,
    pub export_supported: bool,
    pub lossless: bool,
    pub diagram_count: usize,
    pub warnings: Vec<LossWarning>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LossWarning {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagram_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
}

/// Analyze export fidelity from a Document to a target Format.
pub fn report(doc: &Document, format: Format) -> LossinessReport {
    match format {
        Format::JsonIr => LossinessReport {
            target_format: format.as_str().into(),
            export_supported: true,
            lossless: true,
            diagram_count: doc.diagrams.len(),
            warnings: Vec::new(),
        },
        Format::Dot => dot_report(doc),
        Format::PlantUml => plantuml_report(doc),
        Format::Mermaid => mermaid_report(doc),
    }
}

fn dot_report(doc: &Document) -> LossinessReport {
    let mut warnings = Vec::new();
    let flowchart_count = doc
        .diagrams
        .iter()
        .filter(|d| matches!(d, Diagram::Flowchart(_)))
        .count();

    if flowchart_count == 0 {
        return LossinessReport {
            target_format: "dot".into(),
            export_supported: false,
            lossless: false,
            diagram_count: doc.diagrams.len(),
            warnings: vec![LossWarning {
                diagram_index: None,
                kind: None,
                code: "format.unsupported_export".into(),
                message: "DOT export supports flowchart diagrams only".into(),
                count: None,
            }],
        };
    }

    let skipped = doc.diagrams.len() - flowchart_count;
    if skipped > 0 {
        warnings.push(LossWarning {
            diagram_index: None,
            kind: None,
            code: "document.non_flowchart_skipped".into(),
            message: "non-flowchart diagrams are omitted from DOT export".into(),
            count: Some(skipped),
        });
    }

    if doc.diagrams.len() > 1 {
        warnings.push(LossWarning {
            diagram_index: None,
            kind: None,
            code: "document.multi_diagram_blocks".into(),
            message: "multi-diagram export emits separate digraph blocks with // diagram N: comments".into(),
            count: Some(flowchart_count),
        });
    }

    for (i, d) in doc.diagrams.iter().enumerate() {
        warnings.extend(diagram_dot_warnings(i, d));
    }

    let lossless = warnings.iter().all(|w| {
        w.code == "document.multi_diagram_blocks"
    });

    LossinessReport {
        target_format: "dot".into(),
        export_supported: true,
        lossless,
        diagram_count: doc.diagrams.len(),
        warnings,
    }
}

fn plantuml_report(doc: &Document) -> LossinessReport {
    let mut warnings = Vec::new();
    let supported_count = doc
        .diagrams
        .iter()
        .filter(|d| matches!(d, Diagram::Sequence(_) | Diagram::Class(_)))
        .count();

    if supported_count == 0 {
        return LossinessReport {
            target_format: "plantuml".into(),
            export_supported: false,
            lossless: false,
            diagram_count: doc.diagrams.len(),
            warnings: vec![LossWarning {
                diagram_index: None,
                kind: None,
                code: "format.unsupported_export".into(),
                message: "PlantUML export supports sequence and class diagrams only".into(),
                count: None,
            }],
        };
    }

    let skipped = doc.diagrams.len() - supported_count;
    if skipped > 0 {
        warnings.push(LossWarning {
            diagram_index: None,
            kind: None,
            code: "document.non_puml_skipped".into(),
            message: "flowchart/gantt diagrams are omitted from PlantUML export".into(),
            count: Some(skipped),
        });
    }

    if doc.diagrams.len() > 1 {
        warnings.push(LossWarning {
            diagram_index: None,
            kind: None,
            code: "document.multi_diagram_blocks".into(),
            message: "multi-diagram export emits separate @startuml blocks with ' diagram N: comments".into(),
            count: Some(supported_count),
        });
    }

    let lossless = warnings.iter().all(|w| w.code == "document.multi_diagram_blocks");

    LossinessReport {
        target_format: "plantuml".into(),
        export_supported: true,
        lossless,
        diagram_count: doc.diagrams.len(),
        warnings,
    }
}

fn mermaid_report(doc: &Document) -> LossinessReport {
    let mut warnings = Vec::new();

    if doc.diagrams.is_empty() {
        warnings.push(LossWarning {
            diagram_index: None,
            kind: None,
            code: "document.empty".into(),
            message: "document has no diagrams".into(),
            count: None,
        });
    }

    if doc.diagrams.len() > 1 {
        warnings.push(LossWarning {
            diagram_index: None,
            kind: None,
            code: "document.multi_diagram_markers".into(),
            message: "multi-diagram export uses %% diagram N: markers (Mermaid convention, not native multi-figure syntax)".into(),
            count: Some(doc.diagrams.len()),
        });
    }

    for (i, d) in doc.diagrams.iter().enumerate() {
        warnings.extend(diagram_mermaid_warnings(i, d));
    }

    let lossless = warnings.iter().all(|w| {
        w.code == "document.multi_diagram_markers"
    });

    LossinessReport {
        target_format: "mermaid".into(),
        export_supported: true,
        lossless,
        diagram_count: doc.diagrams.len(),
        warnings,
    }
}

fn diagram_dot_warnings(index: usize, d: &Diagram) -> Vec<LossWarning> {
    let kind = d.kind().to_string();
    match d {
        Diagram::Flowchart(fc) => {
            let mut out = Vec::new();
            let hrefs = fc.nodes.iter().filter(|n| n.href.is_some()).count();
            if hrefs > 0 {
                out.push(warn(
                    index,
                    &kind,
                    "flowchart.node.href",
                    "node hyperlink (href) is not written to DOT export",
                    hrefs,
                ));
            }
            let tooltips = fc.nodes.iter().filter(|n| n.tooltip.is_some()).count();
            if tooltips > 0 {
                out.push(warn(
                    index,
                    &kind,
                    "flowchart.node.tooltip",
                    "node tooltip is not written to DOT export",
                    tooltips,
                ));
            }
            if !fc.styles.is_empty() {
                out.push(warn(
                    index,
                    &kind,
                    "flowchart.styles",
                    "per-node style properties are not written to DOT export",
                    fc.styles.len(),
                ));
            }
            if !fc.class_defs.is_empty() || !fc.class_applies.is_empty() {
                out.push(warn(
                    index,
                    &kind,
                    "flowchart.class_defs",
                    "classDef/class assignments are not written to DOT export",
                    fc.class_defs.len() + fc.class_applies.len(),
                ));
            }
            out
        }
        Diagram::Sequence(_) | Diagram::Class(_) | Diagram::Gantt(_) => Vec::new(),
    }
}

fn diagram_mermaid_warnings(index: usize, d: &Diagram) -> Vec<LossWarning> {
    let kind = d.kind().to_string();
    match d {
        Diagram::Flowchart(fc) => {
            let mut out = Vec::new();
            let hrefs = fc.nodes.iter().filter(|n| n.href.is_some()).count();
            if hrefs > 0 {
                out.push(warn(
                    index,
                    &kind,
                    "flowchart.node.href",
                    "node hyperlink (href) is not written to Mermaid export",
                    hrefs,
                ));
            }
            let tooltips = fc.nodes.iter().filter(|n| n.tooltip.is_some()).count();
            if tooltips > 0 {
                out.push(warn(
                    index,
                    &kind,
                    "flowchart.node.tooltip",
                    "node tooltip is not written to Mermaid export",
                    tooltips,
                ));
            }
            out
        }
        Diagram::Sequence(_) | Diagram::Class(_) | Diagram::Gantt(_) => Vec::new(),
    }
}

fn warn(index: usize, kind: &str, code: &str, message: &str, count: usize) -> LossWarning {
    LossWarning {
        diagram_index: Some(index),
        kind: Some(kind.into()),
        code: code.into(),
        message: message.into(),
        count: Some(count),
    }
}

/// One-line human summary for CLI export stderr.
pub fn summary_line(report: &LossinessReport) -> Option<String> {
    if !report.export_supported {
        return Some(format!(
            "export blocked: {}",
            report.warnings.first()?.message
        ));
    }
    if report.lossless {
        return None;
    }
    let n = report
        .warnings
        .iter()
        .filter(|w| {
            w.code != "document.multi_diagram_markers"
                && w.code != "document.multi_diagram_blocks"
        })
        .count();
    if n == 0 {
        return None;
    }
    Some(format!("lossiness: {n} warning(s) — run `diagram lossiness` for details"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::{Diagram as FcDiagram, Node, NodeShape};
    use crate::ir;

    #[test]
    fn json_export_is_lossless() {
        let doc = ir::from_mermaid("graph TD\n  A-->B\n").unwrap();
        let r = report(&doc, Format::JsonIr);
        assert!(r.export_supported);
        assert!(r.lossless);
        assert!(r.warnings.is_empty());
    }

    #[test]
    fn plantuml_export_sequence_supported() {
        let doc = ir::from_mermaid("sequenceDiagram\n  A->>B: hi\n").unwrap();
        let r = report(&doc, Format::PlantUml);
        assert!(r.export_supported);
        assert!(r.lossless);
    }

    #[test]
    fn plantuml_export_rejects_flowchart_only() {
        let doc = ir::from_mermaid("graph TD\n  A-->B\n").unwrap();
        let r = report(&doc, Format::PlantUml);
        assert!(!r.export_supported);
        assert_eq!(r.warnings[0].code, "format.unsupported_export");
    }

    #[test]
    fn dot_export_flowchart_supported() {
        let doc = ir::from_mermaid("graph TD\n  A-->B\n").unwrap();
        let r = report(&doc, Format::Dot);
        assert!(r.export_supported);
        assert!(r.lossless);
    }

    #[test]
    fn dot_export_rejects_sequence_only() {
        let doc = ir::from_mermaid("sequenceDiagram\n  A->>B: hi\n").unwrap();
        let r = report(&doc, Format::Dot);
        assert!(!r.export_supported);
        assert_eq!(r.warnings[0].code, "format.unsupported_export");
    }

    #[test]
    fn mermaid_reports_href_loss() {
        let mut fc = FcDiagram::new("TD");
        fc.add_node(Node {
            id: "A".into(),
            text: "A".into(),
            shape: NodeShape::Rect,
            href: Some("https://example.com".into()),
            tooltip: None,
        })
        .unwrap();
        let doc = Document::single(ir::Diagram::Flowchart(fc));
        let r = report(&doc, Format::Mermaid);
        assert!(r.export_supported);
        assert!(!r.lossless);
        assert!(r.warnings.iter().any(|w| w.code == "flowchart.node.href"));
    }

    #[test]
    fn multi_diagram_info_warning_only() {
        let doc = Document {
            version: 1,
            diagrams: vec![
                ir::from_mermaid("graph TD\n  A-->B\n")
                    .unwrap()
                    .primary()
                    .unwrap()
                    .clone(),
                ir::from_mermaid("sequenceDiagram\n  A->>B: hi\n")
                    .unwrap()
                    .primary()
                    .unwrap()
                    .clone(),
            ],
        };
        let r = report(&doc, Format::Mermaid);
        assert!(r.export_supported);
        assert!(r.lossless);
        assert!(r.warnings.iter().any(|w| w.code == "document.multi_diagram_markers"));
    }
}
