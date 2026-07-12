use clap::Parser;
use crate::{diagram as dg, parser};
use crate::renderer::Theme;

#[derive(Parser)]
pub enum Cli {
    #[command(about = "Parse a diagram file and print canonical JSON IR")]
    Parse {
        path: String,
    },

    #[command(about = "Print canonical JSON IR (alias for parse)")]
    Ir {
        path: String,
    },

    #[command(about = "Import a diagram into canonical JSON IR")]
    Import {
        path: String,
        #[arg(long, short, help = "Output JSON IR file")]
        output: String,
        #[arg(long, help = "Source format: mermaid, json, dot, d2, or plantuml (auto-detect if omitted)")]
        from: Option<String>,
    },

    #[command(about = "Export a diagram to Mermaid or JSON IR")]
    Export {
        path: String,
        #[arg(long, short, help = "Output file path")]
        output: String,
        #[arg(long, help = "Target format: mermaid, json, dot, d2, or plantuml (auto-detect if omitted)")]
        to: Option<String>,
        #[arg(long, help = "Print lossiness summary after export")]
        report: bool,
    },

    #[command(about = "Report export lossiness (what IR fields a format cannot represent)")]
    Lossiness {
        path: String,
        #[arg(long, help = "Target format: mermaid, json, dot, d2, or plantuml (default: mermaid)")]
        to: Option<String>,
    },

    #[command(about = "Show diagram summary (node count, edge count, direction)")]
    Info {
        path: String,
    },

    #[command(about = "Render diagram as SVG or PNG (format from --output extension)")]
    Render {
        path: String,
        #[arg(long, help = "Output file path: .svg, .png, or .pdf (prints SVG to stdout if not set)")]
        output: Option<String>,
        #[arg(long, help = "Render each diagram to separate files in this directory")]
        output_dir: Option<String>,
        #[arg(long, help = "Render only diagram index (0-based) from multi-diagram IR")]
        index: Option<usize>,
        #[arg(long, help = "Watch file for changes and re-render automatically")]
        watch: bool,
        #[arg(long, help = "Theme: dark or light")]
        theme: Option<String>,
    },

    #[command(about = "Start MCP server (stdio transport)")]
    Mcp,

    #[command(about = "Add a node to the diagram")]
    AddNode {
        path: String,
        id: String,
        text: String,
        #[arg(long, help = "Node shape: rect, diamond, stadium, hexagon, cylinder, circle")]
        shape: Option<String>,
        #[arg(long, help = "Hyperlink URL for the node")]
        href: Option<String>,
        #[arg(long, help = "Tooltip text for the node")]
        tooltip: Option<String>,
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
        #[arg(long, help = "New shape: rect, diamond, stadium, hexagon, cylinder, circle")]
        shape: Option<String>,
        #[arg(long, help = "New hyperlink URL for the node")]
        href: Option<String>,
        #[arg(long, help = "New tooltip text for the node")]
        tooltip: Option<String>,
    },

    #[command(about = "Add an edge between two nodes")]
    AddEdge {
        path: String,
        from: String,
        to: String,
        #[arg(long, help = "Edge label")]
        label: Option<String>,
        #[arg(long, help = "Edge style: arrow, dashed, thick")]
        style: Option<String>,
    },

    #[command(about = "Remove an edge between two nodes")]
    RemoveEdge {
        path: String,
        from: String,
        to: String,
    },

    #[command(about = "Update an edge label and/or style")]
    UpdateEdge {
        path: String,
        from: String,
        to: String,
        #[arg(long, help = "New edge label")]
        label: Option<String>,
        #[arg(long, help = "New edge style: arrow, dashed, thick")]
        style: Option<String>,
    },

    #[command(about = "Get a single node by ID")]
    GetNode {
        path: String,
        id: String,
    },

    #[command(about = "Get a single edge by from/to IDs")]
    GetEdge {
        path: String,
        from: String,
        to: String,
    },

    #[command(about = "Get the raw mermaid source code from a diagram file")]
    GetMermaid {
        path: String,
    },

    #[command(about = "Write raw mermaid source code directly to a file")]
    SetMermaid {
        path: String,
        #[arg(help = "Raw mermaid source code to write")]
        source: String,
    },

    #[command(about = "List all nodes in the diagram")]
    ListNodes {
        path: String,
    },

    #[command(about = "List all edges in the diagram")]
    ListEdges {
        path: String,
    },

    #[command(about = "Validate diagram (orphaned nodes, dangling edges, cycles)")]
    Validate {
        path: String,
    },

    #[command(about = "Structural metrics as JSON (depth, orphans, cycles, counts)")]
    Metrics {
        path: String,
    },

    #[command(about = "Create a new diagram scaffold (flowchart, sequence, class, gantt, state)")]
    Create {
        #[arg(long, help = "Diagram kind: flowchart, sequence, class, gantt, state")]
        kind: String,
        #[arg(long, short, help = "Output file path (.mmd or .json)")]
        output: String,
    },

    #[command(about = "Show differences between two diagrams")]
    Diff {
        left: String,
        right: String,
    },

    #[command(about = "Merge two diagrams into one")]
    Merge {
        left: String,
        right: String,
        #[arg(long, help = "Output file path")]
        output: String,
    },

    #[command(about = "Serve a live SVG preview in the browser")]
    Preview {
        path: String,
        #[arg(long, default_value = "3030", help = "Port to listen on")]
        port: u16,
        #[arg(long, help = "Theme: dark or light")]
        theme: Option<String>,
    },

    #[command(about = "Render fenced diagram blocks in Markdown and rewrite image links")]
    Markdown {
        path: String,
        #[arg(long, help = "Directory for rendered diagram images")]
        output_dir: String,
        #[arg(long, help = "Output Markdown file (stdout if omitted)")]
        output: Option<String>,
        #[arg(long, default_value = "png", help = "Image format: png or svg")]
        format: String,
        #[arg(long, help = "Theme: dark or light")]
        theme: Option<String>,
    },
}

impl Cli {
    pub async fn run(&self) -> anyhow::Result<()> {
        match self {
            Self::Parse { path } => cmd_ir(path),
            Self::Ir { path } => cmd_ir(path),
            Self::Import { path, output, from } => cmd_import(path, output, from.as_deref()),
            Self::Export { path, output, to, report } => cmd_export(path, output, to.as_deref(), *report),
            Self::Lossiness { path, to } => cmd_lossiness(path, to.as_deref()),
            Self::Info { path } => cmd_info(path),
            Self::Render { path, output, output_dir, index, watch, theme } => {
                let theme = match theme.as_deref() {
                    Some("light") => Theme::Light,
                    _ => Theme::Dark,
                };
                if *watch {
                    cmd_render_watch(path, output.as_deref(), output_dir.as_deref(), *index, theme).await
                } else {
                    cmd_render(path, output.as_deref(), output_dir.as_deref(), *index, theme)
                }
            }
            Self::Mcp => cmd_mcp().await,
            Self::AddNode { path, id, text, shape, href, tooltip } => cmd_add_node(path, id, text, shape.as_deref(), href.as_deref(), tooltip.as_deref()),
            Self::RemoveNode { path, id } => cmd_remove_node(path, id),
            Self::UpdateNode { path, id, text, shape, href, tooltip } => cmd_update_node(path, id, text.as_deref(), shape.as_deref(), href.as_deref(), tooltip.as_deref()),
            Self::AddEdge { path, from, to, label, style } => cmd_add_edge(path, from, to, label.as_deref(), style.as_deref()),
            Self::RemoveEdge { path, from, to } => cmd_remove_edge(path, from, to),
            Self::UpdateEdge { path, from, to, label, style } => cmd_update_edge(path, from, to, label.as_deref(), style.as_deref()),
            Self::GetNode { path, id } => cmd_get_node(path, id),
            Self::GetEdge { path, from, to } => cmd_get_edge(path, from, to),
            Self::GetMermaid { path } => cmd_get_mermaid(path),
            Self::SetMermaid { path, source } => cmd_set_mermaid(path, source),
            Self::ListNodes { path } => cmd_list_nodes(path),
            Self::ListEdges { path } => cmd_list_edges(path),
            Self::Validate { path } => cmd_validate(path),
            Self::Metrics { path } => cmd_metrics(path),
            Self::Create { kind, output } => cmd_create(kind, output),
            Self::Diff { left, right } => cmd_diff(left, right),
            Self::Merge { left, right, output } => cmd_merge(left, right, output),
            Self::Preview { path, port, theme } => {
                let theme = match theme.as_deref() {
                    Some("light") => Theme::Light,
                    _ => Theme::Dark,
                };
                cmd_preview(path, *port, theme).await
            }
            Self::Markdown { path, output_dir, output, format, theme } => {
                let theme = match theme.as_deref() {
                    Some("light") => Theme::Light,
                    _ => Theme::Dark,
                };
                cmd_markdown(path, output_dir, output.as_deref(), format, theme)
            }
        }
    }
}

fn read_diagram(path: &str) -> anyhow::Result<dg::Diagram> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", path, e))?;
    parser::parse(&content)
        .map_err(|e| anyhow::anyhow!("{}", e))
}

fn write_diagram(path: &str, diagram: &dg::Diagram) -> anyhow::Result<()> {
    let mermaid = diagram.to_mermaid();
    std::fs::write(path, &mermaid)
        .map_err(|e| anyhow::anyhow!("Failed to write '{}': {}", path, e))?;
    Ok(())
}

fn cmd_ir(path: &str) -> anyhow::Result<()> {
    let doc = crate::ir::load_path(path).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("{}", doc.to_json()?);
    Ok(())
}

fn parse_format(s: &str) -> anyhow::Result<crate::formats::Format> {
    crate::formats::Format::parse(s)
        .ok_or_else(|| anyhow::anyhow!("Invalid format '{s}'. Use: mermaid, json, dot, d2, plantuml"))
}

fn cmd_import(path: &str, output: &str, from: Option<&str>) -> anyhow::Result<()> {
    let from_fmt = from.map(parse_format).transpose()?;
    let (doc, fmt) = crate::formats::import_path(path, from_fmt)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    crate::formats::export_path(&doc, output, Some(crate::formats::Format::JsonIr))
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Imported {fmt:?} from {path} → {output}");
    Ok(())
}

fn cmd_export(path: &str, output: &str, to: Option<&str>, report: bool) -> anyhow::Result<()> {
    let to_fmt = to.map(parse_format).transpose()?;
    let (doc, _) = crate::formats::import_path(path, None)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let (fmt, loss) = crate::formats::export_with_report(&doc, output, to_fmt)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Exported {fmt:?} to {output}");
    if report {
        println!("{}", serde_json::to_string_pretty(&loss)?);
    } else if let Some(line) = crate::lossiness::summary_line(&loss) {
        eprintln!("{line}");
    }
    Ok(())
}

fn cmd_lossiness(path: &str, to: Option<&str>) -> anyhow::Result<()> {
    let format = match to {
        Some(s) => parse_format(s)?,
        None => crate::formats::Format::Mermaid,
    };
    let doc = crate::ir::load_path(path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let loss = crate::lossiness::report(&doc, format);
    println!("{}", serde_json::to_string_pretty(&loss)?);
    Ok(())
}

fn cmd_info(path: &str) -> anyhow::Result<()> {
    let doc = crate::ir::load_path(path).map_err(|e| anyhow::anyhow!("{e}"))?;
    for line in crate::ir::info_lines(path, &doc) {
        println!("{line}");
    }
    Ok(())
}

fn cmd_render(
    path: &str,
    output: Option<&str>,
    output_dir: Option<&str>,
    index: Option<usize>,
    theme: Theme,
) -> anyhow::Result<()> {
    let doc = crate::ir::load_path(path).map_err(|e| anyhow::anyhow!("{e}"))?;

    if let Some(dir) = output_dir {
        let raster = output.and_then(crate::preview::raster_format_from_path);
        let paths = crate::preview::write_render_outputs_to_dir(
            path,
            std::path::Path::new(dir),
            theme,
            raster,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
        println!("Rendered {} diagram(s) → {}", paths.len(), dir);
        return Ok(());
    }

    let svg = if let Some(idx) = index {
        doc.render_diagram_at(idx, theme)
            .map_err(|e| anyhow::anyhow!("{e}"))?
    } else {
        doc.render_svg(theme).map_err(|e| anyhow::anyhow!("{e}"))?
    };

    match output {
        Some(out_path) => {
            if let Some(fmt) = crate::preview::raster_format_from_path(out_path) {
                let bytes = match fmt {
                    crate::preview::RasterFormat::Png => {
                        crate::png::svg_to_png(&svg).map_err(|e| anyhow::anyhow!("{e}"))?
                    }
                    crate::preview::RasterFormat::Pdf => {
                        crate::pdf::svg_to_pdf(&svg).map_err(|e| anyhow::anyhow!("{e}"))?
                    }
                };
                std::fs::write(out_path, bytes)?;
            } else {
                std::fs::write(out_path, &svg)?;
            }
        }
        None => println!("{svg}"),
    }
    Ok(())
}

async fn cmd_render_watch(
    path: &str,
    output: Option<&str>,
    output_dir: Option<&str>,
    index: Option<usize>,
    theme: Theme,
) -> anyhow::Result<()> {
    cmd_render(path, output, output_dir, index, theme)?;
    eprintln!("Watching {path} for changes... (press Ctrl+C to stop)");

    let path = path.to_string();
    let output = output.map(|s| s.to_string());
    let output_dir = output_dir.map(|s| s.to_string());

    tokio::task::spawn_blocking(move || {
        use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
        use std::sync::mpsc::channel;

        let (tx, rx) = channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
        watcher.watch(std::path::Path::new(&path), RecursiveMode::NonRecursive)?;

        for event in rx {
            match event {
                Ok(_) => {
                    if let Err(e) = cmd_render(
                        &path,
                        output.as_deref(),
                        output_dir.as_deref(),
                        index,
                        theme,
                    ) {
                        eprintln!("Render error: {e}");
                    } else {
                        eprintln!("Re-rendered {path}");
                    }
                }
                Err(e) => eprintln!("Watch error: {e}"),
            }
        }
        Ok::<(), anyhow::Error>(())
    }).await??;

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

fn cmd_add_node(path: &str, id: &str, text: &str, shape: Option<&str>, href: Option<&str>, tooltip: Option<&str>) -> anyhow::Result<()> {
    let mut diagram = read_diagram(path)?;
    let shape = match shape {
        Some(s) => dg::NodeShape::parse(s)
            .ok_or_else(|| anyhow::anyhow!("Invalid shape '{s}'. Use: rect, diamond, stadium, hexagon, cylinder, circle"))?,
        None => dg::NodeShape::Rect,
    };
    diagram
        .add_node(dg::Node {
            id: id.to_string(),
            text: text.to_string(),
            shape,
            href: href.map(|s| s.to_string()),
            tooltip: tooltip.map(|s| s.to_string()),
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
    href: Option<&str>,
    tooltip: Option<&str>,
) -> anyhow::Result<()> {
    let mut diagram = read_diagram(path)?;
    let shape = match shape {
        Some(s) => Some(
            dg::NodeShape::parse(s)
                .ok_or_else(|| anyhow::anyhow!("Invalid shape '{s}'. Use: rect, diamond, stadium, hexagon, cylinder, circle"))?,
        ),
        None => None,
    };
    diagram
        .update_node(id, text, shape)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    if let Some(h) = href
        && let Some(node) = diagram.nodes.iter_mut().find(|n| n.id == id) {
            node.href = Some(h.to_string());
        }
    if let Some(t) = tooltip
        && let Some(node) = diagram.nodes.iter_mut().find(|n| n.id == id) {
            node.tooltip = Some(t.to_string());
        }
    write_diagram(path, &diagram)?;
    println!("Updated node '{id}'");
    Ok(())
}

fn cmd_add_edge(
    path: &str,
    from: &str,
    to: &str,
    label: Option<&str>,
    style: Option<&str>,
) -> anyhow::Result<()> {
    let mut diagram = read_diagram(path)?;
    let style = match style {
        Some(s) => match s.to_lowercase().as_str() {
            "arrow" => dg::EdgeStyle::Arrow,
            "dashed" => dg::EdgeStyle::Dashed,
            "thick" => dg::EdgeStyle::Thick,
            _ => return Err(anyhow::anyhow!("Invalid edge style '{s}'. Use: arrow, dashed, thick")),
        },
        None => dg::EdgeStyle::Arrow,
    };
    diagram
        .add_edge(dg::Edge {
            from: from.to_string(),
            to: to.to_string(),
            label: label.unwrap_or("").to_string(),
            style,
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

fn cmd_get_mermaid(path: &str) -> anyhow::Result<()> {
    let diagram = read_diagram(path)?;
    println!("{}", diagram.to_mermaid());
    Ok(())
}

fn cmd_set_mermaid(path: &str, source: &str) -> anyhow::Result<()> {
    std::fs::write(path, source)
        .map_err(|e| anyhow::anyhow!("Failed to write '{}': {}", path, e))?;
    println!("Wrote mermaid source to '{path}'");
    Ok(())
}

fn cmd_list_nodes(path: &str) -> anyhow::Result<()> {
    let diagram = read_diagram(path)?;
    for n in &diagram.nodes {
        println!("{} [{}] {}", n.id, n.shape, n.text);
    }
    Ok(())
}

fn cmd_list_edges(path: &str) -> anyhow::Result<()> {
    let diagram = read_diagram(path)?;
    for e in &diagram.edges {
        let label = if e.label.is_empty() { String::new() } else { format!(" |{}|", e.label) };
        println!("{} {} {}{}", e.from, e.style.arrow_str(), e.to, label);
    }
    Ok(())
}

fn cmd_validate(path: &str) -> anyhow::Result<()> {
    let diagram = read_diagram(path)?;
    let issues = diagram.validate();
    if issues.is_empty() {
        println!("Valid: no issues found");
    } else {
        println!("Found {} issue(s):", issues.len());
        for issue in &issues {
            println!("  - {issue}");
        }
    }
    Ok(())
}

fn cmd_metrics(path: &str) -> anyhow::Result<()> {
    let doc = crate::ir::load_path(path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let m = crate::analyze::metrics(&doc);
    println!("{}", serde_json::to_string_pretty(&m)?);
    Ok(())
}

fn cmd_create(kind: &str, output: &str) -> anyhow::Result<()> {
    let kind = crate::generate::write_scaffold(kind, output).map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Created {} diagram at {output}", kind);
    Ok(())
}

fn cmd_diff(left: &str, right: &str) -> anyhow::Result<()> {
    let left_doc = crate::ir::load_path(left).map_err(|e| anyhow::anyhow!("{e}"))?;
    let right_doc = crate::ir::load_path(right).map_err(|e| anyhow::anyhow!("{e}"))?;
    let diff = crate::analyze::diff_documents(&left_doc, &right_doc);
    println!("{}", serde_json::to_string_pretty(&diff)?);
    Ok(())
}

fn cmd_merge(left: &str, right: &str, output: &str) -> anyhow::Result<()> {
    let left_diag = read_diagram(left)?;
    let right_diag = read_diagram(right)?;
    let merged = left_diag.merge(&right_diag);
    std::fs::write(output, merged.to_mermaid())?;
    println!("Merged diagram written to {output}");
    Ok(())
}

fn cmd_markdown(
    path: &str,
    output_dir: &str,
    output: Option<&str>,
    format: &str,
    theme: Theme,
) -> anyhow::Result<()> {
    let image_format = crate::markdown::ImageFormat::parse(format)
        .ok_or_else(|| anyhow::anyhow!("Invalid format '{format}'. Use: png or svg"))?;
    let input = std::path::Path::new(path);
    let img_dir = std::path::Path::new(output_dir);
    let opts = crate::markdown::ProcessOptions {
        image_format,
        theme,
        name_prefix: "doc".into(),
    };

    match output {
        Some(out_path) => {
            let result = crate::markdown::process_markdown_file(
                input,
                std::path::Path::new(out_path),
                img_dir,
                &opts,
            )
            .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!(
                "Rendered {} diagram(s) → {}",
                result.blocks_rendered,
                out_path
            );
        }
        None => {
            let source = std::fs::read_to_string(input)?;
            let stem = input
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "doc".into());
            let mut opts = opts;
            opts.name_prefix = stem;
            let out_md = std::path::Path::new("-");
            let result = crate::markdown::process_markdown(&source, out_md, img_dir, &opts)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            print!("{}", result.output_markdown);
            eprintln!("Rendered {} diagram(s) → {}", result.blocks_rendered, output_dir);
        }
    }
    Ok(())
}

async fn cmd_preview(path: &str, port: u16, theme: Theme) -> anyhow::Result<()> {
    // Fail fast if the file cannot be read/parsed.
    crate::preview::render_file(path, theme).map_err(|e| anyhow::anyhow!("{e}"))?;
    crate::preview::serve(path.to_string(), port, theme).await
}

fn cmd_update_edge(
    path: &str,
    from: &str,
    to: &str,
    label: Option<&str>,
    style: Option<&str>,
) -> anyhow::Result<()> {
    let mut diagram = read_diagram(path)?;
    let style = match style {
        Some(s) => match s.to_lowercase().as_str() {
            "arrow" => Some(dg::EdgeStyle::Arrow),
            "dashed" => Some(dg::EdgeStyle::Dashed),
            "thick" => Some(dg::EdgeStyle::Thick),
            _ => return Err(anyhow::anyhow!("Invalid edge style '{s}'. Use: arrow, dashed, thick")),
        },
        None => None,
    };
    diagram
        .update_edge(from, to, label, style)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    write_diagram(path, &diagram)?;
    println!("Updated edge '{from} -> {to}'");
    Ok(())
}

fn cmd_get_node(path: &str, id: &str) -> anyhow::Result<()> {
    let diagram = read_diagram(path)?;
    match diagram.get_node(id) {
        Some(n) => println!("{} [{}] {}", n.id, n.shape, n.text),
        None => println!("Node '{}' not found", id),
    }
    Ok(())
}

fn cmd_get_edge(path: &str, from: &str, to: &str) -> anyhow::Result<()> {
    let diagram = read_diagram(path)?;
    match diagram.edges.iter().find(|e| e.from == from && e.to == to) {
        Some(e) => {
            let label = if e.label.is_empty() { String::new() } else { format!(" |{}|", e.label) };
            println!("{} {} {}{}", e.from, e.style.arrow_str(), e.to, label);
        }
        None => println!("Edge '{from} -> {to}' not found"),
    }
    Ok(())
}
