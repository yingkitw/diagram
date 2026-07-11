use rmcp::{
    ServerHandler,
    handler::server::wrapper::Parameters,
    model::*,
    schemars, tool, tool_handler, tool_router,
};
use rmcp::service::{MaybeSendFuture, RequestContext, RoleServer};

use crate::{diagram as dg, parser, renderer};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct FilePath {
    #[schemars(description = "Path to a diagram file (Mermaid or JSON IR)")]
    path: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct CreateParams {
    #[schemars(description = "Diagram kind: flowchart, sequence, class, or gantt")]
    kind: String,
    #[schemars(description = "Output file path (.mmd or .json)")]
    output: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ImportParams {
    #[schemars(description = "Source diagram file path")]
    path: String,
    #[schemars(description = "Output JSON IR file path")]
    output: String,
    #[schemars(description = "Source format: mermaid or json (auto-detect if omitted)")]
    from: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct ExportParams {
    #[schemars(description = "Source diagram file path (Mermaid or JSON IR)")]
    path: String,
    #[schemars(description = "Output file path")]
    output: String,
    #[schemars(description = "Target format: mermaid or json (auto-detect if omitted)")]
    to: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct RenderParams {
    #[schemars(description = "Path to the mermaid .mmd file")]
    path: String,
    #[schemars(description = "Theme: dark or light")]
    theme: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct NodeParams {
    #[schemars(description = "Path to the mermaid .mmd file")]
    path: String,
    #[schemars(description = "Node ID")]
    id: String,
    #[schemars(description = "Node display text")]
    text: String,
    #[schemars(description = "Node shape: rect, diamond, stadium, hexagon, cylinder, circle")]
    shape: Option<String>,
    #[schemars(description = "Hyperlink URL for the node")]
    href: Option<String>,
    #[schemars(description = "Tooltip text for the node")]
    tooltip: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct NodeUpdateParams {
    #[schemars(description = "Path to the mermaid .mmd file")]
    path: String,
    #[schemars(description = "Node ID")]
    id: String,
    #[schemars(description = "New display text")]
    text: Option<String>,
    #[schemars(description = "New shape: rect, diamond, stadium, hexagon, cylinder, circle")]
    shape: Option<String>,
    #[schemars(description = "New hyperlink URL for the node")]
    href: Option<String>,
    #[schemars(description = "New tooltip text for the node")]
    tooltip: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MermaidSourceParams {
    #[schemars(description = "Path to the mermaid .mmd file")]
    path: String,
    #[schemars(description = "Raw mermaid source code to write")]
    source: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct EdgeParams {
    #[schemars(description = "Path to the mermaid .mmd file")]
    path: String,
    #[schemars(description = "Source node ID")]
    from: String,
    #[schemars(description = "Target node ID")]
    to: String,
    #[schemars(description = "Edge label")]
    label: Option<String>,
    #[schemars(description = "Edge style: arrow, dashed, or thick")]
    style: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct EdgeRemoveParams {
    #[schemars(description = "Path to the mermaid .mmd file")]
    path: String,
    #[schemars(description = "Source node ID")]
    from: String,
    #[schemars(description = "Target node ID")]
    to: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct EdgeUpdateParams {
    #[schemars(description = "Path to the mermaid .mmd file")]
    path: String,
    #[schemars(description = "Source node ID")]
    from: String,
    #[schemars(description = "Target node ID")]
    to: String,
    #[schemars(description = "New edge label")]
    label: Option<String>,
    #[schemars(description = "New edge style: arrow, dashed, or thick")]
    style: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct NodeIdParams {
    #[schemars(description = "Path to the mermaid .mmd file")]
    path: String,
    #[schemars(description = "Node ID")]
    id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct NodeItem {
    #[schemars(description = "Node ID")]
    id: String,
    #[schemars(description = "Node display text")]
    text: String,
    #[schemars(description = "Node shape: rect, diamond, stadium, hexagon, cylinder, circle")]
    shape: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct BatchNodeParams {
    #[schemars(description = "Path to the mermaid .mmd file")]
    path: String,
    #[schemars(description = "List of nodes to add")]
    items: Vec<NodeItem>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct EdgeItem {
    #[schemars(description = "Source node ID")]
    from: String,
    #[schemars(description = "Target node ID")]
    to: String,
    #[schemars(description = "Edge label")]
    label: Option<String>,
    #[schemars(description = "Edge style: arrow, dashed, or thick")]
    style: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct BatchEdgeParams {
    #[schemars(description = "Path to the mermaid .mmd file")]
    path: String,
    #[schemars(description = "List of edges to add")]
    items: Vec<EdgeItem>,
}

fn read_file(path: &str) -> Result<dg::Diagram, CallToolResult> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| CallToolResult::error(vec![ContentBlock::text(format!(
            "Failed to read file '{}': {}",
            path, e
        ))]))?;
    parser::parse(&content).map_err(|e| {
        CallToolResult::error(vec![ContentBlock::text(e.to_string())])
    })
}

fn write_file(path: &str, diagram: &dg::Diagram) -> Result<(), CallToolResult> {
    let mermaid = diagram.to_mermaid();
    std::fs::write(path, &mermaid).map_err(|e| {
        CallToolResult::error(vec![ContentBlock::text(format!(
            "Failed to write file '{}': {}",
            path, e
        ))])
    })
}

fn modify_file<F>(path: &str, f: F) -> CallToolResult
where
    F: FnOnce(&mut dg::Diagram) -> Result<String, CallToolResult>,
{
    let mut diagram = match read_file(path) {
        Ok(d) => d,
        Err(e) => return e,
    };
    let result = match f(&mut diagram) {
        Ok(r) => r,
        Err(e) => return e,
    };
    if let Err(e) = write_file(path, &diagram) {
        return e;
    }
    CallToolResult::success(vec![ContentBlock::text(result)])
}

#[derive(Debug, Clone)]
pub struct DiagramServer;

#[tool_router]
impl DiagramServer {
    #[tool(description = "Parse a diagram file and return canonical JSON IR")]
    async fn parse_diagram(&self, Parameters(params): Parameters<FilePath>) -> CallToolResult {
        match crate::ir::load_path(&params.path) {
            Ok(doc) => match doc.to_json() {
                Ok(json) => CallToolResult::success(vec![ContentBlock::text(json)]),
                Err(e) => CallToolResult::error(vec![ContentBlock::text(e.to_string())]),
            },
            Err(e) => CallToolResult::error(vec![ContentBlock::text(e.to_string())]),
        }
    }

    #[tool(description = "Get diagram summary (kind, counts, IR version)")]
    async fn get_info(&self, Parameters(params): Parameters<FilePath>) -> CallToolResult {
        match crate::ir::load_path(&params.path) {
            Ok(doc) => {
                let summary = crate::ir::info_json(&params.path, &doc);
                CallToolResult::success(vec![ContentBlock::text(summary.to_string())])
            }
            Err(e) => CallToolResult::error(vec![ContentBlock::text(e.to_string())]),
        }
    }

    #[tool(description = "Import a diagram into canonical JSON IR")]
    async fn import_diagram(&self, Parameters(params): Parameters<ImportParams>) -> CallToolResult {
        let from = match params.from.as_deref() {
            Some(s) => match crate::formats::Format::parse(s) {
                Some(f) => Some(f),
                None => {
                    return CallToolResult::error(vec![ContentBlock::text(format!(
                        "Invalid from format '{s}'. Use: mermaid, json"
                    ))]);
                }
            },
            None => None,
        };
        let (doc, fmt) = match crate::formats::import_path(&params.path, from) {
            Ok(v) => v,
            Err(e) => return CallToolResult::error(vec![ContentBlock::text(e.to_string())]),
        };
        match crate::formats::export_path(&doc, &params.output, Some(crate::formats::Format::JsonIr)) {
            Ok(_) => CallToolResult::success(vec![ContentBlock::text(
                serde_json::json!({
                    "status": "ok",
                    "from": fmt.as_str(),
                    "output": params.output,
                })
                .to_string(),
            )]),
            Err(e) => CallToolResult::error(vec![ContentBlock::text(e.to_string())]),
        }
    }

    #[tool(description = "Export a diagram to Mermaid or JSON IR")]
    async fn export_diagram(&self, Parameters(params): Parameters<ExportParams>) -> CallToolResult {
        let to = match params.to.as_deref() {
            Some(s) => match crate::formats::Format::parse(s) {
                Some(f) => Some(f),
                None => {
                    return CallToolResult::error(vec![ContentBlock::text(format!(
                        "Invalid to format '{s}'. Use: mermaid, json"
                    ))]);
                }
            },
            None => None,
        };
        let (doc, _) = match crate::formats::import_path(&params.path, None) {
            Ok(v) => v,
            Err(e) => return CallToolResult::error(vec![ContentBlock::text(e.to_string())]),
        };
        match crate::formats::export_path(&doc, &params.output, to) {
            Ok(fmt) => CallToolResult::success(vec![ContentBlock::text(
                serde_json::json!({
                    "status": "ok",
                    "to": fmt.as_str(),
                    "output": params.output,
                })
                .to_string(),
            )]),
            Err(e) => CallToolResult::error(vec![ContentBlock::text(e.to_string())]),
        }
    }

    #[tool(description = "Render diagram as SVG (flowchart, sequence, class, or gantt)")]
    async fn render_svg(&self, Parameters(params): Parameters<RenderParams>) -> CallToolResult {
        let theme = match params.theme.as_deref() {
            Some("light") => renderer::Theme::Light,
            _ => renderer::Theme::Dark,
        };
        match crate::preview::render_file(&params.path, theme) {
            Ok(svg) => CallToolResult::success(vec![ContentBlock::text(svg)]),
            Err(e) => CallToolResult::error(vec![ContentBlock::text(e)]),
        }
    }

    #[tool(description = "Add a node to the diagram")]
    async fn add_node(&self, Parameters(params): Parameters<NodeParams>) -> CallToolResult {
        let shape = match &params.shape {
            Some(s) => match dg::NodeShape::parse(s) {
                Some(s) => s,
                None => {
                    return CallToolResult::error(vec![ContentBlock::text(format!(
                        "Invalid shape '{}'. Use: rect, diamond, stadium, hexagon, cylinder, or circle",
                        s
                    ))])
                }
            },
            None => dg::NodeShape::Rect,
        };
        modify_file(&params.path, |diagram| {
            diagram
                .add_node(dg::Node {
                    id: params.id.clone(),
                    text: params.text.clone(),
                    shape,
                    href: params.href.clone(),
                    tooltip: params.tooltip.clone(),
                })
                .map_err(|e| CallToolResult::error(vec![ContentBlock::text(e)]))?;
            Ok(serde_json::json!({"status": "ok", "id": params.id}).to_string())
        })
    }

    #[tool(description = "Add multiple nodes to the diagram at once")]
    async fn add_nodes(&self, Parameters(params): Parameters<BatchNodeParams>) -> CallToolResult {
        let mut parsed_shapes: Vec<(String, String, dg::NodeShape)> = Vec::new();
        for item in &params.items {
            let shape = match &item.shape {
                Some(s) => match dg::NodeShape::parse(s) {
                    Some(s) => s,
                    None => {
                        return CallToolResult::error(vec![ContentBlock::text(format!(
                            "Invalid shape '{}'. Use: rect, diamond, stadium, hexagon, cylinder, or circle",
                            s
                        ))])
                    }
                },
                None => dg::NodeShape::Rect,
            };
            parsed_shapes.push((item.id.clone(), item.text.clone(), shape));
        }
        modify_file(&params.path, |diagram| {
            for (id, text, shape) in &parsed_shapes {
                diagram
                    .add_node(dg::Node {
                        id: id.clone(),
                        text: text.clone(),
                        shape: *shape,
                        href: None,
                        tooltip: None,
                    })
                    .map_err(|e| CallToolResult::error(vec![ContentBlock::text(e)]))?;
            }
            let ids: Vec<String> = parsed_shapes.iter().map(|(id, _, _)| id.clone()).collect();
            Ok(serde_json::json!({"status": "ok", "added": ids}).to_string())
        })
    }

    #[tool(description = "Add multiple edges to the diagram at once")]
    async fn add_edges(&self, Parameters(params): Parameters<BatchEdgeParams>) -> CallToolResult {
        let mut parsed_edges: Vec<(String, String, String, dg::EdgeStyle)> = Vec::new();
        for item in &params.items {
            let style = match &item.style {
                Some(s) => match s.to_lowercase().as_str() {
                    "arrow" => dg::EdgeStyle::Arrow,
                    "dashed" => dg::EdgeStyle::Dashed,
                    "thick" => dg::EdgeStyle::Thick,
                    _ => {
                        return CallToolResult::error(vec![ContentBlock::text(format!(
                            "Invalid edge style '{}'. Use: arrow, dashed, or thick",
                            s
                        ))])
                    }
                },
                None => dg::EdgeStyle::Arrow,
            };
            parsed_edges.push((
                item.from.clone(),
                item.to.clone(),
                item.label.clone().unwrap_or_default(),
                style,
            ));
        }
        modify_file(&params.path, |diagram| {
            for (from, to, label, style) in &parsed_edges {
                diagram
                    .add_edge(dg::Edge {
                        from: from.clone(),
                        to: to.clone(),
                        label: label.clone(),
                        style: *style,
                    })
                    .map_err(|e| CallToolResult::error(vec![ContentBlock::text(e)]))?;
            }
            let edges: Vec<String> = parsed_edges
                .iter()
                .map(|(f, t, _, _)| format!("{} -> {}", f, t))
                .collect();
            Ok(serde_json::json!({"status": "ok", "added": edges}).to_string())
        })
    }

    #[tool(description = "Remove a node and its connected edges from the diagram")]
    async fn remove_node(
        &self,
        Parameters(params): Parameters<NodeUpdateParams>,
    ) -> CallToolResult {
        modify_file(&params.path, |diagram| {
            diagram.remove_node(&params.id);
            Ok(serde_json::json!({"status": "ok", "removed": params.id}).to_string())
        })
    }

    #[tool(description = "Update a node text and/or shape")]
    async fn update_node(
        &self,
        Parameters(params): Parameters<NodeUpdateParams>,
    ) -> CallToolResult {
        let shape = match &params.shape {
            Some(s) => match dg::NodeShape::parse(s) {
                Some(s) => Some(s),
                None => {
                    return CallToolResult::error(vec![ContentBlock::text(format!(
                        "Invalid shape '{}'. Use: rect, diamond, stadium, hexagon, cylinder, or circle",
                        s
                    ))])
                }
            },
            None => None,
        };
        modify_file(&params.path, |diagram| {
            diagram
                .update_node(&params.id, params.text.as_deref(), shape)
                .map_err(|e| CallToolResult::error(vec![ContentBlock::text(e)]))?;
            if let Some(node) = diagram.nodes.iter_mut().find(|n| n.id == params.id) {
                if let Some(h) = &params.href {
                    node.href = Some(h.clone());
                }
                if let Some(t) = &params.tooltip {
                    node.tooltip = Some(t.clone());
                }
            }
            Ok(serde_json::json!({"status": "ok", "updated": params.id}).to_string())
        })
    }

    #[tool(description = "Add an edge between two nodes")]
    async fn add_edge(&self, Parameters(params): Parameters<EdgeParams>) -> CallToolResult {
        let style = match &params.style {
            Some(s) => match s.to_lowercase().as_str() {
                "arrow" => dg::EdgeStyle::Arrow,
                "dashed" => dg::EdgeStyle::Dashed,
                "thick" => dg::EdgeStyle::Thick,
                _ => {
                    return CallToolResult::error(vec![ContentBlock::text(format!(
                        "Invalid edge style '{}'. Use: arrow, dashed, or thick",
                        s
                    ))])
                }
            },
            None => dg::EdgeStyle::Arrow,
        };
        modify_file(&params.path, |diagram| {
            diagram
                .add_edge(dg::Edge {
                    from: params.from.clone(),
                    to: params.to.clone(),
                    label: params.label.clone().unwrap_or_default(),
                    style,
                })
                .map_err(|e| CallToolResult::error(vec![ContentBlock::text(e)]))?;
            Ok(
                serde_json::json!({"status": "ok", "edge": format!("{} -> {}", params.from, params.to)})
                    .to_string(),
            )
        })
    }

    #[tool(description = "Remove an edge between two nodes")]
    async fn remove_edge(
        &self,
        Parameters(params): Parameters<EdgeRemoveParams>,
    ) -> CallToolResult {
        modify_file(&params.path, |diagram| {
            diagram
                .remove_edge(&params.from, &params.to)
                .map_err(|e| CallToolResult::error(vec![ContentBlock::text(e)]))?;
            Ok(
                serde_json::json!({"status": "ok", "removed": format!("{} -> {}", params.from, params.to)})
                    .to_string(),
            )
        })
    }

    #[tool(description = "Update an edge label and/or style")]
    async fn update_edge(
        &self,
        Parameters(params): Parameters<EdgeUpdateParams>,
    ) -> CallToolResult {
        let style = match &params.style {
            Some(s) => match s.to_lowercase().as_str() {
                "arrow" => Some(dg::EdgeStyle::Arrow),
                "dashed" => Some(dg::EdgeStyle::Dashed),
                "thick" => Some(dg::EdgeStyle::Thick),
                _ => {
                    return CallToolResult::error(vec![ContentBlock::text(format!(
                        "Invalid edge style '{}'. Use: arrow, dashed, or thick",
                        s
                    ))])
                }
            },
            None => None,
        };
        modify_file(&params.path, |diagram| {
            diagram
                .update_edge(&params.from, &params.to, params.label.as_deref(), style)
                .map_err(|e| CallToolResult::error(vec![ContentBlock::text(e)]))?;
            Ok(
                serde_json::json!({"status": "ok", "updated": format!("{} -> {}", params.from, params.to)})
                    .to_string(),
            )
        })
    }

    #[tool(description = "Get a single node by ID")]
    async fn get_node(&self, Parameters(params): Parameters<NodeIdParams>) -> CallToolResult {
        let diagram = match read_file(&params.path) {
            Ok(d) => d,
            Err(e) => return e,
        };
        match diagram.get_node(&params.id) {
            Some(n) => {
                let json = serde_json::json!({
                    "id": n.id,
                    "text": n.text,
                    "shape": n.shape.to_string(),
                });
                CallToolResult::success(vec![ContentBlock::text(json.to_string())])
            }
            None => CallToolResult::error(vec![ContentBlock::text(format!(
                "Node '{}' not found",
                params.id
            ))]),
        }
    }

    #[tool(description = "Get a single edge by from/to IDs")]
    async fn get_edge(
        &self,
        Parameters(params): Parameters<EdgeRemoveParams>,
    ) -> CallToolResult {
        let diagram = match read_file(&params.path) {
            Ok(d) => d,
            Err(e) => return e,
        };
        match diagram.edges.iter().find(|e| e.from == params.from && e.to == params.to) {
            Some(e) => {
                let json = serde_json::json!({
                    "from": e.from,
                    "to": e.to,
                    "label": e.label,
                    "style": match e.style {
                        dg::EdgeStyle::Arrow => "arrow",
                        dg::EdgeStyle::Dashed => "dashed",
                        dg::EdgeStyle::Thick => "thick",
                    },
                });
                CallToolResult::success(vec![ContentBlock::text(json.to_string())])
            }
            None => CallToolResult::error(vec![ContentBlock::text(format!(
                "Edge '{} -> {}' not found",
                params.from, params.to
            ))]),
        }
    }

    #[tool(description = "Get the raw mermaid source code from a file")]
    async fn get_mermaid(&self, Parameters(params): Parameters<FilePath>) -> CallToolResult {
        let diagram = match read_file(&params.path) {
            Ok(d) => d,
            Err(e) => return e,
        };
        CallToolResult::success(vec![ContentBlock::text(diagram.to_mermaid())])
    }

    #[tool(description = "Write raw mermaid source code directly to a file")]
    async fn set_mermaid(
        &self,
        Parameters(params): Parameters<MermaidSourceParams>,
    ) -> CallToolResult {
        match std::fs::write(&params.path, &params.source) {
            Ok(_) => CallToolResult::success(vec![ContentBlock::text(
                serde_json::json!({"status": "ok", "path": params.path}).to_string(),
            )]),
            Err(e) => CallToolResult::error(vec![ContentBlock::text(format!(
                "Failed to write file '{}': {}",
                params.path, e
            ))]),
        }
    }

    #[tool(description = "List all nodes in the diagram")]
    async fn list_nodes(&self, Parameters(params): Parameters<FilePath>) -> CallToolResult {
        let diagram = match read_file(&params.path) {
            Ok(d) => d,
            Err(e) => return e,
        };
        let nodes: Vec<serde_json::Value> = diagram
            .nodes
            .iter()
            .map(|n| {
                serde_json::json!({
                    "id": n.id,
                    "text": n.text,
                    "shape": n.shape.to_string(),
                })
            })
            .collect();
        CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&nodes).unwrap_or_default(),
        )])
    }

    #[tool(description = "Structural metrics JSON (depth, orphans, cycles, counts)")]
    async fn metrics_diagram(&self, Parameters(params): Parameters<FilePath>) -> CallToolResult {
        match crate::ir::load_path(&params.path) {
            Ok(doc) => {
                let m = crate::analyze::metrics(&doc);
                match serde_json::to_string_pretty(&m) {
                    Ok(json) => CallToolResult::success(vec![ContentBlock::text(json)]),
                    Err(e) => CallToolResult::error(vec![ContentBlock::text(e.to_string())]),
                }
            }
            Err(e) => CallToolResult::error(vec![ContentBlock::text(e.to_string())]),
        }
    }

    #[tool(description = "Create a new diagram scaffold file")]
    async fn create_diagram(&self, Parameters(params): Parameters<CreateParams>) -> CallToolResult {
        match crate::generate::write_scaffold(&params.kind, &params.output) {
            Ok(kind) => {
                let json = serde_json::json!({
                    "ok": true,
                    "kind": kind.to_string(),
                    "output": params.output,
                });
                CallToolResult::success(vec![ContentBlock::text(json.to_string())])
            }
            Err(e) => CallToolResult::error(vec![ContentBlock::text(e)]),
        }
    }

    #[tool(description = "Validate diagram for orphaned nodes, dangling edges, and cycles")]
    async fn validate_diagram(&self, Parameters(params): Parameters<FilePath>) -> CallToolResult {
        let diagram = match read_file(&params.path) {
            Ok(d) => d,
            Err(e) => return e,
        };
        let issues = diagram.validate();
        let json = serde_json::json!({
            "valid": issues.is_empty(),
            "issues": issues,
        });
        CallToolResult::success(vec![ContentBlock::text(json.to_string())])
    }

    #[tool(description = "List all edges in the diagram")]
    async fn list_edges(&self, Parameters(params): Parameters<FilePath>) -> CallToolResult {
        let diagram = match read_file(&params.path) {
            Ok(d) => d,
            Err(e) => return e,
        };
        let edges: Vec<serde_json::Value> = diagram
            .edges
            .iter()
            .map(|e| {
                serde_json::json!({
                    "from": e.from,
                    "to": e.to,
                    "label": e.label,
                    "style": match e.style {
                        dg::EdgeStyle::Arrow => "arrow",
                        dg::EdgeStyle::Dashed => "dashed",
                        dg::EdgeStyle::Thick => "thick",
                    },
                })
            })
            .collect();
        CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&edges).unwrap_or_default(),
        )])
    }

    #[tool(description = "Compare two diagrams and show differences")]
    async fn diff_diagram(&self, Parameters(params): Parameters<DiffParams>) -> CallToolResult {
        let left = match read_file(&params.left) {
            Ok(d) => d,
            Err(e) => return e,
        };
        let right = match read_file(&params.right) {
            Ok(d) => d,
            Err(e) => return e,
        };
        let diff = left.diff(&right);
        CallToolResult::success(vec![ContentBlock::text(
            serde_json::to_string_pretty(&diff).unwrap_or_default(),
        )])
    }

    #[tool(description = "Merge two diagrams into one and write to a file")]
    async fn merge_diagram(&self, Parameters(params): Parameters<MergeParams>) -> CallToolResult {
        let left = match read_file(&params.left) {
            Ok(d) => d,
            Err(e) => return e,
        };
        let right = match read_file(&params.right) {
            Ok(d) => d,
            Err(e) => return e,
        };
        let merged = left.merge(&right);
        match std::fs::write(&params.output, merged.to_mermaid()) {
            Ok(_) => CallToolResult::success(vec![ContentBlock::text(format!(
                "Merged diagram written to {}", params.output
            ))]),
            Err(e) => CallToolResult::error(vec![ContentBlock::text(format!(
                "Failed to write output file: {}", e
            ))]),
        }
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct DiffParams {
    #[schemars(description = "Path to the left/base mermaid .mmd file")]
    left: String,
    #[schemars(description = "Path to the right/modified mermaid .mmd file")]
    right: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct MergeParams {
    #[schemars(description = "Path to the left/base mermaid .mmd file")]
    left: String,
    #[schemars(description = "Path to the right/modified mermaid .mmd file")]
    right: String,
    #[schemars(description = "Output file path for the merged diagram")]
    output: String,
}

#[tool_handler]
impl ServerHandler for DiagramServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
        )
        .with_instructions(
            "Diagram platform MCP server: render, generate, analyze, and interchange diagrams. Mermaid-compatible today; IR-centric tools. Parse, validate, diff/merge, mutate flowcharts, and render SVG.",
        )
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, ErrorData>> + MaybeSendFuture + '_ {
        std::future::ready(Ok(ListResourcesResult::with_all_items(vec![])))
    }

    fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourceTemplatesResult, ErrorData>> + MaybeSendFuture + '_ {
        std::future::ready(Ok(ListResourceTemplatesResult::with_all_items(vec![
            ResourceTemplate::new("file://{path}", "mermaid-diagram")
                .with_title("Diagram source file")
                .with_description("Read a diagram source file (Mermaid .mmd today) as text")
                .with_mime_type("text/plain"),
        ])))
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, ErrorData>> + MaybeSendFuture + '_ {
        let contents = if request.uri.starts_with("file://") {
            let path = &request.uri[7..];
            match std::fs::read_to_string(path) {
                Ok(text) => vec![ResourceContents::text(text, &request.uri)],
                Err(e) => return std::future::ready(Err(ErrorData::invalid_params(
                    format!("Failed to read file '{}': {}", path, e),
                    None,
                ))),
            }
        } else {
            return std::future::ready(Err(ErrorData::invalid_params(
                format!("Unsupported URI scheme: {}", request.uri),
                None,
            )));
        };
        std::future::ready(Ok(ReadResourceResult::new(contents)))
    }

    fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListPromptsResult, ErrorData>> + MaybeSendFuture + '_ {
        std::future::ready(Ok(ListPromptsResult::with_all_items(vec![
            Prompt::new(
                "create_flowchart",
                Some("Create a new flowchart diagram from a description"),
                None,
            ).with_title("Create Flowchart"),
            Prompt::new(
                "refactor_diagram",
                Some("Refactor an existing diagram to improve clarity"),
                None,
            ).with_title("Refactor Diagram"),
        ])))
    }

    fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<GetPromptResult, ErrorData>> + MaybeSendFuture + '_ {
        let messages = match request.name.as_str() {
            "create_flowchart" => vec![
                PromptMessage::new_text(Role::User, "Create a flowchart diagram. Describe the process, decision points, and outcomes. Use standard mermaid flowchart syntax with appropriate node shapes."),
            ],
            "refactor_diagram" => vec![
                PromptMessage::new_text(Role::User, "Refactor the given diagram to improve clarity, reduce complexity, and ensure consistent naming. Consider merging duplicate nodes, simplifying edge crossings, and improving labels."),
            ],
            _ => {
                return std::future::ready(Err(ErrorData::invalid_params(
                    format!("Prompt '{}' not found", request.name),
                    None,
                )));
            }
        };
        std::future::ready(Ok(GetPromptResult::new(messages)))
    }
}
