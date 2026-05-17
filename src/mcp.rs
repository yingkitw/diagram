use rmcp::{
    ServerHandler,
    handler::server::wrapper::Parameters,
    model::*,
    schemars, tool, tool_handler, tool_router,
};

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
                .build(),
        )
        .with_instructions(
            "Diagram manipulation MCP server. Read, parse, modify, and render mermaid diagrams.",
        )
    }
}
