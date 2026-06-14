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
            let diagram = diagram::parser::parse(&source).unwrap();
            let output = diagram.to_mermaid();
            let reparsed = diagram::parser::parse(&output).unwrap();
            assert_eq!(diagram.nodes.len(), reparsed.nodes.len(), "node count mismatch for {:?}", path);
            assert_eq!(diagram.edges.len(), reparsed.edges.len(), "edge count mismatch for {:?}", path);
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
