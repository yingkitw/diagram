//! Graphviz DOT Compatibility adapter (digraph subset → flowchart IR).

use crate::diagram::{Diagram, Edge, EdgeStyle, Node, NodeShape, NodeStyle, Subgraph};
use crate::ir::{Document, IrError};
use std::collections::HashMap;

/// Whether source looks like Graphviz DOT (not Mermaid `graph TD`).
pub fn is_dot(source: &str) -> bool {
    let trimmed = source.trim_start();
    if trimmed.starts_with("digraph") || trimmed.starts_with("strict digraph") {
        return true;
    }
    let graph_line = if let Some(rest) = trimmed.strip_prefix("strict graph ") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("graph ") {
        rest
    } else {
        return false;
    };
    let head = graph_line
        .split(|c: char| c.is_whitespace() || c == '{' || c == '[' || c == ';')
        .next()
        .unwrap_or("");
    if matches!(
        head.to_uppercase().as_str(),
        "TD" | "LR" | "BT" | "RL" | "TB"
    ) {
        return false;
    }
    graph_line.contains('{')
}

pub fn parse(source: &str) -> Result<Diagram, IrError> {
    let mut p = Parser::new(source);
    p.parse_document()
}

pub fn parse_to_document(source: &str) -> Result<Document, IrError> {
    Ok(Document::single(crate::ir::Diagram::Flowchart(parse(source)?)))
}

/// Export a Document to Graphviz DOT (flowchart diagrams only).
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
        return Err(IrError::from("DOT export supports flowchart diagrams only"));
    }

    let multi = doc.diagrams.len() > 1;
    let blocks: Vec<String> = flowcharts
        .into_iter()
        .map(|(i, fc)| {
            let mut block = String::new();
            if multi {
                block.push_str(&format!("// diagram {i}: flowchart\n"));
            }
            let name = if multi {
                format!("diagram_{i}")
            } else {
                "G".into()
            };
            block.push_str(&export_flowchart(fc, &name));
            block
        })
        .collect();

    Ok(blocks.join("\n\n"))
}

fn export_flowchart(d: &Diagram, name: &str) -> String {
    let mut out = String::new();
    out.push_str("digraph ");
    out.push_str(name);
    out.push_str(" {\n");

    let rankdir = map_rankdir_out(&d.rankdir);
    if rankdir != "TB" {
        out.push_str("    rankdir=");
        out.push_str(rankdir);
        out.push_str(";\n");
    }

    for node in &d.nodes {
        out.push_str("    ");
        out.push_str(&crate::diagram::format_id(&node.id));
        let attrs = node_attrs(node, &d.styles);
        if attrs.is_empty() {
            out.push_str(";\n");
        } else {
            out.push_str(" [");
            out.push_str(&attrs);
            out.push_str("];\n");
        }
    }

    for sg in &d.subgraphs {
        out.push_str("    subgraph ");
        out.push_str(&crate::diagram::format_id(&sg.id));
        out.push_str(" {\n");
        for id in &sg.nodes {
            out.push_str("        ");
            out.push_str(&crate::diagram::format_id(id));
            out.push_str(";\n");
        }
        out.push_str("    }\n");
    }

    for edge in &d.edges {
        out.push_str("    ");
        out.push_str(&crate::diagram::format_id(&edge.from));
        out.push_str(" -> ");
        out.push_str(&crate::diagram::format_id(&edge.to));
        let attrs = edge_attrs(edge);
        if attrs.is_empty() {
            out.push_str(";\n");
        } else {
            out.push_str(" [");
            out.push_str(&attrs);
            out.push_str("];\n");
        }
    }

    out.push_str("}\n");
    out
}

fn node_attrs(node: &Node, styles: &[NodeStyle]) -> String {
    let mut parts = Vec::new();
    if node.text != node.id {
        parts.push(format!("label={}", dot_quote(&node.text)));
    }
    if let Some(shape) = shape_to_dot(node.shape) {
        parts.push(format!("shape={shape}"));
    }
    if let Some(href) = &node.href {
        parts.push(format!("URL={}", dot_quote(href)));
    }
    if let Some(style) = styles.iter().find(|s| s.node_id == node.id) {
        let props = parse_style_props(&style.properties);
        if let Some(fill) = props.get("fill") {
            parts.push(format!("fillcolor={}", dot_quote(fill)));
            parts.push("style=filled".into());
        }
        if let Some(stroke) = props.get("stroke") {
            parts.push(format!("color={}", dot_quote(stroke)));
        }
        if let Some(font) = props.get("color") {
            parts.push(format!("fontcolor={}", dot_quote(font)));
        }
    }
    parts.join(", ")
}

fn parse_style_props(s: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for part in s.split(',') {
        if let Some((k, v)) = part.trim().split_once(':') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    map
}

fn edge_attrs(edge: &Edge) -> String {
    let mut parts = Vec::new();
    if !edge.label.is_empty() {
        parts.push(format!("label={}", dot_quote(&edge.label)));
    }
    match edge.style {
        EdgeStyle::Arrow => {}
        EdgeStyle::Dashed => parts.push("style=dashed".into()),
        EdgeStyle::Thick => parts.push("style=bold".into()),
    }
    parts.join(", ")
}

fn dot_quote(s: &str) -> String {
    format!(
        "\"{}\"",
        s.replace('\\', "\\\\").replace('"', "\\\"")
    )
}

fn map_rankdir_out(value: &str) -> &'static str {
    match value.to_uppercase().as_str() {
        "LR" => "LR",
        "BT" => "BT",
        "RL" => "RL",
        "TD" | "TB" => "TB",
        _ => "TB",
    }
}

fn shape_to_dot(shape: NodeShape) -> Option<&'static str> {
    match shape {
        NodeShape::Rect => None,
        NodeShape::Diamond => Some("diamond"),
        NodeShape::Stadium => Some("ellipse"),
        NodeShape::Hexagon => Some("hexagon"),
        NodeShape::Cylinder => Some("cylinder"),
        NodeShape::Circle => Some("circle"),
    }
}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
    directed: bool,
    rankdir: String,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    subgraphs: Vec<Subgraph>,
    subgraph_stack: Vec<String>,
    styles: Vec<NodeStyle>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            directed: true,
            rankdir: "TD".into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
            subgraph_stack: Vec::new(),
            styles: Vec::new(),
        }
    }

    fn parse_document(&mut self) -> Result<Diagram, IrError> {
        self.skip_ws_and_comments();
        if self.consume_keyword("strict") {
            self.skip_ws_and_comments();
        }
        if self.consume_keyword("digraph") {
            self.directed = true;
        } else if self.consume_keyword("graph") {
            self.directed = false;
        } else {
            return Err(IrError::from("DOT: expected digraph or graph"));
        }
        self.skip_ws_and_comments();
        let _ = self.parse_optional_name();
        self.skip_ws_and_comments();
        self.expect_char('{')?;
        while !self.peek_is('}') && !self.at_end() {
            self.skip_ws_and_comments();
            if self.peek_is('}') {
                break;
            }
            self.parse_statement()?;
            self.skip_ws_and_comments();
            self.try_consume_char(';');
            self.skip_ws_and_comments();
        }
        self.expect_char('}')?;
        Ok(Diagram {
            rankdir: self.rankdir.clone(),
            nodes: std::mem::take(&mut self.nodes),
            edges: std::mem::take(&mut self.edges),
            subgraphs: std::mem::take(&mut self.subgraphs),
            styles: std::mem::take(&mut self.styles),
            class_defs: Vec::new(),
            class_applies: Vec::new(),
            link_styles: Vec::new(),
        })
    }

    fn parse_statement(&mut self) -> Result<(), IrError> {
        if self.consume_keyword("subgraph") {
            self.skip_ws_and_comments();
            let id = self.parse_id()?;
            self.skip_ws_and_comments();
            self.expect_char('{')?;
            self.subgraph_stack.push(id.clone());
            self.subgraphs.push(Subgraph {
                id,
                nodes: Vec::new(),
            });
            while !self.peek_is('}') && !self.at_end() {
                self.skip_ws_and_comments();
                if self.peek_is('}') {
                    break;
                }
                self.parse_statement()?;
                self.skip_ws_and_comments();
                self.try_consume_char(';');
                self.skip_ws_and_comments();
            }
            self.expect_char('}')?;
            self.subgraph_stack.pop();
            return Ok(());
        }

        if self.consume_keyword("node")
            || self.consume_keyword("edge")
            || self.consume_keyword("graph")
        {
            self.skip_ws_and_comments();
            if self.peek_is('[') {
                let _ = self.parse_attrs()?;
            }
            return Ok(());
        }

        if self.consume_keyword("rankdir") {
            self.expect_char('=')?;
            let value = self.parse_id()?;
            self.rankdir = map_rankdir(&value);
            return Ok(());
        }

        let first = self.parse_id()?;
        self.skip_ws_and_comments();

        let node_attrs = if self.peek_is('[') {
            Some(self.parse_attrs()?)
        } else {
            None
        };
        if let Some(ref attrs) = node_attrs {
            self.ensure_node(&first, attrs)?;
            self.skip_ws_and_comments();
        }

        if self.peek_edge_op() {
            let mut chain = vec![first];
            while self.peek_edge_op() {
                self.consume_edge_op();
                self.skip_ws_and_comments();
                let attrs = if self.peek_is('[') {
                    self.parse_attrs()?
                } else {
                    Attrs::default()
                };
                let to = self.parse_id()?;
                let from = chain.last().unwrap().clone();
                self.ensure_node(&from, &Attrs::default())?;
                self.ensure_node(&to, &Attrs::default())?;
                self.add_edge(&from, &to, &attrs)?;
                chain.push(to);
            }
            self.skip_ws_and_comments();
            if self.peek_is('[') {
                let attrs = self.parse_attrs()?;
                if let Some(edge) = self.edges.last_mut() {
                    if let Some(label) = attrs.label {
                        edge.label = label;
                    }
                    if let Some(style) = attrs.edge_style {
                        edge.style = style;
                    }
                }
            }
            return Ok(());
        }

        if node_attrs.is_some() {
            return Ok(());
        }

        self.ensure_node(&first, &Attrs::default())?;
        Ok(())
    }

    fn ensure_node(&mut self, id: &str, attrs: &Attrs) -> Result<(), IrError> {
        if let Some(n) = self.nodes.iter_mut().find(|n| n.id == id) {
            if let Some(text) = &attrs.label {
                n.text = text.clone();
            }
            if let Some(shape) = attrs.shape {
                n.shape = shape;
            }
            if let Some(href) = &attrs.href {
                n.href = Some(href.clone());
            }
            self.apply_node_style(id, attrs);
            return Ok(());
        }
        let node = Node {
            id: id.to_string(),
            text: attrs.label.clone().unwrap_or_else(|| id.to_string()),
            shape: attrs.shape.unwrap_or(NodeShape::Rect),
            href: attrs.href.clone(),
            tooltip: None,
        };
        self.nodes.push(node);
        self.apply_node_style(id, attrs);
        if let Some(sg_id) = self.subgraph_stack.last() {
            if let Some(sg) = self.subgraphs.iter_mut().find(|s| &s.id == sg_id) {
                if !sg.nodes.contains(&id.to_string()) {
                    sg.nodes.push(id.to_string());
                }
            }
        }
        Ok(())
    }

    fn apply_node_style(&mut self, id: &str, attrs: &Attrs) {
        let mut parts = Vec::new();
        if let Some(fill) = &attrs.fillcolor {
            parts.push(format!("fill:{fill}"));
        }
        if let Some(stroke) = &attrs.color {
            parts.push(format!("stroke:{stroke}"));
        }
        if let Some(font) = &attrs.fontcolor {
            parts.push(format!("color:{font}"));
        }
        if parts.is_empty() {
            return;
        }
        let properties = parts.join(",");
        if let Some(existing) = self.styles.iter_mut().find(|s| s.node_id == id) {
            existing.properties = properties;
        } else {
            self.styles.push(NodeStyle {
                node_id: id.to_string(),
                properties,
            });
        }
    }

    fn add_edge(&mut self, from: &str, to: &str, attrs: &Attrs) -> Result<(), IrError> {
        self.edges.push(Edge {
            from: from.to_string(),
            to: to.to_string(),
            label: attrs.label.clone().unwrap_or_default(),
            style: attrs.edge_style.unwrap_or_default(),
        });
        Ok(())
    }

    fn parse_optional_name(&mut self) -> Result<(), IrError> {
        if self.peek_is('{') {
            return Ok(());
        }
        let _ = self.parse_id()?;
        Ok(())
    }

    fn parse_id(&mut self) -> Result<String, IrError> {
        self.skip_ws_and_comments();
        if self.at_end() {
            return Err(IrError::from("DOT: expected identifier"));
        }
        if self.peek_is('"') || self.peek_is('\'') {
            let quote = self.bump_char().unwrap();
            let start = self.pos;
            while !self.at_end() && !self.peek_is(quote) {
                if self.peek_is('\\') {
                    self.pos += 1;
                }
                self.pos += 1;
            }
            if self.at_end() {
                return Err(IrError::from("DOT: unterminated quoted id"));
            }
            let raw = &self.input[start..self.pos];
            self.pos += 1;
            return Ok(raw.replace("\\\"", "\"").replace("\\'", "'"));
        }
        if self.peek_is(':') {
            return Err(IrError::from("DOT: port identifiers not supported"));
        }
        let start = self.pos;
        if self.peek_is('-') && self.remaining().starts_with("--") {
            return Err(IrError::from("DOT: expected identifier"));
        }
        while !self.at_end() {
            let c = self.input[self.pos..].chars().next().unwrap();
            if c.is_alphanumeric() || c == '_' {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        if start == self.pos {
            return Err(IrError::from("DOT: expected identifier"));
        }
        Ok(self.input[start..self.pos].to_string())
    }

    fn parse_attrs(&mut self) -> Result<Attrs, IrError> {
        self.expect_char('[')?;
        let mut attrs = Attrs::default();
        loop {
            self.skip_ws_and_comments();
            if self.peek_is(']') {
                self.pos += 1;
                return Ok(attrs);
            }
            let key = self.parse_attr_key()?;
            self.skip_ws_and_comments();
            self.expect_char('=')?;
            self.skip_ws_and_comments();
            let value = self.parse_attr_value()?;
            attrs.apply(&key, &value);
            self.skip_ws_and_comments();
            if self.try_consume_char(',') {
                continue;
            }
        }
    }

    fn parse_attr_key(&mut self) -> Result<String, IrError> {
        self.skip_ws_and_comments();
        let start = self.pos;
        while !self.at_end() {
            let c = self.input[self.pos..].chars().next().unwrap();
            if c.is_alphanumeric() || c == '_' {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        if start == self.pos {
            return Err(IrError::from("DOT: expected attribute key"));
        }
        Ok(self.input[start..self.pos].to_string())
    }

    fn parse_attr_value(&mut self) -> Result<String, IrError> {
        self.skip_ws_and_comments();
        if self.peek_is('"') || self.peek_is('\'') {
            let quote = self.bump_char().unwrap();
            let start = self.pos;
            while !self.at_end() && !self.peek_is(quote) {
                if self.peek_is('\\') {
                    self.pos += 1;
                }
                self.pos += 1;
            }
            if self.at_end() {
                return Err(IrError::from("DOT: unterminated attribute value"));
            }
            let raw = &self.input[start..self.pos];
            self.pos += 1;
            return Ok(raw.replace("\\\"", "\""));
        }
        let start = self.pos;
        while !self.at_end() {
            let c = self.input[self.pos..].chars().next().unwrap();
            if c.is_whitespace() || c == ',' || c == ']' {
                break;
            }
            self.pos += c.len_utf8();
        }
        if start == self.pos {
            return Err(IrError::from("DOT: expected attribute value"));
        }
        Ok(self.input[start..self.pos].to_string())
    }

    fn peek_edge_op(&mut self) -> bool {
        let saved = self.pos;
        self.skip_ws_and_comments();
        let ok = if self.directed {
            self.remaining().starts_with("->")
        } else {
            self.remaining().starts_with("--")
        };
        self.pos = saved;
        ok
    }

    fn consume_edge_op(&mut self) {
        self.skip_ws_and_comments();
        if self.directed {
            self.pos += 2;
        } else {
            self.pos += 2;
        }
    }

    fn consume_keyword(&mut self, kw: &str) -> bool {
        self.skip_ws_and_comments();
        if self.remaining().starts_with(kw) {
            let next = self.input[self.pos + kw.len()..].chars().next();
            if next.is_none_or(|c| !c.is_alphanumeric() && c != '_') {
                self.pos += kw.len();
                return true;
            }
        }
        false
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while self.pos < self.input.len() {
                let c = self.input[self.pos..].chars().next().unwrap();
                if c.is_whitespace() {
                    self.pos += c.len_utf8();
                } else {
                    break;
                }
            }
            if self.remaining().starts_with("//") {
                while self.pos < self.input.len() && !self.peek_is('\n') {
                    self.pos += 1;
                }
                continue;
            }
            if self.remaining().starts_with("/*") {
                self.pos += 2;
                while self.pos + 1 < self.input.len() && !self.remaining().starts_with("*/") {
                    self.pos += 1;
                }
                self.pos = (self.pos + 2).min(self.input.len());
                continue;
            }
            break;
        }
    }

    fn remaining(&self) -> &str {
        &self.input[self.pos..]
    }

    fn at_end(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn peek_is(&self, c: char) -> bool {
        self.input[self.pos..].chars().next() == Some(c)
    }

    fn try_consume_char(&mut self, expected: char) -> bool {
        self.skip_ws_and_comments();
        if self.peek_is(expected) {
            self.pos += expected.len_utf8();
            true
        } else {
            false
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<(), IrError> {
        self.skip_ws_and_comments();
        if self.peek_is(expected) {
            self.pos += expected.len_utf8();
            Ok(())
        } else {
            Err(IrError::from(format!("DOT: expected '{expected}'")))
        }
    }

    fn bump_char(&mut self) -> Option<char> {
        let c = self.input[self.pos..].chars().next()?;
        self.pos += c.len_utf8();
        Some(c)
    }
}

#[derive(Debug, Default, Clone)]
struct Attrs {
    label: Option<String>,
    shape: Option<NodeShape>,
    edge_style: Option<EdgeStyle>,
    href: Option<String>,
    fillcolor: Option<String>,
    color: Option<String>,
    fontcolor: Option<String>,
}

impl Attrs {
    fn apply(&mut self, key: &str, value: &str) {
        match key.to_lowercase().as_str() {
            "label" => self.label = Some(value.to_string()),
            "shape" => self.shape = map_shape(value),
            "style" => {
                if value.contains("dashed") {
                    self.edge_style = Some(EdgeStyle::Dashed);
                } else if value.contains("bold") {
                    self.edge_style = Some(EdgeStyle::Thick);
                }
            }
            "url" | "href" => self.href = Some(value.to_string()),
            "fillcolor" => self.fillcolor = Some(value.to_string()),
            "color" => self.color = Some(value.to_string()),
            "fontcolor" => self.fontcolor = Some(value.to_string()),
            _ => {}
        }
    }
}

fn map_rankdir(value: &str) -> String {
    match value.to_uppercase().as_str() {
        "LR" => "LR".into(),
        "BT" => "BT".into(),
        "RL" => "RL".into(),
        "TB" | "TD" => "TD".into(),
        _ => "TD".into(),
    }
}

fn map_shape(value: &str) -> Option<NodeShape> {
    match value.to_lowercase().as_str() {
        "box" | "rect" | "rectangle" | "square" => Some(NodeShape::Rect),
        "diamond" => Some(NodeShape::Diamond),
        "ellipse" | "oval" => Some(NodeShape::Stadium),
        "hexagon" => Some(NodeShape::Hexagon),
        "cylinder" => Some(NodeShape::Cylinder),
        "circle" => Some(NodeShape::Circle),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Diagram as IrDiagram, Kind};

    #[test]
    fn is_dot_distinguishes_mermaid() {
        assert!(is_dot("digraph G { A -> B }"));
        assert!(!is_dot("graph TD\n  A-->B"));
        assert!(!is_dot("flowchart LR\n  A-->B"));
    }

    #[test]
    fn parse_minimal_cases() {
        assert!(parse(r#"digraph G { A [label="Start"]; }"#).is_ok());
        assert!(parse(r#"digraph G { A -> B; }"#).is_ok());
        assert!(parse(r#"digraph { rankdir=LR; A -> B; }"#).is_ok());
        assert!(parse(r#"digraph G { B [label="End", shape=diamond]; }"#).is_ok());
        assert!(parse(r#"digraph G { A -> B [label="go"]; }"#).is_ok());
        assert!(parse(r#"digraph { A -> B -> C; }"#).is_ok());
        assert!(parse(r#"digraph G { A [label="Start"]; A -> B [label="go"]; }"#).is_ok());
    }

    #[test]
    fn parse_basic_digraph() {
        let d = parse(
            r#"digraph G {
            A [label="Start"];
            B [label="End", shape=diamond];
            A -> B [label="go"];
        }"#,
        )
        .unwrap();
        assert_eq!(d.nodes.len(), 2);
        assert_eq!(d.edges.len(), 1);
        assert_eq!(d.edges[0].label, "go");
        assert_eq!(d.nodes[1].shape, NodeShape::Diamond);
    }

    #[test]
    fn parse_chained_edges_and_rankdir() {
        let d = parse(
            r#"digraph {
            rankdir=LR;
            A -> B -> C;
        }"#,
        )
        .unwrap();
        assert_eq!(d.rankdir, "LR");
        assert_eq!(d.edges.len(), 2);
        assert_eq!(d.nodes.len(), 3);
    }

    #[test]
    fn parse_subgraph() {
        let d = parse(
            r#"digraph G {
            subgraph cluster_0 {
                A; B;
                A -> B;
            }
            B -> C;
        }"#,
        )
        .unwrap();
        assert_eq!(d.subgraphs.len(), 1);
        assert!(d.subgraphs[0].nodes.contains(&"A".to_string()));
        assert_eq!(d.edges.len(), 2);
    }

    #[test]
    fn parse_to_document_kind() {
        let doc = parse_to_document("digraph { X -> Y }").unwrap();
        assert_eq!(doc.primary().unwrap().kind(), Kind::Flowchart);
    }

    #[test]
    fn implicit_nodes_from_edges() {
        let d = parse("digraph { foo -> bar }").unwrap();
        assert_eq!(d.nodes.len(), 2);
        assert!(d.nodes.iter().any(|n| n.id == "foo"));
    }

    #[test]
    fn export_basic_flowchart() {
        let d = parse(
            r#"digraph G {
            A [label="Start"];
            B [label="End", shape=diamond];
            A -> B [label="go"];
        }"#,
        )
        .unwrap();
        let doc = Document::single(crate::ir::Diagram::Flowchart(d));
        let out = export_document(&doc).unwrap();
        assert!(out.contains("digraph G"));
        assert!(out.contains("label=\"Start\""));
        assert!(out.contains("shape=diamond"));
        assert!(out.contains("label=\"go\""));
    }

    #[test]
    fn export_import_roundtrip() {
        let src = r#"digraph flow {
            rankdir=LR;
            Start [label="Start", shape=box];
            End [label="End", shape=diamond];
            Start -> End [label="go"];
        }"#;
        let doc = parse_to_document(src).unwrap();
        let out = export_document(&doc).unwrap();
        let doc2 = parse_to_document(&out).unwrap();
        let IrDiagram::Flowchart(d1) = doc.primary().unwrap() else {
            panic!("expected flowchart");
        };
        let IrDiagram::Flowchart(d2) = doc2.primary().unwrap() else {
            panic!("expected flowchart");
        };
        assert_eq!(d1.nodes.len(), d2.nodes.len());
        assert_eq!(d1.edges.len(), d2.edges.len());
        assert_eq!(d1.edges[0].label, d2.edges[0].label);
        assert_eq!(d1.nodes[1].shape, d2.nodes[1].shape);
    }

    #[test]
    fn parse_fillcolor_and_url() {
        let d = parse(
            r##"digraph G {
            A [label="Start", fillcolor="#ffcccc", color="#333333", fontcolor="#111111", URL="https://example.com"];
            B [label="End"];
            A -> B;
        }"##,
        )
        .unwrap();
        let a = d.nodes.iter().find(|n| n.id == "A").unwrap();
        assert_eq!(a.href.as_deref(), Some("https://example.com"));
        assert_eq!(d.styles.len(), 1);
        assert!(d.styles[0].properties.contains("fill:#ffcccc"));
        assert!(d.styles[0].properties.contains("stroke:#333333"));
        assert!(d.styles[0].properties.contains("color:#111111"));
    }

    #[test]
    fn export_styles_and_href() {
        let mut d = parse(r#"digraph G { A [label="A"]; B; A -> B; }"#).unwrap();
        d.nodes[0].href = Some("https://example.com".into());
        d.styles.push(NodeStyle {
            node_id: "A".into(),
            properties: "fill:#eee,stroke:#000,color:#111".into(),
        });
        let doc = Document::single(crate::ir::Diagram::Flowchart(d));
        let out = export_document(&doc).unwrap();
        assert!(out.contains("URL=\"https://example.com\""));
        assert!(out.contains("fillcolor=\"#eee\""));
        assert!(out.contains("style=filled"));
        assert!(out.contains("color=\"#000\""));
        assert!(out.contains("fontcolor=\"#111\""));
    }
}
