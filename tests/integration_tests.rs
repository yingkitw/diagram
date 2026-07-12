use std::fs;

fn examples_dir() -> std::path::PathBuf {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("examples");
    path
}

#[test]
fn test_example_simple_flowchart() {
    let path = examples_dir().join("simple-flowchart.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    assert!(diagram.nodes.len() >= 4);
    assert!(diagram.edges.len() >= 4);
}

#[test]
fn test_example_shapes() {
    let path = examples_dir().join("shapes.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    assert_eq!(diagram.nodes.len(), 6);
    assert_eq!(diagram.edges.len(), 5);
}

#[test]
fn test_example_edge_styles() {
    let path = examples_dir().join("edge-styles.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    assert_eq!(diagram.nodes.len(), 5);
    assert_eq!(diagram.edges.len(), 4);
}

#[test]
fn test_example_subgraphs() {
    let path = examples_dir().join("subgraphs.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    assert!(diagram.nodes.len() >= 5);
    assert!(diagram.subgraphs.len() >= 2);
}

#[test]
fn test_example_styling() {
    let path = examples_dir().join("styling.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    assert!(!diagram.styles.is_empty());
    assert!(!diagram.class_defs.is_empty());
    assert!(!diagram.class_applies.is_empty());
}

#[test]
fn test_example_quoted_ids() {
    let path = examples_dir().join("quoted-ids.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    assert_eq!(diagram.nodes.len(), 4);
    assert!(diagram.nodes.iter().any(|n| n.id == "user login"));
    assert!(diagram.nodes.iter().any(|n| n.id == "auth service"));
}

#[test]
fn test_all_examples_roundtrip() {
    for entry in fs::read_dir(examples_dir()).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("mmd") {
            let source = fs::read_to_string(&path).unwrap();
            if diagram::sequence::is_sequence(&source) {
                let diagram = diagram::sequence::parse(&source).unwrap();
                let output = diagram.to_mermaid();
                let reparsed = diagram::sequence::parse(&output).unwrap();
                assert_eq!(
                    diagram.participants.len(),
                    reparsed.participants.len(),
                    "participant count mismatch for {:?}",
                    path
                );
                assert_eq!(
                    diagram.messages.len(),
                    reparsed.messages.len(),
                    "message count mismatch for {:?}",
                    path
                );
            } else if diagram::class::is_class(&source) {
                let diagram = diagram::class::parse(&source).unwrap();
                let output = diagram.to_mermaid();
                let reparsed = diagram::class::parse(&output).unwrap();
                assert_eq!(
                    diagram.classes.len(),
                    reparsed.classes.len(),
                    "class count mismatch for {:?}",
                    path
                );
                assert_eq!(
                    diagram.relations.len(),
                    reparsed.relations.len(),
                    "relation count mismatch for {:?}",
                    path
                );
            } else if diagram::gantt::is_gantt(&source) {
                let diagram = diagram::gantt::parse(&source).unwrap();
                let output = diagram.to_mermaid();
                let reparsed = diagram::gantt::parse(&output).unwrap();
                assert_eq!(
                    diagram.tasks.len(),
                    reparsed.tasks.len(),
                    "task count mismatch for {:?}",
                    path
                );
            } else if diagram::state::is_state(&source) {
                let diagram = diagram::state::parse(&source).unwrap();
                let output = diagram.to_mermaid();
                let reparsed = diagram::state::parse(&output).unwrap();
                assert_eq!(
                    diagram.transitions.len(),
                    reparsed.transitions.len(),
                    "transition count mismatch for {:?}",
                    path
                );
            } else {
                let diagram = diagram::parser::parse(&source).unwrap();
                let output = diagram.to_mermaid();
                let reparsed = diagram::parser::parse(&output).unwrap();
                assert_eq!(diagram.nodes.len(), reparsed.nodes.len(), "node count mismatch for {:?}", path);
                assert_eq!(diagram.edges.len(), reparsed.edges.len(), "edge count mismatch for {:?}", path);
            }
        }
    }
}

#[test]
fn test_example_link_styles() {
    let path = examples_dir().join("link-styles.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    assert_eq!(diagram.edges.len(), 2);
    assert_eq!(diagram.link_styles.len(), 2);
    assert_eq!(diagram.link_styles[0].index, 0);
    assert_eq!(diagram.link_styles[0].properties, "stroke:#ff3,stroke-width:4px");
    assert_eq!(diagram.link_styles[1].index, 1);
}

#[test]
fn test_render_link_styles_example() {
    let path = examples_dir().join("link-styles.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    let layout = diagram::layout::layout(&diagram);
    let svg = diagram::renderer::render_svg(&layout);
    assert!(svg.starts_with("<svg"));
    assert!(svg.ends_with("</svg>"));
    // linkStyle stroke colors should appear in SVG
    assert!(svg.contains("stroke=\"#ff3\""), "expected yellow stroke for edge 0");
    assert!(svg.contains("stroke=\"#f33\""), "expected red stroke for edge 1");
    // linkStyle stroke-width should appear
    assert!(svg.contains("stroke-width=\"4px\""), "expected 4px stroke-width");
    assert!(svg.contains("stroke-width=\"2px\""), "expected 2px stroke-width");
}

#[test]
fn test_render_bezier_curves() {
    let path = examples_dir().join("simple-flowchart.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    let layout = diagram::layout::layout(&diagram);
    let svg = diagram::renderer::render_svg(&layout);
    // Edges should be drawn as cubic bezier curves ("C" command)
    assert!(svg.contains("C "), "expected cubic bezier curves in SVG");
}

#[test]
fn test_render_interactive_svg() {
    let source = "graph TD\n    A[Click me] --> B[End]\n";
    let mut diagram = diagram::parser::parse(source).unwrap();
    // Add href and tooltip to node A
    if let Some(node) = diagram.nodes.iter_mut().find(|n| n.id == "A") {
        node.href = Some("https://example.com".to_string());
        node.tooltip = Some("Go to example".to_string());
    }
    let layout = diagram::layout::layout(&diagram);
    let svg = diagram::renderer::render_svg(&layout);
    assert!(svg.contains("href=\"https://example.com\""), "expected href in SVG");
    assert!(svg.contains("target=\"_blank\""), "expected target=_blank in SVG");
    assert!(svg.contains("<title>Go to example</title>"), "expected tooltip title in SVG");
}

#[test]
fn test_render_light_theme() {
    let path = examples_dir().join("simple-flowchart.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    let layout = diagram::layout::layout(&diagram);
    let svg = diagram::renderer::render_svg_with_theme(&layout, diagram::renderer::Theme::Light);
    // Light theme background should be white
    assert!(svg.contains("fill=\"#ffffff\""), "expected white background for light theme");
    // Light theme node text should be dark
    assert!(svg.contains("fill=\"#1e293b\""), "expected dark text for light theme");
}

#[test]
fn test_diff_and_merge() {
    let left = "graph TD\n    A[Start] --> B[End]\n";
    let right = "graph TD\n    A[Start] --> B[Done]\n    A --> C[New]\n";
    let d_left = diagram::parser::parse(left).unwrap();
    let d_right = diagram::parser::parse(right).unwrap();

    let diff = d_left.diff(&d_right);
    assert_eq!(diff.added_nodes.len(), 1);
    assert_eq!(diff.added_nodes[0].id, "C");
    assert_eq!(diff.modified_nodes.len(), 1);
    assert_eq!(diff.modified_nodes[0].0.id, "B");
    assert_eq!(diff.modified_nodes[0].1.text, "Done");
    assert_eq!(diff.added_edges.len(), 1);
    assert_eq!(diff.added_edges[0].from, "A");
    assert_eq!(diff.added_edges[0].to, "C");

    let merged = d_left.merge(&d_right);
    assert!(merged.nodes.iter().any(|n| n.id == "C"));
    assert!(merged.edges.iter().any(|e| e.from == "A" && e.to == "C"));
    // B should keep left's text since merge doesn't overwrite existing nodes
    assert_eq!(merged.nodes.iter().find(|n| n.id == "B").unwrap().text, "End");
}

#[test]
fn test_render_shapes_example() {
    let path = examples_dir().join("shapes.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    let layout = diagram::layout::layout(&diagram);
    let svg = diagram::renderer::render_svg(&layout);
    assert!(svg.starts_with("<svg"));
    assert!(svg.ends_with("</svg>"));
}

#[test]
fn test_render_styled_example() {
    let path = examples_dir().join("styling.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    assert!(
        diagram.styles.len() >= 3,
        "expected at least 3 inline styles"
    );
    assert!(
        diagram.class_defs.len() >= 2,
        "expected at least 2 classDefs"
    );
    let layout = diagram::layout::layout(&diagram);
    let svg = diagram::renderer::render_svg(&layout);
    assert!(svg.starts_with("<svg"));
    assert!(svg.ends_with("</svg>"));
    // Inline styles should appear in SVG fill attributes
    assert!(svg.contains("fill=\"#bbf\""), "expected style A fill");
    assert!(svg.contains("fill=\"#9f9\""), "expected style C fill (classDef + inline)");
    assert!(svg.contains("fill=\"#f99\""), "expected style D fill (classDef)");
    // Stroke should also be applied
    assert!(svg.contains("stroke=\"#333\""), "expected styled stroke");
}

#[test]
fn test_render_subgraphs_example() {
    let path = examples_dir().join("subgraphs.mmd");
    let source = fs::read_to_string(&path).unwrap();
    let diagram = diagram::parser::parse(&source).unwrap();
    assert!(
        diagram.subgraphs.len() >= 2,
        "expected at least 2 subgraphs"
    );
    let layout = diagram::layout::layout(&diagram);
    let svg = diagram::renderer::render_svg(&layout);
    assert!(svg.starts_with("<svg"));
    assert!(svg.ends_with("</svg>"));
    // Subgraph rectangles should appear in the SVG
    assert!(
        svg.contains("fill-opacity=\"0.6\""),
        "expected semi-transparent subgraph fill"
    );
    assert!(
        svg.contains("stroke-dasharray=\"4,4\""),
        "expected dashed subgraph border"
    );
    // Subgraph labels should appear
    for sg in &diagram.subgraphs {
        assert!(
            svg.contains(&sg.id),
            "expected subgraph label '{}' in SVG",
            sg.id
        );
    }
}

#[tokio::test]
async fn test_preview_server_serves_html_and_svg() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let path = examples_dir().join("simple-flowchart.mmd");
    let path_str = path.to_str().unwrap().to_string();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let serve_path = path_str.clone();
    let handle = tokio::spawn(async move {
        let _ = diagram::preview::serve_with_listener(
            listener,
            serve_path,
            diagram::renderer::Theme::Dark,
        )
        .await;
    });

    async fn http_get(port: u16, path: &str) -> (u16, String) {
        let mut stream = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .expect("connect to preview server");
        let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
        stream.write_all(req.as_bytes()).await.unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).await.unwrap();
        let text = String::from_utf8_lossy(&buf);
        let status = text
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let body = text.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
        (status, body)
    }

    let (status, body) = http_get(port, "/health").await;
    assert_eq!(status, 200);
    assert_eq!(body, "ok");

    let (status, body) = http_get(port, "/").await;
    assert_eq!(status, 200);
    assert!(body.contains("fetch('/svg')"), "expected preview HTML shell");
    assert!(body.contains("simple-flowchart.mmd"));

    let (status, body) = http_get(port, "/svg").await;
    assert_eq!(status, 200);
    assert!(body.contains("<svg"), "expected SVG body, got: {body}");

    let (status, _) = http_get(port, "/missing").await;
    assert_eq!(status, 404);

    handle.abort();
}

#[test]
fn test_preview_render_file() {
    let path = examples_dir().join("simple-flowchart.mmd");
    let svg = diagram::preview::render_file(
        path.to_str().unwrap(),
        diagram::renderer::Theme::Light,
    )
    .unwrap();
    assert!(svg.contains("<svg"));
}

#[test]
fn test_sequence_example_parse_and_render() {
    let path = examples_dir().join("sequence.mmd");
    let source = fs::read_to_string(&path).unwrap();
    assert!(diagram::sequence::is_sequence(&source));
    let diagram = diagram::sequence::parse(&source).unwrap();
    assert_eq!(diagram.participants.len(), 2);
    assert_eq!(diagram.messages.len(), 4);
    let svg = diagram::sequence::render_svg(&diagram, diagram::renderer::Theme::Dark);
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Alice"));
    assert!(svg.contains("Hello Bob"));
    let roundtrip = diagram::sequence::parse(&diagram.to_mermaid()).unwrap();
    assert_eq!(roundtrip.messages.len(), 4);
}

#[test]
fn test_preview_renders_sequence() {
    let path = examples_dir().join("sequence.mmd");
    let svg = diagram::preview::render_file(
        path.to_str().unwrap(),
        diagram::renderer::Theme::Dark,
    )
    .unwrap();
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Alice"));
}

#[test]
fn test_class_example_parse_and_render() {
    let path = examples_dir().join("class.mmd");
    let source = fs::read_to_string(&path).unwrap();
    assert!(diagram::class::is_class(&source));
    let diagram = diagram::class::parse(&source).unwrap();
    assert!(diagram.classes.len() >= 3);
    assert!(diagram.relations.len() >= 2);
    let svg = diagram::class::render_svg(&diagram, diagram::renderer::Theme::Dark);
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Animal"));
    assert!(svg.contains("+String name"));
    let roundtrip = diagram::class::parse(&diagram.to_mermaid()).unwrap();
    assert_eq!(roundtrip.classes.len(), diagram.classes.len());
}

#[test]
fn test_preview_renders_class() {
    let path = examples_dir().join("class.mmd");
    let svg = diagram::preview::render_file(
        path.to_str().unwrap(),
        diagram::renderer::Theme::Light,
    )
    .unwrap();
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Dog"));
}

#[test]
fn test_gantt_example_parse_and_render() {
    let path = examples_dir().join("gantt.mmd");
    let source = fs::read_to_string(&path).unwrap();
    assert!(diagram::gantt::is_gantt(&source));
    let diagram = diagram::gantt::parse(&source).unwrap();
    assert!(diagram.tasks.len() >= 3);
    let svg = diagram::gantt::render_svg(&diagram, diagram::renderer::Theme::Dark);
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Research"));
    assert!(svg.contains("Project Plan"));
}

#[test]
fn test_plantuml_example_parse_and_render() {
    let path = examples_dir().join("sequence.puml");
    let doc = diagram::ir::load_path(path.to_str().unwrap()).unwrap();
    assert_eq!(doc.primary().unwrap().kind(), diagram::ir::Kind::Sequence);
    let svg = diagram::preview::render_file(path.to_str().unwrap(), diagram::renderer::Theme::Dark)
        .unwrap();
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Alice"));
}

#[test]
fn test_plantuml_class_example_parse_and_render() {
    let path = examples_dir().join("class.puml");
    let doc = diagram::ir::load_path(path.to_str().unwrap()).unwrap();
    assert_eq!(doc.primary().unwrap().kind(), diagram::ir::Kind::Class);
    let svg = diagram::preview::render_file(path.to_str().unwrap(), diagram::renderer::Theme::Dark)
        .unwrap();
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Animal"));
    assert!(svg.contains("Dog"));
}

#[test]
fn test_plantuml_activity_example_parse_and_render() {
    let path = examples_dir().join("activity.puml");
    let doc = diagram::ir::load_path(path.to_str().unwrap()).unwrap();
    assert_eq!(doc.primary().unwrap().kind(), diagram::ir::Kind::Flowchart);
    let svg = diagram::preview::render_file(path.to_str().unwrap(), diagram::renderer::Theme::Dark)
        .unwrap();
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Receive request"));
}

#[test]
fn test_dot_example_parse_and_render() {
    let path = examples_dir().join("simple-flowchart.dot");
    let doc = diagram::ir::load_path(path.to_str().unwrap()).unwrap();
    let diagram = match doc.primary().unwrap() {
        diagram::ir::Diagram::Flowchart(d) => d,
        _ => panic!("expected flowchart"),
    };
    assert!(diagram.nodes.len() >= 5);
    assert!(diagram.edges.len() >= 5);
    let svg = diagram::preview::render_file(path.to_str().unwrap(), diagram::renderer::Theme::Dark)
        .unwrap();
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Start"));
}

#[test]
fn test_state_example_parse_and_render() {
    let path = examples_dir().join("state.mmd");
    let source = fs::read_to_string(&path).unwrap();
    assert!(diagram::state::is_state(&source));
    let diagram = diagram::state::parse(&source).unwrap();
    assert!(diagram.transitions.len() >= 5);
    let svg = diagram::state::render_svg(&diagram, diagram::renderer::Theme::Dark);
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Still"));
}

#[test]
fn test_d2_example_parse_and_render() {
    let path = examples_dir().join("simple-flowchart.d2");
    let doc = diagram::ir::load_path(path.to_str().unwrap()).unwrap();
    let diagram = match doc.primary().unwrap() {
        diagram::ir::Diagram::Flowchart(d) => d,
        _ => panic!("expected flowchart"),
    };
    assert!(diagram.nodes.len() >= 5);
    assert!(diagram.edges.len() >= 5);
    let svg = diagram::preview::render_file(path.to_str().unwrap(), diagram::renderer::Theme::Dark)
        .unwrap();
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Start"));
}

#[test]
fn test_multi_document_composite_render() {
    let path = examples_dir().join("multi-document.json");
    let doc = diagram::ir::load_path(path.to_str().unwrap()).unwrap();
    assert_eq!(doc.diagrams.len(), 2);
    let svg = doc.render_svg(diagram::renderer::Theme::Dark).unwrap();
    assert!(svg.matches("<svg").count() >= 1);
    let mmd = doc.to_mermaid().unwrap();
    let doc2 = diagram::formats::import_str(&mmd, diagram::formats::Format::Mermaid).unwrap();
    assert_eq!(doc2.diagrams.len(), 2);
}

#[test]
fn test_render_flowchart_pdf_file() {
    let path = examples_dir().join("simple-flowchart.mmd");
    let out = std::env::temp_dir().join(format!("diagram_int_pdf_{}.pdf", std::process::id()));
    diagram::preview::write_render_output(
        out.to_str().unwrap(),
        path.to_str().unwrap(),
        diagram::renderer::Theme::Dark,
    )
    .unwrap();
    let bytes = std::fs::read(&out).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    let _ = std::fs::remove_file(&out);
}

#[test]
fn test_render_flowchart_png_file() {
    let path = examples_dir().join("simple-flowchart.mmd");
    let out = std::env::temp_dir().join(format!("diagram_int_png_{}.png", std::process::id()));
    diagram::preview::write_render_output(
        out.to_str().unwrap(),
        path.to_str().unwrap(),
        diagram::renderer::Theme::Dark,
    )
    .unwrap();
    let bytes = std::fs::read(&out).unwrap();
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert!(bytes.len() > 500);
    let _ = std::fs::remove_file(&out);
}

#[test]
fn test_render_json_ir_file() {
    let mmd_path = examples_dir().join("simple-flowchart.mmd");
    let doc = diagram::ir::load_path(mmd_path.to_str().unwrap()).unwrap();
    let json_path = std::env::temp_dir().join(format!("diagram_render_ir_{}.json", std::process::id()));
    std::fs::write(&json_path, doc.to_json().unwrap()).unwrap();
    let svg = diagram::preview::render_file(json_path.to_str().unwrap(), diagram::renderer::Theme::Dark)
        .unwrap();
    assert!(svg.contains("<svg"));
    let _ = std::fs::remove_file(json_path);
}

#[test]
fn test_preview_renders_gantt() {
    let path = examples_dir().join("gantt.mmd");
    let svg = diagram::preview::render_file(
        path.to_str().unwrap(),
        diagram::renderer::Theme::Dark,
    )
    .unwrap();
    assert!(svg.contains("<svg"));
    assert!(svg.contains("Implement"));
}
