use rmcp::{
    ServerHandler,
    handler::server::wrapper::Parameters,
    model::*,
    schemars, tool, tool_handler, tool_router,
};
use rmcp::service::{MaybeSendFuture, RequestContext, RoleServer};

use crate::{diagram as dg, layout, parser, renderer};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct FilePath {
    #[schemars(description = "Path to the mermaid .mmd file")]
    path: String,
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
        .map_err(|e| CallToolResult::error(vec![Content::text(format!(
            "Failed to read file '{}': {}",
            path, e
        ))]))?;
    parser::parse(&content).map_err(|e| {
        CallToolResult::error(vec![Content::text(e.to_string())])
    })
}

fn write_file(path: &str, diagram: &dg::Diagram) -> Result<(), CallToolResult> {
    let mermaid = diagram.to_mermaid();
    std::fs::write(path, &mermaid).map_err(|e| {
        CallToolResult::error(vec![Content::text(format!(
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
    CallToolResult::success(vec![Content::text(result)])
}

#[derive(Debug, Clone)]
pub struct DiagramServer;

#[tool_router]
impl DiagramServer {
    #[tool(description = "Parse a mermaid diagram file and return its JSON representation")]
    async fn parse_diagram(&self, Parameters(params): Parameters<FilePath>) -> CallToolResult {
        match read_file(&params.path) {
            Ok(diagram) => {
                let json = serde_json::to_string_pretty(&diagram).unwrap_or_default();
                CallToolResult::success(vec![Content::text(json)])
            }
            Err(e) => e,
        }
    }

    #[tool(description = "Get diagram summary (node count, edge count, type)")]
    async fn get_info(&self, Parameters(params): Parameters<FilePath>) -> CallToolResult {
        let diagram = match read_file(&params.path) {
            Ok(d) => d,
            Err(e) => return e,
        };
        let mut shapes = vec![0usize; 6];
        for n in &diagram.nodes {
            shapes[match n.shape {
                dg::NodeShape::Rect => 0,
                dg::NodeShape::Diamond => 1,
                dg::NodeShape::Stadium => 2,
                dg::NodeShape::Hexagon => 3,
                dg::NodeShape::Cylinder => 4,
                dg::NodeShape::Circle => 5,
            }] += 1;
        }
        let summary = serde_json::json!({
            "path": params.path,
            "direction": diagram.rankdir,
            "nodes": diagram.nodes.len(),
            "edges": diagram.edges.len(),
            "shapes": {
                "rect": shapes[0],
                "diamond": shapes[1],
                "stadium": shapes[2],
                "hexagon": shapes[3],
                "cylinder": shapes[4],
                "circle": shapes[5],
            },
        });
        CallToolResult::success(vec![Content::text(summary.to_string())])
    }

    #[tool(description = "Render diagram as SVG")]
    async fn render_svg(&self, Parameters(params): Parameters<FilePath>) -> CallToolResult {
        let diagram = match read_file(&params.path) {
            Ok(d) => d,
            Err(e) => return e,
        };
        let laid = layout::layout(&diagram);
        let svg = renderer::render_svg(&laid);
        CallToolResult::success(vec![Content::text(svg)])
    }

    #[tool(description = "Add a node to the diagram")]
    async fn add_node(&self, Parameters(params): Parameters<NodeParams>) -> CallToolResult {
        let shape = match &params.shape {
            Some(s) => match dg::NodeShape::from_str(s) {
                Some(s) => s,
                None => {
                    return CallToolResult::error(vec![Content::text(format!(
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
                })
                .map_err(|e| CallToolResult::error(vec![Content::text(e)]))?;
            Ok(serde_json::json!({"status": "ok", "id": params.id}).to_string())
        })
    }

    #[tool(description = "Add multiple nodes to the diagram at once")]
    async fn add_nodes(&self, Parameters(params): Parameters<BatchNodeParams>) -> CallToolResult {
        let mut parsed_shapes: Vec<(String, String, dg::NodeShape)> = Vec::new();
        for item in &params.items {
            let shape = match &item.shape {
                Some(s) => match dg::NodeShape::from_str(s) {
                    Some(s) => s,
                    None => {
                        return CallToolResult::error(vec![Content::text(format!(
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
                    })
                    .map_err(|e| CallToolResult::error(vec![Content::text(e)]))?;
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
                        return CallToolResult::error(vec![Content::text(format!(
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
                    .map_err(|e| CallToolResult::error(vec![Content::text(e)]))?;
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
            Some(s) => match dg::NodeShape::from_str(s) {
                Some(s) => Some(s),
                None => {
                    return CallToolResult::error(vec![Content::text(format!(
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
                .map_err(|e| CallToolResult::error(vec![Content::text(e)]))?;
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
                    return CallToolResult::error(vec![Content::text(format!(
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
                .map_err(|e| CallToolResult::error(vec![Content::text(e)]))?;
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
                .map_err(|e| CallToolResult::error(vec![Content::text(e)]))?;
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
                    return CallToolResult::error(vec![Content::text(format!(
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
                .map_err(|e| CallToolResult::error(vec![Content::text(e)]))?;
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
                CallToolResult::success(vec![Content::text(json.to_string())])
            }
            None => CallToolResult::error(vec![Content::text(format!(
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
                CallToolResult::success(vec![Content::text(json.to_string())])
            }
            None => CallToolResult::error(vec![Content::text(format!(
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
        CallToolResult::success(vec![Content::text(diagram.to_mermaid())])
    }

    #[tool(description = "Write raw mermaid source code directly to a file")]
    async fn set_mermaid(
        &self,
        Parameters(params): Parameters<MermaidSourceParams>,
    ) -> CallToolResult {
        match std::fs::write(&params.path, &params.source) {
            Ok(_) => CallToolResult::success(vec![Content::text(
                serde_json::json!({"status": "ok", "path": params.path}).to_string(),
            )]),
            Err(e) => CallToolResult::error(vec![Content::text(format!(
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
        CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&nodes).unwrap_or_default(),
        )])
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
        CallToolResult::success(vec![Content::text(json.to_string())])
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
        CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&edges).unwrap_or_default(),
        )])
    }
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
            "Diagram manipulation MCP server. Read, parse, modify, and render mermaid diagrams.",
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
            ResourceTemplate::new(
                RawResourceTemplate {
                    uri_template: "file://{path}".to_string(),
                    name: "mermaid-diagram".to_string(),
                    title: Some("Mermaid diagram file".to_string()),
                    description: Some("Read any .mmd file as a text resource".to_string()),
                    mime_type: Some("text/plain".to_string()),
                    icons: None,
                },
                None,
            )
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
                PromptMessage::new_text(PromptMessageRole::User, "Create a flowchart diagram. Describe the process, decision points, and outcomes. Use standard mermaid flowchart syntax with appropriate node shapes."),
            ],
            "refactor_diagram" => vec![
                PromptMessage::new_text(PromptMessageRole::User, "Refactor the given diagram to improve clarity, reduce complexity, and ensure consistent naming. Consider merging duplicate nodes, simplifying edge crossings, and improving labels."),
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
