use crate::diagram::{Diagram, Edge, Node, NodeShape};
use std::collections::HashSet;

pub fn parse(source: &str) -> Result<Diagram, String> {
    let lines: Vec<&str> = source
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("%%") && !l.starts_with("---"))
        .collect();

    if lines.is_empty() {
        return Ok(Diagram::new("TB"));
    }

    let rankdir = if lines[0].starts_with("graph ") {
        parse_rankdir(lines[0])
    } else {
        "TB".to_string()
    };

    let mut nodes: Vec<Node> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();

    for line in &lines {
        if line.starts_with("graph ") {
            continue;
        }

        let segments = split_arrows(line);
        let mut prev_id: Option<String> = None;
        let mut pending_label: Option<String> = None;

        for (i, (part, is_arrow)) in segments.iter().enumerate() {
            let part = part.trim();
            if *is_arrow {
                continue;
            }
            if part.is_empty() {
                continue;
            }

            let (label, rest) = extract_label(part, pending_label.take());
            if let Some(l) = label {
                pending_label = Some(l);
            }

            if rest.is_empty() {
                continue;
            }

            if i > 0 {
                let (id, text, shape) = match parse_node_def(&rest) {
                    Some(t) => t,
                    None => continue,
                };
                ensure_node(&id, &text, shape, &mut nodes, &mut seen_ids);
                let lbl = pending_label.take().unwrap_or_default();
                if let Some(ref prev) = prev_id {
                    edges.push(Edge {
                        from: prev.clone(),
                        to: id.clone(),
                        label: lbl,
                    });
                }
                prev_id = Some(id);
            } else {
                let parts: Vec<&str> = rest
                    .split(|c: char| c == ',' || c.is_whitespace())
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();
                for p in parts {
                    if let Some((id, text, shape)) = parse_node_def(p) {
                        ensure_node(&id, &text, shape, &mut nodes, &mut seen_ids);
                        if let Some(ref prev) = prev_id {
                            edges.push(Edge {
                                from: prev.clone(),
                                to: id.to_string(),
                                label: String::new(),
                            });
                        }
                        prev_id = Some(id);
                    }
                }
            }
        }
    }

    Ok(Diagram {
        rankdir,
        nodes,
        edges,
    })
}

fn split_arrows(s: &str) -> Vec<(String, bool)> {
    let mut result: Vec<(String, bool)> = Vec::new();
    let mut start = 0;
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if i + 3 <= len && &s[i..i + 3] == "-->" {
            if i > start {
                result.push((s[start..i].to_string(), false));
            }
            i += 3;
            start = i;
        } else if i + 2 <= len && &s[i..i + 2] == "->" {
            if i > start {
                result.push((s[start..i].to_string(), false));
            }
            i += 2;
            start = i;
        } else {
            i += 1;
        }
    }
    if start < len {
        result.push((s[start..].to_string(), false));
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

    if let Some((id, text)) = extract_bracketed(s, '[', ']') {
        if is_valid_id(&id) {
            return Some((id, text, NodeShape::Rect));
        }
    }

    if let Some((id, text)) = extract_bracketed(s, '{', '}') {
        if is_valid_id(&id) {
            return Some((id, text, NodeShape::Diamond));
        }
    }

    if let Some((id, text)) = extract_bracketed_double(s, "{{", "}}") {
        if is_valid_id(&id) {
            return Some((id, text, NodeShape::Diamond));
        }
    }

    if let Some((id, text)) = extract_bracketed(s, '(', ')') {
        if is_valid_id(&id) {
            return Some((id, text, NodeShape::Stadium));
        }
    }

    if is_valid_id(s) {
        return Some((s.to_string(), s.to_string(), NodeShape::Rect));
    }

    None
}

fn is_valid_id(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
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
}
