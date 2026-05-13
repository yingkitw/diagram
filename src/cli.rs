use clap::Parser;
use crate::{diagram as dg, layout, parser, renderer};

#[derive(Parser)]
pub enum Cli {
    #[command(about = "Parse a diagram file and print as JSON")]
    Parse {
        path: String,
    },

    #[command(about = "Show diagram summary (node count, edge count, direction)")]
    Info {
        path: String,
    },

    #[command(about = "Render diagram as SVG")]
    Render {
        path: String,
        #[arg(long, help = "Output SVG file path (prints to stdout if not set)")]
        output: Option<String>,
    },

    #[command(about = "Start MCP server (stdio transport)")]
    Mcp,

    #[command(about = "Add a node to the diagram")]
    AddNode {
        path: String,
        id: String,
        text: String,
        #[arg(long, help = "Node shape: rect, diamond, or stadium")]
        shape: Option<String>,
    },

    #[command(about = "Remove a node and its connected edges")]
    RemoveNode {
        path: String,
        id: String,
    },

    #[command(about = "Update a node's text and/or shape")]
    UpdateNode {
        path: String,
        id: String,
        #[arg(long, help = "New display text")]
        text: Option<String>,
        #[arg(long, help = "New shape: rect, diamond, or stadium")]
        shape: Option<String>,
    },

    #[command(about = "Add an edge between two nodes")]
    AddEdge {
        path: String,
        from: String,
        to: String,
        #[arg(long, help = "Edge label")]
        label: Option<String>,
    },

    #[command(about = "Remove an edge between two nodes")]
    RemoveEdge {
        path: String,
        from: String,
        to: String,
    },
}

impl Cli {
    pub async fn run(&self) -> anyhow::Result<()> {
        match self {
            Self::Parse { path } => cmd_parse(path),
            Self::Info { path } => cmd_info(path),
            Self::Render { path, output } => cmd_render(path, output.as_deref()),
            Self::Mcp => cmd_mcp().await,
            Self::AddNode { path, id, text, shape } => cmd_add_node(path, id, text, shape.as_deref()),
            Self::RemoveNode { path, id } => cmd_remove_node(path, id),
            Self::UpdateNode { path, id, text, shape } => cmd_update_node(path, id, text.as_deref(), shape.as_deref()),
            Self::AddEdge { path, from, to, label } => cmd_add_edge(path, from, to, label.as_deref()),
            Self::RemoveEdge { path, from, to } => cmd_remove_edge(path, from, to),
        }
    }
}

fn read_diagram(path: &str) -> anyhow::Result<dg::Diagram> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", path, e))?;
    parser::parse(&content)
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))
}

fn write_diagram(path: &str, diagram: &dg::Diagram) -> anyhow::Result<()> {
    let mermaid = diagram.to_mermaid();
    std::fs::write(path, &mermaid)
        .map_err(|e| anyhow::anyhow!("Failed to write '{}': {}", path, e))?;
    Ok(())
}

fn cmd_parse(path: &str) -> anyhow::Result<()> {
    let diagram = read_diagram(path)?;
    println!("{}", serde_json::to_string_pretty(&diagram)?);
    Ok(())
}

fn cmd_info(path: &str) -> anyhow::Result<()> {
    let diagram = read_diagram(path)?;
    let mut shapes = vec![0usize; 3];
    for n in &diagram.nodes {
        shapes[match n.shape {
            dg::NodeShape::Rect => 0,
            dg::NodeShape::Diamond => 1,
            dg::NodeShape::Stadium => 2,
        }] += 1;
    }
    println!("File: {path}");
    println!("Direction: {}", diagram.rankdir);
    println!("Nodes: {}", diagram.nodes.len());
    println!("  rect:    {}", shapes[0]);
    println!("  diamond: {}", shapes[1]);
    println!("  stadium: {}", shapes[2]);
    println!("Edges: {}", diagram.edges.len());
    Ok(())
}

fn cmd_render(path: &str, output: Option<&str>) -> anyhow::Result<()> {
    let diagram = read_diagram(path)?;
    let laid = layout::layout(&diagram);
    let svg = renderer::render_svg(&laid);
    match output {
        Some(out_path) => std::fs::write(out_path, &svg)?,
        None => println!("{svg}"),
    }
    Ok(())
}

async fn cmd_mcp() -> anyhow::Result<()> {
    use rmcp::{ServiceExt, transport::stdio};

    let service = crate::mcp::DiagramServer
        .serve(stdio())
        .await?;
    eprintln!("Diagram MCP server started (stdio)");
    service.waiting().await?;
    Ok(())
}

fn cmd_add_node(path: &str, id: &str, text: &str, shape: Option<&str>) -> anyhow::Result<()> {
    let mut diagram = read_diagram(path)?;
    let shape = match shape {
        Some(s) => dg::NodeShape::from_str(s)
            .ok_or_else(|| anyhow::anyhow!("Invalid shape '{s}'. Use: rect, diamond, stadium"))?,
        None => dg::NodeShape::Rect,
    };
    diagram
        .add_node(dg::Node {
            id: id.to_string(),
            text: text.to_string(),
            shape,
        })
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    write_diagram(path, &diagram)?;
    println!("Added node '{id}'");
    Ok(())
}

fn cmd_remove_node(path: &str, id: &str) -> anyhow::Result<()> {
    let mut diagram = read_diagram(path)?;
    diagram.remove_node(id);
    write_diagram(path, &diagram)?;
    println!("Removed node '{id}' and its edges");
    Ok(())
}

fn cmd_update_node(
    path: &str,
    id: &str,
    text: Option<&str>,
    shape: Option<&str>,
) -> anyhow::Result<()> {
    let mut diagram = read_diagram(path)?;
    let shape = match shape {
        Some(s) => Some(
            dg::NodeShape::from_str(s)
                .ok_or_else(|| anyhow::anyhow!("Invalid shape '{s}'. Use: rect, diamond, stadium"))?,
        ),
        None => None,
    };
    diagram
        .update_node(id, text, shape)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    write_diagram(path, &diagram)?;
    println!("Updated node '{id}'");
    Ok(())
}

fn cmd_add_edge(
    path: &str,
    from: &str,
    to: &str,
    label: Option<&str>,
) -> anyhow::Result<()> {
    let mut diagram = read_diagram(path)?;
    diagram
        .add_edge(dg::Edge {
            from: from.to_string(),
            to: to.to_string(),
            label: label.unwrap_or("").to_string(),
        })
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    write_diagram(path, &diagram)?;
    println!("Added edge '{from} -> {to}'");
    Ok(())
}

fn cmd_remove_edge(path: &str, from: &str, to: &str) -> anyhow::Result<()> {
    let mut diagram = read_diagram(path)?;
    diagram
        .remove_edge(from, to)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    write_diagram(path, &diagram)?;
    println!("Removed edge '{from} -> {to}'");
    Ok(())
}
