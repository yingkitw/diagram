//! TypeScript source extraction: tree-sitter-typescript → canonical IR.
//!
//! Mirrors the shape of [`super::rust_lang`] but adapts to TS / TS-like AST
//! node kinds (class_declaration, interface_declaration, function_declaration,
//! etc.). MVP supports `.ts` only; `.tsx` JSX can be added by switching the
//! grammar constant.

use super::{CodeGenError, CodeKind};
use crate::class::{Class, ClassDiagram, ClassMember, Relation, RelationKind};
use crate::diagram::{Diagram as Flowchart, Edge, EdgeStyle, Node, NodeShape};
use crate::ir::{Diagram, Document};
use std::collections::HashSet;
use tree_sitter::{Language, Node as TsNode, Parser};

fn typescript_language() -> Language {
    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
}

fn parse_tree(source: &str) -> Result<tree_sitter::Tree, CodeGenError> {
    let mut parser = Parser::new();
    parser
        .set_language(&typescript_language())
        .map_err(|e| {
            CodeGenError::from(format!("failed to set tree-sitter typescript language: {e}"))
        })?;
    parser
        .parse(source.as_bytes(), None)
        .ok_or_else(|| CodeGenError::from("tree-sitter failed to parse typescript source"))
}

fn node_text<'a>(source: &'a str, node: TsNode) -> &'a str {
    &source[node.byte_range()]
}

fn child_by_kind<'a>(node: TsNode<'a>, kind: &str) -> Option<TsNode<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).find(|c| c.kind() == kind)
}

fn field_name<'a>(source: &'a str, node: TsNode<'a>, field: &str) -> Option<&'a str> {
    node.child_by_field_name(field)
        .map(|n| node_text(source, n))
}

/// Iterate the root's top-level nodes, unwrapping `export_statement` so that
/// `export class Foo` is seen the same as `class Foo`. Calls `f` once per
/// effective declaration.
fn for_each_top_level<'a, F: FnMut(TsNode<'a>)>(root: TsNode<'a>, mut f: F) {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "export_statement" {
            let mut c2 = child.walk();
            for inner in child.children(&mut c2) {
                if matches!(
                    inner.kind(),
                    "class_declaration"
                        | "interface_declaration"
                        | "enum_declaration"
                        | "function_declaration"
                        | "lexical_declaration"
                        | "variable_statement"
                        | "type_alias_declaration"
                ) {
                    f(inner);
                }
            }
        } else {
            f(child);
        }
    }
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

    for_each_top_level(root, |node| match node.kind() {
        "class_declaration" => collect_class(source, node, &mut class_diagram),
        "interface_declaration" => collect_interface(source, node, &mut class_diagram),
        "enum_declaration" => collect_enum(source, node, &mut class_diagram),
        _ => {}
    });

    Ok(Document::single(Diagram::Class(class_diagram)))
}

fn collect_class(source: &str, node: TsNode, out: &mut ClassDiagram) {
    let Some(name) = field_name(source, node, "name") else {
        return;
    };
    let class_id = name.to_string();
    let mut members = Vec::new();
    if let Some(body) = child_by_kind(node, "class_body") {
        collect_class_body(source, body, &mut members);
    }
    out.classes.push(Class {
        id: class_id.clone(),
        members,
        stereotype: Some("class".into()),
    });
    collect_heritage(source, node, &class_id, out);
}

fn collect_interface(source: &str, node: TsNode, out: &mut ClassDiagram) {
    let Some(name) = field_name(source, node, "name") else {
        return;
    };
    let mut members = Vec::new();
    if let Some(body) = child_by_kind(node, "interface_body") {
        collect_interface_body(source, body, &mut members);
    }
    out.classes.push(Class {
        id: name.to_string(),
        members,
        stereotype: Some("interface".into()),
    });
}

fn collect_enum(source: &str, node: TsNode, out: &mut ClassDiagram) {
    let Some(name) = field_name(source, node, "name") else {
        return;
    };
    let mut members = Vec::new();
    if let Some(body) = child_by_kind(node, "enum_body") {
        let mut cursor = body.walk();
        for variant in body.children(&mut cursor) {
            if variant.kind() != "enum_assignment" && variant.kind() != "property_identifier" {
                continue;
            }
            let vname = if variant.kind() == "enum_assignment" {
                field_name(source, variant, "name").unwrap_or("_")
            } else {
                node_text(source, variant)
            };
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

fn collect_class_body(source: &str, body: TsNode, out: &mut Vec<ClassMember>) {
    let mut cursor = body.walk();
    for item in body.children(&mut cursor) {
        match item.kind() {
            "method_definition" | "method_signature" => {
                let name = field_name(source, item, "name").unwrap_or("_");
                let params = field_name(source, item, "parameters").unwrap_or("()");
                let ret = clean_type(field_name(source, item, "return_type"));
                let text = if ret.is_empty() {
                    format!("+{name}{params}")
                } else {
                    format!("+{name}{params} {ret}")
                };
                out.push(ClassMember { text });
            }
            "public_field_definition" | "property_signature" => {
                let name = field_name(source, item, "name").unwrap_or("_");
                let ty = clean_type(field_name(source, item, "type"));
                let ty = if ty.is_empty() { "?".to_string() } else { ty };
                out.push(ClassMember {
                    text: format!("+{name}: {ty}"),
                });
            }
            _ => {}
        }
    }
}

/// Strip a leading `: ` from `type_annotation` so members render `+x: T`
/// rather than `+x: : T`. Returns the empty string when input is `None`.
fn clean_type(s: Option<&str>) -> String {
    match s {
        Some(t) => t.trim_start_matches(':').trim().to_string(),
        None => String::new(),
    }
}

fn collect_interface_body(source: &str, body: TsNode, out: &mut Vec<ClassMember>) {
    collect_class_body(source, body, out);
}

fn collect_heritage(source: &str, class_node: TsNode, class_id: &str, out: &mut ClassDiagram) {
    let Some(heritage) = child_by_kind(class_node, "class_heritage") else {
        return;
    };
    let mut cursor = heritage.walk();
    for clause in heritage.children(&mut cursor) {
        match clause.kind() {
            "extends_clause" => collect_type_list(
                source,
                clause,
                class_id,
                RelationKind::Inheritance,
                out,
            ),
            "implements_clause" => collect_type_list(
                source,
                clause,
                class_id,
                RelationKind::Realization,
                out,
            ),
            _ => {}
        }
    }
}

fn collect_type_list(
    source: &str,
    clause: TsNode,
    from: &str,
    kind: RelationKind,
    out: &mut ClassDiagram,
) {
    let mut cursor = clause.walk();
    for child in clause.children(&mut cursor) {
        if child.kind() == "type_identifier" {
            out.relations.push(Relation {
                from: from.to_string(),
                to: node_text(source, child).to_string(),
                kind,
                label: String::new(),
                from_card: None,
                to_card: None,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Tree extraction
// ---------------------------------------------------------------------------

fn extract_tree(source: &str, root: TsNode) -> Result<Document, CodeGenError> {
    let mut fc = Flowchart::new("TD");
    let mut items: Vec<TsItem> = Vec::new();
    let mut uses: Vec<TsUse> = Vec::new();

    // Imports stay at the root (not unwrapped by export) — collect separately.
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "import_statement" {
            collect_imports(source, child, &mut uses);
        }
    }

    for_each_top_level(root, |node| match node.kind() {
        "class_declaration" => {
            if let Some(name) = field_name(source, node, "name") {
                items.push(TsItem {
                    id: format!("class::{name}"),
                    label: format!("class {name}"),
                    shape: NodeShape::Rect,
                    kind_label: "class",
                });
            }
        }
        "interface_declaration" => {
            if let Some(name) = field_name(source, node, "name") {
                items.push(TsItem {
                    id: format!("interface::{name}"),
                    label: format!("interface {name}"),
                    shape: NodeShape::Stadium,
                    kind_label: "interface",
                });
            }
        }
        "enum_declaration" => {
            if let Some(name) = field_name(source, node, "name") {
                items.push(TsItem {
                    id: format!("enum::{name}"),
                    label: format!("enum {name}"),
                    shape: NodeShape::Hexagon,
                    kind_label: "enum",
                });
            }
        }
        "function_declaration" => {
            if let Some(name) = field_name(source, node, "name") {
                items.push(TsItem {
                    id: format!("fn::{name}"),
                    label: format!("fn {name}"),
                    shape: NodeShape::Rect,
                    kind_label: "fn",
                });
            }
        }
        "lexical_declaration" | "variable_statement" => {
            if let Some(name) = top_level_const_name(source, node) {
                items.push(TsItem {
                    id: format!("const::{name}"),
                    label: format!("const {name}"),
                    shape: NodeShape::Rect,
                    kind_label: "const",
                });
            }
        }
        _ => {}
    });

    fc.add_node(Node {
        id: "__root__".into(),
        text: "file".into(),
        shape: NodeShape::Stadium,
        href: None,
        tooltip: None,
    })
    .ok();
    let mut seen: HashSet<String> = HashSet::new();
    for item in &items {
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

    for u in &uses {
        fc.add_node(Node {
            id: u.id.clone(),
            text: u.label.clone(),
            shape: NodeShape::Circle,
            href: None,
            tooltip: None,
        })
        .ok();
        fc.edges.push(Edge {
            from: "__root__".into(),
            to: u.id.clone(),
            label: "import".into(),
            style: EdgeStyle::Dashed,
        });
    }

    Ok(Document::single(Diagram::Flowchart(fc)))
}

struct TsItem {
    id: String,
    label: String,
    shape: NodeShape,
    kind_label: &'static str,
}

struct TsUse {
    id: String,
    label: String,
}

fn top_level_const_name<'a>(source: &'a str, node: TsNode<'a>) -> Option<&'a str> {
    // `const Foo = …` / `let Foo = …` — first variable_declarator's name.
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator"
            && let Some(name) = field_name(source, child, "name")
        {
            return Some(name);
        }
    }
    None
}

fn collect_imports(source: &str, import_node: TsNode, out: &mut Vec<TsUse>) {
    let source_str = field_name(source, import_node, "source");
    let source_label = source_str.unwrap_or("?").to_string();
    let Some(clause) = child_by_kind(import_node, "import_clause") else {
        // bare side-effect import: `import 'foo';`
        let id = format!("import::{source_label}");
        out.push(TsUse {
            id,
            label: source_label,
        });
        return;
    };
    let mut cursor = clause.walk();
    for child in clause.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                let name = node_text(source, child);
                out.push(TsUse {
                    id: format!("import::{name}"),
                    label: format!("{name} ← {source_label}"),
                });
            }
            "named_imports" => {
                let mut c2 = child.walk();
                for spec in child.children(&mut c2) {
                    if spec.kind() == "import_specifier" {
                        let name = field_name(source, spec, "name").unwrap_or("_");
                        out.push(TsUse {
                            id: format!("import::{name}"),
                            label: format!("{name} ← {source_label}"),
                        });
                    }
                }
            }
            "namespace_import" => {
                let name = field_name(source, child, "name").unwrap_or("*");
                out.push(TsUse {
                    id: format!("import::{name}"),
                    label: format!("* as {name} ← {source_label}"),
                });
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Call extraction
// ---------------------------------------------------------------------------

fn extract_call(source: &str, root: TsNode) -> Result<Document, CodeGenError> {
    let mut fc = Flowchart::new("LR");

    // Collect top-level function declarations, unwrapping `export`.
    let mut fn_nodes: Vec<TsNode> = Vec::new();
    let mut fn_ids: Vec<String> = Vec::new();
    for_each_top_level(root, |node| {
        if node.kind() == "function_declaration"
            && let Some(name) = field_name(source, node, "name")
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
            fn_nodes.push(node);
        }
    });

    // Pass 2: walk each function body for call_expression.
    for child in fn_nodes {
        let caller_name = match field_name(source, child, "name") {
            Some(n) => n.to_string(),
            None => continue,
        };
        let caller_id = format!("fn::{caller_name}");
        let calls = collect_calls(source, child);
        for callee in calls {
            let callee_id = format!("fn::{callee}");
            if !fn_ids.iter().any(|n| n == &callee) {
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
            if func.kind() == "identifier" {
                out.push(node_text(source, func).to_string());
            } else if func.kind() == "member_expression"
                && let Some(prop) = func.child_by_field_name("property")
            {
                out.push(node_text(source, prop).to_string());
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
export interface Shape {
    area(): number;
}

export class Circle implements Shape {
    constructor(public radius: number) {}
    area(): number {
        return Math.PI * this.radius * this.radius;
    }
}

export class Point {
    public x: number;
    public y: number;
}

export enum Color {
    Red,
    Green,
    Blue,
}

export function compute(p: Point): number {
    const a = p.area();
    return adjust(a, 1.0);
}

function adjust(v: number, by: number): number {
    return v + by;
}

import { HashMap } from "std-collections";
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
        assert!(names.contains(&"Shape"), "missing interface: {names:?}");
        assert!(names.contains(&"Circle"), "missing class: {names:?}");
        assert!(names.contains(&"Point"), "missing class: {names:?}");
        assert!(names.contains(&"Color"), "missing enum: {names:?}");
        // Circle implements Shape
        assert!(
            d.relations
                .iter()
                .any(|r| r.from == "Circle" && r.to == "Shape" && r.kind == RelationKind::Realization),
            "expected Circle ..|> Shape realization"
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
        assert!(ids.contains(&"class::Circle"), "missing Circle: {ids:?}");
        assert!(ids.contains(&"interface::Shape"), "missing Shape: {ids:?}");
        assert!(ids.contains(&"enum::Color"), "missing Color: {ids:?}");
        assert!(ids.contains(&"fn::compute"));
        assert!(ids.iter().any(|i| i.starts_with("import::")));
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
        assert!(ids.contains(&"fn::adjust"));
        // compute -> adjust (identifier call)
        assert!(d.edges.iter().any(|e| e.from == "fn::compute" && e.to == "fn::adjust"));
    }

    #[test]
    fn parses_empty_file() {
        let doc = extract("", CodeKind::Tree).unwrap();
        assert!(matches!(doc.primary().unwrap(), Diagram::Flowchart(_)));
    }
}