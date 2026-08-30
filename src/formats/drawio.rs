//! draw.io (diagrams.net) XML Compatibility adapter (MXGraph subset → flowchart IR).
//!
//! Hand-rolled scanner over the uncompressed `<mxfile>/<diagram>/<mxGraphModel>`
//! interchange form. Maps vertex cells to flowchart nodes (shape inferred from
//! the `style` string) and edge cells to flowchart edges. Multiple `<diagram>`
//! pages become a multi-diagram `Document`. No XML dependency — matches the
//! DOT/D2 hand-rolled scanner approach to keep the footprint small.

use crate::diagram::{Diagram, Edge, EdgeStyle, Node, NodeShape};
use crate::ir::{Document, IrError};

/// Whether source looks like draw.io XML (handles `<?xml?>` prolog).
pub fn is_drawio(source: &str) -> bool {
    let mut t = source.trim_start();
    if t.starts_with("<?xml")
        && let Some(end) = t.find("?>")
    {
        t = t[end + 2..].trim_start();
    }
    t.starts_with("<mxfile") || t.starts_with("<mxGraphModel")
}

pub fn parse(source: &str) -> Result<Vec<Diagram>, IrError> {
    let diagrams = scan_diagrams(source);
    if diagrams.is_empty() {
        return Err(IrError::from(
            "draw.io source has no <diagram> pages with vertex/edge cells",
        ));
    }
    Ok(diagrams)
}

pub fn parse_to_document(source: &str) -> Result<Document, IrError> {
    let diagrams = parse(source)?;
    Ok(Document {
        version: Document::CURRENT_VERSION,
        diagrams: diagrams.into_iter().map(crate::ir::Diagram::Flowchart).collect(),
    })
}

/// Export a Document to draw.io XML (flowchart diagrams only).
pub fn export_document(doc: &Document) -> Result<String, IrError> {
    let flowcharts: Vec<(usize, &Diagram)> = doc
        .diagrams
        .iter()
        .enumerate()
        .filter_map(|(i, d)| match d {
            crate::ir::Diagram::Flowchart(fc) => Some((i, fc)),
            _ => None,
        })
        .collect();

    if flowcharts.is_empty() {
        return Err(IrError::from(
            "draw.io export supports flowchart diagrams only",
        ));
    }

    let mut out = String::from("<mxfile>\n");
    for (i, fc) in &flowcharts {
        out.push_str(&format!(
            "  <diagram id=\"d{i}\" name=\"diagram-{i}\">\n"
        ));
        out.push_str("    <mxGraphModel>\n      <root>\n");
        out.push_str("        <mxCell id=\"0\"/>\n");
        out.push_str("        <mxCell id=\"1\" parent=\"0\"/>\n");
        // Grid layout: one column per node, 160px stride.
        for (n, node) in fc.nodes.iter().enumerate() {
            let style = shape_to_style(node.shape);
            out.push_str(&format!(
                "        <mxCell id=\"n{}\" value=\"{}\" style=\"{}\" vertex=\"1\" parent=\"1\">\n",
                n,
                xml_escape(&node.text),
                style
            ));
            out.push_str(&format!(
                "          <mxGeometry x=\"{}\" y=\"{}\" width=\"120\" height=\"60\" as=\"geometry\"/>\n",
                n * 160,
                n * 100
            ));
            out.push_str("        </mxCell>\n");
        }
        for (e, edge) in fc.edges.iter().enumerate() {
            let src = node_index(fc, &edge.from);
            let tgt = node_index(fc, &edge.to);
            let (src, tgt) = match (src, tgt) {
                (Some(s), Some(t)) => (format!("n{s}"), format!("n{t}")),
                _ => (xml_escape(&edge.from), xml_escape(&edge.to)),
            };
            let style = edge_style_to_style(edge.style);
            out.push_str(&format!(
                "        <mxCell id=\"e{e}\" value=\"{}\" style=\"{}\" edge=\"1\" parent=\"1\" source=\"{}\" target=\"{}\">\n",
                xml_escape(&edge.label),
                style,
                src,
                tgt
            ));
            out.push_str(
                "          <mxGeometry relative=\"1\" as=\"geometry\"/>\n",
            );
            out.push_str("        </mxCell>\n");
        }
        out.push_str("      </root>\n    </mxGraphModel>\n  </diagram>\n");
    }
    out.push_str("</mxfile>\n");
    Ok(out)
}

fn node_index(fc: &Diagram, id: &str) -> Option<usize> {
    fc.nodes.iter().position(|n| n.id == id)
}

fn shape_to_style(shape: NodeShape) -> &'static str {
    match shape {
        NodeShape::Rect => "rounded=0;whiteSpace=wrap;html=1;",
        NodeShape::Diamond => "rhombus;whiteSpace=wrap;html=1;",
        NodeShape::Stadium => "rounded=1;whiteSpace=wrap;html=1;",
        NodeShape::Hexagon => "shape=hexagon;whiteSpace=wrap;html=1;",
        NodeShape::Cylinder => "shape=cylinder;whiteSpace=wrap;html=1;",
        NodeShape::Circle => "ellipse;whiteSpace=wrap;html=1;",
    }
}

fn edge_style_to_style(style: EdgeStyle) -> &'static str {
    match style {
        EdgeStyle::Arrow => "endArrow=classic;html=1;",
        EdgeStyle::Dashed => "endArrow=classic;html=1;dashed=1;",
        EdgeStyle::Thick => "endArrow=classic;html=1;strokeWidth=3;",
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ── Import scanner ──────────────────────────────────────────────────────────

/// One parsed mxCell.
#[derive(Debug, Default)]
struct MxCell {
    id: String,
    value: String,
    style: String,
    vertex: bool,
    edge: bool,
    source: String,
    target: String,
}

fn scan_diagrams(source: &str) -> Vec<Diagram> {
    // Split on <diagram ...>...</diagram> blocks. If there are no <diagram>
    // tags but the source is a bare <mxGraphModel>, treat the whole input as
    // one diagram.
    let blocks: Vec<&str> = diagram_blocks(source);
    blocks.iter().filter_map(|b| build_flowchart(b)).collect()
}

fn diagram_blocks(source: &str) -> Vec<&str> {
    let mut blocks = Vec::new();
    let mut cursor = 0usize;
    while let Some(start) = source[cursor..].find("<diagram") {
        let open_tag_start = cursor + start;
        let after_open = &source[open_tag_start..];
        let tag_end = match after_open.find('>') {
            Some(p) => open_tag_start + p + 1,
            None => break,
        };
        // find matching </diagram>
        let rest = &source[tag_end..];
        if let Some(end) = rest.find("</diagram>") {
            blocks.push(&source[tag_end..tag_end + end]);
            cursor = tag_end + end + "</diagram>".len();
        } else {
            // no close tag; take the rest
            blocks.push(rest);
            break;
        }
    }
    if blocks.is_empty() && source.contains("<mxGraphModel") {
        blocks.push(source);
    }
    blocks
}

fn build_flowchart(block: &str) -> Option<Diagram> {
    let cells = scan_mx_cells(block);
    let mut fc = Diagram::new("TD");
    // vertex cells (skip root/layer cells id 0/1 and parent-only cells)
    for c in &cells {
        if c.vertex && !c.id.is_empty() && c.id != "0" && c.id != "1" {
            let text = decode_entities(&c.value);
            let shape = style_to_shape(&c.style);
            // Use the cell id as the node id; ensure uniqueness.
            if !fc.nodes.iter().any(|n| n.id == c.id) {
                fc.nodes.push(Node {
                    id: c.id.clone(),
                    text,
                    shape,
                    href: None,
                    tooltip: None,
                });
            }
        }
    }
    for c in &cells {
        if c.edge && !c.source.is_empty() && !c.target.is_empty() {
            let label = decode_entities(&c.value);
            let style = style_to_edge_style(&c.style);
            // Only add edges whose endpoints exist (draw.io files may reference
            // cells outside this page); missing endpoints are skipped.
            if fc.nodes.iter().any(|n| n.id == c.source)
                && fc.nodes.iter().any(|n| n.id == c.target)
            {
                fc.edges.push(Edge {
                    from: c.source.clone(),
                    to: c.target.clone(),
                    label,
                    style,
                });
            }
        }
    }
    if fc.nodes.is_empty() {
        None
    } else {
        Some(fc)
    }
}

/// Scan all `<mxCell ...>` tags in a block and parse their attributes.
fn scan_mx_cells(block: &str) -> Vec<MxCell> {
    let mut cells = Vec::new();
    let mut search = block;
    while let Some(pos) = search.find("<mxCell") {
        let after = &search[pos + "<mxCell".len()..];
        // tag ends at the first '>' (attributes don't contain '>' when escaped).
        let tag_end = match after.find('>') {
            Some(p) => p,
            None => break,
        };
        let tag_body = &after[..tag_end];
        let body = tag_body.trim_end_matches('/').trim();
        let cell = parse_cell_attrs(body);
        if cell.vertex || cell.edge {
            cells.push(cell);
        }
        search = &after[tag_end + 1..];
    }
    cells
}

fn parse_cell_attrs(body: &str) -> MxCell {
    let mut cell = MxCell::default();
    for (k, v) in parse_attrs(body) {
        match k.as_str() {
            "id" => cell.id = v,
            "value" => cell.value = v,
            "style" => cell.style = v,
            "vertex" => cell.vertex = v == "1",
            "edge" => cell.edge = v == "1",
            "source" => cell.source = v,
            "target" => cell.target = v,
            _ => {}
        }
    }
    cell
}

/// Minimal XML attribute parser: `key="value"` pairs, value may contain escaped
/// entities. Handles values containing `;` (draw.io styles) since we only split
/// on the closing quote.
fn parse_attrs(body: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // skip whitespace
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        // read key up to '=' or whitespace
        let key_start = i;
        while i < bytes.len()
            && bytes[i] != b'='
            && !bytes[i].is_ascii_whitespace()
        {
            i += 1;
        }
        let key = &body[key_start..i];
        if key.is_empty() {
            break;
        }
        // skip whitespace, expect '='
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            // boolean-style attr; skip
            continue;
        }
        i += 1; // consume '='
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'"' {
            continue;
        }
        i += 1; // consume opening quote
        let val_start = i;
        while i < bytes.len() && bytes[i] != b'"' {
            i += 1;
        }
        let value = &body[val_start..i];
        if i < bytes.len() {
            i += 1; // consume closing quote
        }
        out.push((key.to_string(), value.to_string()));
    }
    out
}

fn style_to_shape(style: &str) -> NodeShape {
    let s = style;
    if s.contains("rhombus") {
        NodeShape::Diamond
    } else if s.contains("ellipse") {
        NodeShape::Circle
    } else if s.contains("shape=cylinder") {
        NodeShape::Cylinder
    } else if s.contains("shape=hexagon") {
        NodeShape::Hexagon
    } else if s.contains("rounded=1") {
        NodeShape::Stadium
    } else {
        NodeShape::Rect
    }
}

fn style_to_edge_style(style: &str) -> EdgeStyle {
    if style.contains("dashed=1") {
        EdgeStyle::Dashed
    } else if style.contains("strokeWidth=3") || style.contains("strokeWidth=4") {
        EdgeStyle::Thick
    } else {
        EdgeStyle::Arrow
    }
}

fn decode_entities(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "<?xml version=\"1.0\"?>\n<mxfile>\n  <diagram id=\"d0\" name=\"Page-1\">\n    <mxGraphModel>\n      <root>\n        <mxCell id=\"0\"/>\n        <mxCell id=\"1\" parent=\"0\"/>\n        <mxCell id=\"2\" value=\"Start\" style=\"rounded=0;whiteSpace=wrap;html=1;\" vertex=\"1\" parent=\"1\">\n          <mxGeometry x=\"40\" y=\"40\" width=\"120\" height=\"60\" as=\"geometry\"/>\n        </mxCell>\n        <mxCell id=\"3\" value=\"Decide\" style=\"rhombus;whiteSpace=wrap;html=1;\" vertex=\"1\" parent=\"1\">\n          <mxGeometry x=\"40\" y=\"160\" width=\"120\" height=\"80\" as=\"geometry\"/>\n        </mxCell>\n        <mxCell id=\"4\" value=\"End\" style=\"rounded=1;whiteSpace=wrap;html=1;\" vertex=\"1\" parent=\"1\">\n          <mxGeometry x=\"40\" y=\"280\" width=\"120\" height=\"60\" as=\"geometry\"/>\n        </mxCell>\n        <mxCell id=\"5\" value=\"go\" style=\"endArrow=classic;html=1;\" edge=\"1\" parent=\"1\" source=\"2\" target=\"3\">\n          <mxGeometry relative=\"1\" as=\"geometry\"/>\n        </mxCell>\n        <mxCell id=\"6\" style=\"endArrow=classic;html=1;dashed=1;\" edge=\"1\" parent=\"1\" source=\"3\" target=\"4\">\n          <mxGeometry relative=\"1\" as=\"geometry\"/>\n        </mxCell>\n      </root>\n    </mxGraphModel>\n  </diagram>\n</mxfile>\n";

    #[test]
    fn detects_drawio() {
        assert!(is_drawio(SAMPLE));
        assert!(is_drawio("<mxGraphModel><root/></mxGraphModel>"));
        assert!(!is_drawio("digraph G { A -> B }"));
        assert!(!is_drawio("graph TD\n  A-->B\n"));
    }

    #[test]
    fn parses_nodes_and_edges() {
        let doc = parse_to_document(SAMPLE).unwrap();
        let fc = doc.primary().unwrap();
        assert_eq!(fc.kind(), crate::ir::Kind::Flowchart);
        let f = match fc {
            crate::ir::Diagram::Flowchart(f) => f,
            _ => unreachable!(),
        };
        assert_eq!(f.nodes.len(), 3);
        assert_eq!(f.edges.len(), 2);
        assert_eq!(f.nodes[0].text, "Start");
        assert_eq!(f.nodes[0].shape, NodeShape::Rect);
        assert_eq!(f.nodes[1].shape, NodeShape::Diamond);
        assert_eq!(f.nodes[2].shape, NodeShape::Stadium);
        assert_eq!(f.edges[0].label, "go");
        assert_eq!(f.edges[1].style, EdgeStyle::Dashed);
    }

    #[test]
    fn export_roundtrip() {
        let doc = parse_to_document(SAMPLE).unwrap();
        let xml = export_document(&doc).unwrap();
        assert!(xml.contains("<mxfile"));
        assert!(xml.contains("value=\"Start\""));
        assert!(xml.contains("rhombus"));
        // Re-import the exported XML and check structure survives.
        let doc2 = parse_to_document(&xml).unwrap();
        let f1 = match doc.primary().unwrap() {
            crate::ir::Diagram::Flowchart(f) => f,
            _ => unreachable!(),
        };
        let f2 = match doc2.primary().unwrap() {
            crate::ir::Diagram::Flowchart(f) => f,
            _ => unreachable!(),
        };
        assert_eq!(f1.nodes.len(), f2.nodes.len());
        assert_eq!(f1.edges.len(), f2.edges.len());
        assert!(f2.nodes.iter().any(|n| n.text == "Start"));
        assert!(f2.nodes.iter().any(|n| n.shape == NodeShape::Diamond));
    }

    #[test]
    fn rejects_empty() {
        assert!(parse_to_document("<mxfile></mxfile>").is_err());
    }

    #[test]
    fn multi_diagram_pages() {
        let src = "<mxfile>\n  <diagram id=\"a\"><mxGraphModel><root>\n    <mxCell id=\"2\" value=\"A\" style=\"rounded=0;\" vertex=\"1\" parent=\"1\"/>\n    <mxCell id=\"3\" value=\"B\" style=\"rounded=0;\" vertex=\"1\" parent=\"1\"/>\n    <mxCell id=\"4\" style=\"endArrow=classic;\" edge=\"1\" source=\"2\" target=\"3\"/>\n  </root></mxGraphModel></diagram>\n  <diagram id=\"b\"><mxGraphModel><root>\n    <mxCell id=\"2\" value=\"X\" style=\"ellipse;\" vertex=\"1\" parent=\"1\"/>\n  </root></mxGraphModel></diagram>\n</mxfile>\n";
        let doc = parse_to_document(src).unwrap();
        assert_eq!(doc.diagrams.len(), 2);
    }

    #[test]
    fn xml_escaping_in_labels() {
        let src = "<mxfile><diagram><mxGraphModel><root>\n  <mxCell id=\"2\" value=\"a &lt; b &amp; c\" style=\"rounded=0;\" vertex=\"1\" parent=\"1\"/>\n</root></mxGraphModel></diagram></mxfile>";
        let doc = parse_to_document(src).unwrap();
        let f = match doc.primary().unwrap() {
            crate::ir::Diagram::Flowchart(f) => f,
            _ => unreachable!(),
        };
        assert_eq!(f.nodes[0].text, "a < b & c");
        let xml = export_document(&doc).unwrap();
        assert!(xml.contains("a &lt; b &amp; c"));
    }
}
