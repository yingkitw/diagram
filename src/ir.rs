//! Canonical diagram IR: Document / Diagram / Kind.

use crate::class::{self, ClassDiagram};
use crate::diagram as flowchart;
use crate::gantt::{self, GanttDiagram};
use crate::layout;
use crate::parser;
use crate::renderer::{self, Theme};
use crate::sequence::{self, SequenceDiagram};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Diagram kind (semantics), independent of Format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Flowchart,
    Sequence,
    Class,
    Gantt,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flowchart => write!(f, "flowchart"),
            Self::Sequence => write!(f, "sequence"),
            Self::Class => write!(f, "class"),
            Self::Gantt => write!(f, "gantt"),
        }
    }
}

/// A single typed diagram in the canonical IR.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "lowercase")]
pub enum Diagram {
    Flowchart(flowchart::Diagram),
    Sequence(SequenceDiagram),
    Class(ClassDiagram),
    Gantt(GanttDiagram),
}

impl Diagram {
    pub fn kind(&self) -> Kind {
        match self {
            Self::Flowchart(_) => Kind::Flowchart,
            Self::Sequence(_) => Kind::Sequence,
            Self::Class(_) => Kind::Class,
            Self::Gantt(_) => Kind::Gantt,
        }
    }

    pub fn to_mermaid(&self) -> String {
        match self {
            Self::Flowchart(d) => d.to_mermaid(),
            Self::Sequence(d) => d.to_mermaid(),
            Self::Class(d) => d.to_mermaid(),
            Self::Gantt(d) => d.to_mermaid(),
        }
    }

    pub fn render_svg(&self, theme: Theme) -> String {
        match self {
            Self::Flowchart(d) => {
                let laid = layout::layout(d);
                renderer::render_svg_with_theme(&laid, theme)
            }
            Self::Sequence(d) => sequence::render_svg(d, theme),
            Self::Class(d) => class::render_svg(d, theme),
            Self::Gantt(d) => gantt::render_svg(d, theme),
        }
    }
}

/// Top-level IR unit: one or more diagrams plus schema version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub version: u32,
    pub diagrams: Vec<Diagram>,
}

impl Document {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn single(diagram: Diagram) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            diagrams: vec![diagram],
        }
    }

    pub fn primary(&self) -> Option<&Diagram> {
        self.diagrams.first()
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    pub fn to_mermaid(&self) -> Result<String, String> {
        match self.diagrams.as_slice() {
            [] => Err("document has no diagrams".into()),
            [one] => Ok(one.to_mermaid()),
            _ => Err("multi-diagram Mermaid export not supported yet".into()),
        }
    }

    pub fn render_svg(&self, theme: Theme) -> Result<String, String> {
        match self.diagrams.as_slice() {
            [] => Err("document has no diagrams".into()),
            [one] => Ok(one.render_svg(theme)),
            _ => Err("multi-diagram render not supported yet".into()),
        }
    }
}

/// Human-readable summary lines for CLI `info`.
pub fn info_lines(path: &str, doc: &Document) -> Vec<String> {
    let mut lines = vec![
        format!("File: {path}"),
        format!("IR version: {}", doc.version),
        format!("Diagrams: {}", doc.diagrams.len()),
    ];
    let Some(d) = doc.primary() else {
        return lines;
    };
    lines.push(format!("Kind: {}", d.kind()));
    match d {
        Diagram::Flowchart(fc) => {
            let mut shapes = [0usize; 6];
            for n in &fc.nodes {
                shapes[match n.shape {
                    flowchart::NodeShape::Rect => 0,
                    flowchart::NodeShape::Diamond => 1,
                    flowchart::NodeShape::Stadium => 2,
                    flowchart::NodeShape::Hexagon => 3,
                    flowchart::NodeShape::Cylinder => 4,
                    flowchart::NodeShape::Circle => 5,
                }] += 1;
            }
            lines.push(format!("Direction: {}", fc.rankdir));
            lines.push(format!("Nodes: {}", fc.nodes.len()));
            lines.push(format!("  rect:     {}", shapes[0]));
            lines.push(format!("  diamond:  {}", shapes[1]));
            lines.push(format!("  stadium:  {}", shapes[2]));
            lines.push(format!("  hexagon:  {}", shapes[3]));
            lines.push(format!("  cylinder: {}", shapes[4]));
            lines.push(format!("  circle:   {}", shapes[5]));
            lines.push(format!("Edges: {}", fc.edges.len()));
        }
        Diagram::Sequence(s) => {
            lines.push(format!("Participants: {}", s.participants.len()));
            lines.push(format!("Messages: {}", s.messages.len()));
        }
        Diagram::Class(c) => {
            lines.push(format!("Classes: {}", c.classes.len()));
            lines.push(format!("Relations: {}", c.relations.len()));
        }
        Diagram::Gantt(g) => {
            if !g.title.is_empty() {
                lines.push(format!("Title: {}", g.title));
            }
            lines.push(format!("Tasks: {}", g.tasks.len()));
        }
    }
    lines
}

/// JSON summary for MCP `get_info`.
pub fn info_json(path: &str, doc: &Document) -> serde_json::Value {
    let Some(d) = doc.primary() else {
        return serde_json::json!({
            "path": path,
            "ir_version": doc.version,
            "diagrams": doc.diagrams.len(),
        });
    };
    match d {
        Diagram::Flowchart(fc) => {
            let mut shapes = [0usize; 6];
            for n in &fc.nodes {
                shapes[match n.shape {
                    flowchart::NodeShape::Rect => 0,
                    flowchart::NodeShape::Diamond => 1,
                    flowchart::NodeShape::Stadium => 2,
                    flowchart::NodeShape::Hexagon => 3,
                    flowchart::NodeShape::Cylinder => 4,
                    flowchart::NodeShape::Circle => 5,
                }] += 1;
            }
            serde_json::json!({
                "path": path,
                "ir_version": doc.version,
                "diagrams": doc.diagrams.len(),
                "kind": "flowchart",
                "direction": fc.rankdir,
                "nodes": fc.nodes.len(),
                "edges": fc.edges.len(),
                "shapes": {
                    "rect": shapes[0],
                    "diamond": shapes[1],
                    "stadium": shapes[2],
                    "hexagon": shapes[3],
                    "cylinder": shapes[4],
                    "circle": shapes[5],
                },
            })
        }
        Diagram::Sequence(s) => serde_json::json!({
            "path": path,
            "ir_version": doc.version,
            "diagrams": doc.diagrams.len(),
            "kind": "sequence",
            "participants": s.participants.len(),
            "messages": s.messages.len(),
        }),
        Diagram::Class(c) => serde_json::json!({
            "path": path,
            "ir_version": doc.version,
            "diagrams": doc.diagrams.len(),
            "kind": "class",
            "classes": c.classes.len(),
            "relations": c.relations.len(),
        }),
        Diagram::Gantt(g) => serde_json::json!({
            "path": path,
            "ir_version": doc.version,
            "diagrams": doc.diagrams.len(),
            "kind": "gantt",
            "title": g.title,
            "tasks": g.tasks.len(),
        }),
    }
}

#[derive(Debug, Clone)]
pub struct IrError {
    pub message: String,
}

impl fmt::Display for IrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for IrError {}

impl From<String> for IrError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

impl From<&str> for IrError {
    fn from(message: &str) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Parse Mermaid Compatibility source into a single-diagram Document.
pub fn from_mermaid(source: &str) -> Result<Document, IrError> {
    let diagram = if sequence::is_sequence(source) {
        Diagram::Sequence(sequence::parse(source).map_err(|e| e.to_string())?)
    } else if class::is_class(source) {
        Diagram::Class(class::parse(source).map_err(|e| e.to_string())?)
    } else if gantt::is_gantt(source) {
        Diagram::Gantt(gantt::parse(source).map_err(|e| e.to_string())?)
    } else {
        Diagram::Flowchart(parser::parse(source).map_err(|e| e.to_string())?)
    };
    Ok(Document::single(diagram))
}

/// Load a Document from a file path (Mermaid or JSON IR, by detection).
pub fn load_path(path: &str) -> Result<Document, IrError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read '{path}': {e}"))?;
    crate::formats::import_str(&content, crate::formats::detect(&content, Some(path)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flowchart_roundtrip_json() {
        let src = "graph TD\n  A-->B\n";
        let doc = from_mermaid(src).unwrap();
        assert_eq!(doc.primary().unwrap().kind(), Kind::Flowchart);
        let json = doc.to_json().unwrap();
        let doc2 = Document::from_json(&json).unwrap();
        assert_eq!(doc2.version, 1);
        assert_eq!(doc2.primary().unwrap().kind(), Kind::Flowchart);
        assert!(doc2.to_mermaid().unwrap().contains("A"));
    }

    #[test]
    fn sequence_kind() {
        let doc = from_mermaid("sequenceDiagram\n  A->>B: hi\n").unwrap();
        assert_eq!(doc.primary().unwrap().kind(), Kind::Sequence);
        assert!(doc.render_svg(Theme::Dark).unwrap().contains("<svg"));
    }
}
