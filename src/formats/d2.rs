//! D2 Compatibility adapter (flowchart subset ↔ IR, including containers as subgraphs).

use crate::diagram::{Diagram, Edge, EdgeStyle, Node, NodeShape, Subgraph};
use crate::ir::{Document, IrError};
use std::collections::HashMap;

enum BraceBlock {
    Attrs(HashMap<String, String>),
    Container { members: Vec<String> },
}

fn is_d2_attr_key(key: &str) -> bool {
    matches!(key, "shape" | "label" | "style") || key.starts_with("style.")
}

/// Whether source looks like D2 (not Mermaid, DOT, PlantUML, or JSON IR).
pub fn is_d2(source: &str) -> bool {
    let trimmed = skip_leading_comments(source);
    if trimmed.starts_with('{')
        || trimmed.starts_with("digraph")
        || trimmed.starts_with("strict digraph")
        || trimmed.starts_with("sequenceDiagram")
        || trimmed.starts_with("classDiagram")
        || trimmed.starts_with("gantt")
        || trimmed.starts_with("stateDiagram")
        || trimmed.starts_with("erDiagram")
        || trimmed.starts_with("@startuml")
    {
        return false;
    }
    if trimmed.starts_with("graph TD")
        || trimmed.starts_with("graph LR")
        || trimmed.starts_with("graph BT")
        || trimmed.starts_with("graph RL")
        || trimmed.starts_with("graph TB")
        || trimmed.starts_with("flowchart ")
    {
        return false;
    }
    if trimmed.starts_with("graph ") || trimmed.starts_with("strict graph ") {
        return false;
    }

    for line in source.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') || l.starts_with("%%") {
            continue;
        }
        if l.starts_with("direction:") {
            return true;
        }
        if parse_connection(l).is_some() {
            return true;
        }
    }
    false
}

fn skip_leading_comments(source: &str) -> &str {
    let mut rest = source;
    loop {
        let trimmed = rest.trim_start();
        if trimmed.starts_with("%%") || trimmed.starts_with('#') {
            if let Some((_, after)) = trimmed.split_once('\n') {
                rest = after;
                continue;
            }
            return "";
        }
        return trimmed;
    }
}

pub fn parse(source: &str) -> Result<Diagram, IrError> {
    Parser::new(source).parse_document()
}

pub fn parse_to_document(source: &str) -> Result<Document, IrError> {
    Ok(Document::single(crate::ir::Diagram::Flowchart(parse(source)?)))
}

/// Export a Document to D2 (flowchart diagrams only).
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
        return Err(IrError::from("D2 export supports flowchart diagrams only"));
    }

    let multi = doc.diagrams.len() > 1;
    let blocks: Vec<String> = flowcharts
        .into_iter()
        .map(|(i, fc)| {
            let mut block = String::new();
            if multi {
                block.push_str(&format!("# diagram {i}: flowchart\n"));
            }
            block.push_str(&export_flowchart(fc));
            block
        })
        .collect();

    Ok(blocks.join("\n\n"))
}

fn export_flowchart(d: &Diagram) -> String {
    let mut out = String::new();
    if let Some(dir) = map_direction_out(&d.rankdir) {
        out.push_str("direction: ");
        out.push_str(dir);
        out.push_str("\n\n");
    }

    for node in &d.nodes {
        out.push_str(&crate::diagram::format_id(&node.id));
        if node.text != node.id || shape_to_d2(node.shape).is_some() {
            out.push_str(": ");
            if node.text != node.id {
                out.push_str(&d2_quote(&node.text));
            }
            if let Some(shape) = shape_to_d2(node.shape) {
                if node.text == node.id {
                    out.push_str("{\n  shape: ");
                } else {
                    out.push_str(" {\n  shape: ");
                }
                out.push_str(shape);
                out.push_str("\n}\n");
            } else if node.text != node.id {
                out.push('\n');
            }
        } else {
            out.push('\n');
        }
    }

    for sg in &d.subgraphs {
        out.push_str(&crate::diagram::format_id(&sg.id));
        out.push_str(": {\n");
        for id in &sg.nodes {
            out.push_str("  ");
            out.push_str(&crate::diagram::format_id(id));
            out.push('\n');
        }
        out.push_str("}\n\n");
    }

    for edge in &d.edges {
        out.push_str(&crate::diagram::format_id(&edge.from));
        out.push_str(" -> ");
        out.push_str(&crate::diagram::format_id(&edge.to));
        if !edge.label.is_empty() || edge.style != EdgeStyle::Arrow {
            out.push_str(": ");
            if !edge.label.is_empty() {
                out.push_str(&d2_quote(&edge.label));
            }
            if edge.style == EdgeStyle::Dashed {
                if edge.label.is_empty() {
                    out.push_str("{\n  style.stroke-dash: 3\n}");
                } else {
                    out.push_str(" {\n  style.stroke-dash: 3\n}");
                }
            }
        }
        out.push('\n');
    }

    out
}

fn d2_quote(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == ' ')
        && !s.is_empty()
    {
        s.to_string()
    } else {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

fn shape_to_d2(shape: NodeShape) -> Option<&'static str> {
    match shape {
        NodeShape::Rect => None,
        NodeShape::Diamond => Some("diamond"),
        NodeShape::Stadium => Some("oval"),
        NodeShape::Hexagon => Some("hexagon"),
        NodeShape::Cylinder => Some("cylinder"),
        NodeShape::Circle => Some("circle"),
    }
}

fn map_direction_out(value: &str) -> Option<&'static str> {
    match value.to_uppercase().as_str() {
        "LR" => Some("right"),
        "RL" => Some("left"),
        "BT" => Some("up"),
        "TD" | "TB" => None,
        _ => None,
    }
}

fn map_direction_in(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "right" => "LR".into(),
        "left" => "RL".into(),
        "up" => "BT".into(),
        "down" => "TD".into(),
        _ => "TD".into(),
    }
}

fn shape_from_d2(value: &str) -> NodeShape {
    match value.trim().to_lowercase().as_str() {
        "diamond" => NodeShape::Diamond,
        "hexagon" => NodeShape::Hexagon,
        "cylinder" => NodeShape::Cylinder,
        "circle" => NodeShape::Circle,
        "oval" | "ellipse" | "stadium" => NodeShape::Stadium,
        "rectangle" | "square" => NodeShape::Rect,
        _ => NodeShape::Rect,
    }
}

/// Parsed connection tuple: (left_id, op_token, right_id, optional_label, has_arrow_chain).
type ConnectionParts = (String, &'static str, String, Option<String>, bool);

struct Parser<'a> {
    input: &'a str,
    pos: usize,
    rankdir: String,
    nodes: HashMap<String, Node>,
    edges: Vec<Edge>,
    subgraphs: Vec<Subgraph>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            rankdir: "TD".into(),
            nodes: HashMap::new(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
        }
    }

    fn parse_document(mut self) -> Result<Diagram, IrError> {
        self.skip_ws_and_comments();
        while self.pos < self.input.len() {
            self.parse_statement()?;
            self.skip_ws_and_comments();
        }

        let mut nodes: Vec<Node> = self.nodes.into_values().collect();
        nodes.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(Diagram {
            rankdir: self.rankdir,
            nodes,
            edges: self.edges,
            subgraphs: self.subgraphs,
            styles: Vec::new(),
            class_defs: Vec::new(),
            class_applies: Vec::new(),
            link_styles: Vec::new(),
        })
    }

    fn parse_statement(&mut self) -> Result<(), IrError> {
        if self.peek_keyword("direction") {
            self.consume_keyword("direction");
            self.expect_char(':')?;
            let value = self.read_line_value()?;
            self.rankdir = map_direction_in(&value);
            return Ok(());
        }

        let checkpoint = self.pos;
        if let Some((left, op, right, label, dashed)) = self.try_parse_connection()? {
            self.push_edge(left, op, right, label, dashed);
            return Ok(());
        }
        self.pos = checkpoint;

        let id = self.read_id()?;
        self.skip_ws_and_comments();
        if self.peek_char() != Some(':') {
            self.ensure_node(&id, &id);
            return Ok(());
        }
        self.advance();
        self.skip_ws_and_comments();

        let mut label = None;
        if self.peek_char() != Some('{') {
            label = Some(self.read_label_token()?);
            self.skip_ws_and_comments();
        }

        if self.peek_char() == Some('{') {
            match self.read_brace_block()? {
                BraceBlock::Attrs(attrs) => {
                    let mut shape = NodeShape::Rect;
                    let mut text = label.unwrap_or_else(|| id.clone());
                    if let Some(l) = attrs.get("label") {
                        text = unquote(l);
                    }
                    if let Some(s) = attrs.get("shape") {
                        shape = shape_from_d2(s);
                    }
                    self.nodes.insert(
                        id.clone(),
                        Node {
                            id,
                            text,
                            shape,
                            href: None,
                            tooltip: None,
                        },
                    );
                }
                BraceBlock::Container { members } => {
                    self.subgraphs.push(Subgraph { id, nodes: members });
                }
            }
            return Ok(());
        }

        let text = label.unwrap_or_else(|| id.clone());
        self.nodes.insert(
            id.clone(),
            Node {
                id,
                text,
                shape: NodeShape::Rect,
                href: None,
                tooltip: None,
            },
        );
        Ok(())
    }

    fn push_edge(
        &mut self,
        left: String,
        op: &str,
        right: String,
        label: Option<String>,
        dashed: bool,
    ) {
        let style = if dashed || op == "--" {
            EdgeStyle::Dashed
        } else {
            EdgeStyle::Arrow
        };
        self.ensure_node(&left, &left);
        self.ensure_node(&right, &right);
        self.edges.push(Edge {
            from: left,
            to: right,
            label: label.unwrap_or_default(),
            style,
        });
    }

    fn try_parse_connection(
        &mut self,
    ) -> Result<Option<ConnectionParts>, IrError> {
        let start = self.pos;
        let left = self.read_id()?;
        self.skip_ws_and_comments();
        let (op, op_len) = match self.read_connector()? {
            Some(v) => v,
            None => {
                self.pos = start;
                return Ok(None);
            }
        };
        self.pos += op_len;
        self.skip_ws_and_comments();
        let right = self.read_id()?;
        self.skip_ws_and_comments();

        let mut label = None;
        let mut dashed = false;
        if self.peek_char() == Some(':') {
            self.advance();
            self.skip_ws_and_comments();
            if self.peek_char() != Some('{') {
                label = Some(self.read_label_token()?);
                self.skip_ws_and_comments();
            }
            if self.peek_char() == Some('{') {
                match self.read_brace_block()? {
                    BraceBlock::Attrs(attrs) => {
                        if let Some(l) = attrs.get("label") {
                            label = Some(unquote(l));
                        }
                        if attrs.keys().any(|k| k.contains("stroke-dash")) {
                            dashed = true;
                        }
                    }
                    BraceBlock::Container { .. } => {
                        return Err(IrError::from(
                            "D2: edge style block cannot contain nested containers",
                        ));
                    }
                }
            }
        }

        Ok(Some((left, op, right, label, dashed)))
    }

    fn read_connector(&self) -> Result<Option<(&'static str, usize)>, IrError> {
        let rest = &self.input[self.pos..];
        for (op, len) in [("<->", 3), ("->", 2), ("<-", 2), ("--", 2)] {
            if rest.starts_with(op) {
                let before = self.input[..self.pos].chars().last();
                let after = rest.chars().nth(len);
                let valid_before = before.is_none_or(|c| c.is_whitespace() || c == ')' || c == ']' || c == '"');
                let valid_after = after.is_none_or(|c| c.is_whitespace() || c == '(' || c == '[' || c == '"' || c.is_alphanumeric() || c == '_');
                if valid_before && valid_after {
                    return Ok(Some((op, len)));
                }
            }
        }
        Ok(None)
    }

    fn ensure_node(&mut self, id: &str, text: &str) {
        self.nodes.entry(id.to_string()).or_insert_with(|| Node {
            id: id.to_string(),
            text: text.to_string(),
            shape: NodeShape::Rect,
            href: None,
            tooltip: None,
        });
    }

    fn read_brace_block(&mut self) -> Result<BraceBlock, IrError> {
        self.expect_char('{')?;
        let mut attrs = HashMap::new();
        let mut members = Vec::new();
        let mut saw_member = false;

        loop {
            self.skip_ws_and_comments();
            if self.peek_char() == Some('}') {
                self.advance();
                break;
            }

            let checkpoint = self.pos;
            if let Some((left, op, right, label, dashed)) = self.try_parse_connection()? {
                self.push_edge(left.clone(), op, right.clone(), label, dashed);
                if !members.contains(&left) {
                    members.push(left);
                }
                if !members.contains(&right) {
                    members.push(right);
                }
                saw_member = true;
                continue;
            }
            self.pos = checkpoint;

            let key = self.read_id()?;
            self.skip_ws_and_comments();
            if self.peek_char() == Some(':') {
                self.advance();
                self.skip_ws_and_comments();
                if self.peek_char() == Some('{') {
                    saw_member = true;
                    match self.read_brace_block()? {
                        BraceBlock::Attrs(nested_attrs) => {
                            let mut shape = NodeShape::Rect;
                            let mut text = key.clone();
                            if let Some(l) = nested_attrs.get("label") {
                                text = unquote(l);
                            }
                            if let Some(s) = nested_attrs.get("shape") {
                                shape = shape_from_d2(s);
                            }
                            self.nodes.insert(
                                key.clone(),
                                Node {
                                    id: key.clone(),
                                    text,
                                    shape,
                                    href: None,
                                    tooltip: None,
                                },
                            );
                            if !members.contains(&key) {
                                members.push(key);
                            }
                        }
                        BraceBlock::Container { members: nested } => {
                            self.subgraphs.push(Subgraph {
                                id: key,
                                nodes: nested,
                            });
                        }
                    }
                } else if is_d2_attr_key(&key) {
                    let value = self.read_attr_value()?;
                    attrs.insert(key, value);
                } else {
                    saw_member = true;
                    let text = unquote(&self.read_attr_value()?);
                    self.nodes.insert(
                        key.clone(),
                        Node {
                            id: key.clone(),
                            text,
                            shape: NodeShape::Rect,
                            href: None,
                            tooltip: None,
                        },
                    );
                    if !members.contains(&key) {
                        members.push(key);
                    }
                }
            } else {
                saw_member = true;
                self.ensure_node(&key, &key);
                if !members.contains(&key) {
                    members.push(key);
                }
            }
        }

        if saw_member {
            Ok(BraceBlock::Container { members })
        } else {
            Ok(BraceBlock::Attrs(attrs))
        }
    }

    fn read_attr_value(&mut self) -> Result<String, IrError> {
        self.skip_ws_and_comments();
        if self.peek_char() == Some('"') {
            return self.read_quoted();
        }
        let start = self.pos;
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() || c == '}' || c == '\n' {
                break;
            }
            self.advance();
        }
        if self.pos == start {
            return Err(IrError::from("D2: expected attribute value"));
        }
        Ok(self.input[start..self.pos].trim().to_string())
    }

    fn read_id(&mut self) -> Result<String, IrError> {
        self.skip_ws_and_comments();
        if self.peek_char() == Some('"') {
            return self.read_quoted();
        }
        let start = self.pos;
        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' || c == '.' || c == '-' {
                self.advance();
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(IrError::from("D2: expected identifier"));
        }
        Ok(self.input[start..self.pos].to_string())
    }

    fn read_label_token(&mut self) -> Result<String, IrError> {
        self.skip_ws_and_comments();
        if self.peek_char() == Some('"') {
            return self.read_quoted();
        }
        let start = self.pos;
        while let Some(c) = self.peek_char() {
            if c == '\n' || c == '{' {
                break;
            }
            self.advance();
        }
        let raw = self.input[start..self.pos].trim();
        if raw.is_empty() {
            return Err(IrError::from("D2: expected label"));
        }
        Ok(raw.to_string())
    }

    fn read_quoted(&mut self) -> Result<String, IrError> {
        self.expect_char('"')?;
        let mut out = String::new();
        while let Some(c) = self.peek_char() {
            self.advance();
            if c == '"' {
                break;
            }
            if c == '\\' {
                if let Some(next) = self.peek_char() {
                    self.advance();
                    out.push(next);
                }
            } else {
                out.push(c);
            }
        }
        Ok(out)
    }

    fn read_line_value(&mut self) -> Result<String, IrError> {
        self.skip_ws_and_comments();
        let start = self.pos;
        while let Some(c) = self.peek_char() {
            if c == '\n' {
                break;
            }
            self.advance();
        }
        Ok(self.input[start..self.pos].trim().to_string())
    }

    fn peek_keyword(&self, kw: &str) -> bool {
        self.input[self.pos..].starts_with(kw)
            && self.input[self.pos + kw.len()..]
                .chars()
                .next()
                .is_none_or(|c| !c.is_alphanumeric() && c != '_')
    }

    fn consume_keyword(&mut self, kw: &str) {
        self.pos += kw.len();
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while let Some(c) = self.peek_char() {
                if c.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }
            if self.peek_char() == Some('#') {
                while let Some(c) = self.peek_char() {
                    self.advance();
                    if c == '\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) {
        if let Some(c) = self.peek_char() {
            self.pos += c.len_utf8();
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<(), IrError> {
        self.skip_ws_and_comments();
        match self.peek_char() {
            Some(c) if c == expected => {
                self.advance();
                Ok(())
            }
            _ => Err(IrError::from(format!("D2: expected '{expected}'"))),
        }
    }
}

fn unquote(s: &str) -> String {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].replace("\\\"", "\"").replace("\\\\", "\\")
    } else {
        s.to_string()
    }
}

fn parse_connection(line: &str) -> Option<(String, String)> {
    let ops = ["<->", "->", "<-", "--"];
    for op in ops {
        if let Some(idx) = line.find(op) {
            // Avoid Mermaid `-->` / `-.->` / `==>` matching D2 `->` / `--`.
            if op == "->" {
                let before = line[..idx].chars().last();
                if matches!(before, Some('-' | '=' | '.')) {
                    continue;
                }
            }
            if op == "--" {
                let after = line[idx + op.len()..].chars().next();
                if matches!(after, Some('>' | '-' | '.')) {
                    continue;
                }
                let before = line[..idx].chars().last();
                if matches!(before, Some('-' | '=' | '.')) {
                    continue;
                }
            }
            let left = line[..idx].trim();
            let rest = line[idx + op.len()..].trim();
            let right = rest
                .split(':')
                .next()
                .unwrap_or(rest)
                .split_whitespace()
                .next()
                .unwrap_or("");
            if !left.is_empty() && !right.is_empty() {
                // Mermaid node shapes: A[text], A{text}, A((text)) — not D2 ids.
                if left.contains('[')
                    || left.contains(']')
                    || left.contains('{')
                    || left.contains('(')
                    || right.contains('[')
                    || right.contains('{')
                    || right.contains('(')
                {
                    continue;
                }
                return Some((left.to_string(), right.to_string()));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Diagram as IrDiagram, Kind};

    #[test]
    fn is_d2_distinguishes_mermaid_and_dot() {
        assert!(!is_d2("graph TD\n  A-->B\n"));
        assert!(!is_d2("%% comment\ngraph TD\n  A-->B\n"));
        assert!(!is_d2("digraph G { A -> B }"));
        assert!(is_d2("direction: right\na -> b\n"));
        assert!(is_d2("start -> end: go\n"));
        assert!(!is_d2("sequenceDiagram\n  A->>B: hi\n"));
    }

    #[test]
    fn parse_basic_flow() {
        let src = r#"
direction: down
start: Start { shape: oval }
check: Is it working? { shape: diamond }
great: Great!
debug: Debug
end: End

start -> check
check -> great: Yes
check -> debug: No
debug -> check
great -> end
"#;
        let d = parse(src).unwrap();
        assert_eq!(d.rankdir, "TD");
        assert!(d.nodes.len() >= 5);
        assert_eq!(d.edges.len(), 5);
        let check = d.nodes.iter().find(|n| n.id == "check").unwrap();
        assert_eq!(check.shape, NodeShape::Diamond);
        assert_eq!(check.text, "Is it working?");
        assert_eq!(d.edges[1].label, "Yes");
    }

    #[test]
    fn export_basic_flowchart() {
        let src = "graph TD\n  A[Start] --> B{Check}\n  A --> C[End]\n";
        let doc = crate::formats::import_str(src, crate::formats::Format::Mermaid).unwrap();
        let out = export_document(&doc).unwrap();
        assert!(out.contains("A"));
        assert!(out.contains("->"));
        assert!(out.contains("shape: diamond") || out.contains("B"));
    }

    #[test]
    fn export_import_roundtrip() {
        let src = r#"direction: right
a: Alpha
b: Beta { shape: hexagon }
a -> b: link
"#;
        let doc1 = parse_to_document(src).unwrap();
        let out = export_document(&doc1).unwrap();
        let doc2 = parse_to_document(&out).unwrap();
        let IrDiagram::Flowchart(d1) = doc1.primary().unwrap() else {
            panic!("expected flowchart");
        };
        let IrDiagram::Flowchart(d2) = doc2.primary().unwrap() else {
            panic!("expected flowchart");
        };
        assert_eq!(d1.nodes.len(), d2.nodes.len());
        assert_eq!(d1.edges.len(), d2.edges.len());
        assert_eq!(d1.edges[0].label, d2.edges[0].label);
        assert_eq!(d1.rankdir, "LR");
        assert_eq!(d2.rankdir, "LR");
    }

    #[test]
    fn parse_to_document_kind() {
        let doc = parse_to_document("x -> y\n").unwrap();
        assert_eq!(doc.primary().unwrap().kind(), Kind::Flowchart);
    }

    #[test]
    fn parse_dashed_edge_block() {
        let src = "a -> b: retry {\n  style.stroke-dash: 3\n}\n";
        let d = parse(src).unwrap();
        assert_eq!(d.edges.len(), 1);
        assert_eq!(d.edges[0].style, EdgeStyle::Dashed);
        assert_eq!(d.edges[0].label, "retry");
    }

    #[test]
    fn parse_container_as_subgraph() {
        let src = r#"
direction: down
a: Alpha
b: Beta
c: Gamma

group: {
  a
  b
}

a -> b
b -> c
"#;
        let d = parse(src).unwrap();
        assert_eq!(d.subgraphs.len(), 1);
        assert_eq!(d.subgraphs[0].id, "group");
        assert!(d.subgraphs[0].nodes.contains(&"a".into()));
        assert!(d.subgraphs[0].nodes.contains(&"b".into()));
        assert_eq!(d.nodes.len(), 3);
        assert_eq!(d.edges.len(), 2);
    }

    #[test]
    fn export_import_preserves_subgraphs() {
        let src = "graph TD\n  subgraph cluster\n    A[One] --> B[Two]\n  end\n";
        let doc1 = crate::formats::import_str(src, crate::formats::Format::Mermaid).unwrap();
        let out = export_document(&doc1).unwrap();
        assert!(out.contains("cluster: {") || out.contains("cluster:{"));
        let doc2 = parse_to_document(&out).unwrap();
        let IrDiagram::Flowchart(d1) = doc1.primary().unwrap() else {
            panic!("expected flowchart");
        };
        let IrDiagram::Flowchart(d2) = doc2.primary().unwrap() else {
            panic!("expected flowchart");
        };
        assert_eq!(d1.subgraphs.len(), d2.subgraphs.len());
        assert_eq!(d1.subgraphs[0].nodes.len(), d2.subgraphs[0].nodes.len());
    }

    #[test]
    fn nested_container_becomes_separate_subgraph() {
        let src = r#"
outer: {
  a
  inner: {
    b
  }
}
a -> b
"#;
        let d = parse(src).unwrap();
        assert_eq!(d.subgraphs.len(), 2);
        let outer = d.subgraphs.iter().find(|s| s.id == "outer").unwrap();
        let inner = d.subgraphs.iter().find(|s| s.id == "inner").unwrap();
        assert!(outer.nodes.contains(&"a".into()));
        assert!(!outer.nodes.contains(&"b".into()));
        assert!(inner.nodes.contains(&"b".into()));
    }
}
