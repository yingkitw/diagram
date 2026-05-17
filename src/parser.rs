use crate::diagram::{ClassApply, ClassDef, Diagram, Edge, EdgeStyle, Node, NodeShape, NodeStyle, Subgraph};
use std::collections::HashSet;
use std::fmt;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: Option<usize>,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(l) => write!(f, "line {}: {}", l, self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for ParseError {}

pub fn parse(source: &str) -> Result<Diagram, ParseError> {
    let raw_lines: Vec<&str> = source.lines().collect();
    let lines: Vec<(usize, &str)> = raw_lines
        .iter()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim()))
        .filter(|(_, l)| !l.is_empty() && !l.starts_with("%%") && !l.starts_with("---"))
        .collect();

    if lines.is_empty() {
        return Ok(Diagram::new("TB"));
    }

    let rankdir = if lines[0].1.starts_with("graph ") {
        parse_rankdir(lines[0].1)
    } else {
        "TB".to_string()
    };

    let mut nodes: Vec<Node> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut subgraphs: Vec<Subgraph> = Vec::new();
    let mut styles: Vec<NodeStyle> = Vec::new();
    let mut class_defs: Vec<ClassDef> = Vec::new();
    let mut class_applies: Vec<ClassApply> = Vec::new();

    let mut i = 0;
    while i < lines.len() {
        let (_line_num, line_text) = lines[i];
        if line_text.starts_with("graph ") {
            i += 1;
            continue;
        }

        if line_text.starts_with("subgraph ") {
            let sg_id = line_text[9..].trim().to_string();
            let mut sg_nodes: Vec<String> = Vec::new();
            i += 1;
            while i < lines.len() && lines[i].1 != "end" {
                let (_, inner) = lines[i];
                if inner.starts_with("subgraph ") {
                    // Nested subgraphs not supported; skip
                    i += 1;
                    while i < lines.len() && lines[i].1 != "end" {
                        i += 1;
                    }
                    if i < lines.len() {
                        i += 1;
                    }
                    continue;
                }
                collect_line(
                    inner,
                    &mut nodes,
                    &mut edges,
                    &mut seen_ids,
                    &mut sg_nodes,
                    &mut styles,
                    &mut class_defs,
                    &mut class_applies,
                );
                i += 1;
            }
            if i < lines.len() && lines[i].1 == "end" {
                i += 1;
            }
            subgraphs.push(Subgraph {
                id: sg_id,
                nodes: sg_nodes,
            });
            continue;
        }

        collect_line(
            line_text,
            &mut nodes,
            &mut edges,
            &mut seen_ids,
            &mut Vec::new(),
            &mut styles,
            &mut class_defs,
            &mut class_applies,
        );
        i += 1;
    }

    Ok(Diagram {
        rankdir,
        nodes,
        edges,
        subgraphs,
        styles,
        class_defs,
        class_applies,
    })
}

fn collect_line(
    line: &str,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    seen_ids: &mut HashSet<String>,
    sg_nodes: &mut Vec<String>,
    styles: &mut Vec<NodeStyle>,
    class_defs: &mut Vec<ClassDef>,
    class_applies: &mut Vec<ClassApply>,
) {
    if line.starts_with("style ") {
        if let Some((node_id, props)) = parse_style_line(line) {
            styles.push(NodeStyle { node_id, properties: props });
        }
        return;
    }
    if line.starts_with("classDef ") {
        if let Some((name, props)) = parse_classdef_line(line) {
            class_defs.push(ClassDef { name, properties: props });
        }
        return;
    }
    if line.starts_with("class ") {
        if let Some(ca) = parse_class_line(line) {
            class_applies.push(ca);
        }
        return;
    }

    let pl = split_arrows(line);
    let mut prev_id: Option<String> = None;
    let mut pending_label: Option<String> = None;

    for (seg_idx, segment) in pl.segments.iter().enumerate() {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }

        let (label, rest) = extract_label(segment, pending_label.take());
        if let Some(l) = label {
            pending_label = Some(l);
        }

        if rest.is_empty() {
            continue;
        }

        if seg_idx > 0 {
            let (id, text, shape) = match parse_node_def(&rest) {
                Some(t) => t,
                None => continue,
            };
            ensure_node(&id, &text, shape, nodes, seen_ids);
            if !sg_nodes.contains(&id) {
                sg_nodes.push(id.clone());
            }
            let lbl = pending_label.take().unwrap_or_default();
            let style = pl
                .arrow_types
                .get(seg_idx - 1)
                .copied()
                .unwrap_or(EdgeStyle::Arrow);
            if let Some(ref prev) = prev_id {
                edges.push(Edge {
                    from: prev.clone(),
                    to: id.clone(),
                    label: lbl,
                    style,
                });
            }
            prev_id = Some(id);
        } else {
            if let Some((id, text, shape)) = parse_node_def(&rest) {
                ensure_node(&id, &text, shape, nodes, seen_ids);
                if !sg_nodes.contains(&id) {
                    sg_nodes.push(id.clone());
                }
                if let Some(ref prev) = prev_id {
                    edges.push(Edge {
                        from: prev.clone(),
                        to: id.to_string(),
                        label: String::new(),
                        style: EdgeStyle::Arrow,
                    });
                }
                prev_id = Some(id);
            } else {
                for p in split_nodes(&rest) {
                    if let Some((id, text, shape)) = parse_node_def(&p) {
                        ensure_node(&id, &text, shape, nodes, seen_ids);
                        if !sg_nodes.contains(&id) {
                            sg_nodes.push(id.clone());
                        }
                        if let Some(ref prev) = prev_id {
                            edges.push(Edge {
                                from: prev.clone(),
                                to: id.to_string(),
                                label: String::new(),
                                style: EdgeStyle::Arrow,
                            });
                        }
                        prev_id = Some(id);
                    }
                }
            }
        }
    }
}

fn parse_style_line(line: &str) -> Option<(String, String)> {
    let rest = line[6..].trim();
    let mut parts = rest.splitn(2, |c: char| c.is_whitespace());
    let node_id = parts.next()?.trim().to_string();
    let props = parts.next()?.trim().to_string();
    Some((unquote_id(&node_id), props))
}

fn parse_classdef_line(line: &str) -> Option<(String, String)> {
    let rest = line[9..].trim();
    let mut parts = rest.splitn(2, |c: char| c.is_whitespace());
    let name = parts.next()?.trim().to_string();
    let props = parts.next()?.trim().to_string();
    Some((name, props))
}

fn parse_class_line(line: &str) -> Option<ClassApply> {
    let rest = line[6..].trim();
    let mut parts = rest.rsplitn(2, |c: char| c.is_whitespace());
    let class_name = parts.next()?.trim().to_string();
    let ids_str = parts.next()?.trim();
    let node_ids: Vec<String> = ids_str
        .split(',')
        .map(|s| unquote_id(s.trim()))
        .filter(|s| !s.is_empty())
        .collect();
    if node_ids.is_empty() {
        return None;
    }
    Some(ClassApply { node_ids, class_name })
}

struct ParsedLine {
    segments: Vec<String>,
    arrow_types: Vec<EdgeStyle>,
}

fn split_arrows(s: &str) -> ParsedLine {
    let mut segments: Vec<String> = Vec::new();
    let mut arrow_types: Vec<EdgeStyle> = Vec::new();
    let mut start = 0;
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if i + 4 <= len && &s[i..i + 4] == "-.->" {
            if i > start {
                segments.push(s[start..i].to_string());
            }
            arrow_types.push(EdgeStyle::Dashed);
            i += 4;
            start = i;
        } else if i + 3 <= len && &s[i..i + 3] == "-->" {
            if i > start {
                segments.push(s[start..i].to_string());
            }
            arrow_types.push(EdgeStyle::Arrow);
            i += 3;
            start = i;
        } else if i + 3 <= len && &s[i..i + 3] == "==>" {
            if i > start {
                segments.push(s[start..i].to_string());
            }
            arrow_types.push(EdgeStyle::Thick);
            i += 3;
            start = i;
        } else if i + 3 <= len && &s[i..i + 3] == "===" {
            if i > start {
                segments.push(s[start..i].to_string());
            }
            arrow_types.push(EdgeStyle::Thick);
            i += 3;
            start = i;
        } else if i + 2 <= len && &s[i..i + 2] == "->" {
            if i > start {
                segments.push(s[start..i].to_string());
            }
            arrow_types.push(EdgeStyle::Arrow);
            i += 2;
            start = i;
        } else {
            i += 1;
        }
    }
    if start < len {
        segments.push(s[start..].to_string());
    }

    ParsedLine {
        segments,
        arrow_types,
    }
}

fn split_nodes(s: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for c in s.chars() {
        if c == '"' {
            in_quotes = !in_quotes;
            current.push(c);
        } else if (c == ',' || c.is_whitespace()) && !in_quotes {
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                result.push(trimmed.to_string());
            }
            current.clear();
        } else {
            current.push(c);
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        result.push(trimmed.to_string());
    }
    result
}

fn extract_label(s: &str, existing: Option<String>) -> (Option<String>, String) {
    let s = s.trim();
    if let Some(pos) = s.find('|') {
        if let Some(end) = s[pos + 1..].find('|') {
            let label = s[pos + 1..pos + 1 + end].to_string();
            let rest = format!("{} {}", &s[..pos].trim(), &s[pos + 2 + end..].trim());
            return (Some(label), rest.trim().to_string());
        }
    }
    if let Some(existing) = existing {
        return (Some(existing), s.to_string());
    }
    (None, s.to_string())
}

fn parse_rankdir(line: &str) -> String {
    let rest = line[6..].trim().to_uppercase();
    match rest.as_str() {
        "LR" | "RL" | "BT" => rest,
        "TD" => "TB".to_string(),
        _ => "TB".to_string(),
    }
}

fn parse_node_def(s: &str) -> Option<(String, String, NodeShape)> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    if let Some((id, text)) = extract_cylinder(s) {
        if is_valid_id(&id) {
            return Some((unquote_id(&id), text, NodeShape::Cylinder));
        }
    }

    if let Some((id, text)) = extract_bracketed(s, '[', ']') {
        if is_valid_id(&id) {
            return Some((unquote_id(&id), text, NodeShape::Rect));
        }
    }

    if let Some((id, text)) = extract_bracketed_double(s, "{{", "}}") {
        if is_valid_id(&id) {
            return Some((unquote_id(&id), text, NodeShape::Hexagon));
        }
    }

    if let Some((id, text)) = extract_bracketed(s, '{', '}') {
        if is_valid_id(&id) {
            return Some((unquote_id(&id), text, NodeShape::Diamond));
        }
    }

    if let Some((id, text)) = extract_bracketed_double(s, "((", "))") {
        if is_valid_id(&id) {
            return Some((unquote_id(&id), text, NodeShape::Circle));
        }
    }

    if let Some((id, text)) = extract_bracketed(s, '(', ')') {
        if is_valid_id(&id) {
            if text.contains('[') || text.starts_with('(') {
                return None;
            }
            return Some((unquote_id(&id), text, NodeShape::Stadium));
        }
    }

    if is_valid_id(s) {
        let id = unquote_id(s);
        return Some((id.clone(), id, NodeShape::Rect));
    }

    None
}

fn extract_cylinder(s: &str) -> Option<(String, String)> {
    let s = s.trim();
    if let Some(pos) = s.find("[(") {
        if s.ends_with(")]") {
            let id = &s[..pos];
            let inner = &s[pos + 2..s.len() - 2];
            return Some((id.to_string(), inner.to_string()));
        }
    }
    None
}

fn is_valid_id(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        return true;
    }
    s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn unquote_id(s: &str) -> String {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

fn extract_bracketed(s: &str, open: char, close: char) -> Option<(String, String)> {
    if let Some(pos) = s.find(open) {
        if s.ends_with(close) {
            let id = &s[..pos];
            let inner = &s[pos + 1..s.len() - 1];
            return Some((id.to_string(), inner.to_string()));
        }
    }
    None
}

fn extract_bracketed_double(s: &str, open: &str, close: &str) -> Option<(String, String)> {
    if let Some(pos) = s.find(open) {
        if s.ends_with(close) {
            let id = &s[..pos];
            let inner = &s[pos + open.len()..s.len() - close.len()];
            return Some((id.to_string(), inner.to_string()));
        }
    }
    None
}

fn ensure_node(
    id: &str,
    text: &str,
    shape: NodeShape,
    nodes: &mut Vec<Node>,
    seen_ids: &mut HashSet<String>,
) {
    if seen_ids.insert(id.to_string()) {
        nodes.push(Node {
            id: id.to_string(),
            text: text.to_string(),
            shape,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_graph() {
        let source = "graph TD\n    A[Start] --> B{Is it?}\n    B -->|Yes| C[End]";
        let diagram = parse(source).unwrap();
        assert_eq!(diagram.rankdir, "TB");
        assert_eq!(diagram.nodes.len(), 3);
        assert_eq!(diagram.edges.len(), 2);
    }

    #[test]
    fn test_parse_bare_nodes() {
        let source = "graph LR\n    A --> B";
        let diagram = parse(source).unwrap();
        assert_eq!(diagram.nodes.len(), 2);
        assert_eq!(diagram.edges.len(), 1);
    }

    #[test]
    fn test_parse_lr() {
        let source = "graph LR\n    A --> B";
        let diagram = parse(source).unwrap();
        assert_eq!(diagram.rankdir, "LR");
    }

    #[test]
    fn test_to_mermaid_roundtrip() {
        let source = "graph TD\n    A[Start] --> B{Is it?}\n    B -->|Yes| C[End]";
        let diagram = parse(source).unwrap();
        let output = diagram.to_mermaid();
        let parsed = parse(&output).unwrap();
        assert_eq!(parsed.nodes.len(), 3);
        assert_eq!(parsed.edges.len(), 2);
    }

    #[test]
    fn test_cylinder_syntax() {
        assert_eq!(extract_cylinder("B[(DB)]"), Some(("B".into(), "DB".into())));
        assert_eq!(extract_cylinder("X[(text)]"), Some(("X".into(), "text".into())));
    }

    #[test]
    fn test_new_shapes() {
        let source = "graph LR\n    A{{Hex}} --> B[(DB)]\n    B -.-> C((Circle))\n    C ==> D[End]";
        let diagram = parse(source).unwrap();
        assert_eq!(diagram.nodes.len(), 4);
        assert_eq!(diagram.edges.len(), 3);
        assert_eq!(diagram.nodes[0].shape, NodeShape::Hexagon);
        assert_eq!(diagram.nodes[1].shape, NodeShape::Cylinder, "B should be Cylinder, got {:?}", diagram.nodes[1].shape);
        assert_eq!(diagram.nodes[2].shape, NodeShape::Circle);
        assert_eq!(diagram.nodes[3].shape, NodeShape::Rect);
        assert_eq!(diagram.edges[0].style, EdgeStyle::Arrow);
        assert_eq!(diagram.edges[1].style, EdgeStyle::Dashed);
        assert_eq!(diagram.edges[2].style, EdgeStyle::Thick);
    }

    #[test]
    fn test_new_shapes_roundtrip() {
        let source = "graph LR\n    A{{Hex}} --> B[(DB)]\n    B -.-> C((Circle))\n    C ==> D[End]";
        let diagram = parse(source).unwrap();
        let output = diagram.to_mermaid();
        let parsed = parse(&output).unwrap();
        assert_eq!(parsed.nodes.len(), 4);
        assert_eq!(parsed.edges.len(), 3);
        assert_eq!(parsed.nodes[0].shape, NodeShape::Hexagon);
        assert_eq!(parsed.nodes[1].shape, NodeShape::Cylinder);
        assert_eq!(parsed.edges[1].style, EdgeStyle::Dashed);
        assert_eq!(parsed.edges[2].style, EdgeStyle::Thick);
    }

    #[test]
    fn test_quoted_ids() {
        let source = r#"graph TD
    "my node"[Start] --> "other node"[End]"#;
        let diagram = parse(source).unwrap();
        assert_eq!(diagram.nodes.len(), 2);
        assert_eq!(diagram.nodes[0].id, "my node");
        assert_eq!(diagram.nodes[1].id, "other node");
        assert_eq!(diagram.edges.len(), 1);
        assert_eq!(diagram.edges[0].from, "my node");
        assert_eq!(diagram.edges[0].to, "other node");
    }

    #[test]
    fn test_quoted_ids_with_diamond() {
        let source = "graph TD\n    \"user login\"[User Login] --> \"auth service\"{Auth Service}\n    \"auth service\" --> \"token issued\"[Token Issued]\n    \"auth service\" --> \"login failed\"[Login Failed]";
        let diagram = parse(source).unwrap();
        assert_eq!(diagram.nodes.len(), 4, "expected 4 nodes, got {}", diagram.nodes.len());
        assert!(diagram.nodes.iter().any(|n| n.id == "user login"));
    }

    #[test]
    fn test_quoted_ids_roundtrip() {
        let source = r#"graph TD
    "my node"[Start] --> "other node"[End]"#;
        let diagram = parse(source).unwrap();
        let output = diagram.to_mermaid();
        let parsed = parse(&output).unwrap();
        assert_eq!(parsed.nodes[0].id, "my node");
        assert_eq!(parsed.nodes[1].id, "other node");
    }

    #[test]
    fn test_subgraphs() {
        let source = "graph TD\n    subgraph One\n        A --> B\n    end\n    B --> C";
        let diagram = parse(source).unwrap();
        assert_eq!(diagram.nodes.len(), 3);
        assert_eq!(diagram.edges.len(), 2);
        assert_eq!(diagram.subgraphs.len(), 1);
        assert_eq!(diagram.subgraphs[0].id, "One");
        assert!(diagram.subgraphs[0].nodes.contains(&"A".to_string()));
        assert!(diagram.subgraphs[0].nodes.contains(&"B".to_string()));
    }

    #[test]
    fn test_subgraph_roundtrip() {
        let source = "graph TD\n    subgraph One\n        A --> B\n    end\n    B --> C";
        let diagram = parse(source).unwrap();
        let output = diagram.to_mermaid();
        let parsed = parse(&output).unwrap();
        assert_eq!(parsed.subgraphs.len(), 1);
        assert_eq!(parsed.subgraphs[0].id, "One");
    }

    #[test]
    fn test_styles_and_classdef() {
        let source = "graph TD\n    A[Start] --> B[End]\n    style A fill:#f9f\n    classDef myClass fill:#bbf\n    class A,B myClass";
        let diagram = parse(source).unwrap();
        assert_eq!(diagram.styles.len(), 1);
        assert_eq!(diagram.styles[0].node_id, "A");
        assert_eq!(diagram.styles[0].properties, "fill:#f9f");
        assert_eq!(diagram.class_defs.len(), 1);
        assert_eq!(diagram.class_defs[0].name, "myClass");
        assert_eq!(diagram.class_defs[0].properties, "fill:#bbf");
        assert_eq!(diagram.class_applies.len(), 1);
        assert_eq!(diagram.class_applies[0].node_ids, vec!["A", "B"]);
        assert_eq!(diagram.class_applies[0].class_name, "myClass");
    }

    #[test]
    fn test_styles_roundtrip() {
        let source = "graph TD\n    A[Start] --> B[End]\n    style A fill:#f9f\n    classDef myClass fill:#bbf\n    class A,B myClass";
        let diagram = parse(source).unwrap();
        let output = diagram.to_mermaid();
        assert!(output.contains("style A fill:#f9f"));
        assert!(output.contains("classDef myClass fill:#bbf"));
        assert!(output.contains("class A,B myClass"));
    }
}
