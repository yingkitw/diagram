//! Rust source extraction: tree-sitter-rust → canonical IR.

use super::{CodeGenError, CodeKind};
use crate::class::{Class, ClassDiagram, ClassMember, Relation, RelationKind};
use crate::diagram::{Diagram as Flowchart, Edge, EdgeStyle, Node, NodeShape};
use crate::ir::{Diagram, Document};
use std::collections::HashSet;
use tree_sitter::{Language, Node as TsNode, Parser};

fn rust_language() -> Language {
    tree_sitter_rust::LANGUAGE.into()
}

fn parse_tree(source: &str) -> Result<tree_sitter::Tree, CodeGenError> {
    let mut parser = Parser::new();
    parser
        .set_language(&rust_language())
        .map_err(|e| CodeGenError::from(format!("failed to set tree-sitter rust language: {e}")))?;
    parser
        .parse(source.as_bytes(), None)
        .ok_or_else(|| CodeGenError::from("tree-sitter failed to parse rust source"))
}

fn node_text<'a>(source: &'a str, node: TsNode) -> &'a str {
    &source[node.byte_range()]
}

fn child_by_kind<'a>(node: TsNode<'a>, kind: &str) -> Option<TsNode<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|c| c.kind() == kind)
}

fn field_name<'a>(source: &'a str, node: TsNode<'a>, field: &str) -> Option<&'a str> {
    node.child_by_field_name(field)
        .map(|n| node_text(source, n))
}

/// Public entry point dispatched from [`super::from_source`].
pub fn extract(source: &str, kind: CodeKind) -> Result<Document, CodeGenError> {
    let tree = parse_tree(source)?;
    match kind {
        CodeKind::Class => extract_class(source, tree.root_node()),
        CodeKind::Tree => extract_tree(source, tree.root_node()),
        CodeKind::Call => extract_call(source, tree.root_node()),
    }
}

// ---------------------------------------------------------------------------
// Class extraction
// ---------------------------------------------------------------------------

fn extract_class(source: &str, root: TsNode) -> Result<Document, CodeGenError> {
    let mut class_diagram = ClassDiagram {
        classes: Vec::new(),
        relations: Vec::new(),
        notes: Vec::new(),
    };

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "struct_item" => collect_struct(source, child, &mut class_diagram),
            "enum_item" => collect_enum(source, child, &mut class_diagram),
            "trait_item" => collect_trait(source, child, &mut class_diagram),
            "impl_item" => collect_impl(source, child, &mut class_diagram),
            _ => {}
        }
    }

    Ok(Document::single(Diagram::Class(class_diagram)))
}

fn collect_struct(source: &str, node: TsNode, out: &mut ClassDiagram) {
    let Some(name) = field_name(source, node, "name") else {
        return;
    };
    let mut members = Vec::new();
    if let Some(body) = child_by_kind(node, "field_declaration_list") {
        let mut cursor = body.walk();
        for field in body.children(&mut cursor) {
            if field.kind() != "field_declaration" {
                continue;
            }
            let fname = field_name(source, field, "name").unwrap_or("_");
            let ftype = field_name(source, field, "type").unwrap_or("?");
            members.push(ClassMember {
                text: format!("+{fname}: {ftype}"),
            });
        }
    }
    out.classes.push(Class {
        id: name.to_string(),
        members,
        stereotype: Some("struct".into()),
    });
}

fn collect_enum(source: &str, node: TsNode, out: &mut ClassDiagram) {
    let Some(name) = field_name(source, node, "name") else {
        return;
    };
    let mut members = Vec::new();
    if let Some(body) = child_by_kind(node, "enum_variant_list") {
        let mut cursor = body.walk();
        for variant in body.children(&mut cursor) {
            if variant.kind() != "enum_variant" {
                continue;
            }
            let vname = field_name(source, variant, "name").unwrap_or("_");
            members.push(ClassMember {
                text: format!("+{vname}"),
            });
        }
    }
    out.classes.push(Class {
        id: name.to_string(),
        members,
        stereotype: Some("enumeration".into()),
    });
}

fn collect_trait(source: &str, node: TsNode, out: &mut ClassDiagram) {
    let Some(name) = field_name(source, node, "name") else {
        return;
    };
    let mut members = Vec::new();
    if let Some(body) = child_by_kind(node, "declaration_list") {
        collect_method_members(source, body, &mut members);
    }
    out.classes.push(Class {
        id: name.to_string(),
        members,
        stereotype: Some("interface".into()),
    });
}

fn collect_impl(source: &str, node: TsNode, out: &mut ClassDiagram) {
    let type_node = node.child_by_field_name("type");
    let trait_node = node.child_by_field_name("trait");
    let Some(type_name) = type_node.map(|n| node_text(source, n).to_string()) else {
        return;
    };
    let class_name = match trait_node {
        Some(t) => format!("{} for {}", node_text(source, t), type_name),
        None => format!("impl {type_name}"),
    };
    let mut members = Vec::new();
    if let Some(body) = child_by_kind(node, "declaration_list") {
        collect_method_members(source, body, &mut members);
    }
    out.classes.push(Class {
        id: class_name,
        members,
        stereotype: Some("service".into()),
    });
    if let Some(t) = trait_node {
        out.relations.push(Relation {
            from: type_name.clone(),
            to: node_text(source, t).to_string(),
            kind: RelationKind::Realization,
            label: String::new(),
            from_card: None,
            to_card: None,
        });
    }
}

fn collect_method_members(source: &str, body: TsNode, out: &mut Vec<ClassMember>) {
    let mut cursor = body.walk();
    for item in body.children(&mut cursor) {
        if item.kind() != "function_item" {
            continue;
        }
        let name = field_name(source, item, "name").unwrap_or("_");
        let ret = field_name(source, item, "return_type").unwrap_or("");
        let text = if ret.is_empty() {
            format!("+{name}()")
        } else {
            format!("+{name}() {ret}")
        };
        out.push(ClassMember { text });
    }
}

// ---------------------------------------------------------------------------
// Tree extraction (one file's items + use edges)
// ---------------------------------------------------------------------------

fn extract_tree(source: &str, root: TsNode) -> Result<Document, CodeGenError> {
    let mut fc = Flowchart::new("TD");
    let mut nodes: Vec<TreeItem> = Vec::new();
    let mut uses: Vec<TreeUse> = Vec::new();

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "struct_item" => {
                if let Some(name) = field_name(source, child, "name") {
                    nodes.push(TreeItem {
                        id: name.to_string(),
                        label: format!("struct {name}"),
                        shape: NodeShape::Rect,
                        kind_label: "struct",
                    });
                }
            }
            "enum_item" => {
                if let Some(name) = field_name(source, child, "name") {
                    nodes.push(TreeItem {
                        id: name.to_string(),
                        label: format!("enum {name}"),
                        shape: NodeShape::Hexagon,
                        kind_label: "enum",
                    });
                }
            }
            "trait_item" => {
                if let Some(name) = field_name(source, child, "name") {
                    nodes.push(TreeItem {
                        id: name.to_string(),
                        label: format!("trait {name}"),
                        shape: NodeShape::Stadium,
                        kind_label: "trait",
                    });
                }
            }
            "impl_item" => {
                let type_name = child
                    .child_by_field_name("type")
                    .map(|n| node_text(source, n).to_string());
                if let Some(name) = type_name {
                    nodes.push(TreeItem {
                        id: format!("impl_{name}"),
                        label: format!("impl {name}"),
                        shape: NodeShape::Rect,
                        kind_label: "impl",
                    });
                }
            }
            "function_item" => {
                if let Some(name) = field_name(source, child, "name") {
                    nodes.push(TreeItem {
                        id: format!("fn_{name}"),
                        label: format!("fn {name}"),
                        shape: NodeShape::Rect,
                        kind_label: "fn",
                    });
                }
            }
            "use_declaration" => {
                collect_uses(source, child, &mut uses);
            }
            _ => {}
        }
    }

    // Root file node + edges from root to every item.
    fc.add_node(Node {
        id: "__root__".into(),
        text: "file".into(),
        shape: NodeShape::Stadium,
        href: None,
        tooltip: None,
    })
    .ok();
    let mut seen: HashSet<String> = HashSet::new();
    for item in &nodes {
        if seen.insert(item.id.clone()) {
            fc.add_node(Node {
                id: item.id.clone(),
                text: item.label.clone(),
                shape: item.shape,
                href: None,
                tooltip: None,
            })
            .ok();
        }
        fc.edges.push(Edge {
            from: "__root__".into(),
            to: item.id.clone(),
            label: item.kind_label.into(),
            style: EdgeStyle::Arrow,
        });
    }

    // use edges → shorthand label
    for u in &uses {
        if !seen.contains(u.from.as_str()) {
            fc.add_node(Node {
                id: u.from.clone(),
                text: u.label.clone(),
                shape: NodeShape::Circle,
                href: None,
                tooltip: None,
            })
            .ok();
            seen.insert(u.from.clone());
        }
        fc.edges.push(Edge {
            from: "__root__".into(),
            to: u.from.clone(),
            label: "use".into(),
            style: EdgeStyle::Dashed,
        });
    }

    Ok(Document::single(Diagram::Flowchart(fc)))
}

struct TreeItem {
    id: String,
    label: String,
    shape: NodeShape,
    kind_label: &'static str,
}

struct TreeUse {
    from: String,
    label: String,
}

fn collect_uses(source: &str, use_node: TsNode, out: &mut Vec<TreeUse>) {
    let mut cursor = use_node.walk();
    for child in use_node.children(&mut cursor) {
        if child.kind() == "use_list" || child.kind() == "use_as_clause" || child.kind() == "use_wildcard" {
            // Flatten: collect every identifier/alias path inside.
            collect_use_paths(source, child, out);
            return;
        }
        if child.kind() == "scoped_use_identifier"
            || child.kind() == "scoped_identifier"
            || child.kind() == "identifier"
            || child.kind() == "alias"
        {
            collect_use_paths(source, child, out);
            return;
        }
    }
}

fn collect_use_paths(source: &str, node: TsNode, out: &mut Vec<TreeUse>) {
    // Build a label from the textual source of the subtree; map to a node id
    // by the leaf segment so the diagram stays compact.
    let label = node_text(source, node).replace('\n', " ").trim().to_string();
    if label.is_empty() {
        return;
    }
    let leaf = label
        .rsplit("::")
        .next()
        .unwrap_or(&label)
        .split(" as ")
        .next()
        .unwrap_or(label.as_str())
        .trim()
        .to_string();
    out.push(TreeUse {
        from: format!("use::{leaf}"),
        label,
    });
}

// ---------------------------------------------------------------------------
// Call extraction
// ---------------------------------------------------------------------------

fn extract_call(source: &str, root: TsNode) -> Result<Document, CodeGenError> {
    let mut fc = Flowchart::new("LR");

    // Pass 1: collect function definitions.
    let mut fn_ids: Vec<String> = Vec::new();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "function_item"
            && let Some(name) = field_name(source, child, "name")
        {
            let id = format!("fn::{name}");
            fn_ids.push(name.to_string());
            fc.add_node(Node {
                id,
                text: name.to_string(),
                shape: NodeShape::Rect,
                href: None,
                tooltip: None,
            })
            .ok();
        }
    }

    // Pass 2: for each function body, walk for call_expression identifiers.
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() != "function_item" {
            continue;
        }
        let caller_name = match field_name(source, child, "name") {
            Some(n) => n.to_string(),
            None => continue,
        };
        let caller_id = format!("fn::{caller_name}");
        let calls = collect_calls(source, child);
        for callee in calls {
            let callee_id = format!("fn::{callee}");
            if !fn_ids.iter().any(|n| n == &callee) {
                // Synthesize an external callee node so the edge still draws.
                fc.add_node(Node {
                    id: callee_id.clone(),
                    text: callee.clone(),
                    shape: NodeShape::Circle,
                    href: None,
                    tooltip: None,
                })
                .ok();
            }
            fc.edges.push(Edge {
                from: caller_id.clone(),
                to: callee_id,
                label: String::new(),
                style: EdgeStyle::Arrow,
            });
        }
    }

    Ok(Document::single(Diagram::Flowchart(fc)))
}

fn collect_calls(source: &str, root: TsNode) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.kind() == "call_expression"
            && let Some(func) = node.child_by_field_name("function")
        {
            // Simple identifier call: foo(...)
            if func.kind() == "identifier" {
                out.push(node_text(source, func).to_string());
            } else if func.kind() == "field_expression"
                && let Some(field) = func.child_by_field_name("field")
            {
                // method call x.foo(...) — take the field name only.
                out.push(node_text(source, field).to_string());
            }
        }
        let mut cursor = node.walk();
        for c in node.children(&mut cursor) {
            stack.push(c);
        }
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Kind;

    const SAMPLE: &str = r#"
struct Point {
    x: f64,
    y: f64,
}

enum Color {
    Red,
    Green,
    Blue,
}

trait Shape {
    fn area(&self) -> f64;
}

impl Shape for Point {
    fn area(&self) -> f64 {
        self.x * self.y
    }
}

fn compute(p: Point) -> f64 {
    let a = p.area();
    helper(a)
}

fn helper(v: f64) -> f64 {
    v + 1.0
}

use std::collections::HashMap;
"#;

    #[test]
    fn extract_class_kind() {
        let doc = extract(SAMPLE, CodeKind::Class).unwrap();
        assert_eq!(doc.primary().unwrap().kind(), Kind::Class);
        let d = match doc.primary().unwrap() {
            Diagram::Class(c) => c,
            _ => unreachable!(),
        };
        let names: Vec<&str> = d.classes.iter().map(|c| c.id.as_str()).collect();
        assert!(names.contains(&"Point"), "missing struct class: {names:?}");
        assert!(names.contains(&"Color"), "missing enum class: {names:?}");
        assert!(names.contains(&"Shape"), "missing trait class: {names:?}");
        assert!(
            d.relations
                .iter()
                .any(|r| r.from == "Point" && r.to == "Shape"),
            "expected Point --|> Shape realization"
        );
    }

    #[test]
    fn extract_tree_kind() {
        let doc = extract(SAMPLE, CodeKind::Tree).unwrap();
        assert_eq!(doc.primary().unwrap().kind(), Kind::Flowchart);
        let d = match doc.primary().unwrap() {
            Diagram::Flowchart(f) => f,
            _ => unreachable!(),
        };
        let ids: Vec<&str> = d.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"Point"), "missing struct node: {ids:?}");
        assert!(ids.contains(&"Color"), "missing enum node: {ids:?}");
        assert!(ids.contains(&"Shape"), "missing trait node: {ids:?}");
        assert!(ids.iter().any(|i| i.starts_with("use::")));
    }

    #[test]
    fn extract_call_kind() {
        let doc = extract(SAMPLE, CodeKind::Call).unwrap();
        assert_eq!(doc.primary().unwrap().kind(), Kind::Flowchart);
        let d = match doc.primary().unwrap() {
            Diagram::Flowchart(f) => f,
            _ => unreachable!(),
        };
        let ids: Vec<&str> = d.nodes.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"fn::compute"));
        assert!(ids.contains(&"fn::helper"));
        // compute -> helper
        assert!(d.edges.iter().any(|e| e.from == "fn::compute" && e.to == "fn::helper"));
        // Point impl: compute(impl Shape for Point).area() — method call counted
        assert!(d.edges.iter().any(|e| e.from == "fn::compute" && e.to == "fn::area"));
    }

    #[test]
    fn parses_unparseable_with_helpful_error() {
        // Tree-sitter is forgiving; an empty file should still produce an
        // empty document, not an error.
        let doc = extract("", CodeKind::Tree).unwrap();
        assert!(matches!(doc.primary().unwrap(), Diagram::Flowchart(_)));
    }
}